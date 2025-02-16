use log::*;
use rocket::form::FromFormField;
use std::{
    ops::Deref,
    time::{Duration, SystemTime},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u32);
impl Timestamp {
    pub const ZERO: Timestamp = Timestamp(0);
    pub fn now() -> Self {
        let timestamp = (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| {
                error!("Time went backwards");
                Duration::from_secs(0)
            })
            .as_secs()
            / 60) as u32;
        Self(timestamp)
    }
}
impl<'v> FromFormField<'v> for Timestamp {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Timestamp(u32::from_value(field)?))
    }
}
impl Into<u32> for Timestamp {
    fn into(self) -> u32 {
        self.0
    }
}
impl Deref for Timestamp {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
