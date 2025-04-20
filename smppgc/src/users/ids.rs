use std::{convert::Infallible, fmt::Display, ops::Deref, sync::Arc};

use rocket::{
    async_trait,
    http::Status,
    outcome::IntoOutcome,
    request::{FromRequest, Outcome},
};
use uuid::Uuid;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone)]
pub struct SmId(Arc<str>);
impl SmId {
    pub fn from_string(str: String) -> Self {
        Self(str.into())
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct SesId(Uuid);

impl SesId {
    pub(super) fn new() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn parse_str(string: &str) -> Option<Self> {
        let uuid = Uuid::parse_str(&string).ok()?;
        Some(Self(uuid))
    }
    pub fn inner(&self) -> Uuid {
        self.0
    }
}
impl Deref for SesId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Display for SesId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.as_simple().fmt(f)
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for SesId {
    type Error = Infallible;
    async fn from_request(req: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        req.cookies()
            .get("session")
            .map(|c| Self::parse_str(c.value_trimmed()))
            .flatten()
            .or_forward(Status::Unauthorized)
    }
}

#[deprecated(note = "Use smids now")]
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct UserSid(SmId);

impl UserSid {
    pub fn from_smid(smid: SmId) -> UserSid {
        UserSid(smid)
    }
    pub fn to_smid(&self) -> SmId {
        self.0.clone()
    }

    pub fn new() -> UserSid {
        Self(SmId("deprecated_uid".into()))
    }
    pub fn parse_str(string: &str) -> Option<Self> {
        Some(Self(SmId("deprecated_uid".into())))
    }
    pub fn to_bytes_le(&self) -> [u8; 17] {
        [0; 17]
    }
}
impl Display for UserSid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("deprecated uid")
    }
}
