use openai::api::{models::req, Api};

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
        .build();

    let resp = api.account_check().await?;
    println!("{:#?}", resp);
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let resp = api.get_models().await?;
    println!("{:#?}", resp);
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let req = req::GetConversationRequestBuilder::default()
        .conversation_id("78feb7c4-a864-4606-8665-cdb7a1cf4f6d".to_owned())
        .build()?;
    let resp = api.get_conversation(req).await?;
    println!("{:#?}", resp);
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let req = req::GetConversationRequestBuilder::default()
        .offset(0)
        .limit(20)
        .build()?;
    let resp = api.get_conversations(req).await?;
    println!("{:#?}", resp);
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // let req = req::DeleteConversationRequestBuilder::default()
    //     .conversation_id("3de1bd20-ecea-4bf7-96f5-b8eb681b180d".to_owned())
    //     .is_visible(false)
    //     .build()?;
    // let resp = api.delete_conversation(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // let req = req::DeleteConversationRequestBuilder::default()
    //     .is_visible(false)
    //     .build()?;
    // let resp = api.delete_conversations(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // let req = req::RenameConversationRequestBuilder::default()
    //     .conversation_id("78feb7c4-a864-4606-8665-cdb7a1cf4f6d".to_owned())
    //     .title("fuck".to_owned())
    //     .build()?;
    // let resp = api.rename_conversation(req).await?;
    // println!("{:#?}", resp);
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // let req = req::MessageFeedbackRequestBuilder::default()
    //     .message_id("463a23c4-0855-4c5b-976c-7697519335ad".to_owned())
    //     .conversation_id("78feb7c4-a864-4606-8665-cdb7a1cf4f6d".to_owned())
    //     .rating(req::Rating::ThumbsUp)
    //     .build()?;
    // let resp = api.message_feedback(req).await?;
    // println!("{:#?}", resp);

    Ok(())
}
