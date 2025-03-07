use lazy_static::lazy_static;
use log::*;
use rocket::form::FromFormField;
use std::{
    ops::Deref,
    sync::{atomic::AtomicU16, Mutex},
    time::{Duration, SystemTime},
};

lazy_static! {
    pub static ref SMPPGC_EPOCH: SystemTime =
        SystemTime::UNIX_EPOCH + Duration::from_secs((2024 - 1970) * 31557600);
}

fn calc_gctime(time: SystemTime) -> u64 {
    time.duration_since(*SMPPGC_EPOCH)
        .unwrap_or_else(|_| {
            error!("Time went backwards");
            Duration::from_secs(0)
        })
        .as_millis() as u64
}

/// An smpp snowflake id
/// Based on discord/twitter snowflakes
///
/// millis since gc_EPOCH  incrementing integer reserved incremented per id
/// 111111111111111111111111111111111111111111 11111 11111 111111111111
/// 64                                         22         12         0
#[derive(Debug, Copy, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Snowflake(u64);
impl Snowflake {
    pub const ZERO: Snowflake = Snowflake(0);
    pub const MAX_INC: u16 = 2 ^ 12;

    pub fn new(time: SystemTime, inc_part: u16) -> Snowflake {
        Self::from_parts(calc_gctime(time), inc_part)
    }
    pub fn from_parts(time_part: u64, inc_part: u16) -> Self {
        Snowflake(u64::from_le(
            time_part.to_le() << 22 | (inc_part as u64).to_le(),
        ))
    }

    /// Get the incremented per id part of the snowfalke
    #[inline]
    pub fn inc_part(self) -> u16 {
        u16::from_le((self.0.to_le() & 0xfffu64.to_le()) as u16)
    }

    /// Get the incremented per id part of the snowfalke
    #[inline]
    pub fn time_part(self) -> u64 {
        u64::from_le(self.0.to_le() >> 22)
    }

    #[inline]
    pub fn time(self) -> SystemTime {
        *SMPPGC_EPOCH + Duration::from_millis(self.time_part())
    }
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

pub struct SnowflakeGenerator(Mutex<Snowflake>);
impl SnowflakeGenerator {
    pub fn new() -> SnowflakeGenerator {
        Self(Snowflake::new(SystemTime::now(), 0).into())
    }
    pub fn new_snowflake(&self) -> Snowflake {
        let now = SystemTime::now();
        let gc_now = calc_gctime(now);

        let mut latest_snowflake = self.0.lock().unwrap();
        let inc = if gc_now == latest_snowflake.time_part() {
            if latest_snowflake.inc_part() == Snowflake::MAX_INC {
                error!("Ran out of snowflake ids for this time. (waiting for next ms)");
                drop(latest_snowflake); // release lock on latest snowflake
                std::thread::sleep(Duration::from_millis(1)); // wait for next millisecond
                return self.new_snowflake();
            }
            latest_snowflake.inc_part() + 1
        } else {
            0
        };
        *latest_snowflake = Snowflake::from_parts(gc_now, inc);
        *latest_snowflake
    }
}

#[cfg(test)]
mod test {
    use std::{
        ops::Deref,
        time::{Duration, SystemTime},
    };

    use crate::{Snowflake, SMPPGC_EPOCH};

    #[test]
    fn create_snowflake() {
        let time: SystemTime = *SMPPGC_EPOCH.deref() + Duration::from_millis(6969);

        let snowflake = Snowflake::new(time, 0);
        assert_eq!(snowflake.0, u64::from_le(6969u64.to_le() << 22))
    }
}
