//! Reads and persists complete releases using filesystem storage.

use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions, create_dir_all, read, rename, symlink_metadata},
    io::{Error as IoError, ErrorKind, Write},
    path::{Path, PathBuf, StripPrefixError},
};
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

/// A failure to construct a filesystem storage from its settings.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileSystemContentStorageConstructionError {
    /// The configured destination cannot be resolved.
    #[error("resolving content storage root: {0}")]
    ResolveRoot(#[from] IoError),
    /// The resolved destination does not satisfy storage requirements.
    #[error(transparent)]
    Storage(#[from] FileSystemStorageConstructionError),
}

/// CLI settings and construction rules shared by filesystem music-storage consumers.
#[derive(getset::Getters, Debug, clap::Args)]
pub(crate) struct FileSystemContentStorageConfig {
    /// Existing directory holding stored audio releases.
    #[arg(
        id = "content_storage_root_path",
        long = "content-storage--root-path",
        env = "TONHALLE_CONTENT_STORAGE__ROOT_PATH"
    )]
    /// Provides the filesystem destination for adapter-specific compatibility checks.
    #[getset(get = "pub(crate)")]
    root_path: PathBuf,
    /// Optional delivery endpoint used when reading stored releases.
    #[arg(
        id = "content_storage_playback_url_base",
        long = "content-storage--playback-url-base",
        env = "TONHALLE_CONTENT_STORAGE__PLAYBACK_URL_BASE"
    )]
    playback_url_base: Option<Url>,
}

impl FileSystemContentStorageConfig {
    /// Requires a valid delivery endpoint for operations that read playable releases.
    pub(crate) fn validate_playback_url(&self) -> Result<(), FileSystemStorageConstructionError> {
        let mut url = self
            .playback_url_base
            .clone()
            .ok_or(FileSystemStorageConstructionError::MissingPlaybackUrlBase)?;
        normalize_playback_url(&mut url)
    }

    /// Constructs storage with the optional delivery endpoint preserved.
    pub(crate) fn construct_music_storage(
        &self,
    ) -> Result<FileSystemMusicStorage, FileSystemContentStorageConstructionError> {
        let root = self.root_path.canonicalize()?;
        Ok(match &self.playback_url_base {
            Some(url) => FileSystemMusicStorage::new(root, url.clone())?,
            None => FileSystemMusicStorage::for_import(root)?,
        })
    }
}

/// A filesystem adapter for reading and saving stored audio releases.
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
    /// Reading playable releases requires a configured delivery endpoint.
    #[error("content-storage--playback-url-base is required for playback")]
    MissingPlaybackUrlBase,
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
    #[error("enumerating content storage: {0}")]
    Enumerate(#[source] walkdir::Error),
    /// The audio path cannot be grouped into a parent storage release.
    #[error("missing release directory")]
    MissingReleaseDirectory,
}

/// Validates and normalizes a delivery endpoint consistently for configuration and storage.
fn normalize_playback_url(url: &mut Url) -> Result<(), FileSystemStorageConstructionError> {
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
}

impl MusicStorage for FileSystemMusicStorage {
    /// Publishes a validated release without replacing existing audio.
    fn save_release(
        &self,
        release: &ImportRelease,
    ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
        save_release(&self.root_path, release)
    }

    /// Groups audio paths by their stored release directory and constructs domain releases.
    fn scan_releases(&self) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
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

/// Converts physical save failures to release-specific port errors.
fn save_release(
    root: &Path,
    release: &ImportRelease,
) -> Result<SaveReleaseOutcome, SaveReleaseError> {
    persist_release(root, release).map_err(|source| match source {
        FileSystemSaveReleaseError::ConflictingDestination => SaveReleaseError::Conflict {
            id: release.id().clone(),
        },
        source => SaveReleaseError::Internal {
            id: release.id().clone(),
            source: Box::new(source),
        },
    })
}

/// Writes original bytes and UUID filenames before exposing a complete release.
fn persist_release(
    root: &Path,
    release: &ImportRelease,
) -> Result<SaveReleaseOutcome, FileSystemSaveReleaseError> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(".tonhalle-import.lock"))?;
    lock.lock()?;

    let destination = root.join(release.id().as_ref());
    match symlink_metadata(&destination) {
        Ok(metadata) if metadata.is_dir() => return Ok(SaveReleaseOutcome::AlreadyExists),
        Ok(_) => {
            return Err(FileSystemSaveReleaseError::ConflictingDestination);
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let staging_root = root.join(STAGING_DIRECTORY);
    create_dir_all(&staging_root)?;
    if symlink_metadata(&staging_root)?.file_type().is_symlink() {
        return Err(FileSystemSaveReleaseError::InvalidStagingDirectory);
    }
    let staging = tempfile::tempdir_in(staging_root)?;
    for audio in release.audio() {
        let extension = audio.properties().media_type().extension();
        let mut file =
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(
                    staging
                        .path()
                        .join(format!("{}.{}", uuid::Uuid::new_v4(), extension)),
                )?;
        file.write_all(audio.bytes())?;
        file.sync_all()?;
    }
    File::open(staging.path())?.sync_all()?;
    rename(staging.path(), destination)?;
    File::open(root)?.sync_all()?;
    Ok(SaveReleaseOutcome::Saved)
}

/// Verifies release identity and physical causes survive the save-port boundary.
#[cfg(test)]
mod save_tests {
    use std::{
        error::Error,
        fs::{read, write},
        io::{Error as IoError, ErrorKind},
    };

    use super::*;
    use crate::test_support::{sample_audio, sample_fixture};

    #[test]
    fn save_release_distinguishes_conflicts_and_preserves_io_failure_causes() {
        let root = tempfile::tempdir().unwrap();
        let release = ImportRelease::new(vec![sample_audio(&sample_fixture().build())]).unwrap();
        let destination = root.path().join(release.id().as_ref());
        write(&destination, b"existing object").unwrap();
        assert!(matches!(save_release(root.path(), &release),
            Err(SaveReleaseError::Conflict { id }) if &id == release.id()));
        assert_eq!(read(destination).unwrap(), b"existing object");

        let error = save_release(&root.path().join("missing"), &release).unwrap_err();
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
}

/// Verifies filesystem scanning through its domain release contract.
#[cfg(test)]
mod tests {
    use std::fs::{create_dir, create_dir_all, read, remove_dir, write};

    use super::*;
    use crate::{
        domain::{ingestion::IngestTrackError, primitive::release::ReleaseValidationError},
        test_support::{MetadataFixture, release_metadata, sample_audio, sample_metadata},
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
    fn write_audio(root: &Path, source: &MetadataFixture) -> PathBuf {
        let raw = source.build();
        let id = release_metadata(&raw).unwrap().id().clone();
        let directory = root.join(id.as_ref());

        create_dir_all(&directory).unwrap();

        let path = directory.join(format!("{}.wav", source.audio_file().id()));
        write(&path, sample_audio(&raw)).unwrap();

        path
    }

    #[test]
    fn scan_releases_preserves_audio_files_and_escapes_delivery_paths() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("artist with spaces #?%");
        let source = sample_metadata();
        let path = write_audio(&nested, &MetadataFixture::from(&source));
        let original = read(&path).unwrap();

        for _ in 0..2 {
            let release = storage(temp.path())
                .scan_releases()
                .unwrap()
                .pop()
                .unwrap()
                .unwrap();

            assert_eq!(release.tracks().len(), 1);

            let audio = release.tracks()[0].original_audio_file();

            assert_eq!(audio.id(), MetadataFixture::from(&source).audio_file().id());
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

    #[test]
    fn scan_releases_reports_storage_keys_when_release_ids_are_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("invalid-release");

        create_dir(&directory).unwrap();

        write(directory.join("invalid-audio-id.wav"), b"corrupt").unwrap();

        let result = storage(temp.path()).scan_releases().unwrap().pop().unwrap();

        assert!(
            matches!(result, Err(ReleaseScanFailure::InvalidId { object_key, .. }) if object_key == "invalid-release")
        );
    }

    #[test]
    fn scan_releases_collects_all_audio_failures_with_their_release_id() {
        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        let path = write_audio(temp.path(), &MetadataFixture::from(&source));
        let directory = path.parent().unwrap();

        write(directory.join("invalid-audio-id.wav"), b"corrupt").unwrap();
        write(
            directory.join("229fc612-8a56-45d0-bf61-55b36ff3dcba.wav"),
            b"corrupt",
        )
        .unwrap();

        let mut different = MetadataFixture::from(&source);
        different.release_title_mut().push_str(" Other");
        *different.audio_file_mut().id_mut() =
            "925eda40-54cb-4af4-9579-6811fc7477ac".parse().unwrap();

        write(
            directory.join(format!("{}.wav", different.audio_file().id())),
            sample_audio(&different.build()),
        )
        .unwrap();
        write_audio(temp.path(), &different);

        let mut results = storage(temp.path()).scan_releases().unwrap();

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

    #[test]
    fn scan_releases_reports_release_construction_failures() {
        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        write_audio(temp.path(), &MetadataFixture::from(&source));

        let mut duplicate = MetadataFixture::from(&source);
        duplicate.release_title_mut().push(' ');
        *duplicate.audio_file_mut().id_mut() =
            "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();

        write_audio(temp.path(), &duplicate);

        let failure = storage(temp.path())
            .scan_releases()
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
            .unwrap()
            .pop()
            .unwrap()
            .unwrap_err();

        assert!(
            matches!(failure, ReleaseScanFailure::InvalidRelease { id: failed_id, source: ReleaseValidationError::Empty } if failed_id == id)
        );
    }

    #[test]
    fn scan_releases_groups_all_sources_with_the_same_release_id() {
        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        write_audio(&temp.path().join("a"), &MetadataFixture::from(&source));

        let mut other = MetadataFixture::from(&source);
        *other.track_number_mut() = "2".into();
        *other.audio_file_mut().id_mut() = "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();

        let second_path = write_audio(&temp.path().join("b"), &other);

        let mut results = storage(temp.path()).scan_releases().unwrap();

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

        let mut results = storage(temp.path()).scan_releases().unwrap();

        assert_eq!(results.len(), 1);

        assert!(
            matches!(results.pop().unwrap(), Err(ReleaseScanFailure::Audio { id, .. }) if &id == release_metadata(&source).unwrap().id())
        );
    }

    #[test]
    fn scan_releases_returns_an_error_when_storage_enumeration_fails() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("music");

        create_dir(&root).unwrap();

        let storage = storage(&root);

        remove_dir(&root).unwrap();

        assert!(storage.scan_releases().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn scan_releases_rejects_symbolic_links_without_following_them() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let source = sample_metadata();
        let path = write_audio(temp.path(), &MetadataFixture::from(&source));

        symlink(
            &path,
            path.parent()
                .unwrap()
                .join("229fc612-8a56-45d0-bf61-55b36ff3dcba.wav"),
        )
        .unwrap();

        let failure = storage(temp.path())
            .scan_releases()
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
}
