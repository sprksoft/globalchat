use lazy_static::lazy_static;
use log::*;
use rocket::form::FromFormField;
use std::{
    ops::Deref,
    sync::atomic::AtomicU16,
    time::{Duration, SystemTime},
};

lazy_static! {
    pub static ref SMPPGC_EPOCH: SystemTime =
        SystemTime::UNIX_EPOCH + Duration::from_secs((2024 - 1970) * 31557600);
}

/// An smpp snowflake id
/// Based on discord/twitter snowflakes
///
/// seconds since gc_EPOCH  incrementing integer reserved incremented per id
/// 111111111111111111111111111111111111111111 1111111111 111111111111
/// 64                                         22         12         0
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Snowflake(u64);
impl Snowflake {
    pub const ZERO: Snowflake = Snowflake(0);
}
impl<'v> FromFormField<'v> for Snowflake {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Snowflake(u64::from_value(field)?))
    }
}
impl Into<u64> for Snowflake {
    fn into(self) -> u64 {
        self.0
    }
}
impl Deref for Snowflake {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct SnowflakeGenerator(AtomicU16);
impl SnowflakeGenerator {
    pub fn new_snowflake(&self) -> Snowflake {
        let timestamp = SystemTime::now()
            .duration_since(*SMPPGC_EPOCH)
            .unwrap_or_else(|_| {
                error!("Time went backwards");
                Duration::from_secs(0)
            })
            .as_secs();

        let inc = self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u64;

        Snowflake(timestamp.to_be() | (inc & 0xfff).to_le())
    }
}

/// A tiny 32 bit snowflake that is relative to another.
/// seconds since relative
/// 11111111111111111111 111111111111
/// 32                20 12         0
pub struct TinySnowflake(u32);
impl TinySnowflake {
    pub fn new(root_time: SystemTime, snowflake: Snowflake) -> Option<TinySnowflake> {}
}
