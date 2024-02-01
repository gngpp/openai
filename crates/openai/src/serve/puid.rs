use super::error::{ProxyError, ResponseError};
use crate::{gpt_model::GPTModel, with_context, URL_CHATGPT_API};
use moka::sync::Cache;
use std::str::FromStr;
use tokio::sync::OnceCell;

static PUID_CACHE: OnceCell<Cache<String, String>> = OnceCell::const_new();

pub(super) fn reduce_key(token: &str) -> Result<String, ResponseError> {
    let token_profile = crate::token::check(token)
        .map_err(ResponseError::Unauthorized)?
        .ok_or(ResponseError::BadRequest(ProxyError::InvalidAccessToken))?;
    Ok(token_profile.email().to_owned())
}

async fn cache() -> &'static Cache<String, String> {
    PUID_CACHE
        .get_or_init(|| async {
            Cache::builder()
                .time_to_live(std::time::Duration::from_secs(3600 * 24))
                .build()
        })
        .await
}

pub(super) async fn get_or_init(
    token: &str,
    model: &str,
    cache_id: String,
) -> Result<Option<String>, ResponseError> {
    let token = token.trim_start_matches("Bearer ");
    let puid_cache = cache().await;

    if let Some(p) = puid_cache.get(&cache_id) {
        return Ok(Some(p.clone()));
    }

    if GPTModel::from_str(model)?.is_gpt4() {
        let resp = with_context!(api_client)
            .get(format!("{URL_CHATGPT_API}/backend-api/models"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(ResponseError::InternalServerError)?
            .error_for_status()
            .map_err(ResponseError::BadRequest)?;

        if let Some(c) = resp.cookies().into_iter().find(|c| c.name().eq("_puid")) {
            let puid = c.value().to_owned();
            puid_cache.insert(cache_id, puid.clone());
            return Ok(Some(puid));
        };
    }

    Ok(None)
}
