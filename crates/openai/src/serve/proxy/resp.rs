use std::time::UNIX_EPOCH;

use crate::constant::{CF_CLEARANCE, NINJA_VERSION, PUID};
use crate::with_context;
use crate::LIB_VERSION;
use axum::body::Body;
use axum::body::StreamBody;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie;
use axum_extra::extract::cookie::Cookie;
use serde_json::Value;

use crate::serve::error::ResponseError;

use super::ext::ResponseExt;
use super::toapi;

/// Response convert
pub(crate) async fn response_convert(
    resp: ResponseExt,
) -> Result<impl IntoResponse, ResponseError> {
    // If to api is some, then convert to api response
    if resp.context.is_some() {
        return Ok(toapi::response_convert(resp).await?.into_response());
    }

    // Build new response
    let mut builder = Response::builder()
        .status(resp.inner.status())
        .header(NINJA_VERSION, LIB_VERSION);

    // Copy headers except for "set-cookie"
    for kv in resp
        .inner
        .headers()
        .into_iter()
        .filter(|(k, _)| k.ne(&header::SET_COOKIE) && k.ne(&header::CONTENT_LENGTH))
    {
        builder = builder.header(kv.0, kv.1);
    }

    // Filter and transform cookies
    for cookie in resp.inner.cookies() {
        let name = cookie.name().to_lowercase();
        if name.eq(PUID) || name.eq(CF_CLEARANCE) {
            if let Some(expires) = cookie.expires() {
                let timestamp_secs = expires
                    .duration_since(UNIX_EPOCH)
                    .map_err(ResponseError::InternalServerError)?
                    .as_secs_f64();
                let cookie = Cookie::build(cookie.name(), cookie.value())
                    .path("/")
                    .max_age(time::Duration::seconds_f64(timestamp_secs))
                    .same_site(cookie::SameSite::Lax)
                    .secure(false)
                    .http_only(false)
                    .finish();
                builder = builder.header(header::SET_COOKIE, cookie.to_string());
            }
        }
    }

    // Modify files endpoint response
    if with_context!(enable_file_proxy) && resp.inner.url().path().contains("/backend-api/files") {
        let url = resp.inner.url().clone();
        // Files endpoint handling
        let mut json = resp
            .inner
            .json::<Value>()
            .await
            .map_err(ResponseError::BadRequest)?;

        let body_key = if url.path().contains("download") || url.path().contains("uploaded") {
            "download_url"
        } else {
            "upload_url"
        };

        if let Some(download_upload_url) = json.get_mut(body_key) {
            if let Some(download_url_str) = download_upload_url.as_str() {
                const FILES_ENDPOINT: &str = "https://files.oaiusercontent.com";
                if download_url_str.starts_with(FILES_ENDPOINT) {
                    *download_upload_url =
                        serde_json::json!(download_url_str.replace(FILES_ENDPOINT, "/files"));
                }
            }
        }

        let json_bytes = serde_json::to_vec(&json)?;
        Ok(builder
            .body(StreamBody::new(Body::from(json_bytes)))
            .map_err(ResponseError::InternalServerError)?
            .into_response())
    } else {
        // Non-files endpoint handling
        Ok(builder
            .body(StreamBody::new(resp.inner.bytes_stream()))
            .map_err(ResponseError::InternalServerError)?
            .into_response())
    }
}
