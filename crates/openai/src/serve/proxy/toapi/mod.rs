mod model;
mod stream;

use axum::http::header;
use axum::http::Method;
use axum::{
    response::{IntoResponse, Sse},
    Json,
};
use eventsource_stream::Eventsource;
use reqwest::StatusCode;
use std::str::FromStr;

use crate::arkose::ArkoseContext;
use crate::chatgpt::model::req::Metadata;
use crate::chatgpt::model::Role;
use crate::gpt_model::GPTModel;
use crate::now_duration;
use crate::serve::error::ProxyError;
use crate::serve::ProxyResult;
use crate::token;
use crate::{
    arkose::ArkoseToken,
    chatgpt::model::req::{Content, ConversationMode, Messages, PostConvoRequest},
    serve::{
        error::ResponseError,
        puid::{get_or_init, reduce_key},
    },
    with_context,
};
use crate::{
    chatgpt::model::{
        req::{Action, ContentText},
        Author,
    },
    uuid::uuid,
};

use super::ext::{Context, RequestExt, ResponseExt};
use super::header_convert;
use crate::URL_CHATGPT_API;

const SUGGESTIONS: [&'static str; 4] = [
  "Write a script to automate sending daily email reports in Python, and walk me through how I would set it up.",
  "Design a database schema for an online merch store.",
  "I'm planning a 4-day trip to Seoul. Can you suggest an itinerary that doesn't involve popular tourist attractions?",
  "I'm going to cook for my date who claims to be a picky eater. Can you recommend me a dish that's easy to cook?"
];

/// Check if the request is supported
pub(super) fn support(req: &RequestExt) -> bool {
    if req.uri.path().eq("/v1/chat/completions") && req.method.eq(&Method::POST) {
        if let Some(ref token) = req.bearer_auth() {
            return !token::check_sk_or_sess(token);
        }
    }
    false
}

/// Send request to ChatGPT API
pub(super) async fn send_request(req: RequestExt) -> Result<ResponseExt, ResponseError> {
    // Exstract the token from the Authorization header
    let baerer = req
        .bearer_auth()
        .ok_or(ResponseError::BadRequest(ProxyError::AccessTokenRequired))?;

    // Exstract the token from the Authorization header
    let cache_id = reduce_key(baerer)?;

    // Exstract the body
    let bytes = req
        .body
        .as_ref()
        .ok_or_else(|| ResponseError::BadRequest(ProxyError::BodyRequired))?;
    let body = serde_json::from_slice::<model::Req>(bytes)?;

    // Convert to ChatGPT API Message
    let mut messages = Vec::with_capacity(body.messages.len());
    for body_msg in body.messages.iter() {
        let role = if body_msg.role.eq(&Role::System) {
            Role::Critic
        } else {
            body_msg.role
        };
        let message = Messages::builder()
            .id(uuid())
            .author(Author { role })
            .content(
                Content::builder()
                    .content_type(ContentText::Text)
                    .parts(vec![&body_msg.content])
                    .build(),
            )
            .metadata(Metadata {})
            .build();
        messages.push(message)
    }

    // Request client
    let client = with_context!(api_client);

    // OpenAI API to ChatGPT API model mapper
    let gpt_model = GPTModel::from_str(&body.model)?;

    // check if arkose token is required
    let arkose_token: Option<String> =
        if (with_context!(arkose_gpt3_experiment) && gpt_model.is_gpt3()) || gpt_model.is_gpt4() {
            let arkose_token = ArkoseToken::new_from_context(
                ArkoseContext::builder()
                    .client(client.clone())
                    .typed(gpt_model.clone().into())
                    .identifier(Some(baerer.to_owned()))
                    .build(),
            )
            .await?;
            Some(arkose_token.into())
        } else {
            None
        };

    // Create request
    let parent_message_id = uuid();
    let req_body = PostConvoRequest::builder()
        .action(Action::Next)
        .arkose_token(arkose_token.as_deref())
        .conversation_mode(ConversationMode {
            kind: "primary_assistant",
        })
        .force_paragen(false)
        .force_rate_limit(false)
        .history_and_training_disabled(true)
        .messages(messages)
        .model(gpt_model)
        .parent_message_id(&parent_message_id)
        .suggestions(SUGGESTIONS.to_vec())
        .timezone_offset_min(-480)
        .build();

    let mut builder = client
        .post(format!("{URL_CHATGPT_API}/backend-api/conversation"))
        .headers(header_convert(&req.headers, &req.jar, URL_CHATGPT_API)?);

    // Try to get puid from cache
    let puid = get_or_init(baerer, &body.model, cache_id).await?;
    if let Some(puid) = puid {
        builder = builder.header(header::COOKIE, format!("_puid={puid};"))
    }

    // Send request
    let resp = builder
        .json(&req_body)
        .send()
        .await
        .map_err(ResponseError::InternalServerError)?;

    Ok(ResponseExt::builder()
        .inner(resp)
        .context(
            Context::builder()
                .model(body.model)
                .stream(body.stream)
                .build(),
        )
        .build())
}

/// Convert response to ChatGPT API
pub(super) async fn response_convert(
    resp_ext: ResponseExt,
) -> Result<impl IntoResponse, ResponseError> {
    match resp_ext.inner.error_for_status() {
        Ok(resp) => {
            // Get config from request context
            let config = resp_ext.context.ok_or(ResponseError::InternalServerError(
                ProxyError::RequestContentIsEmpty,
            ))?;

            // Get response body event source
            let event_source = resp.bytes_stream().eventsource();

            if config.stream {
                // Create a  stream response
                let stream = stream::stream_handler(event_source, config.model)?;
                Ok(Sse::new(stream).into_response())
            } else {
                // Create a not stream response
                let no_stream = stream::not_stream_handler(event_source, config.model).await?;
                Ok(no_stream.into_response())
            }
        }
        Err(err) => Ok(handle_error_response(err)?.into_response()),
    }
}

/// Handle error response
fn handle_error_response(err: reqwest::Error) -> Result<impl IntoResponse, ResponseError> {
    if let Some(status_code) = err.status() {
        match status_code {
            StatusCode::UNAUTHORIZED => {
                let body = serde_json::json!({
                    "error": {
                        "message": "You didn't provide an API key...",
                        "type": "invalid_request_error",
                        "param": null,
                        "code": null
                    }
                });
                Ok(Json(body).into_response())
            }
            _ => Err(ResponseError::new(err.to_string(), status_code)),
        }
    } else {
        Err(ResponseError::InternalServerError(err))
    }
}

fn generate_id(length: usize) -> String {
    let rand_str = crate::generate_random_string(length);
    format!("chatcmpl-{rand_str}")
}

fn current_timestamp() -> ProxyResult<i64> {
    let time = now_duration()
        .map_err(ProxyError::SystemTimeBeforeEpoch)?
        .as_secs();
    Ok(time as i64)
}
