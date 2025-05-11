use std::sync::Arc;

use crate::users::UserId;

#[derive(Debug, Clone)]
pub struct ChatUser {
    pub(super) username: Arc<str>,
    pub(super) mod_badge: bool,
    pub(super) user_id: UserId,
    pub(super) local_id: u16,
}
impl ChatUser {
    pub fn arc_username(&self) -> Arc<str> {
        self.username.clone()
    }
    pub fn username(&self) -> &str {
        &self.username
    }
    pub fn local_id(&self) -> u16 {
        self.local_id
    }
    pub fn user_id(&self) -> UserId {
        self.user_id
    }
    pub fn mod_badge(&self) -> bool {
        self.mod_badge
    }
}
