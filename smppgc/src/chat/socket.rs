use lmetrics::metrics;
use rocket::{get, response, Responder, Shutdown, State};

use log::*;
use rocket_ws::{Channel, WebSocket};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    chat::{message_limits::LimitType, Chat, ChatEvent, MessageChangeType, NewClientError},
    users::{Ban, BanError, NameClaimError, User, UserManager},
    wf::Filter,
    wsprotocol::{AdminCmd, C2SPacket, KickReason, WsClient},
    Snowflake,
};

use super::{message_limits::MessageLimiter, ChatClient};

#[derive(Responder)]
pub enum ChatSocketResponder<'a> {
    #[response(status = 200)]
    Channel(Channel<'a>),
}
impl<'a> ChatSocketResponder<'a> {
    pub fn ws_close(ws: WebSocket, reason: KickReason) -> ChatSocketResponder<'a> {
        ChatSocketResponder::Channel(ws.channel(move |mut stream| {
            Box::pin(async move { stream.close(Some(reason.into_close_frame())).await })
        }))
    }
    pub fn ws_ban(ws: WebSocket, ban: Ban) -> ChatSocketResponder<'a> {
        ChatSocketResponder::Channel(ws.channel(move |mut stream| {
            Box::pin(async move { stream.close(Some(ban.into_close_frame())).await })
        }))
    }
}

metrics! {
    pub counter messages_total("Total count of sent messages");
    pub counter messages_blocked("Total count of blocked messages", [reason]);
}

#[get("/socket/chat?<username>&<start_time>&<mod_badge>")]
pub async fn chat_socket<'a>(
    username: &str,
    start_time: Option<Snowflake>,
    mod_badge: Option<bool>,
    ws: WebSocket,
    mut user_manager: UserManager<'a>,
    mesg_limiter: &'a State<MessageLimiter>,
    filter: &'a State<Filter>,
    chat: &'a State<Chat>,
    mut shutdown: Shutdown,
    user: Option<User>,
) -> Result<ChatSocketResponder<'a>, response::Debug<sqlx::Error>> {
    let Some(user) = user else {
        return Ok(ChatSocketResponder::ws_close(ws, KickReason::NoSession));
    };

    let start_time = start_time.unwrap_or(Snowflake::ZERO);
    let my_role = user.role();

    if !my_role.is_mod() {
        if let Some(ban) = user_manager.get_ban(user.id()).await? {
            return Ok(ChatSocketResponder::ws_ban(ws, ban));
        }
    }

    let claimed_name = match user_manager.claim_name(&user, username).await {
        Ok(name_lease) => name_lease,
        Err(NameClaimError::Sqlx(e)) => return Err(response::Debug(e)),
        Err(NameClaimError::Invalid(invalid)) => {
            return Ok(ChatSocketResponder::ws_close(ws, invalid.into_kickreason()));
        }
    };

    let mod_badge = my_role.is_mod() && mod_badge.unwrap_or(false);
    let mut chat_client = match chat
        .new_client(&user, claimed_name, mod_badge, my_role.is_mod())
        .await
    {
        Ok(c) => c,
        Err(e) => {
            info!("Closing connection: {:?}", e);
            match e {
                NewClientError::AlreadyInChat => {
                    return Ok(ChatSocketResponder::ws_close(ws, KickReason::AlreadyInChat))
                }
                NewClientError::MaxConcurrentUserCount => {
                    return Ok(ChatSocketResponder::ws_close(ws, KickReason::ChatFull))
                }
            }
        }
    };

    Ok(ChatSocketResponder::Channel(ws.channel(move |stream| {
        Box::pin(async move {
            let chat_hist = chat.history(start_time, my_role.is_mod()).await;

            let mut wsclient = WsClient::new(
                stream,
                chat.users().await,
                chat_hist,
                chat_client.user()
            )
            .await?;

            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        wsclient.disconnect(KickReason::ServerShutdown).await?;
                        continue;
                    }
                    mesg = wsclient.try_recv() => {
                        let Some(packet) = mesg? else { continue; };
                        match packet {
                            C2SPacket::Message(content) => {
                                let mesg = {
                                    match mesg_limiter.feed(user.id(), content) {
                                        Ok(content) => {
                                            let lock = filter.wf.read().await;
                                            let content = lock.check(&content);
                                            drop(lock);
                                            if !content.good() {
                                                messages_blocked::inc("profanity");
                                            }
                                            dbg!(&content);
                                            let mesg = chat_client.new_message(content);
                                            wsclient.forward(&mesg).await?;
                                            mesg
                                        },
                                        Err(LimitType::Rate) => {
                                            messages_blocked::inc("ratelimit");
                                            return wsclient.disconnect(KickReason::RateLimit).await;
                                        },
                                        Err(LimitType::Size) => {
                                            messages_blocked::inc("size");
                                            continue;
                                        },
                                        Err(LimitType::Spam) => {
                                            messages_blocked::inc("spam");
                                            continue;
                                        }
                                    }
                                };
                                messages_total::inc();
                                chat_client.send(mesg);
                            },
                            C2SPacket::AdminCmd(cmd) => {
                                if !my_role.is_mod() {
                                    wsclient.system_message("Geen toegang tot admin cmd's").await?;
                                    continue;
                                }

                                match cmd {
                                    AdminCmd::DelMsg(snowflake) => {
                                        if !chat.retain_messages(|m|m.id() != snowflake).await {
                                            wsclient.system_message("Bericht bestaat niet meer op de server").await?;
                                        }
                                        continue;
                                    },
                                    AdminCmd::BanMsgAuthor{mesg, reason, duration} => {
                                        match chat.get_author_uid(mesg).await {
                                            Some(uid) =>
                                                match user_manager.ban_user(uid, my_role, &reason, duration).await {
                                                    Ok(())=>{
                                                        chat.retain_messages(|m|m.sender.user_id() != uid).await;
                                                        chat.kick(uid);
                                                    },
                                                    Err(BanError::Sqlx(e)) => {error!("While trying to ban user: {}", e); wsclient.system_message("Interne SQL error").await?;},
                                                    Err(BanError::PermissionDenied) => {wsclient.system_message("Niet toegestaan deze persoon te verbannen").await?;}
                                                },
                                            None => { wsclient.system_message("Bericht bestaat niet meer op de server").await?; }
                                        }
                                    }
                                }

                            }
                        }


                    }
                    event = chat_client.recv() => {
                        match event {
                            Ok(event) => {on_event(event, &mut wsclient, Some(&chat_client)).await?;},
                            Err(RecvError::Lagged(count)) => {
                                error!("Lost {} chat events", count);
                            },
                            Err(RecvError::Closed) => {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        })
    })))
}

#[get("/socket/rochat?<start_time>")]
pub async fn readonly_chat_socket<'a>(
    start_time: Option<Snowflake>,
    mut shutdown: Shutdown,
    ws: WebSocket,
    chat: &'a State<Chat>,
) -> Result<ChatSocketResponder<'a>, response::Debug<sqlx::Error>> {
    let start_time = start_time.unwrap_or(Snowflake::ZERO);
    let mut chat_client = match chat.new_roclient().await {
        Ok(c) => c,
        Err(NewClientError::AlreadyInChat) => {
            return Ok(ChatSocketResponder::ws_close(ws, KickReason::AlreadyInChat))
        }
        Err(NewClientError::MaxConcurrentUserCount) => {
            return Ok(ChatSocketResponder::ws_close(ws, KickReason::ChatFull))
        }
    };

    Ok(ChatSocketResponder::Channel(ws.channel(move |stream| {
        Box::pin(async move {
            let chat_hist = chat.history(start_time, false).await;

            let mut wsclient = WsClient::new_ro(stream, chat.users().await, chat_hist).await?;

            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        wsclient.disconnect(KickReason::ServerShutdown).await?;
                        continue;
                    }
                    mesg = wsclient.try_recv() => {
                        let _ = mesg?;
                    }
                    event = chat_client.recv() => {
                        match event {
                            Ok(event) => { on_event(event, &mut wsclient, None).await?; },

                            Err(RecvError::Lagged(count)) => {
                                error!("Lost {} chat events (readonly chat)", count);
                            },
                            Err(RecvError::Closed)=>{
                                return Ok(());
                            }
                        }
                    }
                }
            }
        })
    })))
}

fn is_me(id: u16, chat_client: Option<&ChatClient>) -> bool {
    chat_client
        .map(|chat_client| chat_client.is_me(id))
        .unwrap_or(false)
}

#[inline]
async fn on_event(
    event: ChatEvent,
    wsclient: &mut WsClient,
    chat_client: Option<&ChatClient>,
) -> tokio_tungstenite::tungstenite::Result<()> {
    match event {
        ChatEvent::Join(new_user) => {
            if is_me(new_user.local_id(), chat_client) {
                wsclient.forward_user(&new_user).await?;
            }
        }
        ChatEvent::Leave(_) => {}
        ChatEvent::NewMessage(mesg) => {
            if !is_me(mesg.sender.local_id(), chat_client) {
                if wsclient.role().is_mod() || !mesg.prof() {
                    wsclient.forward(&mesg).await?;
                }
            }
        }
        ChatEvent::MessageChange(message, ty) => match ty {
            MessageChangeType::Deleted => {
                wsclient.forward_message_del(message.id()).await?;
            }
            MessageChangeType::Filter(blocked) => {
                if !wsclient.role().is_mod() && blocked {
                    wsclient.forward_message_del(message.id()).await?;
                } else {
                    wsclient.forward(&message).await?;
                }
            }
        },
        ChatEvent::Kick(user_id) => {
            if chat_client
                .map(|c| c.user().user_id() == user_id)
                .unwrap_or(false)
            {
                wsclient.disconnect(KickReason::Kick).await?;
            }
        }
    }
    Ok(())
}
