#![allow(unused_qualifications)]

use http::HeaderValue;
use validator::Validate;

#[cfg(feature = "server")]
use crate::header;
use crate::{models, types::*};

#[allow(dead_code)]
pub type SSE = std::pin::Pin<
    std::boxed::Box<
        dyn futures_util::Stream<
                Item = std::result::Result<axum::response::sse::Event, std::convert::Infallible>,
            > + std::marker::Send
            + std::marker::Sync,
    >,
>;

#[allow(dead_code)]
fn from_validation_error(e: validator::ValidationError) -> validator::ValidationErrors {
    let mut errs = validator::ValidationErrors::new();
    errs.add("na", e);
    errs
}

#[allow(dead_code)]
pub fn check_xss_string(v: &str) -> std::result::Result<(), validator::ValidationError> {
    if ammonia::is_html(v) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_vec_string(v: &[String]) -> std::result::Result<(), validator::ValidationError> {
    if v.iter().any(|i| ammonia::is_html(i)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map_string(
    v: &std::collections::HashMap<String, String>,
) -> std::result::Result<(), validator::ValidationError> {
    if v.keys().any(|k| ammonia::is_html(k)) || v.values().any(|v| ammonia::is_html(v)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map_nested<T>(
    v: &std::collections::HashMap<String, T>,
) -> std::result::Result<(), validator::ValidationError>
where
    T: validator::Validate,
{
    if v.keys().any(|k| ammonia::is_html(k)) || v.values().any(|v| v.validate().is_err()) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map<T>(
    v: &std::collections::HashMap<String, T>,
) -> std::result::Result<(), validator::ValidationError> {
    if v.keys().any(|k| ammonia::is_html(k)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetReleasePathParams {
    /// Releaseの識別子
    #[validate(
                          regex(path = *RE_GETRELEASEPATHPARAMS_RELEASE_ID),
            )]
    pub release_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETRELEASEPATHPARAMS_RELEASE_ID: regex::Regex = regex::Regex::new("^release-[0-9a-f]{64}$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListReleasesQueryParams {
    /// 取得開始位置を指定するカーソル。 初回または先頭から取得し直す場合は省略する。 続きの取得では前のレスポンスのnextCursorをそのまま指定し、内容を解釈してはならない。
    #[serde(rename = "cursor")]
    #[validate(length(min = 1))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// 1ページの最大件数。 継続リクエストで変更してもよい
    #[serde(rename = "limit")]
    #[validate(range(min = 1u8, max = 100u8))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetTrackPathParams {
    /// Trackの識別子
    #[validate(
                          regex(path = *RE_GETTRACKPATHPARAMS_TRACK_ID),
            )]
    pub track_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETTRACKPATHPARAMS_TRACK_ID: regex::Regex = regex::Regex::new("^track-[0-9a-f]{64}$").unwrap();
}

/// 表示順に並べるアーティストクレジットの1要素
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ArtistCredit {
    /// アーティストの識別子
    #[serde(rename = "artistId")]
    #[validate(
            regex(path = *RE_ARTISTCREDIT_ARTIST_ID),
          custom(function = "check_xss_string"),
    )]
    pub artist_id: String,

    /// アーティスト名
    #[serde(rename = "name")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub name: String,

    /// 次のアーティストとの連結句。 アーティストクレジットが単一要素の場合はnullとする。
    #[serde(rename = "joinPhrase")]
    pub join_phrase: Nullable<String>,
}

lazy_static::lazy_static! {
    static ref RE_ARTISTCREDIT_ARTIST_ID: regex::Regex = regex::Regex::new("^artist-[0-9a-f]{64}$").unwrap();
}

impl ArtistCredit {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(artist_id: String, name: String, join_phrase: Nullable<String>) -> ArtistCredit {
        ArtistCredit {
            artist_id,
            name,
            join_phrase,
        }
    }
}

/// Converts the ArtistCredit value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ArtistCredit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("artistId".to_string()),
            Some(self.artist_id.to_string()),
            Some("name".to_string()),
            Some(self.name.to_string()),
            Some("joinPhrase".to_string()),
            Some(
                self.join_phrase
                    .as_ref()
                    .map_or("null".to_string(), |x| x.to_string()),
            ),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ArtistCredit value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ArtistCredit {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub artist_id: Vec<String>,
            pub name: Vec<String>,
            pub join_phrase: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ArtistCredit".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "artistId" => intermediate_rep.artist_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "joinPhrase" => return std::result::Result::Err(
                        "Parsing a nullable type in this style is not supported in ArtistCredit"
                            .to_string(),
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ArtistCredit".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ArtistCredit {
            artist_id: intermediate_rep
                .artist_id
                .into_iter()
                .next()
                .ok_or_else(|| "artistId missing in ArtistCredit".to_string())?,
            name: intermediate_rep
                .name
                .into_iter()
                .next()
                .ok_or_else(|| "name missing in ArtistCredit".to_string())?,
            join_phrase: std::result::Result::Err(
                "Nullable types not supported in ArtistCredit".to_string(),
            )?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ArtistCredit> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ArtistCredit>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ArtistCredit>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ArtistCredit - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ArtistCredit> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ArtistCredit as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ArtistCredit - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// アーティストの識別子
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ArtistId(pub String);

impl validator::Validate for ArtistId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for ArtistId {
    fn from(x: String) -> Self {
        ArtistId(x)
    }
}

impl std::fmt::Display for ArtistId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ArtistId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(ArtistId(x.to_string()))
    }
}

impl std::convert::From<ArtistId> for String {
    fn from(x: ArtistId) -> Self {
        x.0
    }
}

impl std::ops::Deref for ArtistId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ArtistId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// アーティスト名
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ArtistName(pub String);

impl validator::Validate for ArtistName {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for ArtistName {
    fn from(x: String) -> Self {
        ArtistName(x)
    }
}

impl std::fmt::Display for ArtistName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ArtistName {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(ArtistName(x.to_string()))
    }
}

impl std::convert::From<ArtistName> for String {
    fn from(x: ArtistName) -> Self {
        x.0
    }
}

impl std::ops::Deref for ArtistName {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ArtistName {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// オリジナル音源の情報。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct AudioFile {
    /// 音源ファイルの識別子
    #[serde(rename = "id")]
    #[validate(
            regex(path = *RE_AUDIOFILE_ID),
          custom(function = "check_xss_string"),
    )]
    pub id: String,

    /// 音源のContent-Type
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "mediaType")]
    #[validate(custom(function = "check_xss_string"))]
    pub media_type: String,

    /// 実際の音源データを配信するURL
    #[serde(rename = "playbackUrl")]
    #[validate(custom(function = "check_xss_string"))]
    pub playback_url: String,

    /// サンプルレート（Hz）
    #[serde(rename = "sampleRate")]
    #[validate(range(min = 1u32))]
    pub sample_rate: u32,

    /// ビット深度（bit）。取得できない形式ではnull
    #[serde(rename = "bitDepth")]
    #[validate(range(min = 1u32))]
    pub bit_depth: Nullable<u32>,

    /// チャンネル数
    #[serde(rename = "channels")]
    #[validate(range(min = 1u32))]
    pub channels: u32,

    /// ビットレート（bit/s）。取得できない場合はnull
    #[serde(rename = "bitrate")]
    #[validate(range(min = 1u32))]
    pub bitrate: Nullable<u32>,
}

lazy_static::lazy_static! {
    static ref RE_AUDIOFILE_ID: regex::Regex = regex::Regex::new("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$").unwrap();
}

impl AudioFile {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        id: String,
        media_type: String,
        playback_url: String,
        sample_rate: u32,
        bit_depth: Nullable<u32>,
        channels: u32,
        bitrate: Nullable<u32>,
    ) -> AudioFile {
        AudioFile {
            id,
            media_type,
            playback_url,
            sample_rate,
            bit_depth,
            channels,
            bitrate,
        }
    }
}

/// Converts the AudioFile value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for AudioFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("mediaType".to_string()),
            Some(self.media_type.to_string()),
            Some("playbackUrl".to_string()),
            Some(self.playback_url.to_string()),
            Some("sampleRate".to_string()),
            Some(self.sample_rate.to_string()),
            Some("bitDepth".to_string()),
            Some(
                self.bit_depth
                    .as_ref()
                    .map_or("null".to_string(), |x| x.to_string()),
            ),
            Some("channels".to_string()),
            Some(self.channels.to_string()),
            Some("bitrate".to_string()),
            Some(
                self.bitrate
                    .as_ref()
                    .map_or("null".to_string(), |x| x.to_string()),
            ),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a AudioFile value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for AudioFile {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub media_type: Vec<String>,
            pub playback_url: Vec<String>,
            pub sample_rate: Vec<u32>,
            pub bit_depth: Vec<u32>,
            pub channels: Vec<u32>,
            pub bitrate: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing AudioFile".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "mediaType" => intermediate_rep.media_type.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "playbackUrl" => intermediate_rep.playback_url.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sampleRate" => intermediate_rep.sample_rate.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "bitDepth" => {
                        return std::result::Result::Err(
                            "Parsing a nullable type in this style is not supported in AudioFile"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "channels" => intermediate_rep.channels.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "bitrate" => {
                        return std::result::Result::Err(
                            "Parsing a nullable type in this style is not supported in AudioFile"
                                .to_string(),
                        );
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing AudioFile".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(AudioFile {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in AudioFile".to_string())?,
            media_type: intermediate_rep
                .media_type
                .into_iter()
                .next()
                .ok_or_else(|| "mediaType missing in AudioFile".to_string())?,
            playback_url: intermediate_rep
                .playback_url
                .into_iter()
                .next()
                .ok_or_else(|| "playbackUrl missing in AudioFile".to_string())?,
            sample_rate: intermediate_rep
                .sample_rate
                .into_iter()
                .next()
                .ok_or_else(|| "sampleRate missing in AudioFile".to_string())?,
            bit_depth: std::result::Result::Err(
                "Nullable types not supported in AudioFile".to_string(),
            )?,
            channels: intermediate_rep
                .channels
                .into_iter()
                .next()
                .ok_or_else(|| "channels missing in AudioFile".to_string())?,
            bitrate: std::result::Result::Err(
                "Nullable types not supported in AudioFile".to_string(),
            )?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<AudioFile> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<AudioFile>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<AudioFile>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for AudioFile - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<AudioFile> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <AudioFile as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into AudioFile - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// 音源ファイルの識別子
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct AudioFileId(pub String);

impl validator::Validate for AudioFileId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for AudioFileId {
    fn from(x: String) -> Self {
        AudioFileId(x)
    }
}

impl std::fmt::Display for AudioFileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AudioFileId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(AudioFileId(x.to_string()))
    }
}

impl std::convert::From<AudioFileId> for String {
    fn from(x: AudioFileId) -> Self {
        x.0
    }
}

impl std::ops::Deref for AudioFileId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for AudioFileId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// 識別子の形式、カーソル、またはlimitが不正である
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct BadRequestProblem {
    /// エラー種別を識別するURI参照
    #[serde(rename = "type")]
    #[validate(custom(function = "check_xss_string"))]
    pub r_type: String,

    /// エラー種別の短い説明
    #[serde(rename = "title")]
    #[validate(custom(function = "check_xss_string"))]
    pub title: String,

    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "status")]
    pub status: i32,

    /// 具体的なエラー内容
    #[serde(rename = "detail")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// エラー発生を識別するURI参照
    #[serde(rename = "instance")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl BadRequestProblem {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(title: String, status: i32) -> BadRequestProblem {
        BadRequestProblem {
            r_type: r#"about:blank"#.to_string(),
            title,
            status,
            detail: None,
            instance: None,
        }
    }
}

/// Converts the BadRequestProblem value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for BadRequestProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("type".to_string()),
            Some(self.r_type.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("status".to_string()),
            Some(self.status.to_string()),
            self.detail
                .as_ref()
                .map(|detail| ["detail".to_string(), detail.to_string()].join(",")),
            self.instance
                .as_ref()
                .map(|instance| ["instance".to_string(), instance.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a BadRequestProblem value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for BadRequestProblem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub r_type: Vec<String>,
            pub title: Vec<String>,
            pub status: Vec<i32>,
            pub detail: Vec<String>,
            pub instance: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing BadRequestProblem".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "type" => intermediate_rep.r_type.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "detail" => intermediate_rep.detail.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "instance" => intermediate_rep.instance.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing BadRequestProblem".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(BadRequestProblem {
            r_type: intermediate_rep
                .r_type
                .into_iter()
                .next()
                .ok_or_else(|| "type missing in BadRequestProblem".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in BadRequestProblem".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in BadRequestProblem".to_string())?,
            detail: intermediate_rep.detail.into_iter().next(),
            instance: intermediate_rep.instance.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<BadRequestProblem> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<BadRequestProblem>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<BadRequestProblem>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for BadRequestProblem - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<BadRequestProblem> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <BadRequestProblem as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into BadRequestProblem - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// ディスク番号
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DiscNumber {}

impl DiscNumber {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> DiscNumber {
        DiscNumber {}
    }
}

/// Converts the DiscNumber value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DiscNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DiscNumber value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DiscNumber {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {}

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing DiscNumber".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing DiscNumber".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DiscNumber {})
    }
}

// Methods for converting between header::IntoHeaderValue<DiscNumber> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DiscNumber>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<DiscNumber>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for DiscNumber - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DiscNumber> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <DiscNumber as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into DiscNumber - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// サーバー内部で予期しないエラーが発生した
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct InternalServerErrorProblem {
    /// エラー種別を識別するURI参照
    #[serde(rename = "type")]
    #[validate(custom(function = "check_xss_string"))]
    pub r_type: String,

    /// エラー種別の短い説明
    #[serde(rename = "title")]
    #[validate(custom(function = "check_xss_string"))]
    pub title: String,

    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "status")]
    pub status: i32,

    /// 具体的なエラー内容
    #[serde(rename = "detail")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// エラー発生を識別するURI参照
    #[serde(rename = "instance")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl InternalServerErrorProblem {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(title: String, status: i32) -> InternalServerErrorProblem {
        InternalServerErrorProblem {
            r_type: r#"about:blank"#.to_string(),
            title,
            status,
            detail: None,
            instance: None,
        }
    }
}

/// Converts the InternalServerErrorProblem value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for InternalServerErrorProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("type".to_string()),
            Some(self.r_type.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("status".to_string()),
            Some(self.status.to_string()),
            self.detail
                .as_ref()
                .map(|detail| ["detail".to_string(), detail.to_string()].join(",")),
            self.instance
                .as_ref()
                .map(|instance| ["instance".to_string(), instance.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a InternalServerErrorProblem value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for InternalServerErrorProblem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub r_type: Vec<String>,
            pub title: Vec<String>,
            pub status: Vec<i32>,
            pub detail: Vec<String>,
            pub instance: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing InternalServerErrorProblem".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "type" => intermediate_rep.r_type.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "detail" => intermediate_rep.detail.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "instance" => intermediate_rep.instance.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing InternalServerErrorProblem".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(InternalServerErrorProblem {
            r_type: intermediate_rep
                .r_type
                .into_iter()
                .next()
                .ok_or_else(|| "type missing in InternalServerErrorProblem".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in InternalServerErrorProblem".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in InternalServerErrorProblem".to_string())?,
            detail: intermediate_rep.detail.into_iter().next(),
            instance: intermediate_rep.instance.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<InternalServerErrorProblem> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<InternalServerErrorProblem>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<InternalServerErrorProblem>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for InternalServerErrorProblem - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<InternalServerErrorProblem> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <InternalServerErrorProblem as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into InternalServerErrorProblem - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// 指定したリリースまたはトラックが存在しない
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NotFoundProblem {
    /// エラー種別を識別するURI参照
    #[serde(rename = "type")]
    #[validate(custom(function = "check_xss_string"))]
    pub r_type: String,

    /// エラー種別の短い説明
    #[serde(rename = "title")]
    #[validate(custom(function = "check_xss_string"))]
    pub title: String,

    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "status")]
    pub status: i32,

    /// 具体的なエラー内容
    #[serde(rename = "detail")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// エラー発生を識別するURI参照
    #[serde(rename = "instance")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl NotFoundProblem {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(title: String, status: i32) -> NotFoundProblem {
        NotFoundProblem {
            r_type: r#"about:blank"#.to_string(),
            title,
            status,
            detail: None,
            instance: None,
        }
    }
}

/// Converts the NotFoundProblem value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NotFoundProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("type".to_string()),
            Some(self.r_type.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("status".to_string()),
            Some(self.status.to_string()),
            self.detail
                .as_ref()
                .map(|detail| ["detail".to_string(), detail.to_string()].join(",")),
            self.instance
                .as_ref()
                .map(|instance| ["instance".to_string(), instance.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NotFoundProblem value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NotFoundProblem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub r_type: Vec<String>,
            pub title: Vec<String>,
            pub status: Vec<i32>,
            pub detail: Vec<String>,
            pub instance: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NotFoundProblem".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "type" => intermediate_rep.r_type.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "detail" => intermediate_rep.detail.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "instance" => intermediate_rep.instance.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing NotFoundProblem".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NotFoundProblem {
            r_type: intermediate_rep
                .r_type
                .into_iter()
                .next()
                .ok_or_else(|| "type missing in NotFoundProblem".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in NotFoundProblem".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in NotFoundProblem".to_string())?,
            detail: intermediate_rep.detail.into_iter().next(),
            instance: intermediate_rep.instance.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NotFoundProblem> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NotFoundProblem>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NotFoundProblem>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NotFoundProblem - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NotFoundProblem> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NotFoundProblem as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NotFoundProblem - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// RFC 9457に準拠したAPIエラーの詳細。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Problem {
    /// エラー種別を識別するURI参照
    #[serde(rename = "type")]
    #[validate(custom(function = "check_xss_string"))]
    pub r_type: String,

    /// エラー種別の短い説明
    #[serde(rename = "title")]
    #[validate(custom(function = "check_xss_string"))]
    pub title: String,

    /// HTTPステータスコード
    #[serde(rename = "status")]
    pub status: i32,

    /// 具体的なエラー内容
    #[serde(rename = "detail")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// エラー発生を識別するURI参照
    #[serde(rename = "instance")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl Problem {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(title: String, status: i32) -> Problem {
        Problem {
            r_type: r#"about:blank"#.to_string(),
            title,
            status,
            detail: None,
            instance: None,
        }
    }
}

/// Converts the Problem value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("type".to_string()),
            Some(self.r_type.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("status".to_string()),
            Some(self.status.to_string()),
            self.detail
                .as_ref()
                .map(|detail| ["detail".to_string(), detail.to_string()].join(",")),
            self.instance
                .as_ref()
                .map(|instance| ["instance".to_string(), instance.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Problem value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Problem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub r_type: Vec<String>,
            pub title: Vec<String>,
            pub status: Vec<i32>,
            pub detail: Vec<String>,
            pub instance: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Problem".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "type" => intermediate_rep.r_type.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "detail" => intermediate_rep.detail.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "instance" => intermediate_rep.instance.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Problem".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Problem {
            r_type: intermediate_rep
                .r_type
                .into_iter()
                .next()
                .ok_or_else(|| "type missing in Problem".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in Problem".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in Problem".to_string())?,
            detail: intermediate_rep.detail.into_iter().next(),
            instance: intermediate_rep.instance.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Problem> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Problem>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<Problem>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for Problem - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Problem> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <Problem as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into Problem - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// リリース詳細と全収録曲
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Release {
    /// リリースの識別子
    #[serde(rename = "id")]
    #[validate(
            regex(path = *RE_RELEASE_ID),
          custom(function = "check_xss_string"),
    )]
    pub id: String,

    /// リリースのタイトル
    #[serde(rename = "title")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub title: String,

    /// リリース日
    #[serde(rename = "releaseDate")]
    #[validate(
            regex(path = *RE_RELEASE_RELEASE_DATE),
          custom(function = "check_xss_string"),
    )]
    pub release_date: String,

    /// 表示順に並べたアーティストクレジット
    #[serde(rename = "artistCredits")]
    #[validate(length(min = 1), nested)]
    pub artist_credits: Vec<models::ArtistCredit>,

    /// リリースに含まれる収録トラック一覧。 ディスク番号・トラック番号の昇順。 番号の重複によって順序が決まらない部分は未定義
    #[serde(rename = "tracks")]
    #[validate(length(min = 1), nested)]
    pub tracks: Vec<models::TrackSummary>,
}

lazy_static::lazy_static! {
    static ref RE_RELEASE_ID: regex::Regex = regex::Regex::new("^release-[0-9a-f]{64}$").unwrap();
}
lazy_static::lazy_static! {
    static ref RE_RELEASE_RELEASE_DATE: regex::Regex = regex::Regex::new("^[0-9]{4}(-(0[1-9]|1[0-2])(-(0[1-9]|[12][0-9]|3[01]))?)?$").unwrap();
}

impl Release {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        id: String,
        title: String,
        release_date: String,
        artist_credits: Vec<models::ArtistCredit>,
        tracks: Vec<models::TrackSummary>,
    ) -> Release {
        Release {
            id,
            title,
            release_date,
            artist_credits,
            tracks,
        }
    }
}

/// Converts the Release value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("releaseDate".to_string()),
            Some(self.release_date.to_string()),
            // Skipping artistCredits in query parameter serialization

            // Skipping tracks in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Release value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Release {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub title: Vec<String>,
            pub release_date: Vec<String>,
            pub artist_credits: Vec<Vec<models::ArtistCredit>>,
            pub tracks: Vec<Vec<models::TrackSummary>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Release".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "releaseDate" => intermediate_rep.release_date.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "artistCredits" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in Release"
                                .to_string(),
                        );
                    }
                    "tracks" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in Release"
                                .to_string(),
                        );
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Release".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Release {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in Release".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in Release".to_string())?,
            release_date: intermediate_rep
                .release_date
                .into_iter()
                .next()
                .ok_or_else(|| "releaseDate missing in Release".to_string())?,
            artist_credits: intermediate_rep
                .artist_credits
                .into_iter()
                .next()
                .ok_or_else(|| "artistCredits missing in Release".to_string())?,
            tracks: intermediate_rep
                .tracks
                .into_iter()
                .next()
                .ok_or_else(|| "tracks missing in Release".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Release> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Release>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<Release>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for Release - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Release> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <Release as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into Release - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// リリース日
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReleaseDate(pub String);

impl validator::Validate for ReleaseDate {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for ReleaseDate {
    fn from(x: String) -> Self {
        ReleaseDate(x)
    }
}

impl std::fmt::Display for ReleaseDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ReleaseDate {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(ReleaseDate(x.to_string()))
    }
}

impl std::convert::From<ReleaseDate> for String {
    fn from(x: ReleaseDate) -> Self {
        x.0
    }
}

impl std::ops::Deref for ReleaseDate {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ReleaseDate {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// リリースの識別子
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReleaseId(pub String);

impl validator::Validate for ReleaseId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for ReleaseId {
    fn from(x: String) -> Self {
        ReleaseId(x)
    }
}

impl std::fmt::Display for ReleaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ReleaseId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(ReleaseId(x.to_string()))
    }
}

impl std::convert::From<ReleaseId> for String {
    fn from(x: ReleaseId) -> Self {
        x.0
    }
}

impl std::ops::Deref for ReleaseId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ReleaseId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// カーソル方式のリリース一覧。 最終ページのitemsは空配列でもよい。 nextCursorを含む場合、itemsには1件以上含まれる。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReleasePage {
    #[serde(rename = "items")]
    #[validate(nested)]
    pub items: Vec<models::ReleaseSummary>,

    /// 次ページのカーソル。 最終ページでは省略する
    #[serde(rename = "nextCursor")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl ReleasePage {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(items: Vec<models::ReleaseSummary>) -> ReleasePage {
        ReleasePage {
            items,
            next_cursor: None,
        }
    }
}

/// Converts the ReleasePage value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ReleasePage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping items in query parameter serialization
            self.next_cursor
                .as_ref()
                .map(|next_cursor| ["nextCursor".to_string(), next_cursor.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ReleasePage value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ReleasePage {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub items: Vec<Vec<models::ReleaseSummary>>,
            pub next_cursor: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ReleasePage".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "items" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in ReleasePage"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "nextCursor" => intermediate_rep.next_cursor.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ReleasePage".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ReleasePage {
            items: intermediate_rep
                .items
                .into_iter()
                .next()
                .ok_or_else(|| "items missing in ReleasePage".to_string())?,
            next_cursor: intermediate_rep.next_cursor.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ReleasePage> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ReleasePage>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ReleasePage>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ReleasePage - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ReleasePage> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ReleasePage as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ReleasePage - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// 一覧表示用のリリース概要
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReleaseSummary {
    /// リリースの識別子
    #[serde(rename = "id")]
    #[validate(
            regex(path = *RE_RELEASESUMMARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub id: String,

    /// リリースのタイトル
    #[serde(rename = "title")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub title: String,

    /// リリース日
    #[serde(rename = "releaseDate")]
    #[validate(
            regex(path = *RE_RELEASESUMMARY_RELEASE_DATE),
          custom(function = "check_xss_string"),
    )]
    pub release_date: String,

    /// 表示順に並べたアーティストクレジット
    #[serde(rename = "artistCredits")]
    #[validate(length(min = 1), nested)]
    pub artist_credits: Vec<models::ArtistCredit>,
}

lazy_static::lazy_static! {
    static ref RE_RELEASESUMMARY_ID: regex::Regex = regex::Regex::new("^release-[0-9a-f]{64}$").unwrap();
}
lazy_static::lazy_static! {
    static ref RE_RELEASESUMMARY_RELEASE_DATE: regex::Regex = regex::Regex::new("^[0-9]{4}(-(0[1-9]|1[0-2])(-(0[1-9]|[12][0-9]|3[01]))?)?$").unwrap();
}

impl ReleaseSummary {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        id: String,
        title: String,
        release_date: String,
        artist_credits: Vec<models::ArtistCredit>,
    ) -> ReleaseSummary {
        ReleaseSummary {
            id,
            title,
            release_date,
            artist_credits,
        }
    }
}

/// Converts the ReleaseSummary value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ReleaseSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("releaseDate".to_string()),
            Some(self.release_date.to_string()),
            // Skipping artistCredits in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ReleaseSummary value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ReleaseSummary {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub title: Vec<String>,
            pub release_date: Vec<String>,
            pub artist_credits: Vec<Vec<models::ArtistCredit>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ReleaseSummary".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "releaseDate" => intermediate_rep.release_date.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "artistCredits" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in ReleaseSummary"
                                .to_string(),
                        );
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ReleaseSummary".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ReleaseSummary {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in ReleaseSummary".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in ReleaseSummary".to_string())?,
            release_date: intermediate_rep
                .release_date
                .into_iter()
                .next()
                .ok_or_else(|| "releaseDate missing in ReleaseSummary".to_string())?,
            artist_credits: intermediate_rep
                .artist_credits
                .into_iter()
                .next()
                .ok_or_else(|| "artistCredits missing in ReleaseSummary".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ReleaseSummary> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ReleaseSummary>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ReleaseSummary>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ReleaseSummary - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ReleaseSummary> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ReleaseSummary as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ReleaseSummary - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// リリースのタイトル
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReleaseTitle(pub String);

impl validator::Validate for ReleaseTitle {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for ReleaseTitle {
    fn from(x: String) -> Self {
        ReleaseTitle(x)
    }
}

impl std::fmt::Display for ReleaseTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ReleaseTitle {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(ReleaseTitle(x.to_string()))
    }
}

impl std::convert::From<ReleaseTitle> for String {
    fn from(x: ReleaseTitle) -> Self {
        x.0
    }
}

impl std::ops::Deref for ReleaseTitle {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ReleaseTitle {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// 楽曲情報と唯一のオリジナル音源
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Track {
    /// トラックの識別子
    #[serde(rename = "id")]
    #[validate(
            regex(path = *RE_TRACK_ID),
          custom(function = "check_xss_string"),
    )]
    pub id: String,

    /// リリースの識別子
    #[serde(rename = "releaseId")]
    #[validate(
            regex(path = *RE_TRACK_RELEASE_ID),
          custom(function = "check_xss_string"),
    )]
    pub release_id: String,

    /// トラックのタイトル
    #[serde(rename = "title")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub title: String,

    /// ディスク番号
    #[serde(rename = "discNumber")]
    #[validate(range(min = 1u32))]
    pub disc_number: u32,

    /// トラック番号
    #[serde(rename = "trackNumber")]
    #[validate(range(min = 1u32))]
    pub track_number: u32,

    /// トラックの再生時間（ミリ秒）。取得できない場合はnull
    #[serde(rename = "durationMs")]
    #[validate(range(min = 0u32))]
    pub duration_ms: Nullable<u32>,

    /// 表示順に並べたアーティストクレジット
    #[serde(rename = "artistCredits")]
    #[validate(length(min = 1), nested)]
    pub artist_credits: Vec<models::ArtistCredit>,

    #[serde(rename = "originalAudioFile")]
    #[validate(nested)]
    pub original_audio_file: models::AudioFile,
}

lazy_static::lazy_static! {
    static ref RE_TRACK_ID: regex::Regex = regex::Regex::new("^track-[0-9a-f]{64}$").unwrap();
}
lazy_static::lazy_static! {
    static ref RE_TRACK_RELEASE_ID: regex::Regex = regex::Regex::new("^release-[0-9a-f]{64}$").unwrap();
}

impl Track {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        id: String,
        release_id: String,
        title: String,
        disc_number: u32,
        track_number: u32,
        duration_ms: Nullable<u32>,
        artist_credits: Vec<models::ArtistCredit>,
        original_audio_file: models::AudioFile,
    ) -> Track {
        Track {
            id,
            release_id,
            title,
            disc_number,
            track_number,
            duration_ms,
            artist_credits,
            original_audio_file,
        }
    }
}

/// Converts the Track value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("releaseId".to_string()),
            Some(self.release_id.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("discNumber".to_string()),
            Some(self.disc_number.to_string()),
            Some("trackNumber".to_string()),
            Some(self.track_number.to_string()),
            Some("durationMs".to_string()),
            Some(
                self.duration_ms
                    .as_ref()
                    .map_or("null".to_string(), |x| x.to_string()),
            ),
            // Skipping artistCredits in query parameter serialization

            // Skipping originalAudioFile in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Track value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Track {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub release_id: Vec<String>,
            pub title: Vec<String>,
            pub disc_number: Vec<u32>,
            pub track_number: Vec<u32>,
            pub duration_ms: Vec<u32>,
            pub artist_credits: Vec<Vec<models::ArtistCredit>>,
            pub original_audio_file: Vec<models::AudioFile>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Track".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "releaseId" => intermediate_rep.release_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "discNumber" => intermediate_rep.disc_number.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "trackNumber" => intermediate_rep.track_number.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "durationMs" => {
                        return std::result::Result::Err(
                            "Parsing a nullable type in this style is not supported in Track"
                                .to_string(),
                        );
                    }
                    "artistCredits" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in Track"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "originalAudioFile" => intermediate_rep.original_audio_file.push(
                        <models::AudioFile as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Track".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Track {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in Track".to_string())?,
            release_id: intermediate_rep
                .release_id
                .into_iter()
                .next()
                .ok_or_else(|| "releaseId missing in Track".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in Track".to_string())?,
            disc_number: intermediate_rep
                .disc_number
                .into_iter()
                .next()
                .ok_or_else(|| "discNumber missing in Track".to_string())?,
            track_number: intermediate_rep
                .track_number
                .into_iter()
                .next()
                .ok_or_else(|| "trackNumber missing in Track".to_string())?,
            duration_ms: std::result::Result::Err(
                "Nullable types not supported in Track".to_string(),
            )?,
            artist_credits: intermediate_rep
                .artist_credits
                .into_iter()
                .next()
                .ok_or_else(|| "artistCredits missing in Track".to_string())?,
            original_audio_file: intermediate_rep
                .original_audio_file
                .into_iter()
                .next()
                .ok_or_else(|| "originalAudioFile missing in Track".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Track> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Track>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<Track>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for Track - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Track> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Track as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => {
                    std::result::Result::Ok(header::IntoHeaderValue(value))
                }
                std::result::Result::Err(err) => std::result::Result::Err(format!(
                    r#"Unable to convert header value '{value}' into Track - {err}"#
                )),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// トラックの再生時間（ミリ秒）。取得できない場合はnull
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TrackDurationMs {}

impl TrackDurationMs {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> TrackDurationMs {
        TrackDurationMs {}
    }
}

/// Converts the TrackDurationMs value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TrackDurationMs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TrackDurationMs value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TrackDurationMs {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {}

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TrackDurationMs".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing TrackDurationMs".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TrackDurationMs {})
    }
}

// Methods for converting between header::IntoHeaderValue<TrackDurationMs> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TrackDurationMs>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TrackDurationMs>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TrackDurationMs - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TrackDurationMs> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TrackDurationMs as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TrackDurationMs - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// トラックの識別子
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TrackId(pub String);

impl validator::Validate for TrackId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for TrackId {
    fn from(x: String) -> Self {
        TrackId(x)
    }
}

impl std::fmt::Display for TrackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for TrackId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(TrackId(x.to_string()))
    }
}

impl std::convert::From<TrackId> for String {
    fn from(x: TrackId) -> Self {
        x.0
    }
}

impl std::ops::Deref for TrackId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for TrackId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// トラック番号
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TrackNumber {}

impl TrackNumber {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> TrackNumber {
        TrackNumber {}
    }
}

/// Converts the TrackNumber value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TrackNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TrackNumber value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TrackNumber {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {}

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TrackNumber".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing TrackNumber".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TrackNumber {})
    }
}

// Methods for converting between header::IntoHeaderValue<TrackNumber> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TrackNumber>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TrackNumber>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TrackNumber - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TrackNumber> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TrackNumber as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TrackNumber - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// 収録曲一覧用の楽曲情報
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TrackSummary {
    /// トラックの識別子
    #[serde(rename = "id")]
    #[validate(
            regex(path = *RE_TRACKSUMMARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub id: String,

    /// リリースの識別子
    #[serde(rename = "releaseId")]
    #[validate(
            regex(path = *RE_TRACKSUMMARY_RELEASE_ID),
          custom(function = "check_xss_string"),
    )]
    pub release_id: String,

    /// トラックのタイトル
    #[serde(rename = "title")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub title: String,

    /// ディスク番号
    #[serde(rename = "discNumber")]
    #[validate(range(min = 1u32))]
    pub disc_number: u32,

    /// トラック番号
    #[serde(rename = "trackNumber")]
    #[validate(range(min = 1u32))]
    pub track_number: u32,

    /// トラックの再生時間（ミリ秒）。取得できない場合はnull
    #[serde(rename = "durationMs")]
    #[validate(range(min = 0u32))]
    pub duration_ms: Nullable<u32>,

    /// 表示順に並べたアーティストクレジット
    #[serde(rename = "artistCredits")]
    #[validate(length(min = 1), nested)]
    pub artist_credits: Vec<models::ArtistCredit>,
}

lazy_static::lazy_static! {
    static ref RE_TRACKSUMMARY_ID: regex::Regex = regex::Regex::new("^track-[0-9a-f]{64}$").unwrap();
}
lazy_static::lazy_static! {
    static ref RE_TRACKSUMMARY_RELEASE_ID: regex::Regex = regex::Regex::new("^release-[0-9a-f]{64}$").unwrap();
}

impl TrackSummary {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        id: String,
        release_id: String,
        title: String,
        disc_number: u32,
        track_number: u32,
        duration_ms: Nullable<u32>,
        artist_credits: Vec<models::ArtistCredit>,
    ) -> TrackSummary {
        TrackSummary {
            id,
            release_id,
            title,
            disc_number,
            track_number,
            duration_ms,
            artist_credits,
        }
    }
}

/// Converts the TrackSummary value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TrackSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("releaseId".to_string()),
            Some(self.release_id.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("discNumber".to_string()),
            Some(self.disc_number.to_string()),
            Some("trackNumber".to_string()),
            Some(self.track_number.to_string()),
            Some("durationMs".to_string()),
            Some(
                self.duration_ms
                    .as_ref()
                    .map_or("null".to_string(), |x| x.to_string()),
            ),
            // Skipping artistCredits in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TrackSummary value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TrackSummary {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub release_id: Vec<String>,
            pub title: Vec<String>,
            pub disc_number: Vec<u32>,
            pub track_number: Vec<u32>,
            pub duration_ms: Vec<u32>,
            pub artist_credits: Vec<Vec<models::ArtistCredit>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TrackSummary".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "releaseId" => intermediate_rep.release_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "discNumber" => intermediate_rep.disc_number.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "trackNumber" => intermediate_rep.track_number.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "durationMs" => return std::result::Result::Err(
                        "Parsing a nullable type in this style is not supported in TrackSummary"
                            .to_string(),
                    ),
                    "artistCredits" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in TrackSummary"
                                .to_string(),
                        );
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing TrackSummary".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TrackSummary {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in TrackSummary".to_string())?,
            release_id: intermediate_rep
                .release_id
                .into_iter()
                .next()
                .ok_or_else(|| "releaseId missing in TrackSummary".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in TrackSummary".to_string())?,
            disc_number: intermediate_rep
                .disc_number
                .into_iter()
                .next()
                .ok_or_else(|| "discNumber missing in TrackSummary".to_string())?,
            track_number: intermediate_rep
                .track_number
                .into_iter()
                .next()
                .ok_or_else(|| "trackNumber missing in TrackSummary".to_string())?,
            duration_ms: std::result::Result::Err(
                "Nullable types not supported in TrackSummary".to_string(),
            )?,
            artist_credits: intermediate_rep
                .artist_credits
                .into_iter()
                .next()
                .ok_or_else(|| "artistCredits missing in TrackSummary".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TrackSummary> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TrackSummary>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TrackSummary>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TrackSummary - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TrackSummary> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TrackSummary as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TrackSummary - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// トラックのタイトル
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TrackTitle(pub String);

impl validator::Validate for TrackTitle {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for TrackTitle {
    fn from(x: String) -> Self {
        TrackTitle(x)
    }
}

impl std::fmt::Display for TrackTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for TrackTitle {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(TrackTitle(x.to_string()))
    }
}

impl std::convert::From<TrackTitle> for String {
    fn from(x: TrackTitle) -> Self {
        x.0
    }
}

impl std::ops::Deref for TrackTitle {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for TrackTitle {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}
