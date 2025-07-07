use base64::{DecodeError, Engine};
use rocket::serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid jwt token")]
    Invalid,
    #[error("base64 error while decoding jwt: {0}")]
    Base64(#[from] DecodeError),
    #[error("json error while decoding jwt: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn decode_payload_insecure<T: for<'de> Deserialize<'de>>(token: &str) -> Result<T, Error> {
    let payload = token.split(".").nth(1).ok_or(Error::Invalid)?;
    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload)?;
    let json = serde_json::from_slice::<T>(decoded.as_slice())?;
    Ok(json)
}
