use rocket::fairing::AdHoc;
use serde::Deserialize;

use crate::ratelimit::RateLimitConfig;

#[derive(Clone, Deserialize)]
pub struct UserConfig {
    pub max_session_age: usize,
}

#[derive(Clone, Deserialize)]
pub struct NameConfig {
    pub max_claimed_names: usize,
    pub max_name_retention: usize,
    pub max_username_len: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatConfig {
    pub max_stored_messages: usize,
    pub max_users: u16,
    pub max_ro_users: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MessageLimitsConfig {
    pub small_len: u16,
    pub max_len: u16,
    pub min_len: u16,
    pub large_penalty: u32,

    pub max_spam: u32,

    pub ratelimit: RateLimitConfig,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("config", |r| async {
        r.attach(AdHoc::config::<UserConfig>())
            .attach(AdHoc::config::<NameConfig>())
            .attach(AdHoc::config::<ChatConfig>())
            .attach(AdHoc::config::<MessageLimitsConfig>())
    })
}
