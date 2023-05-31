#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut auth = openai::oauth::OpenOAuth0Builder::builder()
        .email("opengpt@gmail.com".to_string())
        .password("gngpp".to_string())
        .cache(true)
        .cookie_store(true)
        .token_store(openai::token::Policy::file_store())
        .client_timeout(std::time::Duration::from_secs(20))
        .build();
    let token = auth.authenticate().await?;
    println!("Token: {}", token);
    println!("Profile: {:#?}", auth.get_user_info()?);
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    auth.do_refresh_token().await?;
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    auth.do_revoke_token().await?;
    Ok(())
}
