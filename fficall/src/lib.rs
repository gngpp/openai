pub mod ffi;
pub mod model;

use std::ffi::CString;

use model::{RequestPayload, ResponsePayload};
use serde::de::DeserializeOwned;

pub type FiiCallResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

pub trait StreamLine: Sync + Send {
    fn next<T: DeserializeOwned>(self) -> FiiCallResult<Option<T>>;

    fn stop(self) -> FiiCallResult<()>;
}

#[derive(thiserror::Error, Debug)]
pub enum SerdeError {
    #[error("failed serialize")]
    SerializeError,
    #[error("failed deserialize")]
    DeserializeError,
}

pub fn request(payload: RequestPayload) -> FiiCallResult<ResponsePayload> {
    let payload = serde_json::to_string(&payload)?;
    let body_utf8 = unsafe {
        let raw_payload = CString::new(payload)?.into_raw();
        let raw_body = ffi::Request(raw_payload);
        // release memory
        let _ = CString::from_raw(raw_payload);
        let body = CString::from_raw(raw_body);
        let body_utf8 = body.to_bytes().to_vec();
        body_utf8
    };

    Ok(serde_json::from_slice::<model::ResponsePayload>(
        &body_utf8,
    )?)
}

pub fn request_stream(payload: RequestPayload) -> FiiCallResult<ResponsePayload> {
    let payload = serde_json::to_string(&payload)?;
    let body_utf8 = unsafe {
        let raw_payload = CString::new(payload)?.into_raw();
        let raw_body = ffi::RequestStream(raw_payload);
        // release memory
        let _ = CString::from_raw(raw_payload);
        let body = CString::from_raw(raw_body);
        let body_utf8 = body.to_bytes().to_vec();
        body_utf8
    };
    Ok(serde_json::from_slice::<ResponsePayload>(&body_utf8)?)
}
