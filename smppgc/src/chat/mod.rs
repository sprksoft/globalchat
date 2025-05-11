use circular_queue::CircularQueue;
use log::*;
use profanity::ProfanityFilter;
use rocket::{fairing::AdHoc, routes};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    Mutex,
};

mod chatuser;
mod message;
mod message_limits;
pub mod socket;
pub use chatuser::*;
pub use message::*;

pub use message_limits::*;

use crate::{
    users::{ClaimedName, UserInfo},
    utils::IdCounter,
    ChatConfig, Snowflake, SnowflakeGenerator,
};
use lmetrics::metrics;
use thiserror::Error;

metrics! {
    pub counter joined_total("Total joined users",[]);
    pub counter left_total("Total left users", []);
    pub counter history_events_lost_total("Total history events lost. ", [event]);
}

#[derive(Debug, Error)]
pub enum NewClientError {
    #[error("Max concurrent user count reached")]
    MaxConcurrentUserCount,
    #[error("User already in chat")]
    AlreadyInChat,
}

struct StoredUser {
    pub user: ChatUser,
    message_count: usize,
    ghost: bool,
}

#[derive(Copy, Clone)]
pub enum MessageChangeType {
    Censored,
    Deleted,
}
#[derive(Copy, Clone)]
pub struct MessageChange {
    pub message_id: Snowflake,
    pub ty: MessageChangeType,
}

pub struct Chat {
    messages_sender: broadcast::Sender<Message>,
    join_sender: broadcast::Sender<ChatUser>,
    left_sender: broadcast::Sender<ChatUser>,
    message_change_sender: broadcast::Sender<MessageChange>,

    users: Arc<Mutex<HashMap<u16, StoredUser>>>,
    history: Arc<Mutex<CircularQueue<Message>>>,
    client_ids: IdCounter,
    message_ids: Arc<SnowflakeGenerator>,

    shutdown: broadcast::Sender<()>,

    config: ChatConfig,
}
impl Chat {
    pub fn new(config: ChatConfig) -> Self {
        let (messages_sender, messages_receiver) = broadcast::channel(20);
        let (join_sender, _) = broadcast::channel(20);
        let (left_sender, left_receiver) = broadcast::channel(20);

        let (message_change_sender, _) = broadcast::channel(20);

        let users = Arc::new(Mutex::new(HashMap::new()));
        let history = Arc::new(Mutex::new(CircularQueue::with_capacity(
            config.max_stored_messages,
        )));

        let (shutdown, shutdown_receiver) = broadcast::channel(1);

        Self::spawn_histrec(
            left_receiver,
            messages_receiver,
            users.clone(),
            history.clone(),
            shutdown_receiver,
        );

        Self {
            message_ids: SnowflakeGenerator::new().into(),
            message_change_sender,
            messages_sender,
            join_sender,
            left_sender,
            users,
            history,
            client_ids: IdCounter::new(),
            config: config.into(),
            shutdown,
        }
    }
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }

    fn spawn_histrec(
        mut left_receiver: broadcast::Receiver<ChatUser>,
        mut messages_receiver: broadcast::Receiver<Message>,
        users: Arc<Mutex<HashMap<u16, StoredUser>>>,
        history: Arc<Mutex<CircularQueue<Message>>>,
        mut shutdown_receiver: broadcast::Receiver<()>,
    ) {
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_receiver.recv() => {
                        return;
                    }
                    left_client = left_receiver.recv() => {
                        match left_client{
                            Ok(left_client)=>{
                                left_total::inc();
                                trace!("User {} left", left_client.local_id());
                                {
                                    let mut users = users.lock().await;
                                    if let Some(entry) = users.get_mut(&left_client.local_id()){
                                        entry.ghost=true;
                                        if entry.message_count == 0 {
                                            users.remove(&left_client.local_id());
                                        }
                                    }
                                }
                            },
                            Err(RecvError::Closed)=>{
                                return;
                            },
                            Err(RecvError::Lagged(count))=>{
                                history_events_lost_total::inc("message");
                                error!("Lost {} client left events. while recording history", count);
                            }
                        }
                    },
                    mesg = messages_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                {
                                    let mut users = users.lock().await;
                                    if let Some(user) = users.get_mut(&mesg.sender.local_id()) {
                                        user.message_count+=1;
                                    }
                                    if let Some(deleted_message) = history.lock().await.push(mesg) {
                                        let id = deleted_message.sender.local_id();
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
                                history_events_lost_total::inc("message");
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
        userinfo: &UserInfo,
        leased_name: ClaimedName,
        mod_badge: bool,
        bypass_user_count: bool,
    ) -> Result<ChatClient, NewClientError> {
        let mut user_lock = self.users.lock().await;
        if !bypass_user_count
            && self.config.max_users != 0
            && self.config.max_users <= user_lock.len() as u16
        {
            return Err(NewClientError::MaxConcurrentUserCount);
        }

        let user_id = userinfo.id;
        let local_id = self.client_ids.new_id();

        if user_lock
            .iter()
            .find(|(_, u)| !u.ghost && user_id == u.user.user_id())
            .is_some()
        {
            return Err(NewClientError::AlreadyInChat);
        }

        let username: String = leased_name.into();
        let user = ChatUser {
            local_id,
            user_id,
            mod_badge,
            username: username.into(),
        };
        let client = ChatClient {
            user,
            left_sender: self.left_sender.clone(),
            message_sender: self.messages_sender.clone(),
            message_receiver: self.messages_sender.subscribe(),
            join_receiver: self.join_sender.subscribe(),
            message_change_receiver: self.message_change_sender.subscribe(),
            message_id_gen: self.message_ids.clone(),
        };

        let _ = self.join_sender.send(client.user().clone()); // throws error when no receivers

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

    pub async fn history<'a>(
        &'a self,
        starting_snowflake: Snowflake,
        profanity: bool,
    ) -> Vec<Message> {
        self.history
            .lock()
            .await
            .asc_iter()
            .filter(|m| m.id() > starting_snowflake)
            .filter(|m| !m.profanity || profanity)
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
                let _ = self.message_change_sender.send(MessageChange {
                    message_id: mesg.id(),
                    ty: MessageChangeType::Censored,
                });
            }
        }
        *lock = new_messages;
    }
    pub async fn delete_message(&self, snowflake: Snowflake) {
        let mut lock = self.history.lock().await;
        let mut new_messages = CircularQueue::with_capacity(lock.capacity());

        // TODO: When my pullrequest gets released on circular_queue use into Vec<T>
        for mesg in lock.asc_iter().cloned() {
            if mesg.id() == snowflake {
                let _ = self.message_change_sender.send(MessageChange {
                    message_id: snowflake,
                    ty: MessageChangeType::Deleted,
                });
            } else {
                new_messages.push(mesg);
            }
        }
        *lock = new_messages;
    }

    pub async fn users(&self) -> Vec<ChatUser> {
        self.users
            .lock()
            .await
            .iter()
            .map(|u| &u.1.user)
            .cloned()
            .collect()
    }
}

pub struct ChatClient {
    user: ChatUser,
    message_id_gen: Arc<SnowflakeGenerator>,
    left_sender: rocket::tokio::sync::broadcast::Sender<ChatUser>,
    message_sender: broadcast::Sender<Message>,
    pub message_receiver: broadcast::Receiver<Message>,
    pub join_receiver: broadcast::Receiver<ChatUser>,
    pub message_change_receiver: broadcast::Receiver<MessageChange>,
}
impl ChatClient {
    #[inline]
    pub fn user(&self) -> &ChatUser {
        &self.user
    }

    #[inline]
    pub fn new_message(&self, content: Arc<str>, profanity: bool) -> Message {
        Message {
            id: self.message_id_gen.new_snowflake(),
            content,
            profanity,
            sender: self.user().clone(),
        }
    }

    #[inline]
    pub fn send(&self, mesg: Message) {
        let _ = self.message_sender.send(mesg);
    }
}
impl Drop for ChatClient {
    fn drop(&mut self) {
        match self.left_sender.send(self.user().clone()) {
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

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("chat", |r| async {
        let config = r
            .figment()
            .extract::<ChatConfig>()
            .expect("No chat config found");

        r.mount("/", routes![socket::chat_socket])
            .attach(message_limits::stage())
            .manage(Chat::new(config))
            .attach(AdHoc::on_shutdown("Chat shutdown", |r| {
                Box::pin(async move {
                    if let Some(chat) = r.state::<Chat>() {
                        chat.shutdown();
                    }
                })
            }))
    })
}
