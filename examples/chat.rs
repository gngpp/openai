use std::collections::HashMap;

use fficall::model::Identifier;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let email = std::env::var("EMAIL")?;
    let password = std::env::var("PASSWORD")?;
    let store = openai::token::FileStore::default();
    let mut auth = openai::oauth::OAuthBuilder::builder()
        .email(email)
        .password(password)
        .cache(true)
        .cookie_store(true)
        .token_store(store)
        .client_timeout(std::time::Duration::from_secs(20))
        .build();
    let token = auth.do_get_access_token().await?;
    let mut headers = HashMap::new();
    headers.insert(reqwest::header::USER_AGENT.to_string(),
     "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36".to_string());
    headers.insert(
        reqwest::header::AUTHORIZATION.to_string(),
        token.get_bearer_access_token().to_owned(),
    );

    let payload = json!(
        {
            "action": "next",
            "messages": [
              {
                "id": "ec526640-1cac-4a8d-a4c4-5102ccbcacbc",
                "author": {
                  "role": "user"
                },
                "content": {
                  "content_type": "text",
                  "parts": [
                    "Rust Examples"
                  ]
                }
              }
            ],
            "parent_message_id": "e8a1841c-2694-4434-ad4d-ed0b79813879",
            "model": "text-davinci-002-render-sha",
            "timezone_offset_min": -480,
            "history_and_training_disabled": false
          }
    );
    let payload = fficall::model::RequestPayloadBuilder::default()
        .request_url("https://chat.openai.com/backend-api/conversation".to_string())
        .request_method(fficall::model::RequestMethod::POST)
        .tls_client_identifier(Identifier::Chrome105)
        .headers(headers)
        .request_body(payload.to_string())
        .timeout_seconds(200 as u32)
        .without_cookie_jar(false)
        .build()
        .unwrap();
    let resp = fficall::call_request_stream(payload)?;
    if resp.is_success() {
      while let Some(text) = resp.text()? {
          println!("{}", text)
      }
    } else {
        println!("{:?}", resp)
    }

    Ok(())
}
