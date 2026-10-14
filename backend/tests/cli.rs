//! Verifies the executable's command and environment configuration contract.

use std::process::{Command, Output};

/// Runs the executable with isolated application configuration.
fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tonhalle-backend"));
    for (name, _) in std::env::vars_os() {
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
