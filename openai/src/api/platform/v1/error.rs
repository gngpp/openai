use serde::Deserialize;
use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Deserialize)]
pub enum APIError {
    EndpointError(String),
    ParseError(String),
    FileError(String),
    StreamError(String),
}

impl Error for APIError {}

impl Display for APIError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            APIError::EndpointError(message) => write!(f, "{}", message),
            APIError::ParseError(message) => write!(f, "{}", message),
            APIError::FileError(message) => write!(f, "{}", message),
            APIError::StreamError(message) => write!(f, "{}", message),
        }
    }
}
