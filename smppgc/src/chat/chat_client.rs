use std::sync::Arc;

use lmetrics::metrics;
use nanotime::snowflake::SnowflakeGenerator;
use tokio::sync::broadcast::{self, error::RecvError};

use crate::{
    chat::{Chat, ChatEvent, NewClientError, StoredUser},
    models::{ChatUser, ClaimedName, Message, User},
    wf::TokenizedString,
};

metrics! {
    pub counter joined_total("Total joined users",[]);
    pub counter left_total("Total left users", []);
}

pub struct ChatClient {
    user: Arc<ChatUser>,
    message_id_gen: Arc<SnowflakeGenerator>,
    event_sender: broadcast::Sender<ChatEvent>,
    event_receiver: broadcast::Receiver<ChatEvent>,
    user_count: u16,
}
impl ChatClient {
    pub(super) async fn new(
        chat: &Chat,
        user: &User,
        leased_name: ClaimedName,
        mod_badge: bool,
        bypass_user_count: bool,
    ) -> Result<ChatClient, NewClientError> {
        let mut user_lock = chat.users.lock().await;
        if !bypass_user_count
            && chat.config.max_users != 0
            && chat.config.max_users <= user_lock.len() as u16
        {
            return Err(NewClientError::MaxConcurrentUserCount);
        }

        let user_id = user.id();
        let local_id = chat.gen_client_id(&user_lock);

        if user_lock
            .iter()
            .find(|(_, u)| !u.ghost && user_id == u.user.user_id())
            .is_some()
        {
            return Err(NewClientError::AlreadyInChat);
        }

        let username: String = leased_name.into();
        let user = Arc::from(ChatUser {
            local_id,
            user_id,
            mod_badge,
            role: user.role(),
            username,
        });

        let user_count = user_lock.iter().filter(|(_, u)| !u.ghost).count();
        let user_count = if user_count > u16::MAX as usize {
            u16::MAX
        } else {
            user_count as u16
        };
        let client = ChatClient {
            user_count,
            user,
            message_id_gen: chat.message_ids.clone(),
            event_receiver: chat.event_sender.subscribe(),
            event_sender: chat.event_sender.clone(),
        };

        let _ = chat
            .event_sender
            .send(ChatEvent::Join(client.user().clone())); // throws error when no receivers

        user_lock.insert(
            local_id,
            StoredUser {
                user: client.user().clone(),
                ghost: false,
                message_count: 0,
            },
        );
        joined_total::inc();

        Ok(client)
    }

    pub fn user_count(&self) -> u16 {
        self.user_count
    }

    #[inline]
    pub fn user(&self) -> &ChatUser {
        &self.user
    }

    pub fn is_me(&self, id: u16) -> bool {
        self.user().local_id() == id
    }

    #[inline]
    pub fn new_message(&self, content: TokenizedString) -> Message {
        Message {
            id: self.message_id_gen.new_snowflake(),
            content,
            sender: self.user.clone(),
        }
    }

    #[inline]
    pub async fn recv(&mut self) -> Result<ChatEvent, RecvError> {
        let event = self.event_receiver.recv().await?;
        if matches!(event, ChatEvent::Join(_)) {
            self.user_count += 1;
        } else if matches!(event, ChatEvent::Leave(_)) {
            self.user_count -= 1;
        }

        Ok(event)
    }

    #[inline]
    pub fn send(&self, mesg: impl Into<Arc<Message>>) {
        let _ = self.event_sender.send(ChatEvent::NewMessage(mesg.into()));
    }
}
impl Drop for ChatClient {
    fn drop(&mut self) {
        let _ = self
            .event_sender
            .send(ChatEvent::Leave(self.user().clone()));
    }
}
