use crate::names::UserSid;
use std::{net::IpAddr, time::Instant};

use dashmap::DashMap;
use log::*;
use rocket::{fairing::AdHoc, serde::Deserialize};

pub struct UserRateLimiters(pub RateLimiters<IpAddr>);
pub struct IpRateLimiters(pub RateLimiters<IpAddr>);
pub struct MesgRateLimiters(pub RateLimiters<UserSid>);

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

pub struct RateLimiter {
    conf: RateLimitConfig,

    count: u32,
    last_reset: Instant,
}
impl RateLimiter {
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

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct RateLimitConfig {
    pub timeframe: u32,
    pub amount: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct MessageLimits {
    pub small_message_len: usize,
    pub max_message_len: usize,
    pub large_message_penalty: u32,
    #[serde(flatten)]
    pub ratelimit: RateLimitConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct UserLimits {
    pub max_username_len: usize,
    #[serde(flatten)]
    pub ratelimit: RateLimitConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct IpLimits {
    pub xx_penalty: u32,
    pub not_be_penalty: u32,
    #[serde(flatten)]
    pub ratelimit: RateLimitConfig,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("ratelimiting", |r| async {
        let mesg_limits: MessageLimits = r
            .figment()
            .extract_inner("mesg_limits")
            .expect("Failed to read message ratelimiting config value");
        let user_limits: UserLimits = r
            .figment()
            .extract_inner("user_limits")
            .expect("Failed to read user ratelimiting config value");
        let ip_limits: IpLimits = r
            .figment()
            .extract_inner("ip_limits")
            .expect("Failed to read ip ratelimiting config value");

        r.manage(MesgRateLimiters(RateLimiters::<UserSid>::new(
            mesg_limits.ratelimit.clone(),
        )))
        .manage(mesg_limits)
        .manage(UserRateLimiters(RateLimiters::<IpAddr>::new(
            user_limits.ratelimit.clone(),
        )))
        .manage(user_limits)
        .manage(IpRateLimiters(RateLimiters::<IpAddr>::new(
            ip_limits.ratelimit.clone(),
        )))
        .manage(ip_limits)
    })
}
