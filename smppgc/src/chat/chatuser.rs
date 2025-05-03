use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ChatUser {
    username: Arc<str>,
    mod_badge: bool,
    local_id: u16,
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
    pub fn mod_badge(&self) -> bool {
        self.mod_badge
    }
}
