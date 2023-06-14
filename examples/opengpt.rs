use std::time;

use futures_util::StreamExt;
use openai::api::models::req::{self, PostConversationRequest};
use tokio::io::AsyncWriteExt;

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
    let api = openai::api::opengpt::OpenGPTBuilder::builder()
        .access_token(token.access_token().to_owned())
        .cookie_store(false)
        .client_timeout(time::Duration::from_secs(1000))
        .client_connect_timeout(time::Duration::from_secs(1000))
        .build();

    let resp = api.get_models().await?;

    let req = req::PostNextConversationBodyBuilder::default()
        .model(resp.models[0].slug.to_string())
        .prompt("Java Example".to_string())
        .build()?;

    let mut resp: openai::api::PostConversationStreamResponse = api
        .post_conversation(PostConversationRequest::Next(req))
        .await?;

    let mut previous_message = String::new();
    let mut out: tokio::io::Stdout = tokio::io::stdout();
    let mut conversation_id: Option<String> = None;
    let mut message_id: Option<String> = None;
    let mut end_turn: Option<bool> = None;
    while let Some(body) = resp.next().await {
        if conversation_id.is_none() {
            conversation_id = Some(body.conversation_id.to_string())
        }

        if end_turn.is_none() {
            end_turn = body.end_turn()
        }

        if let Some(end) = body.end_turn() {
            if end && message_id.is_none() {
                message_id = Some(body.message_id().to_string())
            }
        }
        let message = &body.message()[0];
        if message.starts_with(&previous_message) {
            let new_chars: String = message.chars().skip(previous_message.len()).collect();
            out.write_all(new_chars.as_bytes()).await?;
        } else {
            out.write_all(message.as_bytes()).await?;
        }
        out.flush().await?;
        previous_message = message.to_string();
    }

    println!("end");

    if let Some(end) = end_turn {
        if end {
            let conversation_id = conversation_id.unwrap_or_default();
            let message_id = message_id.unwrap_or_default();

            let req = req::PostConversationGenTitleRequestBuilder::default()
                .conversation_id(conversation_id.as_ref())
                .message_id(message_id.as_ref())
                .build()?;
            let resp = api.post_conversation_gen_title(req).await?;
            println!("\n{:?}", resp);

            // get conversation
            let req = req::GetConversationRequestBuilder::default()
                .conversation_id(conversation_id.as_ref())
                .build()?;
            let resp = api.get_conversation(req).await?;
            println!("{:#?}", resp);
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    // get conversation list
    // let req = req::GetConversationRequestBuilder::default()
    //     .offset(0)
    //     .limit(20)
    //     .build()?;
    // let resp = api.get_conversations(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // // clart conversation
    // let req = req::PatchConversationRequestBuilder::default()
    //     .conversation_id("3de1bd20-ecea-4bf7-96f5-b8eb681b180d".to_owned())
    //     .is_visible(false)
    //     .build()?;
    // let resp = api.patch_conversation(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // // clart conversation list
    // let req = req::PatchConversationRequestBuilder::default()
    //     .is_visible(false)
    //     .build()?;
    // let resp = api.patch_conversations(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // rename conversation title
    // let req = req::PatchConversationRequestBuilder::default()
    //     .conversation_id("78feb7c4-a864-4606-8665-cdb7a1cf4f6d".to_owned())
    //     .title("fuck".to_owned())
    //     .build()?;
    // let resp = api.patch_conversation(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // // message feedback
    // let req = req::MessageFeedbackRequestBuilder::default()
    //     .message_id("463a23c4-0855-4c5b-976c-7697519335ad".to_owned())
    //     .conversation_id("78feb7c4-a864-4606-8665-cdb7a1cf4f6d".to_owned())
    //     .rating(req::Rating::ThumbsUp)
    //     .build()?;
    // let resp = api.message_feedback(req).await?;
    // println!("{:#?}", resp);

    Ok(())
}
