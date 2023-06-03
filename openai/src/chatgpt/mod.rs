use self::model::{ModelsData, TitleData};

pub mod chat;
pub mod ios;
pub mod model;

pub type OpenAIResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("failed to cookie")]
    FailedGetCookie,
    #[error(" invalid cookie")]
    InvalidCookie,
    #[error("failed token deserialize")]
    DeserializeError,
    #[error("system time exception")]
    SystemTimeExceptionError,
    #[error("failed authentication")]
    FailedAuthenticationError,
}

pub trait Api {
    /// gets the title based on the return message ID
    fn get_title(&self, message_id: String) -> OpenAIResult<TitleData>;

    fn get_models(&self) -> OpenAIResult<ModelsData>;

    // fn get_conversations(&self, access_token: String) -> OpenAIResult<()>;

    // fn delete_conversation(&self, message_id: String) -> OpenAIResult<()>;

    // fn get_conversation(&self, message_id: String) -> OpenAIResult<()>;
}

#[track_caller]
pub fn print_caller() {
    use std::panic::Location;
    println!("called from {}", Location::caller());
}
