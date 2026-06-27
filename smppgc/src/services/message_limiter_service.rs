use dashmap::DashMap;
use rocket::fairing::AdHoc;

use crate::{config::MessageLimitsConfig, models::UserId, ratelimit::RateLimiter};

struct Profile {
    spam: u32,
    last_message_content: Box<str>,
    ratelimiter: RateLimiter,
}

pub enum LimitType {
    Rate,
    Spam,
    Size,
}

pub struct MessageLimiterService {
    config: MessageLimitsConfig,
    map: DashMap<UserId, Profile>,
}
impl MessageLimiterService {
    pub fn new(config: MessageLimitsConfig) -> Self {
        Self {
            config,
            map: DashMap::new(),
        }
    }

    fn size_check(&self, len: usize) -> Result<(), LimitType> {
        if len < self.config.min_len as usize || len > self.config.max_len as usize {
            Err(LimitType::Size)
        } else {
            Ok(())
        }
    }

    pub fn message_size_range(&self) -> (u16, u16) {
        (self.config.min_len, self.config.max_len)
    }

    pub fn feed(&self, user_id: UserId, message: String) -> Result<String, LimitType> {
        self.size_check(message.len())?;

        match self.map.get_mut(&user_id) {
            Some(mut p) => {
                if p.last_message_content.as_ref() == message.as_str() {
                    p.spam += 1;
                } else {
                    p.spam = 0;
                }

                let increase = if message.len() > self.config.small_len as usize {
                    self.config.large_penalty
                } else {
                    1
                };

                let result = if p.spam > self.config.max_spam {
                    LimitType::Spam
                } else if !p.ratelimiter.update(increase) {
                    LimitType::Rate
                } else {
                    p.last_message_content = message.clone().into();
                    return Ok(message);
                };
                p.last_message_content = message.into();
                Err(result)
            }
            None => {
                self.map.insert(
                    user_id,
                    Profile {
                        spam: 0,
                        last_message_content: message.clone().into(),
                        ratelimiter: RateLimiter::new(self.config.ratelimit.clone()),
                    },
                );
                Ok(message)
            }
        }
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Message Limits", |r| async {
        let config = r
            .state::<MessageLimitsConfig>()
            .expect("Failed to load message limits config")
            .clone();
        r.manage(MessageLimiterService::new(config))
    })
}
