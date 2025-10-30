use std::{fmt::Display, num::ParseIntError, str::FromStr, time::SystemTime};

///Imprecise time because it stores time in minutes since unix epoch.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WFTime(u32);

impl WFTime {
    pub fn now() -> Self {
        Self(
            (SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Time older than unix epoch")
                .as_secs()
                * 60) as u32,
        )
    }

    #[inline]
    pub fn duration_since(self, erlier: Self) -> u32 {
        self.0 - erlier.0
    }
}
impl Display for WFTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl FromStr for WFTime {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(u32::from_str(s)?))
    }
}
