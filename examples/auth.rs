use std::time;

use openai::auth::{model::AuthAccountBuilder, AuthHandle};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let email = std::env::var("EMAIL")?;
    let password = std::env::var("PASSWORD")?;
    let auth = openai::auth::AuthClientBuilder::builder()
        .user_agent(openai::HEADER_UA)
        .cookie_store(true)
        .timeout(time::Duration::from_secs(1000))
        .connect_timeout(time::Duration::from_secs(1000))
        .build();
    let token = auth
        .do_access_token(
            &AuthAccountBuilder::default()
                .username(email)
                .password(password)
                .option(openai::auth::AuthStrategy::Web)
                .build()?,
        )
        .await?;
    let auth_token = openai::model::AuthenticateToken::try_from(token)?;
    println!("AuthenticationToken: {:?}", auth_token);
    println!("AccessToken: {}", auth_token.access_token());
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    if let Some(refrsh_token) = auth_token.refresh_token() {
        println!("RefreshToken: {:?}", refrsh_token);
        auth.do_refresh_token(refrsh_token).await?;
    }

    Ok(())
}
