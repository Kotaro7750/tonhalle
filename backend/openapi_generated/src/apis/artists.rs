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
pub enum GetArtistResponse {
    /// 認証有効時、Bearerアクセストークンが未指定または不正である
    Status401 { www_authenticate: Option<String> },
    /// アーティスト詳細の取得に成功した
    Status200(models::Artist),
    /// 識別子の形式、カーソル、またはlimitが不正である
    Status400(models::BadRequestProblem),
    /// 指定したリソースが存在しない
    Status404(models::NotFoundProblem),
    /// サーバー内部で予期しないエラーが発生した
    Status500(models::InternalServerErrorProblem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListArtistsResponse {
    /// 認証有効時、Bearerアクセストークンが未指定または不正である
    Status401 { www_authenticate: Option<String> },
    /// アーティスト一覧の取得に成功した
    Status200(models::ArtistPage),
    /// 識別子の形式、カーソル、またはlimitが不正である
    Status400(models::BadRequestProblem),
    /// サーバー内部で予期しないエラーが発生した
    Status500(models::InternalServerErrorProblem),
}

/// Artists
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Artists<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// アーティスト詳細の取得.
    ///
    /// GetArtist - GET /api/v0/artists/{artistId}
    async fn get_artist(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetArtistPathParams,
    ) -> Result<GetArtistResponse, E>;

    /// アーティスト一覧の取得.
    ///
    /// ListArtists - GET /api/v0/artists
    async fn list_artists(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListArtistsQueryParams,
    ) -> Result<ListArtistsResponse, E>;
}
