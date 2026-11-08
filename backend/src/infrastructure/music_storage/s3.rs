//! Saves and scans audio objects directly using a release-based S3 key layout.

use aws_sdk_s3::{
    Client,
    error::SdkError,
    operation::{
        get_object::GetObjectError, list_objects_v2::ListObjectsV2Error, put_object::PutObjectError,
    },
    primitives::{ByteStream, ByteStreamError},
};
use futures_util::{
    StreamExt,
    stream::{self, BoxStream},
};
use std::collections::BTreeMap;
use tokio::task::spawn_blocking;
use url::Url;
use uuid::Uuid;

use crate::{
    domain::{
        id::{AudioFileId, ParseAudioFileIdError, ReleaseId},
        import::ImportAudio,
        ingestion::{build_release, ingest_track},
        primitive::{audio_file::MediaType, release::Release},
    },
    port::music_storage::{
        AudioScanFailure, DeleteStagingError, MusicStorage, ReleaseScanFailure, SaveReleaseError,
        SaveReleaseOutcome, ScanReleasesError,
    },
};

/// Stores and scans release audio through the asynchronous S3 SDK.
pub(crate) struct S3MusicStorage {
    client: Client,
    bucket: String,
    prefix: String,
    playback_url_base: Option<Url>,
}

/// A failure to check or save a release in S3.
#[derive(Debug, thiserror::Error)]
enum S3SaveReleaseError {
    /// Existing release objects could not be enumerated.
    #[error("checking existing release objects: {0}")]
    List(#[source] Box<SdkError<ListObjectsV2Error>>),
    /// An audio object could not be uploaded.
    #[error("uploading {object_key}: {source}")]
    Put {
        object_key: String,
        #[source]
        source: Box<SdkError<PutObjectError>>,
    },
}

/// A failure to obtain source audio bytes and their stored identity.
#[derive(Debug, thiserror::Error)]
enum S3ReadAudioError {
    /// The object filename does not contain an audio identity.
    #[error("invalid audio filename")]
    InvalidFileName,
    /// The filename does not preserve a valid audio UUID.
    #[error(transparent)]
    InvalidAudioId(#[from] ParseAudioFileIdError),
    /// Reading stored releases requires a delivery endpoint.
    #[error("playback URL base is required")]
    MissingPlaybackUrlBase,
    /// Playback links cannot be built from the delivery endpoint.
    #[error("invalid playback URL base")]
    InvalidPlaybackUrlBase,
    /// S3 could not return the requested object.
    #[error("getting audio object: {0}")]
    Get(#[source] Box<SdkError<GetObjectError>>),
    /// The response body could not be read completely.
    #[error("reading audio body: {0}")]
    Body(#[from] ByteStreamError),
}

impl S3MusicStorage {
    /// Connects storage to its bucket namespace and optional delivery endpoint.
    pub(crate) fn new(
        client: Client,
        bucket: String,
        prefix: String,
        playback_url_base: Option<Url>,
    ) -> Self {
        Self {
            client,
            bucket,
            prefix,
            playback_url_base,
        }
    }

    /// Addresses a storage-relative object within the configured namespace.
    fn key(&self, relative: &str) -> String {
        if self.prefix.is_empty() {
            relative.into()
        } else {
            format!("{}/{relative}", self.prefix)
        }
    }

    /// Checks for any existing release objects without uploading content.
    async fn check_destination(&self, id: &ReleaseId) -> Result<bool, S3SaveReleaseError> {
        let prefix = self.key(&format!("{}/", id));
        let existing = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .max_keys(1)
            .send()
            .await
            .map_err(|source| S3SaveReleaseError::List(Box::new(source)))?;
        Ok(!existing.contents().is_empty())
    }

    /// Enumerates temporary objects completely before copying or deleting them.
    async fn staging_keys(
        &self,
        id: Option<&ReleaseId>,
    ) -> Result<Vec<String>, S3SaveReleaseError> {
        let relative = id.map_or_else(
            || ".tonhalle-import/".to_owned(),
            |id| format!(".tonhalle-import/{id}/"),
        );
        let mut pages = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(self.key(&relative))
            .into_paginator()
            .send();
        let mut keys = Vec::new();
        while let Some(page) = pages.next().await {
            let page = page.map_err(|source| S3SaveReleaseError::List(Box::new(source)))?;
            keys.extend(
                page.contents()
                    .iter()
                    .filter_map(|object| object.key().map(str::to_owned)),
            );
        }
        Ok(keys)
    }

    /// Encodes a copy source as a bucket and key path, including reserved key characters.
    fn copy_source(&self, key: &str) -> String {
        let mut url = Url::parse("https://s3.invalid/").unwrap();
        url.path_segments_mut()
            .unwrap()
            .clear()
            .push(&self.bucket)
            .extend(key.split('/'));
        url.path().trim_start_matches('/').to_owned()
    }

    /// Preserves the release identity and the underlying SDK failure.
    fn save_error(
        id: &ReleaseId,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> SaveReleaseError {
        SaveReleaseError::Internal {
            id: id.clone(),
            source: Box::new(source),
        }
    }

    async fn read(&self, key: &str) -> Result<Vec<u8>, S3ReadAudioError> {
        Ok(self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|source| S3ReadAudioError::Get(Box::new(source)))?
            .body
            .collect()
            .await?
            .into_bytes()
            .to_vec())
    }

    /// Reads all enumerated audio sources before constructing a release.
    async fn scan_release(
        &self,
        id: ReleaseId,
        keys: Vec<String>,
    ) -> Result<Release, ReleaseScanFailure> {
        // Attempt every audio object so one failure does not hide other failures in the release.
        let mut tracks = Vec::new();
        let mut failures = Vec::new();
        for key in keys {
            // Recover the stored identity and escaped playback URL before downloading the source bytes.
            let source = async {
                let filename = key.rsplit('/').next().unwrap_or_default();
                let (stem, _) = filename
                    .rsplit_once('.')
                    .ok_or(S3ReadAudioError::InvalidFileName)?;
                let audio_id: AudioFileId = stem.parse()?;
                let mut url = self
                    .playback_url_base
                    .clone()
                    .ok_or(S3ReadAudioError::MissingPlaybackUrlBase)?;
                url.path_segments_mut()
                    .map_err(|_| S3ReadAudioError::InvalidPlaybackUrlBase)?
                    .pop_if_empty()
                    .extend(key.split('/'));
                Ok::<_, S3ReadAudioError>((audio_id, url, self.read(&key).await?))
            }
            .await;

            // Parse downloaded audio on a blocking worker while keeping read and parse failures distinct.
            let result = match source {
                Err(source) => Err(AudioScanFailure::Read {
                    object_key: key.clone(),
                    source: source.into(),
                }),
                Ok((audio_id, url, bytes)) => {
                    let id = id.clone();
                    let object_key = key.clone();
                    spawn_blocking(move || {
                        ingest_track(&bytes, audio_id, object_key.clone(), url, &id)
                            .map_err(|source| AudioScanFailure::Parse { object_key, source })
                    })
                    .await
                    .map_err(|source| AudioScanFailure::Read {
                        object_key: key,
                        source: Box::new(source),
                    })
                    .and_then(|result| result)
                }
            };
            match result {
                Ok(track) => tracks.push(track),
                Err(error) => failures.push(error),
            }
        }

        // Reject the release after all sources have been attempted if any audio failed.
        if !failures.is_empty() {
            return Err(ReleaseScanFailure::Audio { id, failures });
        }

        // Validate consistency across the accepted tracks before returning the domain release.
        let release_id = id.clone();
        spawn_blocking(move || build_release(release_id, tracks))
            .await
            .map_err(|source| ReleaseScanFailure::Audio {
                id: id.clone(),
                failures: vec![AudioScanFailure::Read {
                    object_key: self.key(&format!("{id}/")),
                    source: Box::new(source),
                }],
            })?
            .map_err(|source| ReleaseScanFailure::InvalidRelease { id, source })
    }
}

#[async_trait::async_trait]
impl MusicStorage for S3MusicStorage {
    async fn stage_audio(&self, audio: ImportAudio) -> Result<(), SaveReleaseError> {
        let id = audio.release_id().clone();
        let media = audio.properties().media_type();
        let key = self.key(&format!(
            ".tonhalle-import/{id}/{}.{}",
            Uuid::new_v4(),
            media.extension()
        ));
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type(media.as_str())
            .body(ByteStream::from(audio.into_bytes()))
            .send()
            .await
            .map_err(|source| {
                Self::save_error(
                    &id,
                    S3SaveReleaseError::Put {
                        object_key: key,
                        source: Box::new(source),
                    },
                )
            })?;
        Ok(())
    }

    async fn commit_release(
        &self,
        id: &ReleaseId,
        dry_run: bool,
    ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
        if self
            .check_destination(id)
            .await
            .map_err(|source| Self::save_error(id, source))?
        {
            return Ok(SaveReleaseOutcome::AlreadyExists);
        }
        if dry_run {
            return Ok(SaveReleaseOutcome::Saved);
        }
        let keys = self
            .staging_keys(Some(id))
            .await
            .map_err(|source| Self::save_error(id, source))?;
        if keys.is_empty() {
            return Err(Self::save_error(
                id,
                std::io::Error::other("no staged audio"),
            ));
        }
        for key in keys {
            let filename = key.rsplit('/').next().unwrap();
            self.client
                .copy_object()
                .bucket(&self.bucket)
                .key(self.key(&format!("{id}/{filename}")))
                .copy_source(self.copy_source(&key))
                .send()
                .await
                .map_err(|source| Self::save_error(id, source))?;
        }
        Ok(SaveReleaseOutcome::Saved)
    }

    async fn delete_staging(&self, id: Option<&ReleaseId>) -> Result<(), DeleteStagingError> {
        let keys = self
            .staging_keys(id)
            .await
            .map_err(DeleteStagingError::new)?;
        for chunk in keys.chunks(1000) {
            let objects = chunk
                .iter()
                .map(|key| {
                    aws_sdk_s3::types::ObjectIdentifier::builder()
                        .key(key)
                        .build()
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(DeleteStagingError::new)?;
            let delete = aws_sdk_s3::types::Delete::builder()
                .set_objects(Some(objects))
                .build()
                .map_err(DeleteStagingError::new)?;
            let result = self
                .client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await
                .map_err(DeleteStagingError::new)?;
            if !result.errors().is_empty() {
                return Err(DeleteStagingError::new(std::io::Error::other(format!(
                    "S3 staging deletion failed: {:?}",
                    result.errors()
                ))));
            }
        }
        Ok(())
    }

    fn scan_releases(
        &self,
    ) -> BoxStream<'_, Result<Result<Release, ReleaseScanFailure>, ScanReleasesError>> {
        stream::try_unfold(None, move |state| async move {
            let mut groups = match state {
                Some(groups) => groups,
                None => {
                    // Enumerate every page before scanning releases, since one release may span multiple pages.
                    let prefix = self.key("");
                    let mut pages = self
                        .client
                        .list_objects_v2()
                        .bucket(&self.bucket)
                        .prefix(&prefix)
                        .into_paginator()
                        .send();
                    let mut groups = BTreeMap::<String, Vec<String>>::new();
                    while let Some(page) = pages.next().await {
                        let page = page.map_err(ScanReleasesError::new)?;
                        // Group only supported audio at the expected release/filename depth within the configured prefix.
                        for object in page.contents() {
                            let Some(key) = object.key() else { continue };
                            let Some((release, filename)) = key
                                .strip_prefix(&prefix)
                                .and_then(|key| key.split_once('/'))
                            else {
                                continue;
                            };
                            if filename.contains('/')
                                || filename
                                    .rsplit_once('.')
                                    .and_then(|(_, extension)| MediaType::from_extension(extension))
                                    .is_none()
                            {
                                continue;
                            }
                            groups.entry(release.into()).or_default().push(key.into());
                        }
                    }

                    groups.into_iter()
                }
            };
            let Some((stem, keys)) = groups.next() else {
                return Ok(None);
            };
            let result = match stem.parse() {
                Ok(id) => self.scan_release(id, keys).await,
                Err(source) => Err(ReleaseScanFailure::InvalidId {
                    object_key: self.key(&stem),
                    source,
                }),
            };
            Ok(Some((result, Some(groups))))
        })
        .boxed()
    }
}

/// Verifies direct audio persistence and scanning without live AWS services.
#[cfg(test)]
mod tests {
    use aws_sdk_s3::{
        operation::{
            copy_object::{CopyObjectError, CopyObjectOutput},
            delete_objects::DeleteObjectsOutput,
            get_object::GetObjectOutput,
            list_objects_v2::{ListObjectsV2Error, ListObjectsV2Output},
            put_object::PutObjectOutput,
        },
        types::Object,
    };
    use aws_smithy_mocks::{MockResponse, RuleMode, mock, mock_client};
    use futures_util::TryStreamExt;
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::test_support::{sample_import_audio, validate_import_release};
    use crate::{domain::ingestion::metadata::sample_metadata, test_support::sample_audio};

    /// Constructs an adapter whose delivery endpoint maps the bucket root.
    fn storage(client: Client) -> S3MusicStorage {
        S3MusicStorage::new(
            client,
            "music".into(),
            "space #".into(),
            Some("https://example.com/music/".parse().unwrap()),
        )
    }

    /// Lists only objects within the requested namespace, as S3 does.
    fn listing(objects: &BTreeMap<String, Vec<u8>>, prefix: &str) -> ListObjectsV2Output {
        let mut output = ListObjectsV2Output::builder().is_truncated(false);
        for key in objects.keys().filter(|key| key.starts_with(prefix)) {
            output = output.contents(Object::builder().key(key).build());
        }
        output.build()
    }

    /// Simulates stored objects while allowing a publication failure after one copy.
    fn object_client(objects: Arc<Mutex<BTreeMap<String, Vec<u8>>>>, fail_copy: bool) -> Client {
        let state = objects.clone();
        let list = mock!(Client::list_objects_v2).then_compute_output(move |request| {
            listing(&state.lock().unwrap(), request.prefix().unwrap())
        });
        let state = objects.clone();
        let put = mock!(Client::put_object)
            .match_requests(|request| request.content_type() == Some("audio/wav"))
            .then_compute_output(move |request| {
                state.lock().unwrap().insert(
                    request.key().unwrap().into(),
                    request.body().bytes().unwrap().to_vec(),
                );
                PutObjectOutput::builder().build()
            });
        let state = objects.clone();
        let copy = mock!(Client::copy_object).then_compute_response(move |request| {
            let mut objects = state.lock().unwrap();
            if fail_copy && objects.keys().any(|key| !key.contains(".tonhalle-import/")) {
                return MockResponse::Error(CopyObjectError::ObjectNotInActiveTierError(
                    aws_sdk_s3::types::error::ObjectNotInActiveTierError::builder().build(),
                ));
            }
            let source = objects
                .keys()
                .find(|key| {
                    let mut url = Url::parse("https://s3.invalid/").unwrap();
                    url.path_segments_mut()
                        .unwrap()
                        .clear()
                        .push("music")
                        .extend(key.split('/'));
                    Some(url.path().trim_start_matches('/')) == request.copy_source()
                })
                .unwrap()
                .clone();
            let bytes = objects[&source].clone();
            objects.insert(request.key().unwrap().into(), bytes);
            MockResponse::Output(CopyObjectOutput::builder().build())
        });
        let state = objects.clone();
        let delete = mock!(Client::delete_objects).then_compute_output(move |request| {
            let mut objects = state.lock().unwrap();
            for object in request.delete().unwrap().objects() {
                objects.remove(object.key());
            }
            DeleteObjectsOutput::builder().build()
        });
        let get = mock!(Client::get_object).then_compute_output(move |request| {
            GetObjectOutput::builder()
                .body(ByteStream::from(
                    objects.lock().unwrap()[request.key().unwrap()].clone(),
                ))
                .build()
        });
        mock_client!(
            aws_sdk_s3,
            RuleMode::MatchAny,
            [&list, &put, &copy, &delete, &get]
        )
    }

    #[tokio::test]
    async fn commit_release_preserves_audio_and_skips_existing_release_while_cleanup_removes_only_staging()
     {
        for configured_prefix in ["", "space #%/音"] {
            let objects = Arc::new(Mutex::new(BTreeMap::new()));
            let mut storage = storage(object_client(objects.clone(), false));
            storage.prefix = configured_prefix.into();
            let mut second = sample_metadata();
            *second.track_number_mut() = "2".into();
            let bytes = vec![sample_audio(&sample_metadata()), sample_audio(&second)];
            let release = validate_import_release(bytes.clone()).unwrap();
            for bytes in &bytes {
                storage
                    .stage_audio(sample_import_audio(bytes.clone()))
                    .await
                    .unwrap();
            }
            assert_eq!(objects.lock().unwrap().len(), 2);
            assert!(
                storage
                    .scan_releases()
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                storage.commit_release(release.id(), false).await.unwrap(),
                SaveReleaseOutcome::Saved
            );
            assert_eq!(objects.lock().unwrap().len(), 4);
            storage.delete_staging(Some(release.id())).await.unwrap();
            storage.delete_staging(None).await.unwrap();
            let snapshot = objects.lock().unwrap().clone();
            assert_eq!(snapshot.len(), 2);
            let prefix = storage.key(&format!("{}/", release.id()));
            for (key, value) in &snapshot {
                key.strip_prefix(&prefix)
                    .unwrap()
                    .strip_suffix(".wav")
                    .unwrap()
                    .parse::<AudioFileId>()
                    .unwrap();
                assert!(bytes.contains(value));
            }
            assert_eq!(
                storage.commit_release(release.id(), false).await.unwrap(),
                SaveReleaseOutcome::AlreadyExists
            );
            assert_eq!(*objects.lock().unwrap(), snapshot);
            let scanned = storage
                .scan_releases()
                .try_collect::<Vec<_>>()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .unwrap();
            assert_eq!(scanned.id(), release.id());
            assert_eq!(scanned.tracks().len(), 2);
            for track in scanned.tracks() {
                assert!(snapshot.contains_key(track.original_audio_file().object_key()));
            }
        }
    }

    #[tokio::test]
    async fn commit_release_leaves_partial_audio_on_copy_failure_and_cleanup_preserves_it() {
        let objects = Arc::new(Mutex::new(BTreeMap::new()));
        let storage = storage(object_client(objects.clone(), true));
        let mut second = sample_metadata();
        *second.track_number_mut() = "2".into();
        let bytes = vec![sample_audio(&sample_metadata()), sample_audio(&second)];
        let release = validate_import_release(bytes.clone()).unwrap();
        for bytes in bytes {
            storage
                .stage_audio(sample_import_audio(bytes))
                .await
                .unwrap();
        }
        assert!(storage.commit_release(release.id(), false).await.is_err());
        assert_eq!(objects.lock().unwrap().len(), 3);
        storage.delete_staging(None).await.unwrap();
        assert_eq!(objects.lock().unwrap().len(), 1);
        let scanned = storage
            .scan_releases()
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
            .pop()
            .unwrap()
            .unwrap();
        assert_eq!(scanned.tracks().len(), 1);
        assert_eq!(
            storage.commit_release(release.id(), false).await.unwrap(),
            SaveReleaseOutcome::AlreadyExists
        );
    }

    #[tokio::test]
    async fn delete_staging_paginates_and_batches_deletes_and_reports_object_errors() {
        for fail_delete in [false, true] {
            let list = mock!(Client::list_objects_v2).then_compute_output(|request| {
                let mut output = ListObjectsV2Output::builder();
                if request.continuation_token().is_none() {
                    output = output.is_truncated(true).next_continuation_token("next");
                    for i in 0..1000 {
                        output = output.contents(
                            Object::builder()
                                .key(format!("space #/.tonhalle-import/release-a/{i}.wav"))
                                .build(),
                        );
                    }
                } else {
                    output = output.is_truncated(false).contents(
                        Object::builder()
                            .key("space #/.tonhalle-import/release-b/last.wav")
                            .build(),
                    );
                }
                output.build()
            });
            let deleted = Arc::new(Mutex::new(Vec::new()));
            let state = deleted.clone();
            let delete = mock!(Client::delete_objects).then_compute_output(move |request| {
                let objects = request.delete().unwrap().objects();
                assert!(
                    objects
                        .iter()
                        .all(|object| object.key().starts_with("space #/.tonhalle-import/"))
                );
                state.lock().unwrap().push(objects.len());
                let mut output = DeleteObjectsOutput::builder();
                if fail_delete {
                    output = output.errors(
                        aws_sdk_s3::types::Error::builder()
                            .key(objects[0].key())
                            .code("AccessDenied")
                            .build(),
                    );
                }
                output.build()
            });
            let client = mock_client!(aws_sdk_s3, RuleMode::MatchAny, [&list, &delete]);
            let result = storage(client).delete_staging(None).await;
            if fail_delete {
                assert!(result.unwrap_err().to_string().contains("AccessDenied"));
            } else {
                result.unwrap();
                assert_eq!(*deleted.lock().unwrap(), vec![1000, 1]);
            }
        }
    }

    #[tokio::test]
    async fn scan_releases_groups_audio_across_pages_and_reports_invalid_release_ids() {
        let release = validate_import_release(vec![sample_audio(&sample_metadata())]).unwrap();
        let id = release.id().clone();
        let first_key = format!("space #/{id}/22222222-2222-4222-8222-222222222222.wav");
        let second_key = format!("space #/{id}/33333333-3333-4333-8333-333333333333.wav");
        let first = mock!(Client::list_objects_v2)
            .match_requests(|request| {
                request.prefix() == Some("space #/") && request.continuation_token().is_none()
            })
            .then_compute_output(move |_| {
                ListObjectsV2Output::builder()
                    .is_truncated(true)
                    .next_continuation_token("page2")
                    .contents(Object::builder().key(&first_key).build())
                    .build()
            });
        let second = mock!(Client::list_objects_v2)
            .match_requests(|request| request.continuation_token() == Some("page2"))
            .then_compute_output(move |_| {
                ListObjectsV2Output::builder()
                    .is_truncated(false)
                    .contents(Object::builder().key(&second_key).build())
                    .contents(Object::builder().key("space #/invalid/audio.wav").build())
                    .contents(Object::builder().key("space #/README.txt").build())
                    .build()
            });
        let get = mock!(Client::get_object).then_output(|| {
            GetObjectOutput::builder()
                .body(ByteStream::from_static(b"corrupt"))
                .build()
        });
        let client = mock_client!(aws_sdk_s3, RuleMode::MatchAny, [&first, &second, &get]);
        let storage = storage(client);
        let results = storage
            .scan_releases()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .any(|result| matches!(result, Err(ReleaseScanFailure::InvalidId { .. })))
        );
        assert!(results.iter().any(|result| matches!(result, Err(ReleaseScanFailure::Audio { id: failed, failures }) if failed == &id && failures.len() == 2)));
        assert_eq!(get.num_calls(), 2);
        assert_eq!(first.num_calls(), 1);
        assert_eq!(second.num_calls(), 1);
    }

    #[tokio::test]
    async fn scan_releases_returns_error_if_a_later_listing_page_fails() {
        let first = mock!(Client::list_objects_v2)
            .match_requests(|request| request.continuation_token().is_none())
            .then_output(|| {
                ListObjectsV2Output::builder()
                    .is_truncated(true)
                    .next_continuation_token("page2")
                    .build()
            });
        let second = mock!(Client::list_objects_v2)
            .match_requests(|request| request.continuation_token() == Some("page2"))
            .then_error(|| {
                ListObjectsV2Error::generic(
                    aws_sdk_s3::error::ErrorMetadata::builder()
                        .code("AccessDenied")
                        .build(),
                )
            });
        let client = mock_client!(aws_sdk_s3, RuleMode::MatchAny, [&first, &second]);
        let storage = storage(client);
        assert!(
            storage
                .scan_releases()
                .try_collect::<Vec<_>>()
                .await
                .is_err()
        );
    }
}
