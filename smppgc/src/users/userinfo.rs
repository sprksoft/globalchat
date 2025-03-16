use std::sync::Arc;

use crate::users::UserSid;

#[derive(Clone, Debug, Hash)]
pub struct UserInfo {
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
