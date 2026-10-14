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
pub enum GetReleaseResponse {
    /// リリース詳細の取得に成功した
    Status200(models::Release),
    /// 識別子の形式、カーソル、またはlimitが不正である
    Status400(models::BadRequestProblem),
    /// 指定したリリースまたはトラックが存在しない
    Status404(models::NotFoundProblem),
    /// サーバー内部で予期しないエラーが発生した
    Status500(models::InternalServerErrorProblem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListReleasesResponse {
    /// リリース一覧の取得に成功した
    Status200(models::ReleasePage),
    /// 識別子の形式、カーソル、またはlimitが不正である
    Status400(models::BadRequestProblem),
    /// サーバー内部で予期しないエラーが発生した
    Status500(models::InternalServerErrorProblem),
}

/// Releases
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Releases<E: std::fmt::Debug + Send + Sync + 'static = ()>:
    super::ErrorHandler<E>
{
    /// リリース詳細の取得.
    ///
    /// GetRelease - GET /api/v0/releases/{releaseId}
    async fn get_release(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        path_params: &models::GetReleasePathParams,
    ) -> Result<GetReleaseResponse, E>;

    /// リリース一覧の取得.
    ///
    /// ListReleases - GET /api/v0/releases
    async fn list_releases(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        query_params: &models::ListReleasesQueryParams,
    ) -> Result<ListReleasesResponse, E>;
}
