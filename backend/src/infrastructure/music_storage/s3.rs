//! Saves and scans audio objects directly using a release-based S3 key layout.

use aws_sdk_s3::{
    Client,
    error::SdkError,
    operation::{
        get_object::GetObjectError, list_objects_v2::ListObjectsV2Error, put_object::PutObjectError,
    },
    primitives::{ByteStream, ByteStreamError},
};
use std::collections::BTreeMap;
use tokio::task::spawn_blocking;
use url::Url;
use uuid::Uuid;

use crate::{
    domain::{
        id::{AudioFileId, ParseAudioFileIdError, ReleaseId},
        import::ImportRelease,
        ingestion::{build_release, ingest_track},
        primitive::{audio_file::MediaType, release::Release},
    },
    port::music_storage::{
        AudioScanFailure, MusicStorage, ReleaseScanFailure, SaveReleaseError, SaveReleaseOutcome,
        ScanReleasesError,
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

    /// Saves audio directly after checking for existing objects in the release.
    ///
    /// This is best effort: concurrent writes are not serialized, and failed saves may leave a partial release that later imports skip.
    async fn save(
        &self,
        release: &ImportRelease,
    ) -> Result<SaveReleaseOutcome, S3SaveReleaseError> {
        // Retain any existing release, including partial audio left by a previous failed save.
        let prefix = self.key(&format!("{}/", release.id()));
        let existing = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .max_keys(1)
            .send()
            .await
            .map_err(|source| S3SaveReleaseError::List(Box::new(source)))?;
        if !existing.contents().is_empty() {
            return Ok(SaveReleaseOutcome::AlreadyExists);
        }

        // Upload each original independently with a new audio identity and its delivery content type.
        for audio in release.audio() {
            let media = audio.properties().media_type();
            let key = format!("{prefix}{}.{}", Uuid::new_v4(), media.extension());
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(&key)
                .content_type(media.as_str())
                .body(ByteStream::from(audio.bytes().clone()))
                .send()
                .await
                .map_err(|source| S3SaveReleaseError::Put {
                    object_key: key,
                    source: Box::new(source),
                })?;
        }
        Ok(SaveReleaseOutcome::Saved)
    }

    /// Reads an object completely while preserving SDK and body failures.
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
    async fn save_release(
        &self,
        release: &ImportRelease,
    ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
        self.save(release)
            .await
            .map_err(|source| SaveReleaseError::Internal {
                id: release.id().clone(),
                source: source.into(),
            })
    }

    async fn scan_releases(
        &self,
    ) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
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

        // Resolve each stored release identity and retain per-release failures alongside successful scans.
        let mut releases = Vec::new();
        for (stem, keys) in groups {
            match stem.parse() {
                Ok(id) => releases.push(self.scan_release(id, keys).await),
                Err(source) => releases.push(Err(ReleaseScanFailure::InvalidId {
                    object_key: self.key(&stem),
                    source,
                })),
            }
        }
        Ok(releases)
    }
}

/// Verifies direct audio persistence and scanning without live AWS services.
#[cfg(test)]
mod tests {
    use aws_sdk_s3::{
        operation::{
            get_object::GetObjectOutput,
            list_objects_v2::{ListObjectsV2Error, ListObjectsV2Output},
            put_object::{PutObjectError, PutObjectOutput},
        },
        types::Object,
    };
    use aws_smithy_mocks::{MockResponse, RuleMode, mock, mock_client};
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use super::*;
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

    #[tokio::test]
    async fn save_release_round_trips_audio_without_management_objects_and_skips_existing_release()
    {
        for (configured_prefix, object_prefix, url_prefix) in [
            ("", "", "https://example.com/music/"),
            (
                "space #",
                "space #/",
                "https://example.com/music/space%20%23/",
            ),
        ] {
            let objects = Arc::new(Mutex::new(BTreeMap::<String, Vec<u8>>::new()));
            let state = objects.clone();
            let list = mock!(Client::list_objects_v2)
                .match_requests(|request| request.bucket() == Some("music"))
                .then_compute_output(move |request| {
                    listing(&state.lock().unwrap(), request.prefix().unwrap())
                });
            let state = objects.clone();
            let put = mock!(Client::put_object)
                .match_requests(|request| {
                    request.bucket() == Some("music")
                        && request.if_none_match().is_none()
                        && request.content_type() == Some("audio/wav")
                })
                .then_compute_output(move |request| {
                    state.lock().unwrap().insert(
                        request.key().unwrap().into(),
                        request.body().bytes().unwrap().to_vec(),
                    );
                    PutObjectOutput::builder().build()
                });
            let state = objects.clone();
            let get = mock!(Client::get_object).then_compute_output(move |request| {
                GetObjectOutput::builder()
                    .body(ByteStream::from(
                        state.lock().unwrap()[request.key().unwrap()].clone(),
                    ))
                    .build()
            });
            let client = mock_client!(aws_sdk_s3, RuleMode::MatchAny, [&list, &put, &get]);
            let mut storage = storage(client);
            storage.prefix = configured_prefix.into();
            let mut second = sample_metadata();
            *second.track_number_mut() = "2".into();
            let bytes = vec![sample_audio(&sample_metadata()), sample_audio(&second)];
            let release = ImportRelease::new(bytes.clone()).unwrap();
            assert_eq!(
                storage.save_release(&release).await.unwrap(),
                SaveReleaseOutcome::Saved
            );
            let snapshot = objects.lock().unwrap().clone();
            assert_eq!(snapshot.len(), 2);
            let prefix = format!("{object_prefix}{}/", release.id());
            for (key, value) in &snapshot {
                let filename = key.strip_prefix(&prefix).unwrap();
                assert!(!filename.contains('/'));
                filename
                    .strip_suffix(".wav")
                    .unwrap()
                    .parse::<AudioFileId>()
                    .unwrap();
                assert!(bytes.contains(value));
            }
            assert_eq!(
                storage.save_release(&release).await.unwrap(),
                SaveReleaseOutcome::AlreadyExists
            );
            assert_eq!(*objects.lock().unwrap(), snapshot);
            let scanned = storage
                .scan_releases()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .unwrap();
            assert_eq!(scanned.id(), release.id());
            assert_eq!(scanned.tracks().len(), 2);
            for track in scanned.tracks() {
                let audio = track.original_audio_file();
                assert!(audio.playback_url().as_str().starts_with(url_prefix));
                assert!(snapshot.contains_key(audio.object_key()));
            }

            assert_eq!(put.num_calls(), 2);
        }
    }

    #[tokio::test]
    async fn save_release_leaves_partial_audio_on_failure_and_skips_it_on_retry() {
        let objects = Arc::new(Mutex::new(BTreeMap::<String, Vec<u8>>::new()));
        let state = objects.clone();
        let list = mock!(Client::list_objects_v2).then_compute_output(move |request| {
            listing(&state.lock().unwrap(), request.prefix().unwrap())
        });
        let state = objects.clone();
        let put = mock!(Client::put_object).then_compute_response(move |request| {
            let mut objects = state.lock().unwrap();
            if objects.is_empty() {
                objects.insert(
                    request.key().unwrap().into(),
                    request.body().bytes().unwrap().to_vec(),
                );
                MockResponse::Output(PutObjectOutput::builder().build())
            } else {
                MockResponse::Error(PutObjectError::InvalidRequest(
                    aws_sdk_s3::types::error::InvalidRequest::builder().build(),
                ))
            }
        });
        let state = objects.clone();
        let get = mock!(Client::get_object).then_compute_output(move |request| {
            GetObjectOutput::builder()
                .body(ByteStream::from(
                    state.lock().unwrap()[request.key().unwrap()].clone(),
                ))
                .build()
        });
        let client = mock_client!(aws_sdk_s3, RuleMode::MatchAny, [&list, &put, &get]);
        let storage = storage(client);
        let mut second = sample_metadata();
        *second.track_number_mut() = "2".into();
        let release = ImportRelease::new(vec![
            sample_audio(&sample_metadata()),
            sample_audio(&second),
        ])
        .unwrap();
        assert!(storage.save_release(&release).await.is_err());
        assert_eq!(objects.lock().unwrap().len(), 1);
        let scanned = storage
            .scan_releases()
            .await
            .unwrap()
            .pop()
            .unwrap()
            .unwrap();
        assert_eq!(scanned.tracks().len(), 1);
        assert_eq!(
            storage.save_release(&release).await.unwrap(),
            SaveReleaseOutcome::AlreadyExists
        );

        assert_eq!(put.num_calls(), 2);
    }

    #[tokio::test]
    async fn scan_releases_groups_audio_across_pages_and_reports_invalid_release_ids() {
        let release = ImportRelease::new(vec![sample_audio(&sample_metadata())]).unwrap();
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
        let results = storage.scan_releases().await.unwrap();
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
        assert!(storage.scan_releases().await.is_err());
    }
}
