//! Verifies the executable's command and environment configuration contract.

use std::{
    env::vars_os,
    fs::{copy, create_dir, read_dir, remove_dir_all, write},
    path::Path,
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("--music-storage--root-path"));
    assert!(!run(&["server"], &[]).status.success());
}

#[test]
fn server_cli_overrides_environment_before_validation() {
    let root = tempfile::tempdir().unwrap();
    let env = [
        (
            "TONHALLE_MUSIC_STORAGE__ROOT_PATH",
            root.path().to_str().unwrap(),
        ),
        (
            "TONHALLE_MUSIC_STORAGE__PLAYBACK_URL_BASE",
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

/// Counts published audio files for the executable smoke test.
fn saved_audio_count(root: &Path) -> usize {
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
        .count()
}

#[test]
fn import_cli_reports_saved_and_skipped_release_counts() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sample");
    let destination = tempfile::tempdir().unwrap();
    let args = [
        "import",
        "--import-source--root-path",
        source.to_str().unwrap(),
        "--music-storage--root-path",
        destination.path().to_str().unwrap(),
    ];
    let first = run(&args, &[]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(String::from_utf8_lossy(&first.stdout).contains("saved: 3, skipped: 0, failed: 0"));
    assert_eq!(saved_audio_count(destination.path()), 6);
    let retry = run(&args, &[]);
    assert!(retry.status.success());
    assert!(String::from_utf8_lossy(&retry.stdout).contains("saved: 0, skipped: 3, failed: 0"));
}

#[test]
fn import_cli_continues_after_invalid_release_and_reports_nonzero_status() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    create_dir(source.path().join("a-invalid")).unwrap();
    write(source.path().join("a-invalid/audio.wav"), b"invalid").unwrap();
    create_dir(source.path().join("b-valid")).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sample/release-2c1bb1cd9fcd50dd123abf124fefd09c111b08bb4b8453a4c232073cdbfda16e/f8c2a61f-a7ec-4fdb-b154-44f38aa0d7c7.mp3");
    copy(fixture, source.path().join("b-valid/track.mp3")).unwrap();
    let env = [
        (
            "TONHALLE_IMPORT_SOURCE__ROOT_PATH",
            source.path().to_str().unwrap(),
        ),
        (
            "TONHALLE_MUSIC_STORAGE__ROOT_PATH",
            destination.path().to_str().unwrap(),
        ),
    ];
    let result = run(&["import"], &env);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("saved: 1, skipped: 0, failed: 1"));
    assert_eq!(saved_audio_count(destination.path()), 1);
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
            "--music-storage--root-path",
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
    let server = run(&["server", "--music-storage--root-path", root], &[]);
    assert!(!server.status.success());
    assert!(
        String::from_utf8_lossy(&server.stderr)
            .contains("MusicStorageConfig.playback_url_base is required")
    );
    let import_args = [
        "import",
        "--import-source--root-path",
        source.path().to_str().unwrap(),
        "--music-storage--root-path",
        root,
    ];
    assert!(run(&import_args, &[]).status.success());
    let mut with_url = import_args.to_vec();
    with_url.extend([
        "--music-storage--playback-url-base",
        "https://example.com/audio",
    ]);
    assert!(run(&with_url, &[]).status.success());
    *with_url.last_mut().unwrap() = "ftp://example.com/";
    let invalid = run(&with_url, &[]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("playback URL base"));
}

#[test]
fn storage_modes_validate_selected_settings_and_reject_unknown_modes() {
    let missing = run(&["import", "--music-storage--mode", "filesystem"], &[]);
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("implementation settings are required")
    );

    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let env = [
        ("TONHALLE_IMPORT_SOURCE__MODE", "filesystem"),
        ("TONHALLE_MUSIC_STORAGE__MODE", "filesystem"),
        (
            "TONHALLE_IMPORT_SOURCE__ROOT_PATH",
            source.path().to_str().unwrap(),
        ),
        (
            "TONHALLE_MUSIC_STORAGE__ROOT_PATH",
            destination.path().to_str().unwrap(),
        ),
    ];
    assert!(run(&["import"], &env).status.success());
    assert!(
        run(
            &[
                "import",
                "--import-source--mode",
                "filesystem",
                "--music-storage--mode",
                "filesystem"
            ],
            &env
        )
        .status
        .success()
    );
    for flag in ["--import-source--mode", "--music-storage--mode"] {
        let invalid = run(&["import", flag, "unknown"], &env);
        assert!(!invalid.status.success());
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("invalid value"));
    }

    let missing_source = run(
        &[
            "import",
            "--music-storage--root-path",
            destination.path().to_str().unwrap(),
        ],
        &[],
    );
    assert!(!missing_source.status.success());
    assert!(
        String::from_utf8_lossy(&missing_source.stderr).contains("selected import source mode")
    );
}

#[test]
fn server_authorization_requires_provider_settings_and_allows_cli_disable() {
    let root = tempfile::tempdir().unwrap();
    let env = [
        (
            "TONHALLE_MUSIC_STORAGE__ROOT_PATH",
            root.path().to_str().unwrap(),
        ),
        ("TONHALLE_AUTHORIZATION__ENABLED", "true"),
    ];
    let output = run(&["server"], &env);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("authorization.issuer"));
    let output = run(&["server", "--authorization-enabled", "false"], &env);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(!stderr.contains("authorization.issuer"), "{stderr}");
    assert!(stderr.contains("playback_url_base"), "{stderr}");
}

#[test]
fn content_access_cli_selects_provider_and_validates_cloudfront_settings() {
    let output = run(
        &["server"],
        &[("TONHALLE_CONTENT_ACCESS__MODE", "cloud-front")],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("key_pair_id is required"));
    let output = run(
        &[
            "server",
            "--content-access--mode",
            "nop",
            "--cors-origin",
            "*",
        ],
        &[("TONHALLE_CONTENT_ACCESS__MODE", "cloud-front")],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CORS origin"), "{stderr}");
    assert!(!stderr.contains("key_pair_id"));
    let output = run(
        &["server", "--content-access--mode", "cloud-front"],
        &[
            (
                "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__KEY_PAIR_ID",
                "test-key",
            ),
            (
                "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__PRIVATE_KEY",
                "opaque-private-key",
            ),
            (
                "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__RESOURCE_URL",
                "https://media.example.com/*",
            ),
            (
                "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__VALIDITY_SECONDS",
                "0",
            ),
        ],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("validity_seconds must be greater than zero")
    );
}

#[test]
fn import_check_cli_lists_releases_without_destination_configuration() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sample");
    let output = run(
        &[
            "import-check",
            "--import-source--root-path",
            source.to_str().unwrap(),
        ],
        &[("TONHALLE_MUSIC_STORAGE__MODE", "invalid")],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let releases = stdout
        .lines()
        .filter(|line| line.starts_with("release-"))
        .collect::<Vec<_>>();
    assert_eq!(releases.len(), 3, "{stdout}");
    assert!(releases.iter().all(|line| {
        line.split_once('\t')
            .is_some_and(|(_, title)| !title.is_empty())
    }));
    let help = run(&["import-check", "--help"], &[]);
    assert!(help.status.success());
    assert!(!String::from_utf8_lossy(&help.stdout).contains("--music-storage"));
    assert!(!run(&["import-check"], &[]).status.success());
}

#[test]
fn import_check_cli_lists_valid_releases_and_fails_when_another_candidate_is_invalid() {
    let source = tempfile::tempdir().unwrap();
    create_dir(source.path().join("invalid")).unwrap();
    write(source.path().join("invalid/audio.wav"), b"invalid").unwrap();
    create_dir(source.path().join("valid")).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sample/release-2c1bb1cd9fcd50dd123abf124fefd09c111b08bb4b8453a4c232073cdbfda16e/f8c2a61f-a7ec-4fdb-b154-44f38aa0d7c7.mp3");
    copy(fixture, source.path().join("valid/track.mp3")).unwrap();
    let output = run(
        &["import-check"],
        &[(
            "TONHALLE_IMPORT_SOURCE__ROOT_PATH",
            source.path().to_str().unwrap(),
        )],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("release-2c1bb1cd9fcd50dd123abf124fefd09c111b08bb4b8453a4c232073cdbfda16e\t")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 release checks failed"));
}
