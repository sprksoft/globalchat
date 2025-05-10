use std::{net::IpAddr, sync::Arc};

use dashmap::DashMap;
use rocket::{fairing::AdHoc, request::FromRequest};

use crate::{
    ratelimit::{RateLimitConfig, RateLimiter, RateLimiters},
    users::UserId,
    wsprotocol::KickReason, MessageConfig,
};

pub type MessageLen = u16;
pub type BadWordLen = u8;

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct MessageLimits {
    pub small_len: MessageLen,
    pub max_len: MessageLen,
    pub min_len: MessageLen,
    pub large_penalty: u32,

    pub max_spam: u32,
    pub spam_penalty: u32,

    pub ratelimit: RateLimitConfig,
}

struct Profile {
    spam: u32,
    last_message_content: Arc<str>,
    ratelimiter: RateLimiter,
}

#[derive(Clone, Copy)]
pub enum LimitType {
    Rate,
    Spam,
    TooSmallOrLarge
}

pub struct MessageLimiter {
    config: MessageLimits,
    map: DashMap<UserId, Profile>,
}
impl MessageLimiter {
    pub fn new(config: MessageLimits) -> Self {
        Self {
            config: RateLimiters::new(config),
        }
    }
    pub fn feed_message(&self, user_id: UserId, message: Arc<str>) -> Result<(), LimitType> {
        if message.len() < self.config.min_len || message.len() > self.config.max_len {
            return Err(LimitType::TooSmallOrLarge)
        }
        match self.map.get_mut(&user_id) {
            Some(p) => {
                if p.last_message_content == message {
                    p.spam += 1;
                }else{
                    p.spam = 0;
                }
                if p.spam > self.config.max_spam {
                    return Err(LimitType::Spam)
                }

                let increase = if message.len() > self.config.small_len {
                    self.config.large_penalty
                }else {
                    1
                }
                p.last_message_content = message;
                if p.ratelimiter.update(increase) {
                    Ok(())
                }else {
                    Err(LimitType::Rate)
                }
            }
            None => {
                self.map.insert(
                    user_id,
                    Profile {
                        spam: 0,
                        last_message_content: message,
                        ratelimiter: (),
                    },
                );
                return Ok(());
            }
        }
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Message Limits", |r|async {
        let config: MessageLimits = r.figment().extract_inner("message_limits").expect("Failed to load message limits");
        r.manage(MessageLimiter::new(config))
    })
}
