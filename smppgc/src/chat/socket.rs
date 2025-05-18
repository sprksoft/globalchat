use lmetrics::metrics;
use profanity::ProfanityFilter;
use rocket::{get, response, Responder, Shutdown, State};
use tokio::sync::RwLock;

use log::*;
use rocket_ws::{Channel, WebSocket};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    chat::{
        message_limits::LimitType, Chat, MessageChange, MessageChangeType, MessageLen,
        NewClientError,
    },
    users::{Ban, User, UserManager},
    wsprotocol::{KickReason, WsClient},
    Snowflake,
};

use super::message_limits::MessageLimiter;

#[derive(Responder)]
pub enum ChatSocketResponder<'a> {
    #[response(status = 500)]
    Error(&'static str),
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
    let is_mod = user.role().is_mod();

    if !is_mod {
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

    let mod_badge = is_mod && mod_badge.unwrap_or(false);
    let mut chat_client = match chat
        .new_client(&user, claimed_name, mod_badge, is_mod)
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
            let chat_hist = chat.history(start_time, is_mod).await;

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
                        return wsclient.disconnect(KickReason::ServerShutdown).await;
                    }
                    mesg = wsclient.try_recv() => {
                        let Some(mesg) = mesg? else { continue; };

                        if is_mod {
                            match parse_admin_cmd(&mesg.content) {
                                Some(AdminCmd::DelMsg(snowflake)) => {
                                    chat.delete_message(snowflake).await;
                                    continue;
                                },
                                Some(AdminCmd::Invalid) => {
                                    continue;
                                    //TODO: notify client of invalid command
                                }
                                Some(AdminCmd::UnknownCmd) => {
                                    continue;
                                    //TODO: notify client of unknown command
                                }
                                None => {},
                            }

                        }
                        let mesg = {
                            match mesg_limiter.feed(user.id(), &&prof_filter.read().await, mesg.content) {
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
                        println!("message sent to chat");
                    }
                    mesg = chat_client.message_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                if mesg.sender.local_id() != chat_client.user().local_id() {
                                    if is_mod || !mesg.profanity {
                                        wsclient.forward(&mesg).await?;
                                    }
                                }
                            }
                            Err(RecvError::Lagged(count)) => {
                                error!("Lost {} messages", count);
                            },
                            Err(RecvError::Closed)=>{
                                return Ok(());
                            }
                        }
                    }
                    mesg_change = chat_client.message_change_receiver.recv() => {
                        match mesg_change{
                            Ok(MessageChange { message_id, mut ty}) => {
                                if !is_mod {
                                    ty = MessageChangeType::Deleted;
                                }
                                wsclient.forward_message_change(message_id, ty).await?;
                            }
                            Err(RecvError::Lagged(count)) => {
                                error!("Lost {} message changes", count);
                            },
                            Err(RecvError::Closed)=>{
                                return Ok(());
                            }
                        }
                    }
                    joined_client = chat_client.join_receiver.recv() => {
                        match joined_client{
                            Ok(joined_client) => {
                                //dbg!("join", &joined_client, &chat_client.user_info());
                                if joined_client.local_id() != chat_client.user().local_id() {
                                    //dbg!("forwarding", &joined_client);
                                    wsclient.forward_user(&joined_client).await?;
                                }
                            },
                            Err(RecvError::Lagged(count)) => {
                                error!("{} Join messages lost", count);
                            }, Err(RecvError::Closed)=>{
                                return Ok(());
                            }
                        }
                    }
                }
            }
        })
    })))
}

pub enum AdminCmd {
    DelMsg(Snowflake),
    Invalid,
    UnknownCmd,
}

fn parse_admin_cmd(str: &str) -> Option<AdminCmd> {
    let admin_cmd_prefix = "%admin ";
    if !str.starts_with(admin_cmd_prefix) {
        return None;
    }
    let str = &str[admin_cmd_prefix.len()..];

    let cmd = "/delmsg ";
    if str.starts_with(cmd) {
        match str[cmd.len()..].parse().map(|n| Snowflake::from_u64(n)) {
            Ok(s) => Some(AdminCmd::DelMsg(s)),
            Err(e) => {
                error!("Invalid /delmsg command. Failed to parse snowflake: {}", e);
                Some(AdminCmd::Invalid)
            }
        }
    } else {
        Some(AdminCmd::UnknownCmd)
    }
}
