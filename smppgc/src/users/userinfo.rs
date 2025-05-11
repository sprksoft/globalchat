use std::time::SystemTime;

use super::{role::Role, UserId};

#[derive(Clone, Debug)]
pub struct UserInfo {
    pub role: Role,
    pub id: UserId,
    pub irl_name: Box<str>,
    pub ban_end_timestamp: SystemTime,
}
impl UserInfo {
    pub fn is_banned(&self, now: SystemTime) -> bool {
        now.duration_since(self.ban_end_timestamp).is_ok()
    }
}
