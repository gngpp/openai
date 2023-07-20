#![recursion_limit = "256"]
pub mod arkose;
pub mod auth;
pub mod chatgpt;
pub mod error;
#[cfg(feature = "stream")]
pub mod eventsource;
pub mod log;
pub mod model;
pub mod platform;
pub mod unescape;
pub mod uuid;

#[cfg(feature = "serve")]
pub mod serve;
pub mod token;

pub const HEADER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36";
pub const URL_CHATGPT_API: &str = "https://chat.openai.com";
pub const URL_PLATFORM_API: &str = "https://api.openai.com";
pub const ORIGIN_CHATGPT: &str = "https://chat.openai.com/chat";
pub const HOST_CHATGPT: &str = "chat.openai.com";
