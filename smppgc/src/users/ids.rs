use std::{convert::Infallible, fmt::Display, ops::Deref};

use rocket::{
    async_trait,
    http::Status,
    outcome::IntoOutcome,
    request::{FromRequest, Outcome},
};
use uuid::Uuid;

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct UserId(pub(super) i32);
impl Deref for UserId {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<i32> for UserId {
    fn into(self) -> i32 {
        self.0
    }
}
impl UserId {
    pub fn to_i32(self) -> i32 {
        self.0
    }
}
