use lmetrics::metrics;
use profanity::ProfanityFilter;
use rocket::{get, response, Responder, Shutdown, State};
use tokio::sync::RwLock;

use log::*;
use rocket_ws::{Channel, WebSocket};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    chat::{
        message_limits::LimitType, Chat, ChatEvent, MessageChangeType, MessageLen, NewClientError,
    },
    users::{Ban, BanError, User, UserManager},
    wsprotocol::{AdminCmd, C2SPacket, KickReason, WsClient},
    Snowflake,
};

use super::message_limits::MessageLimiter;

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
    prof_filter: &'a State<RwLock<ProfanityFilter>>,
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

    let claimed_name = match user_manager.claim_name(&user, username).await? {
        Ok(name_lease) => name_lease,
        Err(e) => {
            return Ok(ChatSocketResponder::ws_close(ws, e.into_kickreason()));
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
                            C2SPacket::Message(mesg_content) => {
                            let mesg = {
                                match mesg_limiter.feed(user.id(), &&prof_filter.read().await, mesg_content) {
                                    Ok(c) => {let mesg = chat_client.new_message(c.into(), false); wsclient.forward(&mesg).await?; mesg},
                                    Err(LimitType::Rate) => {
                                        messages_blocked::inc("ratelimit");
                                        return wsclient.disconnect(KickReason::RateLimit).await;
                                    },
                                    Err(LimitType::Profanity{content, bad_word, span}) => {
                                        let span = span.start as MessageLen..span.end as MessageLen;
                                        wsclient.profanity_warning(&content, &bad_word, span).await?;
                                        messages_blocked::inc("profanity");
                                        chat_client.new_message(content.into(), true)
                                    }
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
                            Ok(ChatEvent::Join(new_user)) => {
                                if new_user.local_id() != chat_client.user().local_id() {
                                    wsclient.forward_user(&new_user).await?;
                                }
                            },
                            Ok(ChatEvent::Leave(_)) => {},
                            Ok(ChatEvent::Message(mesg)) => {
                                if mesg.sender.local_id() != chat_client.user().local_id() {
                                    if my_role.is_mod() || !mesg.profanity {
                                        wsclient.forward(&mesg).await?;
                                    }
                                }
                            },
                            Ok(ChatEvent::MessageChange(snowflake, mut ty)) => {
                                if !my_role.is_mod() {
                                    ty = MessageChangeType::Deleted;
                                }
                                wsclient.forward_message_change(snowflake, ty).await?;

                            }
                            Ok(ChatEvent::Kick(user_id)) => {
                                if user_id == user.id() {
                                    wsclient.disconnect(KickReason::Kick).await?;
                                }
                            },

                            Err(RecvError::Lagged(count)) => {
                                error!("Lost {} chat events", count);
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
                            Ok(ChatEvent::Join(new_user)) => {
                                wsclient.forward_user(&new_user).await?;
                            },
                            Ok(ChatEvent::Leave(_)) => {},
                            Ok(ChatEvent::Message(mesg)) => {
                                if !mesg.profanity {
                                    wsclient.forward(&mesg).await?;
                                }
                            },
                            Ok(ChatEvent::MessageChange(snowflake, _)) => {
                                wsclient.forward_message_change(snowflake, MessageChangeType::Deleted).await?;

                            }
                            Ok(ChatEvent::Kick(_)) => {},

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
