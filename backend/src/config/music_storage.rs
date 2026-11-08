//! Defines shared stored-audio configuration independently of the consuming command.

use clap::Args;
use object_store::aws::AmazonS3Builder;
use object_store_aws::AwsCredentials;
use std::{io::Error as IoError, path::PathBuf, sync::Arc};
use url::Url;

use crate::{
    infrastructure::music_storage::{
        filesystem::{
            FileSystemMusicStorage, FileSystemStorageConstructionError, normalize_playback_url,
        },
        s3::S3MusicStorage,
    },
    port::music_storage::MusicStorage,
};

/// A failure to select, validate, or construct music storage.
#[derive(Debug, thiserror::Error)]
pub(crate) enum MusicStorageConfigError {
    /// The selected implementation has no settings.
    #[error("implementation settings are required for the selected music storage mode")]
    MissingSettings,
    /// Playback links require an HTTP(S) directory URL.
    #[error("playback URL base must be an HTTP(S) directory URL without query or fragment")]
    InvalidPlaybackUrlBase,
    /// S3 settings are invalid or cannot construct storage.
    #[error(transparent)]
    S3(#[from] S3MusicStorageConfigError),
    /// Filesystem settings are invalid or cannot construct storage.
    #[error(transparent)]
    FileSystem(#[from] FileSystemMusicStorageConfigError),
}

/// Supported music-storage implementations.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(super) enum MusicStorageMode {
    /// Stores audio in a local directory tree.
    Filesystem,
    /// Stores audio objects in S3.
    S3,
}

/// Settings selecting and constructing music storage for servers and imports.
#[derive(getset::Getters, getset::CopyGetters, Debug, Args)]
pub(crate) struct MusicStorageConfig {
    /// Storage implementation selected independently of its settings.
    #[arg(
        id = "music_storage_mode",
        long = "music-storage--mode",
        env = "TONHALLE_MUSIC_STORAGE__MODE",
        value_enum,
        default_value = "filesystem"
    )]
    #[getset(get_copy = "pub(super)")]
    mode: MusicStorageMode,
    /// Optional delivery endpoint used when reading stored releases.
    #[arg(
        id = "music_storage_playback_url_base",
        long = "music-storage--playback-url-base",
        env = "TONHALLE_MUSIC_STORAGE__PLAYBACK_URL_BASE"
    )]
    #[getset(get = "pub(super)")]
    playback_url_base: Option<Url>,
    /// Optional settings for the filesystem implementation.
    #[command(flatten)]
    #[getset(get = "pub(super)")]
    file_system: Option<FileSystemMusicStorageConfig>,
    /// Optional S3 implementation settings.
    #[command(flatten)]
    #[getset(get = "pub(super)")]
    s3: Option<S3MusicStorageConfig>,
}

/// Defines validation and construction required of music-storage settings.
#[async_trait::async_trait]
trait MusicStorageImplementation: Sync {
    /// Validates music-storage settings independently of the consuming command.
    fn validate(&self) -> Result<(), MusicStorageConfigError>;

    /// Constructs storage exposed through its implementation-independent port.
    async fn construct(
        &self,
        playback_url_base: Option<Url>,
    ) -> Result<Arc<dyn MusicStorage>, MusicStorageConfigError>;
}

impl MusicStorageConfig {
    /// Selects the configured implementation and requires its settings.
    fn settings(&self) -> Result<&dyn MusicStorageImplementation, MusicStorageConfigError> {
        match self.mode {
            MusicStorageMode::S3 => self
                .s3
                .as_ref()
                .map(|settings| settings as &dyn MusicStorageImplementation)
                .ok_or(MusicStorageConfigError::MissingSettings),
            MusicStorageMode::Filesystem => self
                .file_system
                .as_ref()
                .map(|settings| settings as &dyn MusicStorageImplementation)
                .ok_or(MusicStorageConfigError::MissingSettings),
        }
    }

    /// Validates settings for the selected storage implementation.
    pub(crate) fn validate(&self) -> Result<(), MusicStorageConfigError> {
        if let Some(url) = &self.playback_url_base {
            normalize_playback_url(&mut url.clone())
                .map_err(|_| MusicStorageConfigError::InvalidPlaybackUrlBase)?;
        }
        self.settings()?.validate()
    }

    /// Constructs the selected storage implementation.
    pub(crate) async fn construct_music_storage(
        &self,
    ) -> Result<Arc<dyn MusicStorage>, MusicStorageConfigError> {
        self.validate()?;
        let mut playback_url_base = self.playback_url_base.clone();
        if let Some(url) = &mut playback_url_base {
            normalize_playback_url(url)
                .map_err(|_| MusicStorageConfigError::InvalidPlaybackUrlBase)?;
        }
        self.settings()?.construct(playback_url_base).await
    }
}

/// A failure to validate filesystem music-storage settings or construct storage.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileSystemMusicStorageConfigError {
    /// Filesystem storage requires a root directory.
    #[error("FileSystemMusicStorageConfig.root_path is required")]
    MissingRootPath,
    /// The configured destination cannot be resolved.
    #[error("resolving music storage root: {0}")]
    ResolveRoot(#[from] IoError),
    /// The resolved destination does not satisfy storage requirements.
    #[error(transparent)]
    Storage(#[from] FileSystemStorageConstructionError),
}

/// CLI settings and construction rules shared by filesystem music-storage consumers.
#[derive(getset::Getters, Debug, clap::Args)]
pub(crate) struct FileSystemMusicStorageConfig {
    /// Existing directory holding stored audio releases.
    #[arg(
        id = "music_storage_root_path",
        long = "music-storage--root-path",
        env = "TONHALLE_MUSIC_STORAGE__ROOT_PATH"
    )]
    #[getset(get = "pub(super)")]
    root_path: Option<PathBuf>,
}

#[async_trait::async_trait]
impl MusicStorageImplementation for FileSystemMusicStorageConfig {
    fn validate(&self) -> Result<(), MusicStorageConfigError> {
        if self.root_path.is_none() {
            return Err(FileSystemMusicStorageConfigError::MissingRootPath.into());
        }
        Ok(())
    }

    async fn construct(
        &self,
        playback_url_base: Option<Url>,
    ) -> Result<Arc<dyn MusicStorage>, MusicStorageConfigError> {
        self.validate()?;
        let root = self
            .root_path
            .as_ref()
            .ok_or(FileSystemMusicStorageConfigError::MissingRootPath)?
            .canonicalize()
            .map_err(FileSystemMusicStorageConfigError::from)?;
        let storage = match playback_url_base {
            Some(url) => FileSystemMusicStorage::new(root, url),
            None => FileSystemMusicStorage::for_import(root),
        }
        .map_err(FileSystemMusicStorageConfigError::from)?;
        Ok(Arc::new(storage))
    }
}

/// A failure to validate S3 music-storage settings or construct storage.
#[derive(Debug, thiserror::Error)]
pub(crate) enum S3MusicStorageConfigError {
    /// S3 storage requires an explicit AWS region.
    #[error("S3MusicStorageConfig.region is required")]
    MissingRegion,
    /// S3 storage requires a bucket name.
    #[error("S3MusicStorageConfig.bucket_name is required")]
    MissingBucketName,
    /// The object namespace cannot contain empty or dot path segments.
    #[error("S3MusicStorageConfig.prefix must contain nonempty path segments without dot segments")]
    InvalidPrefix,
    /// The AWS credential provider could not be initialized.
    #[error("initializing AWS credentials: {0}")]
    Credentials(#[from] object_store_aws::Error),
    /// The object-store client could not be constructed.
    #[error("constructing S3 object store: {0}")]
    Construct(#[from] object_store::Error),
}

/// Shared S3 settings for importing and serving stored music.
#[derive(getset::Getters, Debug, Args)]
pub(crate) struct S3MusicStorageConfig {
    /// AWS region used for S3 requests.
    #[arg(
        long = "music-storage--s3--region",
        env = "TONHALLE_MUSIC_STORAGE__S3__REGION"
    )]
    region: Option<String>,
    /// Bucket holding audio objects.
    #[arg(
        long = "music-storage--s3--bucket-name",
        env = "TONHALLE_MUSIC_STORAGE__S3__BUCKET_NAME"
    )]
    bucket_name: Option<String>,
    /// Optional namespace within the bucket.
    #[arg(
        long = "music-storage--s3--prefix",
        env = "TONHALLE_MUSIC_STORAGE__S3__PREFIX"
    )]
    prefix: Option<String>,
}

#[async_trait::async_trait]
impl MusicStorageImplementation for S3MusicStorageConfig {
    fn validate(&self) -> Result<(), MusicStorageConfigError> {
        if self
            .region
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(S3MusicStorageConfigError::MissingRegion.into());
        }
        if self
            .bucket_name
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(S3MusicStorageConfigError::MissingBucketName.into());
        }
        if let Some(prefix) = &self.prefix
            && !prefix.is_empty()
            && prefix
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(S3MusicStorageConfigError::InvalidPrefix.into());
        }
        Ok(())
    }

    async fn construct(
        &self,
        playback_url_base: Option<Url>,
    ) -> Result<Arc<dyn MusicStorage>, MusicStorageConfigError> {
        self.validate()?;
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(
                self.region
                    .clone()
                    .ok_or(S3MusicStorageConfigError::MissingRegion)?,
            ))
            .load()
            .await;
        let credentials = AwsCredentials::from_config(config)
            .await
            .map_err(S3MusicStorageConfigError::Credentials)?;
        let store = AmazonS3Builder::new()
            .with_region(
                self.region
                    .as_ref()
                    .ok_or(S3MusicStorageConfigError::MissingRegion)?,
            )
            .with_bucket_name(
                self.bucket_name
                    .as_ref()
                    .ok_or(S3MusicStorageConfigError::MissingBucketName)?,
            )
            .with_credentials(Arc::new(credentials))
            .build()
            .map_err(S3MusicStorageConfigError::Construct)?;
        Ok(Arc::new(S3MusicStorage::new(
            Arc::new(store),
            self.prefix.clone().unwrap_or_default(),
            playback_url_base,
        )))
    }
}

/// Builds valid storage settings for tests without invoking the CLI parser.
#[cfg(test)]
pub(super) fn sample_config(
    mode: MusicStorageMode,
    playback_url_base: Option<Url>,
) -> MusicStorageConfig {
    MusicStorageConfig {
        mode,
        playback_url_base,
        file_system: match mode {
            MusicStorageMode::Filesystem => Some(FileSystemMusicStorageConfig {
                root_path: Some(PathBuf::from("unused")),
            }),
            MusicStorageMode::S3 => None,
        },
        s3: match mode {
            MusicStorageMode::Filesystem => None,
            MusicStorageMode::S3 => Some(S3MusicStorageConfig {
                region: Some("ap-northeast-1".into()),
                bucket_name: Some("music".into()),
                prefix: None,
            }),
        },
    }
}

/// Verifies storage validation independently of CLI parsing.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s3_music_storage_config_validate_requires_region_and_bucket() {
        let mut config = S3MusicStorageConfig {
            region: Some("ap-northeast-1".into()),
            bucket_name: Some("music".into()),
            prefix: None,
        };
        assert!(config.validate().is_ok());
        for region in [None, Some(String::new()), Some(" ".into())] {
            config.region = region;
            assert!(matches!(
                config.validate(),
                Err(MusicStorageConfigError::S3(
                    S3MusicStorageConfigError::MissingRegion
                ))
            ));
        }
        config.region = Some("ap-northeast-1".into());
        for bucket in [None, Some(String::new()), Some(" ".into())] {
            config.bucket_name = bucket;
            assert!(matches!(
                config.validate(),
                Err(MusicStorageConfigError::S3(
                    S3MusicStorageConfigError::MissingBucketName
                ))
            ));
        }
    }

    #[test]
    fn s3_music_storage_config_validate_rejects_invalid_prefixes() {
        let mut config = S3MusicStorageConfig {
            region: Some("ap-northeast-1".into()),
            bucket_name: Some("music".into()),
            prefix: None,
        };
        for (prefix, valid) in [
            (None, true),
            (Some(""), true),
            (Some("music/audio"), true),
            (Some("/music"), false),
            (Some("music/"), false),
            (Some("music//audio"), false),
            (Some("../audio"), false),
            (Some("music/./audio"), false),
        ] {
            config.prefix = prefix.map(String::from);
            let result = config.validate();
            if valid {
                assert!(result.is_ok(), "{prefix:?}");
            } else {
                assert!(
                    matches!(
                        result,
                        Err(MusicStorageConfigError::S3(
                            S3MusicStorageConfigError::InvalidPrefix
                        ))
                    ),
                    "{prefix:?}"
                );
            }
        }
    }

    #[test]
    fn music_storage_config_validate_accepts_optional_http_playback_url() {
        for (url, valid) in [
            (None, true),
            (Some("https://example.com/audio"), true),
            (Some("http://localhost:8000/"), true),
            (Some("ftp://example.com/"), false),
            (Some("https://example.com/?token=value"), false),
            (Some("https://example.com/#fragment"), false),
        ] {
            let config = MusicStorageConfig {
                mode: MusicStorageMode::Filesystem,
                playback_url_base: url.map(|url| url.parse().unwrap()),
                file_system: Some(FileSystemMusicStorageConfig {
                    root_path: Some(PathBuf::from("unused")),
                }),
                s3: None,
            };
            let result = config.validate();
            if valid {
                assert!(result.is_ok(), "{url:?}");
            } else {
                assert!(
                    matches!(result, Err(MusicStorageConfigError::InvalidPlaybackUrlBase)),
                    "{url:?}"
                );
            }
        }
    }
}
