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
pub enum GetTrackResponse {
    /// トラック詳細の取得に成功した
    Status200(models::Track),
    /// 識別子の形式、カーソル、またはlimitが不正である
    Status400(models::BadRequestProblem),
    /// 指定したリリースまたはトラックが存在しない
    Status404(models::NotFoundProblem),
    /// サーバー内部で予期しないエラーが発生した
    Status500(models::InternalServerErrorProblem),
}

/// Tracks
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Tracks<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    /// トラック詳細の取得.
    ///
    /// GetTrack - GET /api/v0/tracks/{trackId}
    async fn get_track(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        path_params: &models::GetTrackPathParams,
    ) -> Result<GetTrackResponse, E>;
}
