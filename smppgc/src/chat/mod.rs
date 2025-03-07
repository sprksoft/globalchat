use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use circular_queue::CircularQueue;
use log::*;
use profanity::ProfanityFilter;
use tokio::sync::{
    broadcast::{self, error::RecvError},
    Mutex,
};

mod message;
pub use message::*;

use crate::{
    users::{ClaimedName, UserInfo, UserSid},
    utils::IdCounter,
    ChatConfig, Snowflake, SnowflakeGenerator,
};
use lmetrics::metrics;
use thiserror::Error;

metrics! {
    pub counter joined_total("Total joined users",[]);
    pub counter left_total("Total left users", []);
    pub counter history_messages_lost_total("Total count of messages lost while trying to record msg history", []);
    pub counter client_left_events_lost_total("Total count of client_left events lost.", []);
}

#[derive(Debug, Error)]
pub enum NewClientError {
    #[error("Max concurrent user count reached")]
    MaxConcurrentUserCount,
}

struct ChatUserInfo {
    message_count: usize,
    ghost: bool,
    user_info: UserInfo,
}

pub struct Chat {
    messages_sender: broadcast::Sender<Message>,
    join_sender: broadcast::Sender<UserInfo>,
    left_sender: broadcast::Sender<UserInfo>,
    message_delete_sender: broadcast::Sender<Message>,

    users: Arc<Mutex<HashMap<u16, ChatUserInfo>>>,
    history: Arc<Mutex<CircularQueue<Message>>>,
    client_ids: IdCounter,
    message_ids: Arc<SnowflakeGenerator>,

    config: ChatConfig,
}
impl Chat {
    pub fn new(config: ChatConfig) -> Self {
        let (messages_sender, messages_receiver) = broadcast::channel(20);
        let (join_sender, _) = broadcast::channel(20);
        let (left_sender, left_receiver) = broadcast::channel(20);

        let (message_delete_sender, _) = broadcast::channel(20);

        let users = Arc::new(Mutex::new(HashMap::new()));
        let history = Arc::new(Mutex::new(CircularQueue::with_capacity(
            config.max_stored_messages,
        )));

        Self::spawn_histrec(
            left_receiver,
            messages_receiver,
            users.clone(),
            history.clone(),
        );

        Self {
            message_ids: SnowflakeGenerator::new().into(),
            message_delete_sender,
            messages_sender,
            join_sender,
            left_sender,
            users,
            history,
            client_ids: IdCounter::new(),
            config: config.into(),
        }
    }

    fn spawn_histrec(
        mut left_receiver: broadcast::Receiver<UserInfo>,
        mut messages_receiver: broadcast::Receiver<Message>,
        users: Arc<Mutex<HashMap<u16, ChatUserInfo>>>,
        history: Arc<Mutex<CircularQueue<Message>>>,
    ) {
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    left_client = left_receiver.recv() => {
                        match left_client{
                            Ok(left_client)=>{
                                left_total::inc();
                                trace!("User {} left", left_client.id());
                                {
                                    let mut users = users.lock().await;
                                    if let Some(entry) = users.get_mut(&left_client.id()){
                                        entry.ghost=true;
                                        if entry.message_count == 0 {
                                            users.remove(&left_client.id());
                                        }
                                    }
                                }
                            },
                            Err(RecvError::Closed)=>{
                                return;
                            },
                            Err(RecvError::Lagged(count))=>{
                                client_left_events_lost_total::inc();
                                error!("main client_left receiver lagged behind {} times. Ghosts will appear", count);
                            }
                        }
                    },
                    mesg = messages_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                {
                                    let mut users = users.lock().await;
                                    if let Some(user) = users.get_mut(&mesg.sender.id()) {
                                        user.message_count+=1;
                                    }
                                    if let Some(deleted_message) = history.lock().await.push(mesg) {
                                        let id = deleted_message.sender.id();
                                        if let Some(entry) = users.get_mut(&id){
                                            entry.message_count = entry.message_count.saturating_sub(1);
                                            if entry.ghost == true && entry.message_count == 0{
                                                users.remove(&id);
                                            }
                                        }
                                    }
                                }
                            },
                            Err(RecvError::Closed) => {
                                return;
                            },
                            Err(RecvError::Lagged(count))=>{
                                history_messages_lost_total::inc();
                                error!("Lost {} messages. while recording storing history", count);
                            }
                        }
                    }

                }
            }
        });
    }

    pub async fn new_client(
        &self,
        static_id: UserSid,
        leased_name: ClaimedName,
    ) -> Result<ChatClient, NewClientError> {
        if self.config.max_users != 0
            && self.config.max_users <= self.users.lock().await.len() as u16
        {
            return Err(NewClientError::MaxConcurrentUserCount);
        }

        let id = self.client_ids.new_id();
        let user_info = UserInfo {
            username: leased_name.into(),
            static_id,
            id,
        };
        let client = ChatClient {
            user_info,
            left_sender: self.left_sender.clone(),
            message_sender: self.messages_sender.clone(),
            message_receiver: self.messages_sender.subscribe(),
            join_receiver: self.join_sender.subscribe(),
            message_delete_receiver: self.message_delete_sender.subscribe(),
            message_id_gen: self.message_ids.clone(),
        };

        let _ = self.join_sender.send(client.user_info()); // throws error when no receivers

        self.users.lock().await.insert(
            id,
            ChatUserInfo {
                message_count: 0,
                ghost: false,
                user_info: client.user_info(),
            },
        );
        joined_total::inc();

        Ok(client)
    }

    pub async fn history<'a>(
        &'a self,
        starting_snowflake: Snowflake,
        profanity: bool,
    ) -> Vec<Message> {
        self.history
            .lock()
            .await
            .asc_iter()
            .filter(|m| m.id() > starting_snowflake && (!profanity && !m.profanity))
            .cloned()
            .collect()
    }

    pub async fn run_filter(&self, filter: &ProfanityFilter) {
        let mut lock = self.history.lock().await;
        let mut new_messages = CircularQueue::with_capacity(lock.capacity());
        // TODO: When my pullrequest gets released on circular_queue use into Vec<T>
        for mut mesg in lock.iter().cloned() {
            let (content_tokenized, new_content) = filter.tokenize(&mesg.content);
            mesg.content = new_content.into();
            if filter.check(&content_tokenized).is_none() {
                new_messages.push(mesg);
            } else {
                let _ = self.message_delete_sender.send(mesg);
            }
        }
        *lock = new_messages;
    }

    pub async fn users(&self) -> Vec<UserInfo> {
        self.users
            .lock()
            .await
            .iter()
            .map(|u| &u.1.user_info)
            .cloned()
            .collect()
    }
}

pub struct ChatClient {
    user_info: UserInfo,
    message_id_gen: Arc<SnowflakeGenerator>,
    left_sender: rocket::tokio::sync::broadcast::Sender<UserInfo>,
    message_sender: broadcast::Sender<Message>,
    pub message_receiver: broadcast::Receiver<Message>,
    pub join_receiver: broadcast::Receiver<UserInfo>,
    pub message_delete_receiver: broadcast::Receiver<Message>,
}
impl ChatClient {
    #[inline]
    pub fn user_info(&self) -> UserInfo {
        self.user_info.clone()
    }

    #[inline]
    pub fn new_message(&self, content: Arc<str>, profanity: bool) -> Message {
        Message {
            id: self.message_id_gen.new_snowflake(),
            content,
            profanity,
            sender: self.user_info(),
        }
    }

    #[inline]
    pub fn send(&self, mesg: Message) {
        let _ = self.message_sender.send(mesg);
    }
}
impl Drop for ChatClient {
    fn drop(&mut self) {
        match self.left_sender.send(self.user_info()) {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "Failed to send leave event (This will cause ghosts to appear): {}",
                    err
                )
            }
        };
    }
}
