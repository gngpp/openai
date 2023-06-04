pub mod ffi;
pub mod model;

use std::ffi::CString;

use model::RequestPayload;

pub type GoHttpResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum SerdeError {
    #[error("failed serialize")]
    SerializeError,
    #[error("failed deserialize")]
    DeserializeError,
}

pub fn call_request(payload: RequestPayload) -> GoHttpResult<model::ResponsePayload> {
    let str = serde_json::to_string(&payload)?;
    let c_str = CString::new(str)?;
    let body_utf8 = unsafe {
        let c_char = ffi::Request(c_str.into_raw());
        let body_c_char = CString::from_raw(c_char);
        let body_utf8 = body_c_char.to_bytes().to_vec();
        drop(body_c_char);
        body_utf8
    };

    log::debug!(
        "[gohttp] call_request body: {}",
        String::from_utf8(body_utf8.to_vec())?
    );
    let body = serde_json::from_slice::<model::ResponsePayload>(&body_utf8)?;
    Ok(body)
}
