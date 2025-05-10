use crate::users::UserId;
use std::{net::IpAddr, num::ParseIntError, str::FromStr, time::Instant};

use dashmap::DashMap;
use log::*;
use rocket::{
    fairing::AdHoc,
    serde::{de::Visitor, Deserialize},
};
use thiserror::Error;

#[derive(Copy, Clone)]
pub struct RateLimitConfig {
    pub timeframe: u32,
    pub amount: u32,
}
impl Deserialize for RateLimitConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: rocket::serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(RatelimitConfigVisitor)
    }
}

pub struct RatelimitConfigVisitor;
impl<'de> Visitor<'de> for RatelimitConfigVisitor {
    type Value = RateLimitConfig;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a ratelimit config")
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: rocket::serde::de::Error,
    {
        RateLimitConfig::from_str(v).map_err(|e| E::custom(e))
    }
}

#[derive(Debug, Error)]
enum ParseRatelimitError {
    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("rate limit needs to be in the format: <amount>/<timeframe>")]
    ExpectedSlash,
}
impl FromStr for RateLimitConfig {
    type Err = ParseRatelimitError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (amount_str, timeframe_str) = s
            .split_once('/')
            .ok_or(ParseRatelimitError::ExpectedSlash)?;
        Ok(RateLimitConfig {
            amount: amount_str.parse()?,
            timeframe: timeframe_str.parse()?,
        })
    }
}

pub struct RateLimiter {
    conf: RateLimitConfig,

    count: u32,
    last_reset: Instant,
}
impl RateLimiter {
    pub fn from_str(str: &str) -> Result<Self, ParseRatelimitError> {
        Ok(Self::new(RateLimitConfig::from_str(str)?))
    }

    pub fn new(conf: RateLimitConfig) -> Self {
        Self {
            conf,
            count: 0,
            last_reset: Instant::now(),
        }
    }
    /// Resets the rate limiter.
    /// frames: is the amount of frames to reset by.
    pub fn reset(&mut self, frames: u32) {
        //If max count was over the limit last time frame. Take that into account in this frame.
        self.count = self.count.saturating_sub(self.conf.amount * frames);
        if frames > 0 {
            self.last_reset = Instant::now();
        }
    }
    pub fn update(&mut self, increase: u32) -> bool {
        if self.conf.timeframe == 0 {
            warn!("timeframe can't be 0")
        }
        self.count = self.count.saturating_add(increase);
        let elapsed = self.last_reset.elapsed();
        let elapsed_frames = elapsed.as_secs() as u32 / self.conf.timeframe;
        self.reset(elapsed_frames);

        let block = self.count >= self.conf.amount;
        !block
    }
}

impl Deserialize for RateLimiter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: rocket::serde::Deserializer<'de>,
    {
        let config = deserializer.deserialize_str(RatelimitConfigVisitor)?;
        Ok(RateLimiter::new(config))
    }
}

pub struct RateLimiters<T: std::hash::Hash> {
    conf: RateLimitConfig,
    limiters: DashMap<T, RateLimiter>,
}
impl<T: std::hash::Hash + std::cmp::Eq> RateLimiters<T> {
    pub fn new(conf: RateLimitConfig) -> Self {
        Self {
            conf,
            limiters: DashMap::with_capacity(1000),
        }
    }

    pub fn update(&self, key: T, increase: u32) -> bool {
        match self.limiters.get_mut(&key) {
            Some(mut limiter) => limiter.update(increase),
            None => {
                self.limiters
                    .insert(key, RateLimiter::new(self.conf.clone()));
                true
            }
        }
    }
}
