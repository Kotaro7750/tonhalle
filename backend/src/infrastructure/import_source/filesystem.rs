//! Groups filesystem audio by containing directory without modifying sources.

use std::{
    collections::BTreeMap,
    fs::{read, symlink_metadata},
    io::{Error as IoError, ErrorKind},
    path::{Component, Path, PathBuf, StripPrefixError},
};
use tokio::task::spawn_blocking;

use crate::{
    domain::primitive::audio_file::MediaType,
    port::import_source::{
        ImportCandidate, ImportSource, ListImportReleasesError, ReadImportAudioError,
    },
};

/// A failure to construct a filesystem import source.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileSystemImportSourceConstructionError {
    /// The configured root cannot be resolved.
    #[error("resolving import source root: {0}")]
    ResolveRoot(#[from] IoError),
    /// The configured root does not identify a directory.
    #[error("import source must be a directory")]
    NotDirectory,
}

/// A physical enumeration failure before all release candidates are known.
#[derive(Debug, thiserror::Error)]
enum FileSystemImportEnumerationError {
    /// Filesystem traversal could not finish.
    #[error("traversing import source: {0}")]
    Traverse(#[from] walkdir::Error),
    /// An enumerated object cannot be addressed within the source.
    #[error("resolving import object location: {0}")]
    OutsideRoot(#[from] StripPrefixError),
    /// An enumerated locator cannot be represented as text.
    #[error("non-UTF-8 import path")]
    NonUtf8Path,
}

/// A read-only directory tree containing release directories.
#[derive(Clone)]
pub(crate) struct FileSystemImportSource {
    root: PathBuf,
}

impl FileSystemImportSource {
    /// Resolves the source root and rejects missing or non-directory inputs.
    pub(crate) fn new(root: PathBuf) -> Result<Self, FileSystemImportSourceConstructionError> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(FileSystemImportSourceConstructionError::NotDirectory);
        }
        Ok(Self { root })
    }
}

/// Recognizes supported container filename extensions without treating names as metadata.
pub(crate) fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(MediaType::from_extension)
        .is_some()
}

impl FileSystemImportSource {
    /// Enumerates filesystem candidates while retaining physical failure details.
    fn enumerate_releases(&self) -> Result<Vec<ImportCandidate>, FileSystemImportEnumerationError> {
        let mut groups = BTreeMap::<String, Vec<String>>::new();
        for entry in walkdir::WalkDir::new(&self.root)
            .sort_by_file_name()
            .follow_links(false)
        {
            let entry = entry?;
            if entry.file_type().is_dir() || !is_audio_path(entry.path()) {
                continue;
            }
            let relative = entry.path().strip_prefix(&self.root)?;
            let key = relative
                .to_str()
                .ok_or(FileSystemImportEnumerationError::NonUtf8Path)?
                .to_owned();
            let parent = relative
                .parent()
                .unwrap_or(Path::new(""))
                .to_str()
                .ok_or(FileSystemImportEnumerationError::NonUtf8Path)?
                .to_owned();
            groups.entry(parent).or_default().push(key);
        }
        Ok(groups
            .into_iter()
            .map(|(key, audio_keys)| ImportCandidate::new(key, audio_keys))
            .collect())
    }
}

#[async_trait::async_trait]
impl ImportSource for FileSystemImportSource {
    /// Treats each directory containing audio as one release and scans nested directories.
    async fn list_releases(&self) -> Result<Vec<ImportCandidate>, ListImportReleasesError> {
        let source = self.clone();
        spawn_blocking(move || source.enumerate_releases())
            .await
            .map_err(ListImportReleasesError::new)?
            .map_err(ListImportReleasesError::new)
    }

    /// Reads source bytes without blocking an asynchronous runtime worker.
    async fn read_audio(&self, key: &str) -> Result<Vec<u8>, ReadImportAudioError> {
        let source = self.clone();
        let audio_key = key.to_owned();
        spawn_blocking(move || source.read_audio_blocking(&audio_key))
            .await
            .map_err(|source| ReadImportAudioError::Internal {
                key: key.to_owned(),
                source: Box::new(source),
            })?
    }
}

impl FileSystemImportSource {
    /// Rejects links and paths outside the root before reading original bytes.
    fn read_audio_blocking(&self, key: &str) -> Result<Vec<u8>, ReadImportAudioError> {
        let path = Path::new(key);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ReadImportAudioError::InvalidKey {
                key: key.to_owned(),
            });
        }
        let path = self.root.join(path);
        if !symlink_metadata(&path)
            .map_err(|source| read_audio_error(key, source))?
            .is_file()
            || !path
                .canonicalize()
                .map_err(|source| read_audio_error(key, source))?
                .starts_with(&self.root)
        {
            return Err(ReadImportAudioError::InvalidObject {
                key: key.to_owned(),
            });
        }
        read(path).map_err(|source| read_audio_error(key, source))
    }
}

/// Maps filesystem read failures to source-port semantics without losing their cause.
fn read_audio_error(key: &str, source: IoError) -> ReadImportAudioError {
    if source.kind() == ErrorKind::NotFound {
        ReadImportAudioError::NotFound {
            key: key.to_owned(),
            source: Box::new(source),
        }
    } else {
        ReadImportAudioError::Internal {
            key: key.to_owned(),
            source: Box::new(source),
        }
    }
}

/// Verifies semantic read errors and retained causes at the source-port boundary.
#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs::create_dir,
        io::{Error as IoError, ErrorKind},
    };

    use super::*;

    #[tokio::test]
    async fn read_audio_distinguishes_invalid_keys_missing_objects_and_invalid_objects() {
        let root = tempfile::tempdir().unwrap();
        let source = FileSystemImportSource::new(root.path().to_owned()).unwrap();
        assert!(matches!(source.read_audio("../outside.wav").await,
            Err(ReadImportAudioError::InvalidKey { key }) if key == "../outside.wav"));
        let missing = source.read_audio("missing.wav").await.unwrap_err();
        assert!(
            matches!(&missing, ReadImportAudioError::NotFound { key, .. } if key == "missing.wav")
        );
        assert_eq!(
            missing
                .source()
                .unwrap()
                .downcast_ref::<IoError>()
                .unwrap()
                .kind(),
            ErrorKind::NotFound
        );
        create_dir(root.path().join("directory.wav")).unwrap();
        assert!(matches!(source.read_audio("directory.wav").await,
            Err(ReadImportAudioError::InvalidObject { key }) if key == "directory.wav"));
    }

    #[tokio::test]
    async fn list_releases_preserves_enumeration_failure_as_its_source() {
        let root = tempfile::tempdir().unwrap();
        let source = FileSystemImportSource::new(root.path().to_owned()).unwrap();
        root.close().unwrap();
        let error = source
            .list_releases()
            .await
            .err()
            .expect("a missing root cannot be enumerated");
        let cause = error.source().unwrap();
        assert!(matches!(
            cause.downcast_ref::<FileSystemImportEnumerationError>(),
            Some(FileSystemImportEnumerationError::Traverse(_))
        ));
        assert!(cause.source().is_some());
    }
    #[tokio::test]
    async fn list_releases_groups_direct_audio_children_and_read_audio_preserves_bytes() {
        let root = tempfile::tempdir().unwrap();
        for (path, bytes) in [
            ("artist/album/one.WAV", b"first".as_slice()),
            ("artist/album/two.flac", b"second".as_slice()),
            ("artist/album/disc/tone.mp3", b"nested".as_slice()),
            ("artist/album/notes.txt", b"ignored".as_slice()),
        ] {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let source = FileSystemImportSource::new(root.path().to_owned()).unwrap();
        let candidates = source.list_releases().await.unwrap();
        let groups: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.audio_keys().clone())
            .collect();
        assert_eq!(
            groups,
            vec![
                vec!["artist/album/one.WAV", "artist/album/two.flac"],
                vec!["artist/album/disc/tone.mp3"]
            ]
        );
        assert_eq!(source.read_audio(&groups[0][0]).await.unwrap(), b"first");
        assert_eq!(source.read_audio(&groups[1][0]).await.unwrap(), b"nested");
        assert_eq!(
            std::fs::read(root.path().join("artist/album/one.WAV")).unwrap(),
            b"first"
        );
    }
}
