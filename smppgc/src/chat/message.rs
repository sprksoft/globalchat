use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: Arc<str>,
    pub content: Arc<str>,
    pub timestamp: u32,
    pub sender_id: u16,
}
impl Message {
    pub fn new_response(message: &Message, content: Arc<str>) -> Self {
        Self {
            sender: "system".into(),
            content: content,
            timestamp: message.timestamp,
            sender_id: message.sender_id,
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
