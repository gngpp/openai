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
    } else if resp.inner.url().path().contains("fc/gt2/public_key") {
        // Files endpoint handling
        let mut json = resp
            .inner
            .json::<Value>()
            .await
            .map_err(ResponseError::BadRequest)?;

        if let Some(download_upload_url) = json.get_mut("challenge_url_cdn") {
            if let Some(download_url_str) = download_upload_url.as_str() {
                const FILES_ENDPOINT: &str = "https://tcr9i.chat.openai.com/";
                if download_url_str.starts_with(FILES_ENDPOINT) {
                    *download_upload_url =
                        serde_json::json!(download_url_str.replace(FILES_ENDPOINT, "/"));
                }
            }
        }

        if let Some(token) = json.get_mut("token") {
            if let Some(token_str) = token.as_str() {
                *token = serde_json::json!(token_str.replace("at=40|ag=101", "at=40|sup=1|rid=52|ag=101"));
                // const FILES_ENDPOINT: &str = "https://tcr9i.chat.openai.com/";
                // if download_url_str.starts_with(FILES_ENDPOINT) {
                //     *download_upload_url =
                //         serde_json::json!(download_url_str.replace(FILES_ENDPOINT, "/"));
                // }
                let token_str=token.as_str().unwrap();
                *token = serde_json::json!(token_str.replace("tcr9i.chat.openai.com", "client-api.arkoselabs.com"));

            }
        }

        let json_bytes = serde_json::to_vec(&json)?;
        Ok(builder
            .body(StreamBody::new(Body::from(json_bytes)))
            .map_err(ResponseError::InternalServerError)?
            .into_response())
    } else if resp.inner.url().path().contains("game_core_bootstrap.js") {
        print!("game_core_bootstrap.js");
        // http://192.168.252.128:7999/cdn/fc/assets/ec-game-core/bootstrap/1.18.0/standard/game_core_bootstrap.js
        // u("".concat(g(x), "/fc/").concat(t.f.ANALYTICS), {
        // u("".concat(window.origin, "/fc/").concat(t.f.ANALYTICS), {
        let mut body = resp.inner.text().await.map_err(ResponseError::BadRequest)?;
        const TARGET: &str = r#"u("".concat(g(x),"/fc/").concat(t.f.ANALYTICS)"#;
        const REPLACEMENT: &str = r#"u("".concat(window.origin,"/fc/").concat(t.f.ANALYTICS)"#;
        if body.contains(TARGET) {
            body = body.replace(TARGET, REPLACEMENT);
        }
        Ok(builder
            .body(StreamBody::new(Body::from(body)))
            .map_err(ResponseError::InternalServerError)?
            .into_response())
    } else if resp
        .inner
        .url()
        .path()
        .contains("ec-game-core/bootstrap/1.18.0/standard/sri.json")
    {
        print!("sri.json");
        let mut body = resp.inner.text().await.map_err(ResponseError::BadRequest)?;
        const TARGET: &str =
            r#"sha384-jb6MvhwzbHyUsRRonN5UHg/rGm2Pn2L8ePRzyTc8nUqzJv/oc2NNJmOfjF81c5yG"#;
        const REPLACEMENT: &str =
            r#"sha384-6YfRrK+HT8C+OXv1Su9lyivH6T5qMbaBEPQTebJbBu4S7WbMsnRittQJgNn3KFsq"#;
        if body.contains(TARGET) {
            body = body.replace(TARGET, REPLACEMENT);
        }
        Ok(builder
            .body(StreamBody::new(Body::from(body)))
            .map_err(ResponseError::InternalServerError)?
            .into_response())
    } else if resp
        .inner
        .url()
        .path()
        .contains("v2/2.3.4/enforcement.c70df15cb97792b18c2f4978b68954a0.html")
    {
        Ok(builder
                .body(StreamBody::new(Body::from(r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width,initial-scale=1">
        <meta http-equiv="Content-Security-Policy" content="style-src 'self' 'nonce-CZQ6o0B9znpEszouyF9InPmzND5ldwvtPGiu'; default-src 'self' data: *.arkoselabs.com *.funcaptcha.com *.arkoselabs.cn *.arkose.com.cn;">
        <meta http-equiv="X-UA-Compatible" content="ie=edge">
        <style nonce="CZQ6o0B9znpEszouyF9InPmzND5ldwvtPGiu">
            html, body {
                margin: 0;
                padding: 0;
                height: 100%;
            }

            * {
                box-sizing: border-box;
            }

            #app {
                height: 100%;
                overflow: hidden;
            }
        </style>
    </head>
    <body>
        <div id="app"></div>
        <script type="text/javascript" id="enforcementScript" src="enforcement.c70df15cb97792b18c2f4978b68954a0.js" crossorigin="anonymous" integrity="sha384-MrEVYBkrhJXI5+vVQ013Z0iX2Y0HznYzq3M607WsjLNJJ9e+KyOt9Y3amdoCm3Lm" data-nonce="CZQ6o0B9znpEszouyF9InPmzND5ldwvtPGiu"></script>
    </body>
</html>
"#)))
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
