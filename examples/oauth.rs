#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let store = openai::token::FileStore::default();
    let mut auth = openai::oauth::OpenOAuth0Builder::builder()
        .email("gngppx@gmail.com".to_string())
        .password("Fengge123???".to_string())
        .cache(true)
        .cookie_store(true)
        .token_store(store)
        .client_timeout(std::time::Duration::from_secs(20))
        .build();
    let token = auth.do_get_access_token().await?;
    println!("AccessToken: {}", token.access_token());
    println!("RefreshToken: {}", token.refresh_token());
    println!("Profile: {:#?}", token.profile());
    // tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // auth.do_refresh_token().await?;
    // tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // auth.do_revoke_token().await?;
    Ok(())
}
