use std::time;

use openai::auth::OAuthAccountBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let email = std::env::var("EMAIL")?;
    let password = std::env::var("PASSWORD")?;
    let auth = openai::auth::AuthClientBuilder::builder()
        .user_agent(openai::HEADER_UA)
        .chrome_builder(reqwest::browser::ChromeVersion::V108)
        .cookie_store(true)
        .timeout(time::Duration::from_secs(1000))
        .connect_timeout(time::Duration::from_secs(1000))
        .build();
    let token = auth
        .do_access_token(
            &OAuthAccountBuilder::default()
                .username(email)
                .password(password)
                .build()?,
        )
        .await?;
    println!("AccessToken: {}", token.access_token);
    println!("RefreshToken: {}", token.refresh_token);
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    auth.do_refresh_token(&token.refresh_token).await?;
    Ok(())
}
