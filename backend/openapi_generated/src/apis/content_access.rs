use async_trait::async_trait;
use axum::extract::*;
use axum_extra::extract::CookieJar;
use bytes::Bytes;
use headers::Host;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{models, types::*};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ClearContentAccessResponse {
    /// 認証有効時、Bearerアクセストークンが未指定または不正である
    Status401 { www_authenticate: Option<String> },
    /// コンテンツアクセス設定の解除が完了した
    Status204 { set_cookie: Option<String> },
    /// サーバー内部で予期しないエラーが発生した
    Status500(models::InternalServerErrorProblem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ConfigureContentAccessResponse {
    /// 認証有効時、Bearerアクセストークンが未指定または不正である
    Status401 { www_authenticate: Option<String> },
    /// コンテンツアクセスに必要な設定が完了した
    Status200 {
        body: models::ContentAccessConfiguration,
        set_cookie: Option<String>,
    },
    /// サーバー内部で予期しないエラーが発生した
    Status500(models::InternalServerErrorProblem),
}

/// ContentAccess
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait ContentAccess<E: std::fmt::Debug + Send + Sync + 'static = ()>:
    super::ErrorHandler<E>
{
    type Claims;

    /// コンテンツアクセス設定の解除.
    ///
    /// ClearContentAccess - DELETE /api/v0/content-access
    async fn clear_content_access(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
    ) -> Result<ClearContentAccessResponse, E>;

    /// コンテンツアクセスの設定.
    ///
    /// ConfigureContentAccess - POST /api/v0/content-access
    async fn configure_content_access(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
    ) -> Result<ConfigureContentAccessResponse, E>;
}
