use std::{
    fmt::Display,
    marker::PhantomData,
    num::ParseIntError,
    str::FromStr,
    time::{Duration, SystemTime},
};

mod epoch;
pub mod snowflake;
pub use epoch::*;

// 32bit time that stores minutes since gc epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NanoTime<E = GCEpoch>(u32, PhantomData<E>);

impl<E: Epoch> NanoTime<E> {
    pub fn now() -> Self {
        Self(
            (SystemTime::now()
                .duration_since(E::sys_time())
                .expect("Time older than epoch")
                .as_secs()
                / 60) as u32,
            PhantomData,
        )
    }

    pub fn to_unix_secs(self) -> u64 {
        (E::sys_time() + Duration::from_secs(self.0 as u64 * 60))
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time older than unix epoch")
            .as_secs()
    }

    #[inline]
    pub fn duration_since(self, erlier: Self) -> u32 {
        self.0 - erlier.0
    }
}
impl<E> Display for NanoTime<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl<E> From<u32> for NanoTime<E> {
    fn from(value: u32) -> Self {
        Self(value, PhantomData)
    }
}
impl FromStr for NanoTime {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(u32::from_str(s)?, PhantomData))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for NanoTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}
