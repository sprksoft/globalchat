use circular_queue::CircularQueue;
use log::*;
use nanotime::snowflake::{Snowflake, SnowflakeGenerator};
use rocket::{fairing::AdHoc, routes};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    Mutex, MutexGuard,
};
use wordfilter::{TokenizedString, WordFilter};

mod chatuser;
mod message;
mod message_limits;
pub mod socket;
pub use chatuser::*;
pub use message::*;

pub use message_limits::*;

use crate::{
    users::{ClaimedName, User, UserId},
    utils::IdCounter,
    ChatConfig,
};
use lmetrics::metrics;
use thiserror::Error;

metrics! {
    pub counter ro_joined_total("Total joined readonly users", []);
    pub counter ro_left_total("Total joined readonly users", []);
    pub counter joined_total("Total joined users",[]);
    pub counter left_total("Total left users", []);
    pub counter history_events_lost_total("Total history events lost.");
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

#[derive(Clone)]
pub enum ChatEvent {
    Join(ChatUser),
    Leave(ChatUser),
    NewMessage(Arc<Message>),
    MessageChange(Arc<Message>, MessageChangeType),
    Kick(UserId),
}
#[derive(Clone, Copy)]
pub enum MessageChangeType {
    Filter(bool),
    Deleted,
}

type Users = HashMap<u16, StoredUser>;

pub struct Chat {
    event_sender: broadcast::Sender<ChatEvent>,

    users: Arc<Mutex<Users>>,
    history: Arc<Mutex<CircularQueue<Arc<Message>>>>,
    client_ids: IdCounter,
    message_ids: Arc<SnowflakeGenerator>,

    shutdown: broadcast::Sender<()>,
    ro_user_count: Arc<AtomicUsize>,

    config: ChatConfig,
}
impl Chat {
    pub fn new(config: ChatConfig) -> Self {
        let (event_sender, event_receiver) = broadcast::channel(20);

        let users = Arc::new(Mutex::new(HashMap::new()));
        let history = Arc::new(Mutex::new(CircularQueue::with_capacity(
            config.max_stored_messages,
        )));

        let (shutdown, shutdown_receiver) = broadcast::channel(1);

        Self::spawn_histrec(
            event_receiver,
            users.clone(),
            history.clone(),
            shutdown_receiver,
        );

        Self {
            event_sender,
            users,
            history,
            client_ids: IdCounter::new(),
            message_ids: SnowflakeGenerator::new().into(),
            shutdown,
            ro_user_count: Arc::new(AtomicUsize::new(0)),
            config: config.into(),
        }
    }
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }

    fn spawn_histrec(
        mut event_receiver: broadcast::Receiver<ChatEvent>,
        users: Arc<Mutex<Users>>,
        history: Arc<Mutex<CircularQueue<Arc<Message>>>>,
        mut shutdown_receiver: broadcast::Receiver<()>,
    ) {
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_receiver.recv() => {
                        return;
                    }
                    event = event_receiver.recv() => {
                        match event {
                            Ok(ChatEvent::Leave(left_client)) => {
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
                            Ok(ChatEvent::NewMessage(mesg)) => {
                                {
                                    let mut users = users.lock().await;
                                    if let Some(user) = users.get_mut(&mesg.sender.local_id()) {
                                        user.message_count+=1;
                                    }
                                    if let Some(deleted_message) = history.lock().await.push(mesg) {
                                        Self::del_message(&deleted_message, users);
                                    }
                                }

                            }
                            Ok(ChatEvent::Join(_)) | Ok(ChatEvent::MessageChange(_, _)) | Ok(ChatEvent::Kick(_)) => {},
                            Err(RecvError::Closed) => {
                                return;
                            },
                            Err(RecvError::Lagged(count))=>{
                                history_events_lost_total::inc_by(count);
                                error!("Lost {} chat events. while recording history", count);
                            }
                        }
                    }

                }
            }
        });
    }

    fn del_message<'a>(mesg: &Message, mut users: MutexGuard<'a, Users>) {
        let id = mesg.sender.local_id();
        if let Some(entry) = users.get_mut(&id) {
            entry.message_count = entry.message_count.saturating_sub(1);
            if entry.ghost == true && entry.message_count == 0 {
                users.remove(&id);
            }
        }
    }

    pub async fn new_client(
        &self,
        user: &User,
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

        let user_id = user.id();
        let local_id = self.client_ids.new_id();

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
        let client = ChatClient {
            user,
            message_id_gen: self.message_ids.clone(),
            event_receiver: self.event_sender.subscribe(),
            event_sender: self.event_sender.clone(),
        };

        let _ = self
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

    pub async fn new_roclient(&self) -> Result<RoChatClient, NewClientError> {
        if self
            .ro_user_count
            .load(std::sync::atomic::Ordering::Relaxed)
            >= self.config.max_ro_users
        {
            return Err(NewClientError::MaxConcurrentUserCount);
        }
        self.ro_user_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        ro_joined_total::inc();
        Ok(RoChatClient {
            event_receiver: self.event_sender.subscribe(),
            ro_user_count: self.ro_user_count.clone(),
        })
    }

    pub fn kick(&self, user_id: UserId) {
        let _ = self.event_sender.send(ChatEvent::Kick(user_id));
    }

    pub async fn history<'a>(
        &'a self,
        starting_snowflake: Snowflake,
        profanity: bool,
    ) -> Vec<Arc<Message>> {
        self.history
            .lock()
            .await
            .asc_iter()
            .filter(|m| m.id() > starting_snowflake)
            .filter(|m| !m.prof() || profanity)
            .cloned()
            .collect()
    }

    pub async fn run_filter(&self, filter: &WordFilter) {
        let mut lock = self.history.lock().await;
        for mesg in lock.iter_mut() {
            let mut new_mesg: Message = (*mesg.as_ref()).clone();
            if new_mesg.content.recheck(filter) {
                let new_mesg: Arc<Message> = Arc::from(new_mesg);
                let _ = self.event_sender.send(ChatEvent::MessageChange(
                    new_mesg.clone(),
                    MessageChangeType::Filter(new_mesg.prof()),
                ));
                *mesg = new_mesg;
            }
        }
    }
    pub async fn retain_messages<F: Fn(&Message) -> bool>(&self, f: F) -> bool {
        let mut lock = self.history.lock().await;
        let mut new_messages = CircularQueue::with_capacity(lock.capacity());

        // TODO: When my pullrequest gets released on circular_queue use into Vec<T>
        let mut deleted = false;
        for mesg in lock.asc_iter().cloned() {
            if f(&mesg) {
                new_messages.push(mesg);
            } else {
                Self::del_message(&mesg, self.users.lock().await);
                let _ = self
                    .event_sender
                    .send(ChatEvent::MessageChange(mesg, MessageChangeType::Deleted));
                deleted = true;
            }
        }
        *lock = new_messages;
        deleted
    }
    pub async fn get_author_uid(&self, snowflake: Snowflake) -> Option<UserId> {
        let lock = self.history.lock().await;
        lock.iter()
            .find(|m| m.id() == snowflake)
            .map(|m| m.sender.user_id())
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

pub struct RoChatClient {
    ro_user_count: Arc<AtomicUsize>,
    event_receiver: broadcast::Receiver<ChatEvent>,
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

pub struct ChatClient {
    user: Arc<ChatUser>,
    message_id_gen: Arc<SnowflakeGenerator>,
    event_sender: broadcast::Sender<ChatEvent>,
    event_receiver: broadcast::Receiver<ChatEvent>,
}
impl ChatClient {
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
            content: content.into(),
            sender: self.user.clone(),
        }
    }

    #[inline]
    pub async fn recv(&mut self) -> Result<ChatEvent, RecvError> {
        self.event_receiver.recv().await
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

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("chat", |r| async {
        let config = r
            .figment()
            .extract::<ChatConfig>()
            .expect("No chat config found");

        r.mount(
            "/",
            routes![socket::chat_socket, socket::readonly_chat_socket],
        )
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
