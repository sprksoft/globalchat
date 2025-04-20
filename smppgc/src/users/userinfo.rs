use std::{sync::Arc, time::SystemTime};

use crate::users::UserSid;

use super::{role::Role, SmId};

#[derive(Clone, Debug, Hash)]
pub struct UserInfo {
    pub mod_badge: bool,
    pub username: Arc<str>,
    pub id: u16,
    pub static_id: UserSid,
}
impl Eq for UserInfo {}
impl UserInfo {
    pub fn id(&self) -> u16 {
        self.id
    }
    #[inline]
    pub fn static_id(&self) -> UserSid {
        self.static_id.clone()
    }
    pub fn username(&self) -> &str {
        &self.username
    }
}
impl PartialEq for UserInfo {
    fn eq(&self, other: &Self) -> bool {
        other.id == self.id
    }
    fn ne(&self, other: &Self) -> bool {
        other.id != self.id
    }
}

#[derive(Clone, Debug)]
pub struct UserInfo2 {
    pub role: Role,
    pub smid: SmId,
    pub irl_name: Box<str>,
    pub ban_end_timestamp: SystemTime,
}
impl UserInfo2 {
    pub fn is_banned(&self, now: SystemTime) -> bool {
        now.duration_since(self.ban_end_timestamp).is_ok()
    }
}
