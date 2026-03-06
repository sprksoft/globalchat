use std::sync::Arc;

use lmetrics::metrics;
use nanotime::snowflake::Snowflake;
use rocket::{get, request::Outcome, response, Responder, Shutdown, State};

use log::*;
use rocket_db_pools::Connection;
use rocket_ws::{Channel, WebSocket};
use sqlx::query;
use tokio::sync::broadcast::error::RecvError;
use wordfilter::TokenTag;

use crate::{
    chat::{message_limits::LimitType, Chat, ChatEvent, MessageChangeType, NewClientError},
    db::Db,
    disclaimer::DisclaimerVer,
    metrics::RequestTime,
    users::{Ban, BanError, NameClaimError, User, UserGuardError, UserManager},
    utils::static_routing,
    wf::Filter,
    wsprotocol::{AdminCmd, C2SPacket, ModCmd, ProtoError, WsClient},
};

use super::{message_limits::MessageLimiter, ChatClient};

metrics!(
    pub counter total_connections_waiting_millis("Total amount milliseconds users spend waiting for the connection to the agent to succeed");
    pub counter total_connections_by_seconds("Total amount of connections split by the time in seconds they took time", [time]);

    pub counter messages_total("Total count of sent messages");
    pub counter messages_blocked("Total count of blocked messages", [reason]);
);

#[derive(Responder)]
pub enum ChatSocketResponder<'a> {
    #[response(status = 200)]
    Channel(Channel<'a>),
}
impl<'a> ChatSocketResponder<'a> {
    pub fn ws_close(ws: WebSocket, reason: ProtoError) -> ChatSocketResponder<'a> {
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

#[get("/socket/chat?<username>&<start_time>&<mod_badge>")]
pub async fn chat_socket<'a>(
    req_time: RequestTime,
    username: &str,
    start_time: Option<Snowflake>,
    mod_badge: Option<bool>,
    mut db: Connection<Db>,
    ws: WebSocket,
    mut user_manager: UserManager,
    mesg_limiter: &'a State<MessageLimiter>,
    filter: &'a State<Arc<Filter>>,
    chat: &'a State<Chat>,
    mut shutdown: Shutdown,
    user: Outcome<User, UserGuardError>,
    disclaimer_ver: DisclaimerVer,
) -> Result<ChatSocketResponder<'a>, response::Debug<sqlx::Error>> {
    if disclaimer_ver != DisclaimerVer::LATEST {
        return Ok(ChatSocketResponder::ws_close(ws, ProtoError::Disclaimer));
    }
    let user = match user {
        Outcome::Success(u) => u,
        Outcome::Error(e) => {
            error!("User guard failed in gcagent: {:?}", e);
            return Ok(ChatSocketResponder::ws_close(ws, ProtoError::Unexpected));
        }
        Outcome::Forward(_) => return Ok(ChatSocketResponder::ws_close(ws, ProtoError::NoSession)),
    };

    let start_time = start_time.unwrap_or(Snowflake::ZERO);
    let my_role = user.role();

    if !my_role.is_mod() {
        if let Some(ban) = user_manager.get_ban(&mut db, user.id()).await? {
            return Ok(ChatSocketResponder::ws_ban(ws, ban));
        }
    }

    let claimed_name = match user_manager.claim_name(&mut db, &user, username).await {
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
                    chat.kick(user.id());
                    return Ok(ChatSocketResponder::ws_close(ws, ProtoError::AlreadyInChat));
                }
                NewClientError::MaxConcurrentUserCount => {
                    return Ok(ChatSocketResponder::ws_close(ws, ProtoError::ChatFull))
                }
            }
        }
    };

    Ok(ChatSocketResponder::Channel(ws.channel(move |stream| {
        Box::pin(async move {
            let mut wsclient = WsClient::new(
                stream,
                chat,
                chat_client.user(),
                start_time
            )
            .await?;

            let dur = req_time.0.elapsed();
            let dur_millis = dur.as_millis();
            if dur_millis > 4000 {
                warn!("Connection to the agent took more than 4 seconds: {}ms", dur_millis);
            }
            total_connections_waiting_millis::inc_by(dur_millis as u64);
            total_connections_by_seconds::inc(&dur.as_secs().to_string());

            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        wsclient.disconnect(ProtoError::Shutdown).await?;
                        continue;
                    }
                    mesg = wsclient.try_recv() => {
                        let packet = mesg?;
                        on_packet(packet, &mut wsclient, &chat_client, &chat, &filter, &mesg_limiter, &mut user_manager, &mut db).await?;
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

#[inline]
async fn on_packet(
    packet: C2SPacket,
    wsclient: &mut WsClient,
    chat_client: &ChatClient,
    chat: &Chat,
    filter: &Filter,
    mesg_limiter: &MessageLimiter,
    user_manager: &mut UserManager,
    db: &mut Connection<Db>,
) -> tokio_tungstenite::tungstenite::Result<()> {
    match packet {
        C2SPacket::Message(content) => {
            let mesg = {
                match mesg_limiter.feed(chat_client.user().user_id(), content) {
                    Ok(content) => {
                        let content = filter.check(&content).await;

                        if let Some((_, tag)) = content
                            .words()
                            .find(|(_, tag)| !(tag.is_good() || tag.is_whitespace()))
                        {
                            messages_blocked::inc(if tag.is_unknown() {
                                "profanity (unknown)"
                            } else if tag.is_bad() {
                                "profanity (bad)"
                            } else {
                                "profanity (unreachable (this is a bug))"
                            });
                        }

                        let mesg = chat_client.new_message(content);
                        let db_call = async {
                            let content = mesg.content.str();
                            query!("INSERT INTO messages(snowflake, sender_id, sender_name, content) VALUES($1, $2, $3, $4)", mesg.id().to_u64().cast_signed(), mesg.sender.user_id().to_i32(), mesg.sender.username(), content).execute(&mut ***db).await;
                        };
                        let (_, result) = tokio::join!(db_call, wsclient.forward(&mesg));
                        result?;
                        mesg
                    }
                    Err(LimitType::Rate) => {
                        messages_blocked::inc("ratelimit");
                        return wsclient.disconnect(ProtoError::RateLimit).await;
                    }
                    Err(LimitType::Size) => {
                        messages_blocked::inc("size");
                        return Ok(());
                    }
                    Err(LimitType::Spam) => {
                        messages_blocked::inc("spam");
                        return Ok(());
                    }
                }
            };
            messages_total::inc();
            chat_client.send(mesg);
        }
        C2SPacket::Report { message_id, reason } => {
            match chat.report_message(db, message_id, chat_client.user().user_id(), reason).await {
                Ok(()) => {},
                Err(e) => {
                    error!("Failed to report message: sql error: {}", e);
                    wsclient.system_message("Kon bericht niet rapporteren");
                }
            }
        }
        C2SPacket::ModCmd(cmd) => {
            if !chat_client.user().role().is_mod() {
                wsclient
                    .system_message("Geen toegang tot mod cmd's")
                    .await?;
                return Ok(());
            }
            on_mod_cmd(cmd, wsclient, chat, filter, user_manager).await?;
        }
        C2SPacket::AdminCmd(cmd) => {
            if chat_client.user().role().is_admin() {
                wsclient
                    .system_message("Geen toegang tot admin cmd's")
                    .await?;
                return Ok(());
            }
            on_admin_cmd(cmd, chat, filter).await?;
        }
    }
    Ok(())
}

#[inline]
async fn on_mod_cmd(
    cmd: ModCmd,
    wsclient: &mut WsClient,
    chat: &Chat,
    filter: &Filter,
    user_manager: &mut UserManager,
) -> tokio_tungstenite::tungstenite::Result<()> {
    match cmd {
        ModCmd::DelMsg(snowflake) => {
            if !chat.retain_messages(|m| m.id() != snowflake).await {
                wsclient
                    .system_message("Bericht bestaat niet meer op de server")
                    .await?;
            }
        }
        ModCmd::BanMsgAuthor {
            mesg,
            reason,
            duration,
        } => match chat.get_author_uid(mesg).await {
            Some(uid) => match user_manager
                .ban_user(&mut, uid, wsclient.role(), &reason, duration)
                .await
            {
                Ok(()) => {
                    chat.retain_messages(|m| m.sender.user_id() != uid).await;
                    chat.kick(uid);
                }
                Err(BanError::Sqlx(e)) => {
                    error!("While trying to ban user: {}", e);
                    wsclient.system_message("Interne SQL error").await?;
                }
                Err(BanError::PermissionDenied) => {
                    wsclient
                        .system_message("Niet toegestaan deze persoon te verbannen")
                        .await?;
                }
            },
            None => {
                wsclient
                    .system_message("Bericht bestaat niet meer op de server")
                    .await?;
            }
        },
        ModCmd::WFMark { word, good } => {
            filter.mark_word(&word, good).await;
        }
    }
    Ok(())
}

#[inline]
async fn on_admin_cmd(
    cmd: AdminCmd,
    chat: &Chat,
    filter: &Filter,
) -> tokio_tungstenite::tungstenite::Result<()> {
    match cmd {
        AdminCmd::WFCommit => {
            debug!("WFCommit");
            filter.rerun(chat).await;
        }
        AdminCmd::WFLock {
            word,
            reason,
            locked,
        } => {
            let should_rerun = if locked {
                filter.lock_word(&word, reason.into()).await
            } else {
                filter.unlock_word(&word).await
            };
            if should_rerun {
                filter.rerun(&chat).await;
            }
        }
    }
    Ok(())
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
            return Ok(ChatSocketResponder::ws_close(ws, ProtoError::AlreadyInChat))
        }
        Err(NewClientError::MaxConcurrentUserCount) => {
            return Ok(ChatSocketResponder::ws_close(ws, ProtoError::ChatFull))
        }
    };

    Ok(ChatSocketResponder::Channel(ws.channel(move |stream| {
        Box::pin(async move {
            let mut wsclient = WsClient::new_ro(stream, chat, start_time).await?;

            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        wsclient.disconnect(ProtoError::Shutdown).await?;
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
            if let Some(cc) = chat_client {
                wsclient.update_user_count(cc.user_count).await?;
            }
            if !is_me(new_user.local_id(), chat_client) {
                wsclient.forward_user(&new_user).await?;
            }
        }
        ChatEvent::Leave(_) => {
            if let Some(cc) = chat_client {
                wsclient.update_user_count(cc.user_count).await?;
            }
        }
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
                if wsclient.role().is_mod() || !blocked {
                    wsclient.forward(&message).await?;
                } else {
                    wsclient.forward_message_del(message.id()).await?;
                }
            }
        },
        ChatEvent::Kick(user_id) => {
            if chat_client
                .map(|c| c.user().user_id() == user_id)
                .unwrap_or(false)
            {
                wsclient.disconnect(ProtoError::Kick).await?;
            }
        }
    }
    Ok(())
}
