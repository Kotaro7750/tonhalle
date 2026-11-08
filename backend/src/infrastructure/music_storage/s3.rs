//! Saves and scans audio objects directly using a release-based S3 key layout.

use futures_util::{
    StreamExt, TryStreamExt,
    stream::{self, BoxStream},
};
use object_store::{
    Attribute, Attributes, ObjectStore, ObjectStoreExt,
    buffered::{BufReader, BufWriter},
    path::Path,
};
use std::{collections::BTreeMap, io, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;
use tokio_util::io::SyncIoBridge;
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

/// Stores and scans release audio through the object-store API.
pub(crate) struct S3MusicStorage {
    store: Arc<dyn ObjectStore>,
    prefix: String,
    playback_url_base: Option<Url>,
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
    /// The response body could not be read completely.
    #[error("reading audio body: {0}")]
    Body(#[from] object_store::Error),
}

impl S3MusicStorage {
    /// Connects storage to its bucket namespace and optional delivery endpoint.
    pub(crate) fn new(
        store: Arc<dyn ObjectStore>,
        prefix: String,
        playback_url_base: Option<Url>,
    ) -> Self {
        Self {
            store,
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
    async fn check_destination(&self, id: &ReleaseId) -> object_store::Result<bool> {
        let prefix = Path::parse(self.key(id.as_ref()))?;
        self.store
            .list(Some(&prefix))
            .try_next()
            .await
            .map(|object| object.is_some())
    }

    /// Enumerates staging completely before copying or deleting any objects.
    async fn staging_keys(&self, id: Option<&ReleaseId>) -> object_store::Result<Vec<Path>> {
        let relative = id.map_or_else(
            || ".tonhalle-import".to_owned(),
            |id| format!(".tonhalle-import/{id}"),
        );
        let prefix = Path::parse(self.key(&relative))?;
        self.store
            .list(Some(&prefix))
            .map_ok(|object| object.location)
            .try_collect()
            .await
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

    /// Attempts every audio source before constructing a validated release.
    async fn scan_release(
        &self,
        id: ReleaseId,
        keys: Vec<String>,
    ) -> Result<Release, ReleaseScanFailure> {
        let mut tracks = Vec::new();
        let mut failures = Vec::new();
        for key in keys {
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
                let path = Path::parse(&key).map_err(object_store::Error::from)?;
                let meta = self.store.head(&path).await?;
                let reader = BufReader::with_capacity(self.store.clone(), &meta, 8192);
                Ok::<_, S3ReadAudioError>((audio_id, url, SyncIoBridge::new(reader)))
            }
            .await;
            let result = match source {
                Err(source) => Err(AudioScanFailure::Read {
                    object_key: key.clone(),
                    source: Box::new(source),
                }),
                Ok((audio_id, url, reader)) => {
                    let id = id.clone();
                    let object_key = key.clone();
                    spawn_blocking(move || {
                        ingest_track(reader, audio_id, object_key.clone(), url, &id)
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
        if !failures.is_empty() {
            return Err(ReleaseScanFailure::Audio { id, failures });
        }
        let release_id = id.clone();
        spawn_blocking(move || build_release(release_id, tracks))
            .await
            .map_err(|source| ReleaseScanFailure::Audio {
                id: id.clone(),
                failures: vec![AudioScanFailure::Read {
                    object_key: self.key(id.as_ref()),
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
        let path = Path::parse(&key).map_err(|source| Self::save_error(&id, source))?;
        let mut attributes = Attributes::new();
        attributes.insert(Attribute::ContentType, media.as_str().to_owned().into());
        let writer = BufWriter::with_capacity(self.store.clone(), path, 5 * 1024 * 1024)
            .with_max_concurrency(1)
            .with_attributes(attributes);
        let mut writer = SyncIoBridge::new(writer);
        let (copied, mut writer) = spawn_blocking(move || {
            let copied = audio
                .into_reader()
                .and_then(|mut reader| io::copy(&mut reader, &mut writer));
            (copied, writer.into_inner())
        })
        .await
        .map_err(|source| Self::save_error(&id, source))?;
        if let Err(source) = copied {
            if let Err(error) = writer.abort().await {
                tracing::warn!(release_id = %id, ?error, "Aborting staging upload failed");
            }
            return Err(Self::save_error(&id, source));
        }
        writer
            .shutdown()
            .await
            .map_err(|source| Self::save_error(&id, source))?;
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
            let filename = key.filename().unwrap();
            let destination = Path::parse(self.key(&format!("{id}/{filename}")))
                .map_err(|source| Self::save_error(id, source))?;
            self.store
                .copy(&key, &destination)
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
        self.store
            .delete_stream(stream::iter(keys.into_iter().map(Ok)).boxed())
            .try_collect::<Vec<_>>()
            .await
            .map_err(DeleteStagingError::new)?;
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
                    let path = Path::parse(prefix.trim_end_matches('/'))
                        .map_err(ScanReleasesError::new)?;
                    let mut objects = self.store.list(Some(&path));
                    let mut groups = BTreeMap::<String, Vec<String>>::new();
                    while let Some(object) =
                        objects.try_next().await.map_err(ScanReleasesError::new)?
                    {
                        let key = object.location.as_ref();
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

/// Verifies storage behavior through the object-store contract without live S3 writes.
#[cfg(test)]
mod tests {
    use futures_util::{
        StreamExt, TryStreamExt,
        stream::{self, BoxStream},
    };
    use object_store::{
        CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
        ObjectStoreExt, PutMultipartOptions, PutOptions, PutPayload, PutResult, UploadPart,
        local::LocalFileSystem, memory::InMemory, path::Path,
    };
    use std::{
        fmt,
        fs::File,
        io::{Read, Seek, SeekFrom, Write},
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use super::*;
    use crate::{
        domain::{import::ImportedReleaseValidator, ingestion::metadata::sample_metadata},
        test_support::{
            sample_audio, sample_import_audio, validate_import_release, write_sparse_audio,
        },
    };

    /// Adds observable reads and deterministic infrastructure failures to a real object-store implementation.
    #[derive(Debug)]
    struct TestStore {
        inner: Arc<dyn ObjectStore>,
        ranges: Mutex<Vec<std::ops::Range<u64>>>,
        copies: AtomicUsize,
        aborted_uploads: Arc<AtomicUsize>,
        fail_copy: bool,
        fail_list: bool,
    }

    impl TestStore {
        /// Wraps storage with fault injection disabled.
        fn new(inner: Arc<dyn ObjectStore>) -> Self {
            Self {
                inner,
                ranges: Mutex::new(Vec::new()),
                copies: AtomicUsize::new(0),
                aborted_uploads: Arc::new(AtomicUsize::new(0)),
                fail_copy: false,
                fail_list: false,
            }
        }
    }

    impl fmt::Display for TestStore {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestStore")
        }
    }

    /// Supplies an identifiable infrastructure failure.
    fn failure() -> object_store::Error {
        object_store::Error::Generic {
            store: "test",
            source: "injected failure".into(),
        }
    }

    #[async_trait::async_trait]
    impl ObjectStore for TestStore {
        async fn get_opts(
            &self,
            path: &Path,
            options: GetOptions,
        ) -> object_store::Result<GetResult> {
            let is_range = options.range.is_some();
            let result = self.inner.get_opts(path, options).await?;
            if is_range {
                self.ranges.lock().unwrap().push(result.range.clone());
            }
            Ok(result)
        }
        async fn put_opts(
            &self,
            path: &Path,
            payload: PutPayload,
            options: PutOptions,
        ) -> object_store::Result<PutResult> {
            self.inner.put_opts(path, payload, options).await
        }
        async fn put_multipart_opts(
            &self,
            path: &Path,
            options: PutMultipartOptions,
        ) -> object_store::Result<Box<dyn MultipartUpload>> {
            let upload = self.inner.put_multipart_opts(path, options).await?;
            Ok(Box::new(TrackedUpload {
                upload,
                aborted: self.aborted_uploads.clone(),
            }))
        }
        fn delete_stream(
            &self,
            paths: BoxStream<'static, object_store::Result<Path>>,
        ) -> BoxStream<'static, object_store::Result<Path>> {
            self.inner.delete_stream(paths)
        }
        fn list(
            &self,
            prefix: Option<&Path>,
        ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
            if self.fail_list {
                self.inner
                    .list(prefix)
                    .chain(stream::once(async { Err(failure()) }))
                    .boxed()
            } else {
                self.inner.list(prefix)
            }
        }
        async fn list_with_delimiter(
            &self,
            prefix: Option<&Path>,
        ) -> object_store::Result<ListResult> {
            self.inner.list_with_delimiter(prefix).await
        }
        async fn copy_opts(
            &self,
            from: &Path,
            to: &Path,
            options: CopyOptions,
        ) -> object_store::Result<()> {
            if self.fail_copy && self.copies.fetch_add(1, Ordering::Relaxed) > 0 {
                return Err(failure());
            }
            self.inner.copy_opts(from, to, options).await
        }
    }

    /// Records whether a failed source transfer aborts its multipart upload.
    #[derive(Debug)]
    struct TrackedUpload {
        upload: Box<dyn MultipartUpload>,
        aborted: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl MultipartUpload for TrackedUpload {
        fn put_part(&mut self, data: PutPayload) -> UploadPart {
            self.upload.put_part(data)
        }
        async fn complete(&mut self) -> object_store::Result<PutResult> {
            self.upload.complete().await
        }
        async fn abort(&mut self) -> object_store::Result<()> {
            self.aborted.fetch_add(1, Ordering::Relaxed);
            self.upload.abort().await
        }
    }

    /// Fails only within the audio payload, leaving metadata reads and seeks intact.
    struct PayloadFailure(File);

    impl Read for PayloadFailure {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if (6 * 1024 * 1024..7 * 1024 * 1024).contains(&self.0.stream_position()?) {
                return Err(io::Error::other("injected source read failure"));
            }
            self.0.read(buffer)
        }
    }
    impl Seek for PayloadFailure {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.0.seek(position)
        }
    }

    #[tokio::test]
    async fn stage_audio_streams_multipart_and_aborts_when_source_reads_fail() {
        for fail_read in [false, true] {
            let memory = Arc::new(InMemory::new());
            let observed = Arc::new(TestStore::new(memory.clone()));
            let storage = storage(observed.clone(), "content");
            let mut file = tempfile::tempfile().unwrap();
            write_sparse_audio(&mut file, &sample_metadata(), 12 * 1024 * 1024);
            let length = file.metadata().unwrap().len();
            let audio = if fail_read {
                ImportedReleaseValidator::default()
                    .push(PayloadFailure(file))
                    .unwrap()
            } else {
                ImportedReleaseValidator::default().push(file).unwrap()
            };
            let result = storage.stage_audio(audio).await;
            let objects = memory.list(None).try_collect::<Vec<_>>().await.unwrap();
            if fail_read {
                assert!(result.is_err());
                assert!(objects.is_empty());
                assert_eq!(observed.aborted_uploads.load(Ordering::Relaxed), 1);
            } else {
                result.unwrap();
                assert_eq!(objects.len(), 1);
                assert_eq!(objects[0].size, length);
                assert_eq!(observed.aborted_uploads.load(Ordering::Relaxed), 0);
                let result = memory.get(&objects[0].location).await.unwrap();
                assert_eq!(
                    result
                        .attributes
                        .get(&Attribute::ContentType)
                        .unwrap()
                        .as_ref(),
                    "audio/wav"
                );
            }
        }
    }

    /// Connects a test store to the same playback namespace as production.
    fn storage(store: Arc<dyn ObjectStore>, prefix: &str) -> S3MusicStorage {
        S3MusicStorage::new(
            store,
            prefix.into(),
            Some("https://example.com/music/".parse().unwrap()),
        )
    }

    #[tokio::test]
    async fn commit_release_preserves_bytes_content_type_and_existing_releases() {
        for prefix in ["", "space #%/音"] {
            let store = Arc::new(InMemory::new());
            let storage = storage(store.clone(), prefix);
            let first = sample_metadata();
            let mut second = first.clone();
            *second.track_number_mut() = "2".into();
            let bytes = vec![sample_audio(&first), sample_audio(&second)];
            let release = validate_import_release(bytes.clone()).unwrap();
            assert_eq!(
                storage.commit_release(release.id(), true).await.unwrap(),
                SaveReleaseOutcome::Saved
            );
            assert!(
                store
                    .list(None)
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap()
                    .is_empty()
            );
            for audio in &bytes {
                storage
                    .stage_audio(sample_import_audio(audio.clone()))
                    .await
                    .unwrap();
            }
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
            storage.delete_staging(None).await.unwrap();
            let objects = store.list(None).try_collect::<Vec<_>>().await.unwrap();
            assert_eq!(objects.len(), 2);
            for object in &objects {
                let result = store.get(&object.location).await.unwrap();
                assert_eq!(
                    result
                        .attributes
                        .get(&Attribute::ContentType)
                        .unwrap()
                        .as_ref(),
                    "audio/wav"
                );
                assert!(bytes.contains(&result.bytes().await.unwrap().to_vec()));
            }
            for dry_run in [true, false] {
                assert_eq!(
                    storage.commit_release(release.id(), dry_run).await.unwrap(),
                    SaveReleaseOutcome::AlreadyExists
                );
            }
            let mut scanned = storage
                .scan_releases()
                .try_collect::<Vec<_>>()
                .await
                .unwrap();
            let scanned = scanned.pop().unwrap().unwrap();
            assert_eq!(scanned.id(), release.id());
            assert_eq!(scanned.tracks().len(), 2);
            assert!(scanned.tracks().iter().all(|track| {
                track
                    .original_audio_file()
                    .playback_url()
                    .as_str()
                    .starts_with("https://example.com/music/")
            }));
        }
    }

    #[tokio::test]
    async fn commit_release_preserves_partial_publication_and_cleanup_only_removes_staging() {
        let memory = Arc::new(InMemory::new());
        let mut fault = TestStore::new(memory.clone());
        fault.fail_copy = true;
        let storage = storage(Arc::new(fault), "content");
        let first = sample_metadata();
        let mut second = first.clone();
        *second.track_number_mut() = "2".into();
        let bytes = vec![sample_audio(&first), sample_audio(&second)];
        let release = validate_import_release(bytes.clone()).unwrap();
        for audio in bytes {
            storage
                .stage_audio(sample_import_audio(audio))
                .await
                .unwrap();
        }
        assert!(storage.commit_release(release.id(), false).await.is_err());
        storage.delete_staging(Some(release.id())).await.unwrap();
        let objects = memory.list(None).try_collect::<Vec<_>>().await.unwrap();
        assert_eq!(objects.len(), 1);
        assert!(!objects[0].location.as_ref().contains(".tonhalle-import"));
    }

    #[tokio::test]
    async fn scan_releases_collects_all_audio_errors_and_propagates_late_listing_failures() {
        let memory = Arc::new(InMemory::new());
        let release = validate_import_release(vec![sample_audio(&sample_metadata())]).unwrap();
        for name in [
            "8d42d700-ea25-45a3-bb86-00e916339e7c",
            "229fc612-8a56-45d0-bf61-55b36ff3dcba",
        ] {
            memory
                .put(
                    &Path::from(format!("content/{}/{name}.wav", release.id())),
                    b"corrupt".as_slice().into(),
                )
                .await
                .unwrap();
        }
        memory
            .put(
                &Path::from("content/invalid/audio.wav"),
                b"corrupt".as_slice().into(),
            )
            .await
            .unwrap();
        let storage_ok = storage(memory.clone(), "content");
        let scanned = storage_ok
            .scan_releases()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(scanned.len(), 2);
        assert!(scanned.iter().any(|result| matches!(result, Err(ReleaseScanFailure::Audio { failures, .. }) if failures.len() == 2)));
        assert!(
            scanned
                .iter()
                .any(|result| matches!(result, Err(ReleaseScanFailure::InvalidId { .. })))
        );
        let mut fault = TestStore::new(memory);
        fault.fail_list = true;
        assert!(
            storage(Arc::new(fault), "content")
                .scan_releases()
                .try_collect::<Vec<_>>()
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn scan_releases_skips_large_payload_and_reads_trailing_tags_with_bounded_ranges() {
        let directory = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        let mut header = sample_audio(&source);
        let data = header
            .windows(4)
            .position(|chunk| chunk == b"data")
            .unwrap();
        let original = u32::from_le_bytes(header[data + 4..data + 8].try_into().unwrap());
        let tail = header[data + 8 + original as usize..].to_vec();
        let payload_length = 256_u32 * 1024 * 1024;
        let tail_offset = data as u64 + 8 + u64::from(payload_length);
        let length = tail_offset + tail.len() as u64;
        header[4..8].copy_from_slice(&((length - 8) as u32).to_le_bytes());
        header[data + 4..data + 8].copy_from_slice(&payload_length.to_le_bytes());
        let release = validate_import_release(vec![sample_audio(&source)]).unwrap();
        let release_dir = directory.path().join(release.id().as_ref());
        std::fs::create_dir(&release_dir).unwrap();
        let mut file =
            File::create(release_dir.join("229fc612-8a56-45d0-bf61-55b36ff3dcba.wav")).unwrap();
        file.write_all(&header[..data + 8]).unwrap();
        file.seek(SeekFrom::Start(tail_offset)).unwrap();
        file.write_all(&tail).unwrap();
        let store = Arc::new(TestStore::new(Arc::new(
            LocalFileSystem::new_with_prefix(directory.path()).unwrap(),
        )));
        let storage = storage(store.clone(), "");
        let mut releases = storage
            .scan_releases()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(releases.len(), 1);
        let parsed = releases.pop().unwrap().unwrap();
        assert_eq!(parsed.id(), release.id());
        assert_eq!(parsed.title(), source.release_title());
        assert_eq!(parsed.tracks().len(), 1);
        assert_eq!(parsed.tracks()[0].title(), source.track_title());
        let ranges = store.ranges.lock().unwrap();
        assert!(ranges.iter().any(|range| range.start >= tail_offset));
        assert!(
            ranges
                .iter()
                .all(|range| range.start < 8192 || range.start >= tail_offset)
        );
        assert!(
            ranges
                .iter()
                .map(|range| range.end - range.start)
                .sum::<u64>()
                < 64 * 1024
        );
    }

    #[tokio::test]
    async fn delete_staging_preserves_other_releases_until_all_staging_is_requested() {
        let memory = Arc::new(InMemory::new());
        let storage = storage(memory.clone(), "content");
        let first = sample_metadata();
        let mut second = first.clone();
        second.release_title_mut().push_str(" other");
        let first_audio = sample_import_audio(sample_audio(&first));
        let first_id = first_audio.release_id().clone();
        storage.stage_audio(first_audio).await.unwrap();
        storage
            .stage_audio(sample_import_audio(sample_audio(&second)))
            .await
            .unwrap();
        storage.delete_staging(Some(&first_id)).await.unwrap();
        storage.delete_staging(Some(&first_id)).await.unwrap();
        let remaining = memory.list(None).try_collect::<Vec<_>>().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(!remaining[0].location.as_ref().contains(first_id.as_ref()));
        storage.delete_staging(None).await.unwrap();
        storage.delete_staging(None).await.unwrap();
        assert!(
            memory
                .list(None)
                .try_collect::<Vec<_>>()
                .await
                .unwrap()
                .is_empty()
        );
    }
}
