use crate::models::{Role, UserId};

#[derive(Debug, Clone)]
pub struct ChatUser {
    pub(crate) username: String,
    pub(crate) mod_badge: bool,
    pub(crate) user_id: UserId,
    pub(crate) local_id: u16,
    pub(crate) role: Role,
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
