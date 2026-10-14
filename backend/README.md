# Backend

## Common

Run `tonhalle-backend server` to start the API server. A subcommand is required.
Use `tonhalle-backend server --help` to list server options.

CLI arguments take precedence over environment variables, followed by defaults.
Argument names omit `TONHALLE_`, use lowercase, and replace each underscore with a hyphen.

| Variable | CLI argument | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `TONHALLE_LISTEN_ADDRESS` | `--listen-address` | No | `0.0.0.0` | IP address to bind. |
| `TONHALLE_LISTEN_PORT` | `--listen-port` | No | `3000` | TCP port to bind. |

The container starts the `server` subcommand by default.

## Cross-Origin Resource Sharing

CORS is disabled when no origin is provided.

| Variable | CLI argument | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `TONHALLE_CORS_ORIGIN` | `--cors-origin` | No | - | A single HTTP(S) browser origin without a path or trailing slash. |

## Content Storage

Provide required settings through CLI arguments or environment variables.

| Variable | CLI argument | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `TONHALLE_CONTENT_STORAGE__ROOT_PATH` | `--content-storage--root-path` | Yes | - | Existing audio directory scanned at startup. Mount it when using a container. |
| `TONHALLE_CONTENT_STORAGE__PLAYBACK_URL_BASE` | `--content-storage--playback-url-base` | Yes | - | HTTP(S) directory URL for playback links; query strings and fragments are rejected. |

## Logging

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `RUST_LOG` | No | `info` | Tracing filter for JSON logs; invalid filters fall back to `info`. |
