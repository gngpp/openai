use std::time;

use openai::oauth::OAuthAccountBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let email = std::env::var("EMAIL")?;
    let password = std::env::var("PASSWORD")?;
    let mut auth = openai::oauth::OAuthClientBuilder::builder()
        .user_agent(openai::api::HEADER_UA)
        .chrome_builder(reqwest::browser::ChromeVersion::V108)
        .cookie_store(true)
        .client_timeout(time::Duration::from_secs(1000))
        .client_connect_timeout(time::Duration::from_secs(1000))
        .build();
    let token = auth
        .do_access_token(
            OAuthAccountBuilder::default()
                .email(email)
                .password(password)
                .build()?,
        )
        .await?;
    println!("AccessToken: {}", token.access_token());
    println!("RefreshToken: {}", token.refresh_token());
    println!("Profile: {:#?}", token.profile());
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    auth.do_refresh_token(token.refresh_token()).await?;
    Ok(())
}
