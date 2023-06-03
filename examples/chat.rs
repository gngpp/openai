use std::collections::HashMap;

use gohttp::model::Identifier;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let email = std::env::var("EMAIL")?;
    let password = std::env::var("PASSWORD")?;
    let store = openai::token::FileStore::default();
    let mut auth = openai::oauth::OpenOAuth0Builder::builder()
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
    let payload = gohttp::model::RequestPayloadBuilder::default()
        .request_url("https://chat.openai.com/backend-api/models".to_string())
        .request_method(gohttp::model::RequestMethod::GET)
        .tls_client_identifier(Identifier::Chrome105)
        .headers(headers)
        .timeout_seconds(2 as u32)
        .without_cookie_jar(false)
        .build()
        .unwrap();
    let body = gohttp::call_request(payload)?;
    if body.is_success() {
        let data = body.json::<openai::chatgpt::model::ModelsData>().await?;
        println!("{:#?}", data);
    }

    Ok(())
}

#[macro_export]
macro_rules! info {
    // trace!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // trace!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::info!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // info!("a {} event", "log")
    ($($arg:tt)+) => (log::info!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}

#[macro_export]
macro_rules! debug {
    // info!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // info!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::debug!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // info!("a {} event", "log")
    ($($arg:tt)+) => (log::debug!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}

#[macro_export]
macro_rules! warn {
    // info!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // info!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::warn!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // info!("a {} event", "log")
    ($($arg:tt)+) => (log::warn!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}

#[macro_export]
macro_rules! trace {
    // info!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // info!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::trace!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // info!("a {} event", "log")
    ($($arg:tt)+) => (log::trace!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}

#[macro_export]
macro_rules! error {
    // info!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // info!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::error!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // info!("a {} event", "log")
    ($($arg:tt)+) => (log::error!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}
