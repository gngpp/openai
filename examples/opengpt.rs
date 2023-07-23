use std::time;

use futures_util::StreamExt;
use openai::chatgpt::model::{
    req::{self, PostConvoRequest},
    resp,
};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let access_token = std::env::var("TOKEN")?;
    let api = openai::chatgpt::OpenGPTBuilder::builder()
        .access_token(access_token)
        .cookie_store(true)
        .client_timeout(time::Duration::from_secs(1000))
        .client_connect_timeout(time::Duration::from_secs(1000))
        .build();

    let resp = api.get_models().await?;
    let model = resp.real_models();

    let parent_message_id = openai::uuid::uuid();
    let message_id = openai::uuid::uuid();
    let req = req::PostNextConvoRequestBuilder::default()
        .model(model[0])
        .message_id(&message_id)
        .parent_message_id(&parent_message_id)
        .prompt("Rust Example")
        .build()?;

    let mut resp = api
        .post_conversation(PostConvoRequest::try_from(req)?)
        .await;

    let mut previous_message = String::new();
    let mut out: tokio::io::Stdout = tokio::io::stdout();
    let mut conversation_id: Option<String> = None;
    let mut end_message_id: Option<String> = None;
    let mut end_turn: Option<bool> = None;
    while let Some(body) = resp.next().await {
        if let Ok(resp) = body {
            match resp {
                resp::PostConvoResponse::Conversation(body) => {
                    if conversation_id.is_none() {
                        conversation_id = Some(body.conversation_id.to_string())
                    }

                    if end_message_id.is_none() {
                        end_message_id = Some(body.message_id().to_string())
                    }

                    if let Some(end) = body.end_turn() {
                        end_turn = Some(end)
                    }

                    let message = &body.message()[0];
                    if message.starts_with(&previous_message) {
                        let new_chars = message.trim_start_matches(&previous_message);
                        out.write_all(new_chars.as_bytes()).await?;
                    } else {
                        out.write_all(message.as_bytes()).await?;
                    }
                    out.flush().await?;
                    previous_message = message.to_string();
                }
                resp::PostConvoResponse::Moderation(body) => {
                    println!("{:?}", body);
                }
            }
        }
    }

    let conversation_id = conversation_id.unwrap_or_default();
    let end_message_id = end_message_id.unwrap_or_default();

    if let Some(end) = end_turn {
        if end {
            let req = req::PostConvoGenTitleRequestBuilder::default()
                .conversation_id(&conversation_id)
                .message_id(&end_message_id)
                .build()?;
            let resp = api.post_conversation_gen_title(req).await?;
            println!("\n{:?}", resp);

            // get conversation
            let req = req::GetConvoRequestBuilder::default()
                .conversation_id(conversation_id.as_ref())
                .build()?;
            let resp = api.get_conversation(req).await?;
            println!("{:?}", resp);
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            // // clart conversation
            let req = req::PatchConvoRequestBuilder::default()
                .conversation_id(conversation_id.as_ref())
                .is_visible(false)
                .build()?;
            let resp = api.patch_conversation(req).await?;
            println!("{:?}", resp);
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    } else {
        let req = req::PostContinueConvoRequestBuilder::default()
            .model(model[0])
            .conversation_id(conversation_id.as_ref())
            .parent_message_id(end_message_id.as_ref())
            .build()?;

        let mut resp = api
            .post_conversation(PostConvoRequest::try_from(req)?)
            .await;
        while let Some(body) = resp.next().await {
            if let Ok(body) = body {
                match body {
                    resp::PostConvoResponse::Conversation(body) => {
                        let message = &body.message()[0];
                        if message.starts_with(&previous_message) {
                            let new_chars = message.trim_start_matches(&previous_message);
                            out.write_all(new_chars.as_bytes()).await?;
                        } else {
                            out.write_all(message.as_bytes()).await?;
                        }
                        out.flush().await?;
                        previous_message = message.to_string();
                    }
                    resp::PostConvoResponse::Moderation(body) => {
                        println!("{:?}", body);
                    }
                }
            }
        }
    }

    // get conversation list
    // let req = req::GetConvoRequestBuilder::default()
    //     .offset(0)
    //     .limit(20)
    //     .build()?;
    // let resp = api.get_conversations(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // // clart conversation list
    // let req = req::PatchConvoRequestBuilder::default()
    //     .is_visible(false)
    //     .build()?;
    // let resp = api.patch_conversations(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // rename conversation title
    // let req = req::PatchConvoRequestBuilder::default()
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
