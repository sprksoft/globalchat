use std::sync::Arc;

use crate::{users::UserInfo, Timestamp};

#[derive(Clone, Debug)]
pub struct Message {
    pub content: Arc<str>,
    pub sender: UserInfo,
    pub timestamp: Timestamp,
}
impl Message {
    pub fn new(sender: UserInfo, timestamp: Timestamp, content: Arc<str>) -> Self {
        Self {
            content,
            sender,
            timestamp,
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        true
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
