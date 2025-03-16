use std::{fmt::Display, ops::Deref};

use uuid::Uuid;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct UserSid(Uuid);

impl UserSid {
    pub const SYSTEM: UserSid = UserSid(Uuid::from_u128(0));

    pub fn new() -> UserSid {
        Self(Uuid::new_v4())
    }
    pub fn parse_str(string: &str) -> Option<Self> {
        if string.len() != 33 {
            return None;
        }
        let uuid = Uuid::parse_str(&string[1..]).ok()?;
        Some(Self(uuid))
    }
    pub fn to_bytes_le(&self) -> [u8; 17] {
        let mut out = [0x61; 17]; //a
        out.clone_from_slice(&self.0.to_bytes_le());
        out
    }
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}
impl Deref for UserSid {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Display for UserSid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a")?;
        self.0.as_simple().fmt(f)
    }
}
