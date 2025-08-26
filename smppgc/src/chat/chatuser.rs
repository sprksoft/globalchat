use crate::users::{role::Role, UserId};

#[derive(Debug, Clone)]
pub struct ChatUser {
    pub(super) username: String,
    pub(super) mod_badge: bool,
    pub(super) user_id: UserId,
    pub(super) local_id: u16,
    pub(super) role: Role,
}
impl ChatUser {
    pub fn username(&self) -> &str {
        &self.username
    }
    pub fn local_id(&self) -> u16 {
        self.local_id
    }
    pub fn user_id(&self) -> UserId {
        self.user_id
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn mod_badge(&self) -> bool {
        self.mod_badge
    }
}
