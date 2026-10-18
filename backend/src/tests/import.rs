//! Verifies release-level import persistence, restart safety, and index compatibility.

use crate::{
    domain::{AudioFileId, import::ImportRelease},
    infrastructure::{
        import_source::filesystem::FileSystemImportSource,
        music_storage::filesystem::FileSystemMusicStorage,
    },
    port::music_storage::{MusicStorage, SaveReleaseOutcome},
    test_support::{sample_audio, sample_fixture},
    usecase::{BuildMetadataIndexUseCase, import_music::ImportMusicUseCase},
};
use std::{fs, sync::Arc};

/// Writes arbitrary filenames beneath an import release directory.
fn write_source(root: &std::path::Path, group: &str, name: &str, bytes: &[u8]) {
    fs::create_dir_all(root.join(group)).unwrap();
    fs::write(root.join(group).join(name), bytes).unwrap();
}

/// Opens saved releases through the same adapter used at server startup.
fn read_releases(root: &std::path::Path) -> Vec<crate::domain::primitive::release::Release> {
    let storage = FileSystemMusicStorage::new(
        root.to_owned(),
        "https://example.com/audio/".parse().unwrap(),
    )
    .unwrap();
    BuildMetadataIndexUseCase::new(Arc::new(storage))
        .execute()
        .unwrap()
}

#[test]
fn import_music_preserves_bytes_issues_persistent_uuids_and_skips_existing_releases() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let first = sample_fixture();
    let mut second = sample_fixture();
    second.track_title = "Second track".into();
    second.track_number = "2".into();
    let mut third = sample_fixture();
    third.release_title = "Another release".into();
    let originals = [
        sample_audio(&first.build()),
        sample_audio(&second.build()),
        sample_audio(&third.build()),
    ];
    write_source(source.path(), "artist/album", "first.WAV", &originals[0]);
    write_source(source.path(), "artist/album", "second.wav", &originals[1]);
    write_source(source.path(), "artist/another", "third.wav", &originals[2]);
    let usecase = ImportMusicUseCase::new(
        Arc::new(FileSystemImportSource::new(source.path().to_owned()).unwrap()),
        Arc::new(FileSystemMusicStorage::for_import(destination.path().to_owned()).unwrap()),
    );
    let result = usecase.execute().unwrap();
    assert_eq!((result.saved, result.skipped, result.failed), (2, 0, 0));
    let releases = read_releases(destination.path());
    assert_eq!(releases.len(), 2);
    let mut identities = Vec::new();
    for release in &releases {
        for track in release.tracks() {
            let audio = track.original_audio_file();
            assert!(
                originals.contains(&fs::read(destination.path().join(audio.object_key())).unwrap())
            );
            let path = std::path::Path::new(audio.object_key());
            let id: AudioFileId = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
            assert_eq!(id, audio.id());
            assert!(identities.iter().all(|previous| previous != &id));
            identities.push(id);
        }
    }
    assert_eq!(identities.len(), 3);
    let result = usecase.execute().unwrap();
    assert_eq!((result.saved, result.skipped, result.failed), (0, 2, 0));
    let after = read_releases(destination.path());
    let after_ids: Vec<_> = after
        .iter()
        .flat_map(|release| release.tracks())
        .map(|track| track.original_audio_file().id())
        .collect();
    assert_eq!(identities, after_ids);
    assert_eq!(
        fs::read(source.path().join("artist/album/first.WAV")).unwrap(),
        originals[0]
    );
}

#[test]
fn import_music_continues_after_read_validation_and_save_failures_and_can_retry() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let mut blocked = sample_fixture();
    blocked.release_title = "Blocked".into();
    let blocked_bytes = sample_audio(&blocked.build());
    let blocked_release = ImportRelease::new(vec![blocked_bytes.clone()]).unwrap();
    let collision = destination.path().join(blocked_release.id().as_ref());
    fs::write(&collision, b"must not be overwritten").unwrap();
    write_source(source.path(), "a-save-failure", "audio.wav", &blocked_bytes);
    write_source(source.path(), "b-invalid", "audio.wav", b"invalid audio");
    let bytes = sample_audio(&sample_fixture().build());
    write_source(source.path(), "c-duplicate", "one.wav", &bytes);
    write_source(source.path(), "c-duplicate", "two.wav", &bytes);
    write_source(source.path(), "d-success", "audio.wav", &bytes);
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        source.path().join("missing"),
        source.path().join("d-success/broken.wav"),
    )
    .unwrap();

    let usecase = ImportMusicUseCase::new(
        Arc::new(FileSystemImportSource::new(source.path().to_owned()).unwrap()),
        Arc::new(FileSystemMusicStorage::for_import(destination.path().to_owned()).unwrap()),
    );
    let result = usecase.execute().unwrap();
    #[cfg(unix)]
    assert_eq!((result.saved, result.skipped, result.failed), (0, 0, 4));
    #[cfg(not(unix))]
    assert_eq!((result.saved, result.skipped, result.failed), (1, 0, 3));
    assert_eq!(fs::read(&collision).unwrap(), b"must not be overwritten");
    assert!(read_releases(destination.path()).len() <= 1);

    fs::remove_file(collision).unwrap();
    fs::remove_dir_all(source.path().join("b-invalid")).unwrap();
    fs::remove_file(source.path().join("c-duplicate/two.wav")).unwrap();
    #[cfg(unix)]
    fs::remove_file(source.path().join("d-success/broken.wav")).unwrap();
    let result = usecase.execute().unwrap();
    assert_eq!((result.saved, result.skipped, result.failed), (2, 1, 0));
    assert_eq!(read_releases(destination.path()).len(), 2);
}

#[test]
fn music_storage_serializes_concurrent_saves_and_hides_incomplete_staging() {
    let destination = tempfile::tempdir().unwrap();
    let bytes = sample_audio(&sample_fixture().build());
    let partial = destination.path().join(".tonhalle-import/interrupted");
    fs::create_dir_all(&partial).unwrap();
    fs::write(partial.join("partial.wav"), &bytes).unwrap();
    assert!(read_releases(destination.path()).is_empty());
    let storage =
        Arc::new(FileSystemMusicStorage::for_import(destination.path().to_owned()).unwrap());
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let storage = storage.clone();
            let bytes = bytes.clone();
            std::thread::spawn(move || {
                storage
                    .save_release(&ImportRelease::new(vec![bytes]).unwrap())
                    .unwrap()
            })
        })
        .collect();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
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
    assert_eq!(read_releases(destination.path()).len(), 1);
}
