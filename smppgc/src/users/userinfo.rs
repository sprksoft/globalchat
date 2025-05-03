use std::{sync::Arc, time::SystemTime};

use crate::users::UserSid;

use super::{role::Role, SmId};

#[derive(Clone, Debug)]
pub struct UserInfo2 {
    pub role: Role,
    pub smid: SmId,
    pub id: i32,
    pub irl_name: Box<str>,
    pub ban_end_timestamp: SystemTime,
}
impl UserInfo2 {
    pub fn is_banned(&self, now: SystemTime) -> bool {
        now.duration_since(self.ban_end_timestamp).is_ok()
    }
}
