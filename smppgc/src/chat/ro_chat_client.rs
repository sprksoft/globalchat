use std::sync::{atomic::AtomicUsize, Arc};

use lmetrics::metrics;
use tokio::sync::broadcast::{self, error::RecvError};

use crate::chat::{Chat, ChatEvent, NewClientError};

metrics! {
    pub counter ro_joined_total("Total joined readonly users", []);
    pub counter ro_left_total("Total joined readonly users", []);
}

pub struct RoChatClient {
    ro_user_count: Arc<AtomicUsize>,
    event_receiver: broadcast::Receiver<ChatEvent>,
}
impl RoChatClient {
    pub(super) fn new(chat: &Chat) -> Result<RoChatClient, NewClientError> {
        let ro_user_count = chat.ro_user_count.clone();

        if ro_user_count.load(std::sync::atomic::Ordering::Relaxed) >= chat.config.max_ro_users {
            return Err(NewClientError::MaxConcurrentUserCount);
        }
        ro_user_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        ro_joined_total::inc();
        Ok(RoChatClient {
            event_receiver: chat.event_sender.subscribe(),
            ro_user_count,
        })
    }
}

impl RoChatClient {
    #[inline]
    pub async fn recv(&mut self) -> Result<ChatEvent, RecvError> {
        self.event_receiver.recv().await
    }
}
impl Drop for RoChatClient {
    fn drop(&mut self) {
        ro_left_total::inc();
        self.ro_user_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
