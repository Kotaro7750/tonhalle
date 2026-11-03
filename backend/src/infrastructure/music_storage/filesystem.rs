//! Reads and persists complete releases using filesystem storage.

use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions, create_dir_all, read, rename, symlink_metadata},
    io::{Error as IoError, ErrorKind, Write},
    path::{Path, PathBuf, StripPrefixError},
};
use tokio::task::spawn_blocking;
use url::Url;
use walkdir::WalkDir;

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

/// A filesystem adapter for reading and saving stored audio releases.
#[derive(Clone)]
pub(crate) struct FileSystemMusicStorage {
    root_path: PathBuf,
    playback_url_base: Option<Url>,
}

/// An invalid content root or playback endpoint configuration.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileSystemStorageConstructionError {
    /// Startup cannot scan the configured audio directory.
    #[error("content root must be an existing directory")]
    InvalidContentRoot,
    /// Playback links cannot be built from the configured delivery endpoint.
    #[error("playback URL base must be an HTTP(S) directory URL without query or fragment")]
    InvalidPlaybackUrlBase,
}

/// A failure to read a stored audio file or resolve its location.
#[derive(Debug, thiserror::Error)]
enum ReadAudioFileError {
    /// The filename cannot supply a stored audio identity.
    #[error("invalid audio file name")]
    InvalidFileName,
    /// The file cannot be addressed relative to the configured storage directory.
    #[error("audio object is outside content root: {0}")]
    OutsideContentRoot(#[source] StripPrefixError),
    /// The relative path cannot be used as a text object key.
    #[error("non-UTF-8 object key")]
    NonUtf8ObjectKey,
    /// Playback links cannot be built from the configured delivery endpoint.
    #[error("invalid playback URL base")]
    InvalidPlaybackUrlBase,
    /// The stored file could not be read.
    #[error("reading audio: {0}")]
    Read(#[source] IoError),
    /// The filename does not preserve a valid UUID v4 audio identity.
    #[error(transparent)]
    InvalidAudioId(#[from] ParseAudioFileIdError),
    /// The audio path must be rejected without following a symbolic link or reading a directory.
    #[error("audio object must be a regular file")]
    NotRegularFile,
}

/// A filesystem traversal failure that prevents a complete storage scan.
#[derive(Debug, thiserror::Error)]
enum FileSystemScanError {
    /// Traversal failed before the complete set of audio files could be read.
    #[error("enumerating music storage: {0}")]
    Enumerate(#[source] walkdir::Error),
    /// The audio path cannot be grouped into a parent storage release.
    #[error("missing release directory")]
    MissingReleaseDirectory,
}

/// Validates and normalizes a delivery endpoint consistently for configuration and storage.
pub(crate) fn normalize_playback_url(
    url: &mut Url,
) -> Result<(), FileSystemStorageConstructionError> {
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(FileSystemStorageConstructionError::InvalidPlaybackUrlBase);
    }

    if !url.path().ends_with('/') {
        url.path_segments_mut()
            .map_err(|_| FileSystemStorageConstructionError::InvalidPlaybackUrlBase)?
            .push("");
    }
    Ok(())
}

impl FileSystemMusicStorage {
    /// Opens a destination for writing without requiring a delivery endpoint.
    pub(crate) fn for_import(
        root_path: PathBuf,
    ) -> Result<Self, FileSystemStorageConstructionError> {
        if !root_path.is_dir() {
            return Err(FileSystemStorageConstructionError::InvalidContentRoot);
        }
        Ok(Self {
            root_path,
            playback_url_base: None,
        })
    }

    /// Validates the content root and canonicalizes an HTTP(S) delivery directory URL.
    pub(crate) fn new(
        root_path: PathBuf,
        mut playback_url_base: Url,
    ) -> Result<Self, FileSystemStorageConstructionError> {
        if !root_path.is_dir() {
            return Err(FileSystemStorageConstructionError::InvalidContentRoot);
        }

        normalize_playback_url(&mut playback_url_base)?;

        Ok(Self {
            root_path,
            playback_url_base: Some(playback_url_base),
        })
    }

    /// Reads original bytes with their existing UUID and escaped playback URL.
    fn read_audio(
        &self,
        path: &Path,
    ) -> Result<(AudioFileId, String, Url, Vec<u8>), ReadAudioFileError> {
        if !symlink_metadata(path)
            .map_err(ReadAudioFileError::Read)?
            .is_file()
        {
            return Err(ReadAudioFileError::NotRegularFile);
        }

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or(ReadAudioFileError::InvalidFileName)?;
        let audio_id = stem.parse::<AudioFileId>()?;

        let relative = path
            .strip_prefix(&self.root_path)
            .map_err(ReadAudioFileError::OutsideContentRoot)?;
        let segments = relative
            .iter()
            .map(|s| s.to_str().ok_or(ReadAudioFileError::NonUtf8ObjectKey))
            .collect::<Result<Vec<_>, ReadAudioFileError>>()?;
        let object_key = segments.join("/");

        let mut playback_url = self
            .playback_url_base
            .clone()
            .ok_or(ReadAudioFileError::InvalidPlaybackUrlBase)?;
        playback_url
            .path_segments_mut()
            .map_err(|_| ReadAudioFileError::InvalidPlaybackUrlBase)?
            .pop_if_empty()
            .extend(segments);

        let bytes = read(path).map_err(ReadAudioFileError::Read)?;

        Ok((audio_id, object_key, playback_url, bytes))
    }

    /// Obtains the release identity from its directory without reading any audio bytes.
    fn release_id(&self, path: &Path) -> Result<ReleaseId, ReleaseScanFailure> {
        let object_key = path
            .strip_prefix(&self.root_path)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .parse()
            .map_err(|source| ReleaseScanFailure::InvalidId { object_key, source })
    }

    /// Reads every source belonging to one identity before constructing its domain release.
    fn scan_release(
        &self,
        id: ReleaseId,
        audio_files: Vec<PathBuf>,
    ) -> Result<Release, ReleaseScanFailure> {
        let mut tracks = Vec::new();
        let mut failures = Vec::new();

        for path in audio_files {
            let object_key = path
                .strip_prefix(&self.root_path)
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let result = self
                .read_audio(&path)
                .map_err(|source| AudioScanFailure::Read {
                    object_key: object_key.clone(),
                    source: Box::new(source),
                })
                .and_then(|(audio_id, key, playback_url, bytes)| {
                    ingest_track(&bytes, audio_id, key, playback_url, &id)
                        .map_err(|source| AudioScanFailure::Parse { object_key, source })
                });

            match result {
                Ok(track) => tracks.push(track),
                Err(failure) => failures.push(failure),
            }
        }

        if !failures.is_empty() {
            return Err(ReleaseScanFailure::Audio { id, failures });
        }

        build_release(id.clone(), tracks)
            .map_err(|source| ReleaseScanFailure::InvalidRelease { id, source })
    }

    /// Writes original bytes and UUID filenames before exposing a complete release.
    fn persist_release(
        &self,
        release: &ImportRelease,
    ) -> Result<SaveReleaseOutcome, FileSystemSaveReleaseError> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.root_path.join(".tonhalle-import.lock"))?;
        lock.lock()?;

        let destination = self.root_path.join(release.id().as_ref());
        match symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_dir() => return Ok(SaveReleaseOutcome::AlreadyExists),
            Ok(_) => {
                return Err(FileSystemSaveReleaseError::ConflictingDestination);
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let staging_root = self.root_path.join(STAGING_DIRECTORY);
        create_dir_all(&staging_root)?;
        if symlink_metadata(&staging_root)?.file_type().is_symlink() {
            return Err(FileSystemSaveReleaseError::InvalidStagingDirectory);
        }
        let staging = tempfile::tempdir_in(staging_root)?;
        for audio in release.audio() {
            let extension = audio.properties().media_type().extension();
            let mut file = OpenOptions::new().write(true).create_new(true).open(
                staging
                    .path()
                    .join(format!("{}.{}", uuid::Uuid::new_v4(), extension)),
            )?;
            file.write_all(audio.bytes())?;
            file.sync_all()?;
        }
        File::open(staging.path())?.sync_all()?;
        rename(staging.path(), destination)?;
        File::open(&self.root_path)?.sync_all()?;
        Ok(SaveReleaseOutcome::Saved)
    }
}

impl FileSystemMusicStorage {
    /// Publishes a validated release without replacing existing audio.
    fn save_release_blocking(
        &self,
        release: &ImportRelease,
    ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
        self.persist_release(release)
            .map_err(|source| match source {
                FileSystemSaveReleaseError::ConflictingDestination => SaveReleaseError::Conflict {
                    id: release.id().clone(),
                },
                source => SaveReleaseError::Internal {
                    id: release.id().clone(),
                    source: Box::new(source),
                },
            })
    }

    /// Groups audio paths by their stored release directory and constructs domain releases.
    fn scan_releases_blocking(
        &self,
    ) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
        let mut groups: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();

        // Do not follow symlinks outside the configured read-only content root.
        for entry in WalkDir::new(&self.root_path)
            .sort_by_file_name()
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| entry.depth() != 1 || entry.file_name() != STAGING_DIRECTORY)
        {
            let entry = entry
                .map_err(|error| ScanReleasesError::new(FileSystemScanError::Enumerate(error)))?;
            let path = entry.path();

            if entry.file_type().is_dir()
                && path != self.root_path
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("release-"))
            {
                groups.entry(path.to_path_buf()).or_default();
            }

            if !path
                .extension()
                .and_then(|extension| extension.to_str())
                .and_then(MediaType::from_extension)
                .is_some()
            {
                continue;
            }

            let parent = path
                .parent()
                .ok_or_else(|| {
                    ScanReleasesError::new(FileSystemScanError::MissingReleaseDirectory)
                })?
                .to_path_buf();

            groups.entry(parent).or_default().push(path.to_path_buf());
        }

        let mut releases = BTreeMap::<ReleaseId, Vec<PathBuf>>::new();
        let mut results = Vec::new();

        for (path, mut audio_files) in groups {
            match self.release_id(&path) {
                Ok(id) => releases.entry(id).or_default().append(&mut audio_files),
                Err(failure) => results.push(Err(failure)),
            }
        }

        results.extend(
            releases
                .into_iter()
                .map(|(id, audio_files)| self.scan_release(id, audio_files)),
        );

        Ok(results)
    }
}

#[async_trait::async_trait]
impl MusicStorage for FileSystemMusicStorage {
    async fn save_release(
        &self,
        release: &ImportRelease,
    ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
        let storage = self.clone();
        let release = release.clone();
        let id = release.id().clone();
        spawn_blocking(move || storage.save_release_blocking(&release))
            .await
            .map_err(|source| SaveReleaseError::Internal {
                id,
                source: Box::new(source),
            })?
    }

    async fn scan_releases(
        &self,
    ) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
        let storage = self.clone();
        spawn_blocking(move || storage.scan_releases_blocking())
            .await
            .map_err(ScanReleasesError::new)?
    }
}

/// Reserves the internal staging directory from normal music-storage enumeration.
const STAGING_DIRECTORY: &str = ".tonhalle-import";

/// A filesystem failure while staging or publishing one release.
#[derive(Debug, thiserror::Error)]
enum FileSystemSaveReleaseError {
    /// The release location is occupied by an incompatible object.
    #[error("release destination is not a directory")]
    ConflictingDestination,
    /// A staging symlink cannot be used for writing.
    #[error("staging directory must not be a symlink")]
    InvalidStagingDirectory,
    /// A filesystem operation failed.
    #[error("filesystem save operation failed: {0}")]
    Io(#[from] IoError),
}

/// Verifies filesystem storage through its scanning and saving contracts.
#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs::{create_dir, create_dir_all, read, remove_dir, write},
        io::{Error as IoError, ErrorKind},
    };

    use super::*;
    use crate::{
        domain::{
            ingestion::{
                IngestTrackError,
                metadata::{RawMetadata, sample_metadata},
            },
            primitive::release::ReleaseValidationError,
        },
        test_support::{release_metadata, sample_audio, sample_audio_file},
    };

    /// Opens a temporary read-only store with an external playback URL.
    fn storage(root: &Path) -> FileSystemMusicStorage {
        FileSystemMusicStorage::new(
            root.to_path_buf(),
            "https://example.com/music".parse().unwrap(),
        )
        .unwrap()
    }

    /// Writes a valid source using the filesystem adapter's stored identity layout.
    fn write_audio(root: &Path, source: &RawMetadata) -> PathBuf {
        write_audio_with_id(root, source, sample_audio_file().id())
    }

    /// Writes metadata under an explicitly chosen stored audio identity.
    fn write_audio_with_id(root: &Path, source: &RawMetadata, audio_id: AudioFileId) -> PathBuf {
        let id = release_metadata(source).unwrap().id().clone();
        let directory = root.join(id.as_ref());

        create_dir_all(&directory).unwrap();

        let path = directory.join(format!("{}.wav", audio_id));
        write(&path, sample_audio(source)).unwrap();

        path
    }

    #[tokio::test]
    async fn save_release_distinguishes_conflicts_and_preserves_io_failure_causes() {
        let root = tempfile::tempdir().unwrap();
        let release = ImportRelease::new(vec![sample_audio(&sample_metadata())]).unwrap();
        let storage = FileSystemMusicStorage::for_import(root.path().to_path_buf()).unwrap();
        let destination = root.path().join(release.id().as_ref());
        write(&destination, b"existing object").unwrap();
        assert!(matches!(storage.save_release(&release).await,
            Err(SaveReleaseError::Conflict { id }) if &id == release.id()));
        assert_eq!(read(destination).unwrap(), b"existing object");

        let missing = root.path().join("missing");
        std::fs::create_dir(&missing).unwrap();
        let storage = FileSystemMusicStorage::for_import(missing.clone()).unwrap();
        std::fs::remove_dir(missing).unwrap();
        let error = storage.save_release(&release).await.unwrap_err();
        assert!(matches!(&error, SaveReleaseError::Internal { id, .. } if id == release.id()));
        let io = error
            .source()
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<IoError>()
            .unwrap();
        assert_eq!(io.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn scan_releases_preserves_audio_files_and_escapes_delivery_paths() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("artist with spaces #?%");
        let source = sample_metadata();
        let path = write_audio(&nested, &source.clone());
        let original = read(&path).unwrap();

        for _ in 0..2 {
            let release = storage(temp.path())
                .scan_releases()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .unwrap();

            assert_eq!(release.tracks().len(), 1);

            let audio = release.tracks()[0].original_audio_file();

            assert_eq!(audio.id(), sample_audio_file().id());
            assert_eq!(
                audio.playback_url().as_str(),
                format!(
                    "https://example.com/music/artist%20with%20spaces%20%23%3F%25/{}/{}.wav",
                    release.id(),
                    audio.id()
                )
            );
            assert_eq!(read(&path).unwrap(), original);
        }
    }

    #[tokio::test]
    async fn scan_releases_reports_storage_keys_when_release_ids_are_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("invalid-release");

        create_dir(&directory).unwrap();

        write(directory.join("invalid-audio-id.wav"), b"corrupt").unwrap();

        let result = storage(temp.path())
            .scan_releases()
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(
            matches!(result, Err(ReleaseScanFailure::InvalidId { object_key, .. }) if object_key == "invalid-release")
        );
    }

    #[tokio::test]
    async fn scan_releases_collects_all_audio_failures_with_their_release_id() {
        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        let path = write_audio(temp.path(), &source.clone());
        let directory = path.parent().unwrap();

        write(directory.join("invalid-audio-id.wav"), b"corrupt").unwrap();
        write(
            directory.join("229fc612-8a56-45d0-bf61-55b36ff3dcba.wav"),
            b"corrupt",
        )
        .unwrap();

        let mut different = source.clone();
        different.release_title_mut().push_str(" Other");
        let audio_id = "925eda40-54cb-4af4-9579-6811fc7477ac".parse().unwrap();

        write(
            directory.join(format!("{}.wav", audio_id)),
            sample_audio(&different),
        )
        .unwrap();
        write_audio_with_id(temp.path(), &different, audio_id);

        let mut results = storage(temp.path()).scan_releases().await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);

        let error = results
            .swap_remove(results.iter().position(|result| result.is_err()).unwrap())
            .unwrap_err();
        let ReleaseScanFailure::Audio { id, failures } = error else {
            panic!("unexpected scan failure")
        };

        assert_eq!(&id, release_metadata(&source).unwrap().id());
        assert_eq!(failures.len(), 3);

        assert!(failures.iter().any(|failure| matches!(failure, AudioScanFailure::Read { object_key, .. } if object_key.ends_with("invalid-audio-id.wav"))));

        assert!(failures.iter().any(|failure| matches!(
            failure,
            AudioScanFailure::Parse {
                source: IngestTrackError::ExtractMetadata(_),
                ..
            }
        )));

        assert!(failures.iter().any(|failure| matches!(
            failure,
            AudioScanFailure::Parse {
                source: IngestTrackError::ReleaseIdMismatch { .. },
                ..
            }
        )));

        assert!(path.exists());
    }

    #[tokio::test]
    async fn scan_releases_reports_release_construction_failures() {
        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        write_audio(temp.path(), &source.clone());

        let mut duplicate = source.clone();
        duplicate.release_title_mut().push(' ');
        let audio_id = "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();

        write_audio_with_id(temp.path(), &duplicate, audio_id);

        let failure = storage(temp.path())
            .scan_releases()
            .await
            .unwrap()
            .pop()
            .unwrap()
            .unwrap_err();

        assert!(matches!(
            failure,
            ReleaseScanFailure::InvalidRelease {
                source: ReleaseValidationError::MetadataMismatch { .. },
                ..
            }
        ));

        let empty = tempfile::tempdir().unwrap();
        let id = release_metadata(&source).unwrap().id().clone();

        create_dir(empty.path().join(id.as_ref())).unwrap();

        let failure = storage(empty.path())
            .scan_releases()
            .await
            .unwrap()
            .pop()
            .unwrap()
            .unwrap_err();

        assert!(
            matches!(failure, ReleaseScanFailure::InvalidRelease { id: failed_id, source: ReleaseValidationError::Empty } if failed_id == id)
        );
    }

    #[tokio::test]
    async fn scan_releases_groups_all_sources_with_the_same_release_id() {
        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        write_audio(&temp.path().join("a"), &source.clone());

        let mut other = source.clone();
        *other.track_number_mut() = "2".into();
        let audio_id = "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();

        let second_path = write_audio_with_id(&temp.path().join("b"), &other, audio_id);

        let mut results = storage(temp.path()).scan_releases().await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results.pop().unwrap().unwrap().tracks().len(), 2);

        write(
            second_path
                .parent()
                .unwrap()
                .join("925eda40-54cb-4af4-9579-6811fc7477ac.wav"),
            b"corrupt",
        )
        .unwrap();

        let mut results = storage(temp.path()).scan_releases().await.unwrap();

        assert_eq!(results.len(), 1);

        assert!(
            matches!(results.pop().unwrap(), Err(ReleaseScanFailure::Audio { id, .. }) if &id == release_metadata(&source).unwrap().id())
        );
    }

    #[tokio::test]
    async fn scan_releases_returns_an_error_when_storage_enumeration_fails() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("music");

        create_dir(&root).unwrap();

        let storage = storage(&root);

        remove_dir(&root).unwrap();

        assert!(storage.scan_releases().await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn scan_releases_rejects_symbolic_links_without_following_them() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        let path = write_audio(temp.path(), &source.clone());

        symlink(
            &path,
            path.parent()
                .unwrap()
                .join("229fc612-8a56-45d0-bf61-55b36ff3dcba.wav"),
        )
        .unwrap();

        let failure = storage(temp.path())
            .scan_releases()
            .await
            .unwrap()
            .pop()
            .unwrap()
            .unwrap_err();
        let ReleaseScanFailure::Audio { failures, .. } = failure else {
            panic!("unexpected scan failure")
        };

        assert!(
            matches!(&failures[0], AudioScanFailure::Read { source, .. } if matches!(source.downcast_ref::<ReadAudioFileError>(), Some(ReadAudioFileError::NotRegularFile)))
        );
    }
    #[tokio::test]
    async fn save_release_preserves_bytes_and_persistent_uuids_without_overwriting_existing_releases()
     {
        let root = tempfile::tempdir().unwrap();
        let storage = FileSystemMusicStorage::for_import(root.path().to_owned()).unwrap();
        let mut second = sample_metadata();
        *second.track_number_mut() = "2".into();
        let bytes = vec![sample_audio(&sample_metadata()), sample_audio(&second)];
        let release = ImportRelease::new(bytes.clone()).unwrap();
        assert_eq!(
            storage.save_release(&release).await.unwrap(),
            SaveReleaseOutcome::Saved
        );
        let directory = root.path().join(release.id().as_ref());
        let snapshot = || {
            let mut files = std::fs::read_dir(&directory)
                .unwrap()
                .map(|entry| {
                    let path = entry.unwrap().path();
                    let id: AudioFileId =
                        path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
                    (id.to_string(), read(path).unwrap())
                })
                .collect::<Vec<_>>();
            files.sort();
            files
        };
        let original = snapshot();
        assert_eq!(original.len(), 2);
        assert_ne!(original[0].0, original[1].0);
        for (_, stored) in &original {
            assert!(bytes.contains(stored));
        }
        let reopened = FileSystemMusicStorage::for_import(root.path().to_owned()).unwrap();
        assert_eq!(
            reopened.save_release(&release).await.unwrap(),
            SaveReleaseOutcome::AlreadyExists
        );
        assert_eq!(snapshot(), original);
    }

    #[tokio::test]
    async fn save_release_serializes_concurrent_writes() {
        let root = tempfile::tempdir().unwrap();
        let storage = std::sync::Arc::new(
            FileSystemMusicStorage::for_import(root.path().to_owned()).unwrap(),
        );
        let release = std::sync::Arc::new(
            ImportRelease::new(vec![sample_audio(&sample_metadata())]).unwrap(),
        );
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let storage = storage.clone();
                let release = release.clone();
                tokio::spawn(async move { storage.save_release(&release).await.unwrap() })
            })
            .collect();
        let mut results = Vec::new();
        for worker in workers {
            results.push(worker.await.unwrap());
        }
        assert_eq!(
            results
                .iter()
                .filter(|result| **result == SaveReleaseOutcome::Saved)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| **result == SaveReleaseOutcome::AlreadyExists)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn scan_releases_hides_incomplete_staging() {
        let root = tempfile::tempdir().unwrap();
        let partial = root.path().join(STAGING_DIRECTORY).join("interrupted");
        create_dir_all(&partial).unwrap();
        write_audio(&partial, &sample_metadata());
        assert!(
            storage(root.path())
                .scan_releases()
                .await
                .unwrap()
                .is_empty()
        );
    }
}
