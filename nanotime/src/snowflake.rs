use log::*;
use std::{
    ops::Deref,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use crate::{Epoch, GCEpoch};

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

    fn calc_time_part(time: SystemTime) -> u64 {
        time.duration_since(GCEpoch::sys_time())
            .unwrap_or_else(|_| {
                error!("Time went backwards");
                Duration::from_secs(0)
            })
            .as_millis() as u64
    }
    pub fn now() -> Self {
        Self::from_parts(Self::calc_time_part(SystemTime::now()), 0)
    }

    pub fn new(time: SystemTime, inc_part: u16) -> Self {
        Self::from_parts(Self::calc_time_part(time), inc_part)
    }
    pub fn from_parts(time_part: u64, inc_part: u16) -> Self {
        Snowflake(u64::from_le(
            time_part.to_le() << 22 | (inc_part as u64).to_le(),
        ))
    }
    pub fn from_u64(u: u64) -> Self {
        Self(u)
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
        GCEpoch::sys_time() + Duration::from_millis(self.time_part())
    }
}

#[cfg(feature = "rocket")]
impl<'v> rocket::form::FromFormField<'v> for Snowflake {
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
    pub fn new() -> Self {
        Self(Snowflake::new(SystemTime::now(), 0).into())
    }
    pub fn new_snowflake(&self) -> Snowflake {
        let mut latest_snowflake = self.0.lock().unwrap();
        let now = Snowflake::now();
        let inc = if now.time_part() == latest_snowflake.time_part() {
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
        *latest_snowflake = Snowflake::from_parts(now.time_part(), inc);
        *latest_snowflake
    }
}

#[cfg(test)]
mod test {
    use crate::{snowflake::Snowflake, Epoch, GCEpoch};
    use std::time::{Duration, SystemTime};

    #[test]
    fn create_snowflake() {
        let time: SystemTime = GCEpoch::sys_time() + Duration::from_millis(6969);

        let snowflake = Snowflake::new(time, 0);
        assert_eq!(snowflake.0, u64::from_le(6969u64.to_le() << 22))
    }
}
