pub mod ext;
pub mod req;
pub mod resp;
mod toapi;

use super::error::ResponseError;
use crate::constant::CF_CLEARANCE;
use crate::constant::PUID;
use crate::debug;
use axum::http::header;
use axum::http::HeaderMap;
use axum_extra::extract::CookieJar;

/// Request headers convert
pub(crate) fn header_convert(
    h: &HeaderMap,
    jar: &CookieJar,
    origin: &'static str,
) -> Result<HeaderMap, ResponseError> {
    let mut headers = HeaderMap::new();
    let mut cookies = Vec::new();

    // Support for team accounts.
    // The chat will be sent to the team account if the header is present, otherwise
    // it will be sent to the personal account.
    h.get("Chatgpt-Account-Id").map(|h| {
        headers.insert("Chatgpt-Account-Id", h.clone());
        cookies.push(format!("_account={}", h.to_str().unwrap_or_default()));
    });

    h.get("Access-Control-Request-Headers")
        .map(|h| headers.insert("Access-Control-Request-Headers", h.clone()));

    h.get("Access-Control-Request-Method").map(|h| {
        headers.insert("Access-Control-Request-Method", h.clone());
    });

    h.get("X-Ms-Blob-Type")
        .map(|v| headers.insert("X-Ms-Blob-Type", v.clone()));

    h.get("X-Ms-Version")
        .map(|v| headers.insert("X-Ms-Version", v.clone()));

    h.get(header::ACCEPT_LANGUAGE)
        .map(|h| headers.insert(header::ACCEPT_LANGUAGE, h.clone()))
        .flatten()
        .or_else(|| {
            headers.insert(
                header::ACCEPT_LANGUAGE,
                header::HeaderValue::from_static("en-US,en;q=0.9"),
            )
        });

    h.get(header::ACCEPT_ENCODING)
        .map(|h| headers.insert(header::ACCEPT_ENCODING, h.clone()))
        .flatten()
        .or_else(|| {
            headers.insert(
                header::ACCEPT_ENCODING,
                header::HeaderValue::from_static("gzip, deflate, br"),
            )
        });

    h.get(header::DNT)
        .map(|h| headers.insert(header::DNT, h.clone()))
        .flatten()
        .or_else(|| headers.insert(header::DNT, header::HeaderValue::from_static("1")));

    h.get(header::UPGRADE_INSECURE_REQUESTS)
        .map(|h| headers.insert(header::UPGRADE_INSECURE_REQUESTS, h.clone()))
        .flatten()
        .or_else(|| {
            headers.insert(
                header::UPGRADE_INSECURE_REQUESTS,
                header::HeaderValue::from_static("1"),
            )
        });

    h.get(header::AUTHORIZATION)
        .map(|h| headers.insert(header::AUTHORIZATION, h.clone()));

    h.get(header::COOKIE)
        .map(|h| headers.insert(header::COOKIE, h.clone()));

    h.get(header::CONTENT_TYPE)
        .map(|h| headers.insert(header::CONTENT_TYPE, h.clone()));

    headers.insert(header::ORIGIN, header::HeaderValue::from_static(origin));
    headers.insert(header::REFERER, header::HeaderValue::from_static(origin));

    jar.iter()
        .filter(|c| {
            let name = c.name().to_lowercase();
            name.eq(PUID) || name.eq(CF_CLEARANCE)
        })
        .for_each(|c| {
            let c = format!("{}={}", c.name(), cookie_encoded(c.value()));
            debug!("cookie: {}", c);
            cookies.push(c);
        });

    // setting cookie
    if !cookies.is_empty() {
        headers.insert(
            header::COOKIE,
            header::HeaderValue::from_str(&cookies.join(";"))
                .map_err(ResponseError::InternalServerError)?,
        );
    }
    Ok(headers)
}

fn cookie_encoded(input: &str) -> String {
    let separator = ':';
    if let Some((name, value)) = input.split_once(separator) {
        let encoded_value = value
            .chars()
            .map(|ch| match ch {
                '!' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '/' | ':'
                | ';' | '=' | '?' | '@' | '[' | ']' | '~' => {
                    format!("%{:02X}", ch as u8)
                }
                _ => ch.to_string(),
            })
            .collect::<String>();

        format!("{name}:{encoded_value}")
    } else {
        input.to_string()
    }
}
