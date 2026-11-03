# Backend

## Usage

### API Server

Start the API server with the settings for the selected music-storage implementation.
The following example uses `filesystem`:

```sh
tonhalle-backend server \
  --music-storage--mode filesystem \
  --music-storage--root-path /path/to/storage \
  --music-storage--playback-url-base https://example.com/audio/
```

The server builds the metadata index from stored audio at startup.
Audio is delivered through a separate HTTP endpoint.

The container starts the `server` subcommand by default.

### Import

Import audio from the selected import-source implementation into the selected music-storage implementation.
The two implementations are selected independently.
The following example uses `filesystem` for both:

```sh
tonhalle-backend import \
  --import-source--mode filesystem \
  --music-storage--mode filesystem \
  --import-source--root-path /path/to/source \
  --music-storage--root-path /path/to/storage
```

Each release is validated and saved as a whole.
Existing releases are skipped without being overwritten.
If a release fails, the remaining releases are still processed.
The command prints `saved`, `skipped`, and `failed` counts and exits with a nonzero status if any release failed.

Import does not update the metadata index.
Start or restart the API server after importing to rebuild the index.

### Command Help

A subcommand is required.
Use `tonhalle-backend server --help` or `tonhalle-backend import --help` to list command options.
When running from source, use `cargo run --locked --` in the `backend/` directory in place of `tonhalle-backend`.

## Configuration

### Common

Provide settings through CLI arguments or environment variables.
CLI arguments take precedence over environment variables, followed by defaults.
Argument names omit `TONHALLE_`, use lowercase, and replace each underscore with a hyphen.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_LISTEN_ADDRESS` | `--listen-address` | `server` | No | `0.0.0.0` | IP address to bind. |
| `TONHALLE_LISTEN_PORT` | `--listen-port` | `server` | No | `3000` | TCP port to bind. |

### Cross-Origin Resource Sharing

CORS is disabled when no origin is provided.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_CORS_ORIGIN` | `--cors-origin` | `server` | No | - | A single HTTP(S) browser origin without a path or trailing slash. |

### Music Storage

#### Common

Select the music-storage implementation with `--music-storage--mode`.
Currently, `filesystem` is supported.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_MUSIC_STORAGE__MODE` | `--music-storage--mode` | `server`, `import` | No | `filesystem` | Music-storage implementation. |

#### Filesystem

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_MUSIC_STORAGE__ROOT_PATH` | `--music-storage--root-path` | `server`, `import` | Yes | - | Existing filesystem directory scanned by `server` and written to by `import`. Mount it when using a container. |
| `TONHALLE_MUSIC_STORAGE__PLAYBACK_URL_BASE` | `--music-storage--playback-url-base` | `server`, `import` | Only for `server` | - | HTTP(S) base URL for playback links; query strings and fragments are rejected. |

### Import Source

#### Common

Select the import-source implementation with `--import-source--mode`.
Currently, `filesystem` is supported.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_IMPORT_SOURCE__MODE` | `--import-source--mode` | `import` | No | `filesystem` | Import-source implementation. |

#### Filesystem

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_IMPORT_SOURCE__ROOT_PATH` | `--import-source--root-path` | `import` | Yes | - | Existing filesystem source directory tree, read without modification. |

### Logging

| Variable | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `RUST_LOG` | `server`, `import` | No | `info` | Tracing filter for JSON logs; invalid filters fall back to `info`. |
