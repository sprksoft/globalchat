use std::sync::Arc;

use crate::Snowflake;

use super::ChatUser;

#[derive(Clone, Debug)]
pub struct Message {
    pub content: Arc<str>,
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
        for char in self.content.chars() {
            if !char.is_whitespace() {
                return false;
            }
        }
        true
    }
}
