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

The server accepts requests while scanning stored audio in the background.
Validated releases become available as they are indexed, so listings may be incomplete during the scan.
Rejected releases are logged and skipped; a scan failure leaves already indexed releases available.
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

Audio is read and validated one file at a time, then staged under its release ID without accumulating audio bytes in memory.
A release is published only after all of its audio passes validation and staging.
After the first read, validation, or staging failure in a release, remaining audio is still checked but is not staged.
Temporary audio is deleted after all candidates have been processed; cleanup failures make the command fail.
Interrupted imports may leave temporary audio, which is cleared before staging that release again.
Run only one import at a time against a destination.
Existing releases are skipped without being overwritten.
If a release fails, the remaining releases are still processed.
The command prints `saved`, `skipped`, and `failed` counts and exits with a nonzero status if any release failed.

Import does not update the metadata index.
Start or restart the API server after importing to rebuild the index.

Add `--dry-run` to check source releases and destination save conditions without writing files or objects.
This option still requires music-storage configuration and reports `saved` counts as if the releases had been saved.

### Import Check

Read and validate source releases without configuring or accessing music storage:

```sh
tonhalle-backend import-check \
  --import-source--mode filesystem \
  --import-source--root-path /path/to/source
```

The command uses the same source reading and domain validation as `import`.
It prints each valid release's ID and title, separated by a tab.
Both `import-check` and `import` reserve a release ID when the first valid audio in a candidate identifies it.
Later candidates with that ID fail as duplicates, even if the earlier candidate subsequently fails.
If a release fails validation or cannot be read, the remaining releases are still checked and listed, and the command exits with a nonzero status.
It does not check whether releases already exist at a destination.

### Command Help

A subcommand is required.
Use `tonhalle-backend server --help`, `tonhalle-backend import --help`, or `tonhalle-backend import-check --help` to list command options.
When running from source, use `cargo run --locked --` in the `backend/` directory in place of `tonhalle-backend`.

## Configuration

### Common

Provide settings through CLI arguments or environment variables.
CLI arguments take precedence over environment variables, followed by defaults.
Unless listed otherwise below, argument names omit `TONHALLE_`, use lowercase, and replace each underscore with a hyphen.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_LISTEN_ADDRESS` | `--listen-address` | `server` | No | `0.0.0.0` | IP address to bind. |
| `TONHALLE_LISTEN_PORT` | `--listen-port` | `server` | No | `3000` | TCP port to bind. |

### Cross-Origin Resource Sharing

CORS is disabled when no origin is provided.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_CORS_ORIGIN` | `--cors-origin` | `server` | No | - | A single HTTP(S) browser origin without a path or trailing slash. |

### OAuth 2.0 Authorization

Authorization is disabled by default.
When enabled, all routes under `/api/v0` require an OAuth 2.0 Bearer access token; `/health` remains public.
Tokens are issued by an external identity provider and validated using RS256 signatures, the configured issuer and audience, and expiration.
Signing keys are retrieved from the provider's JWKS endpoint at startup and refreshed hourly.
The server fails to start if required authorization settings are missing or the initial JWKS retrieval fails.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_AUTHORIZATION__ENABLED` | `--authorization-enabled` | `server` | No | `false` | Enables access-token validation; accepts `true` or `false`. |
| `TONHALLE_AUTHORIZATION__ISSUER` | `--authorization-issuer` | `server` | When enabled | - | Required token issuer (`iss`). |
| `TONHALLE_AUTHORIZATION__AUDIENCE` | `--authorization-audience` | `server` | When enabled | - | Required token audience (`aud`). |
| `TONHALLE_AUTHORIZATION__JWKS_URL` | `--authorization-jwks-url` | `server` | When enabled | - | Provider's JWKS endpoint URL. |

Missing or invalid tokens return `401 Unauthorized`.
For Cognito tokens, `token_use` must be `access`; tokens without this claim are also accepted.
Endpoint-specific scope or role checks are not configured.
For cross-origin browser requests, also configure `TONHALLE_CORS_ORIGIN`; CORS permits the `Authorization` header.
Audio delivery uses a separate endpoint, whose access control must be configured independently.

### Content Access

Delivery access defaults to `nop` and is independent of the music-storage implementation.
`POST /api/v0/content-access` issues delivery credentials and returns their renewal deadline as `invalidAfter`.
`DELETE /api/v0/content-access` clears the client cookies and returns `204`.
With `nop`, POST returns `{}` and neither operation sets cookies.
Both routes use the server's existing Bearer-token policy when authorization is enabled.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_CONTENT_ACCESS__MODE` | `--content-access--mode` | `server` | No | `nop` | `nop` or `cloud-front`. |
| `TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__KEY_PAIR_ID` | `--content-access--cloud-front--key-pair-id` | `server` | For CloudFront | - | Public-key ID trusted by the CloudFront distribution. |
| `TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__PRIVATE_KEY` | `--content-access--cloud-front--private-key` | `server` | For CloudFront | - | PKCS#8 PEM private key matching the public key. |
| `TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__RESOURCE_URL` | `--content-access--cloud-front--resource-url` | `server` | For CloudFront | - | Signed resource pattern, such as `https://media.example.com/music/*`. |
| `TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__VALIDITY_SECONDS` | `--content-access--cloud-front--validity-seconds` | `server` | For CloudFront | - | Positive policy lifetime in seconds. |
| `TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__COOKIE_DOMAIN` | `--content-access--cloud-front--cookie-domain` | `server` | No | - | Shared parent domain for the API and delivery hosts; omitted cookies are host-only. |
| `TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__COOKIE_PATH` | `--content-access--cloud-front--cookie-path` | `server` | No | `/` | Cookie path matching the delivery path. |

CloudFront mode sets `CloudFront-Policy`, `CloudFront-Signature`, and `CloudFront-Key-Pair-Id` cookies with `Secure`, `HttpOnly`, and `SameSite=Lax`.
Clients treat these cookies as opaque and renew access before `invalidAfter`.
DELETE removes browser cookies; it does not revoke copies of previously issued credentials before policy expiration.
Credential responses use `Cache-Control: no-store`.

Configure HTTPS and API/delivery hostnames that can share the cookie domain.
For cross-origin browser calls, configure `TONHALLE_CORS_ORIGIN` and send requests with `credentials: "include"`; the API allows credentialed requests from that exact origin.
CloudFront trusted keys and delivery-side CORS remain deployment configuration.
This feature provides the backend endpoints; clients must call them and renew credentials when needed.

### Music Storage

#### Common

Select the music-storage implementation with `--music-storage--mode`.
Supported values are `filesystem` and `s3`.
Playback settings are shared by all implementations.
Implementation-specific settings are required and validated only for the selected implementation.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_MUSIC_STORAGE__MODE` | `--music-storage--mode` | `server`, `import` | No | `filesystem` | Music-storage implementation. |
| `TONHALLE_MUSIC_STORAGE__PLAYBACK_URL_BASE` | `--music-storage--playback-url-base` | `server`, `import` | Only for `server` | - | HTTP(S) delivery directory corresponding to the filesystem root or S3 bucket root; query strings and fragments are rejected. |

#### Filesystem

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_MUSIC_STORAGE__ROOT_PATH` | `--music-storage--root-path` | `server`, `import` | Yes | - | Existing filesystem directory scanned by `server` and written to by `import`. Mount it when using a container. |

#### S3

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_MUSIC_STORAGE__S3__REGION` | `--music-storage--s3--region` | `server`, `import` | Yes | - | AWS region for S3 requests. |
| `TONHALLE_MUSIC_STORAGE__S3__BUCKET_NAME` | `--music-storage--s3--bucket-name` | `server`, `import` | Yes | - | Existing bucket holding audio objects. |
| `TONHALLE_MUSIC_STORAGE__S3__PREFIX` | `--music-storage--s3--prefix` | `server`, `import` | No | Empty | Object-key prefix without leading/trailing slashes, empty segments, or `.` / `..` segments. |

Credentials are loaded through the AWS SDK's standard credential provider chain, such as environment variables, shared AWS profiles, or IAM roles.
The bucket must allow listing objects, reading audio for `server`, and uploading, copying, and deleting temporary audio for `import` (including the read permission required for copying).

Audio objects are stored at `{prefix}/{ReleaseId}/{AudioFileId}.{extension}`; an empty prefix omits the initial `{prefix}/`.
Playback URLs append the complete object key to the configured playback URL base.
For example, prefix `music` and base `https://example.com/` produce `https://example.com/music/{ReleaseId}/{AudioFileId}.{extension}`.
Configure the delivery endpoint to serve those object keys.

Temporary objects are stored at `{prefix}/.tonhalle-import/{ReleaseId}/{AudioFileId}.{extension}` and excluded from scans.
Publication copies them to the regular release prefix, preserving their audio IDs and content types.
S3 saves are best effort and use no management objects.
Concurrent updates are not coordinated.
A failed save may leave a partial release that is visible to scans.
If any object already exists under that release's prefix, a later import skips the release, including partially saved releases.
To retry an incomplete release, remove its partial objects before importing again.

### Import Source

#### Common

Select the import-source implementation with `--import-source--mode`.
Currently, `filesystem` is supported.

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_IMPORT_SOURCE__MODE` | `--import-source--mode` | `import`, `import-check` | No | `filesystem` | Import-source implementation. |

#### Filesystem

| Variable | CLI argument | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `TONHALLE_IMPORT_SOURCE__ROOT_PATH` | `--import-source--root-path` | `import`, `import-check` | Yes | - | Existing filesystem source directory tree, read without modification. |

### Logging

| Variable | Commands | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `RUST_LOG` | `server`, `import`, `import-check` | No | `info` | Tracing filter for JSON logs; invalid filters fall back to `info`. |
