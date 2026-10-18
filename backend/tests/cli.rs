//! Verifies the executable's command and environment configuration contract.

use std::{
    collections::BTreeMap,
    env::vars_os,
    fs::{copy, create_dir, read, read_dir, remove_dir_all, write},
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Runs the executable with isolated application configuration.
fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tonhalle-backend"));
    for (name, _) in vars_os() {
        if name.to_string_lossy().starts_with("TONHALLE_") {
            command.env_remove(name);
        }
    }
    command
        .args(args)
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

#[test]
fn cli_requires_a_subcommand_and_accepts_help_without_settings() {
    assert!(!run(&[], &[]).status.success());
    let output = run(&["server", "--help"], &[]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("--content-storage--root-path"));
    assert!(!run(&["server"], &[]).status.success());
}

#[test]
fn server_cli_overrides_environment_before_validation() {
    let root = tempfile::tempdir().unwrap();
    let env = [
        (
            "TONHALLE_CONTENT_STORAGE__ROOT_PATH",
            root.path().to_str().unwrap(),
        ),
        (
            "TONHALLE_CONTENT_STORAGE__PLAYBACK_URL_BASE",
            "http://localhost:8000/",
        ),
        ("TONHALLE_LISTEN_PORT", "invalid"),
        ("TONHALLE_CORS_ORIGIN", "*"),
    ];
    let output = run(&["server"], &env);
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid digit"));
    let output = run(&["server", "--listen-port", "0"], &env);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CORS origin"), "{stderr}");
    assert!(!stderr.contains("invalid digit"));
}

/// Returns a stable snapshot of published filenames and unchanged audio bytes.
fn saved_audio(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file() && entry.path().parent() != Some(root))
        .filter(|entry| {
            !entry
                .path()
                .components()
                .any(|part| part.as_os_str() == ".tonhalle-import")
        })
        .map(|entry| {
            (
                entry.path().strip_prefix(root).unwrap().to_owned(),
                read(entry.path()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn import_cli_imports_multiple_releases_and_retries_without_changing_saved_audio() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sample");
    let destination = tempfile::tempdir().unwrap();
    let args = [
        "import",
        "--import-source--root-path",
        source.to_str().unwrap(),
        "--content-storage--root-path",
        destination.path().to_str().unwrap(),
    ];
    let first = run(&args, &[]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(String::from_utf8_lossy(&first.stdout).contains("saved: 3, skipped: 0, failed: 0"));
    let files = saved_audio(destination.path());
    assert_eq!(files.len(), 6);
    for path in files.keys() {
        let id = uuid::Uuid::parse_str(path.file_stem().unwrap().to_str().unwrap()).unwrap();
        assert_eq!(id.get_version_num(), 4);
    }
    let retry = run(&args, &[]);
    assert!(retry.status.success());
    assert!(String::from_utf8_lossy(&retry.stdout).contains("saved: 0, skipped: 3, failed: 0"));
    assert_eq!(files, saved_audio(destination.path()));
}

#[test]
fn import_cli_continues_after_invalid_release_and_reports_nonzero_status() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    create_dir(source.path().join("a-invalid")).unwrap();
    write(source.path().join("a-invalid/audio.wav"), b"invalid").unwrap();
    create_dir(source.path().join("b-valid")).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sample/release-1ac1508956628bfc1927b6fde571efbd604820dbacdeb82ca5d5d58eafe76be7/f8c2a61f-a7ec-4fdb-b154-44f38aa0d7c7.mp3");
    copy(fixture, source.path().join("b-valid/track.mp3")).unwrap();
    let env = [
        (
            "TONHALLE_IMPORT_SOURCE__ROOT_PATH",
            source.path().to_str().unwrap(),
        ),
        (
            "TONHALLE_CONTENT_STORAGE__ROOT_PATH",
            destination.path().to_str().unwrap(),
        ),
    ];
    let result = run(&["import"], &env);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("saved: 1, skipped: 0, failed: 1"));
    assert_eq!(saved_audio(destination.path()).len(), 1);
    remove_dir_all(source.path().join("a-invalid")).unwrap();
    let result = run(&["import"], &env);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("saved: 0, skipped: 1, failed: 0"));
}

#[test]
fn import_cli_rejects_overlapping_roots_before_modifying_source() {
    let source = tempfile::tempdir().unwrap();
    let nested = source.path().join("destination");
    create_dir(&nested).unwrap();
    let result = run(
        &[
            "import",
            "--import-source--root-path",
            source.path().to_str().unwrap(),
            "--content-storage--root-path",
            nested.to_str().unwrap(),
        ],
        &[],
    );
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("must not overlap"));
    assert_eq!(read_dir(nested).unwrap().count(), 0);
}

#[test]
fn storage_config_accepts_optional_playback_url_but_server_requires_it() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let root = destination.path().to_str().unwrap();
    let server = run(&["server", "--content-storage--root-path", root], &[]);
    assert!(!server.status.success());
    assert!(
        String::from_utf8_lossy(&server.stderr)
            .contains("content-storage--playback-url-base is required")
    );
    let import_args = [
        "import",
        "--import-source--root-path",
        source.path().to_str().unwrap(),
        "--content-storage--root-path",
        root,
    ];
    assert!(run(&import_args, &[]).status.success());
    let mut with_url = import_args.to_vec();
    with_url.extend([
        "--content-storage--playback-url-base",
        "https://example.com/audio",
    ]);
    assert!(run(&with_url, &[]).status.success());
    *with_url.last_mut().unwrap() = "ftp://example.com/";
    let invalid = run(&with_url, &[]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("playback URL base"));
}
