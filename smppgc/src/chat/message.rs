use super::ChatUser;
use crate::Snowflake;
use wordfilter::TokenizedString;

#[derive(Clone, Debug)]
pub struct Message {
    pub content: TokenizedString,
    pub profanity: bool,
    pub sender: ChatUser,
    pub id: Snowflake,
}
impl Message {
    pub fn is_valid(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        true
    }
    pub fn id(&self) -> Snowflake {
        self.id
    }
    pub fn len(&self) -> usize {
        self.content.len()
    }
    pub fn is_empty(&self) -> bool {
        self.content
            .words()
            .filter(|(w, _)| w.trim().len() > 0)
            .next()
            .is_none()
    }
}
