use lmetrics::metrics;
use profanity::ProfanityFilter;
use rocket::{get, response, Responder, Shutdown, State};
use std::net::IpAddr;
use tokio::sync::RwLock;

use log::*;
use rocket_ws::{Channel, WebSocket};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    auth::GcMod,
    chat::{Chat, MessageChange, MessageChangeType, NewClientError},
    ratelimit::RateLimitIpPenalty,
    ratelimit::{MesgIpRateLimiters, MesgRateLimiters},
    users::{SesId, Session, SessionMgr, UserConfig, UserManager, UserSid},
    utils::IpCountry,
    wsprotocol::{KickReason, WsClient},
    MessageConfig, Snowflake,
};

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
    mesg_limits: &State<MessageConfig>,
    user_config: &State<UserConfig>,
    ip_limits: &State<RateLimitIpPenalty>,
    mesg_ratelimiters: &'a State<MesgRateLimiters>,
    ip_ratelimiters: &'a State<MesgIpRateLimiters>,
    prof_filter: &'a State<RwLock<ProfanityFilter>>,
    chat: &'a State<Chat>,
    mut shutdown: Shutdown,
    addr: IpAddr,
    country: IpCountry,
    ses: Option<Session>,
    session_mgr: &'a State<SessionMgr>,
    gcmod: Option<GcMod>,
) -> Result<ChatSocketResponder<'a>, response::Debug<sqlx::Error>> {
    if !ip_ratelimiters.0.update(addr, 0) {
        return Ok(ChatSocketResponder::ws_close(ws, KickReason::IpRateLimit));
    }
    let ip_penalty_multiplier = if country.is_be() {
        1
    } else if country.is_unknown() {
        ip_limits.xx_penalty
    } else {
        ip_limits.not_be_penalty
    };

    let Some(ses) = ses else {
        return Ok(ChatSocketResponder::ws_close(ws, KickReason::NoSession));
    };

    //TODO: Fix this: workaround because not all code is written yet
    let sid = UserSid::from_smid(ses.user_info.smid.clone());
    let start_time = start_time.unwrap_or(Snowflake::ZERO);

    let claimed_name = match user_manager.claim_name(&ses.user_info, username).await? {
        Ok(name_lease) => name_lease,
        Err(e) => {
            return Ok(ChatSocketResponder::ws_close(ws, e.into_kickreason()));
        }
    };

    let mod_badge = gcmod.is_some() && mod_badge.unwrap_or(false);
    let mut chat_client = match chat.new_client(sid.clone(), claimed_name, mod_badge).await {
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

    let mesg_limits = mesg_limits.inner().clone();
    Ok(ChatSocketResponder::Channel(ws.channel(move |stream| {
        Box::pin(async move {
            let chat_hist = chat.history(start_time, gcmod.is_some()).await;

            let mut wsclient = WsClient::new(
                stream,
                chat_client.user_info(),
                chat.users().await,
                chat_hist,
            )
            .await?;

            let mut last_message = None;
            let mut same_mesg_streak=0;
            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        return wsclient.kick(KickReason::ServerShutdown).await;
                    }
                    mesg = wsclient.try_recv() => {
                        let Some(mesg) = mesg? else { continue; };

                        if gcmod.is_some() {
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

                        if mesg.len() > mesg_limits.max_message_len as usize || mesg.len() < mesg_limits.min_message_len as usize{
                            messages_blocked::inc("size");
                            continue;
                        }
                        let rl_points = if mesg.len() > mesg_limits.small_message_len { mesg_limits.large_message_penalty } else { 1 };
                        if !ip_ratelimiters.0.update(addr, rl_points*ip_penalty_multiplier){
                            messages_blocked::inc("ipratelimit");
                            wsclient.kick(KickReason::IpRateLimit).await?;
                            continue;
                        }
                        if Some(&mesg.content) == last_message.as_ref(){
                            same_mesg_streak+=1;
                        }else{
                            same_mesg_streak=0;
                            last_message = Some(mesg.content.clone());
                        }
                        if same_mesg_streak >= mesg_limits.max_same_message_streak{
                            mesg_ratelimiters.0.update(sid.clone(), mesg_limits.same_message_penalty);
                            messages_blocked::inc("same_mesg_spam");
                        }
                        if !mesg_ratelimiters.0.update(sid.clone(), rl_points){
                            wsclient.kick(KickReason::RateLimit).await?;
                            messages_blocked::inc("ratelimit");
                            continue;
                        }


                        let (prof_span, content) = {
                            let lock = prof_filter.read().await;
                            let (tokenized_mesg, content) = lock.tokenize(&mesg.content.trim());

                            (lock.check(&tokenized_mesg).map(|m|(m.span, m.rule.to_string_friendly())), content)
                        };
                        if content.len() > mesg_limits.max_message_len as usize || content.len() < mesg_limits.min_message_len as usize{
                            messages_blocked::inc("size");
                            continue;
                        }
                        let mesg = chat_client.new_message(content.into(), prof_span.is_some());
                        if let Some((span, bad_word)) = prof_span {
                            let span = span.start as crate::MessageLen..span.end as crate::MessageLen;
                            wsclient.profanity_warning(&mesg.content, &bad_word, span).await?;
                            messages_blocked::inc("profanity");
                        }else{
                            wsclient.forward(&mesg).await?;
                        }
                        chat_client.send(mesg);



                    }
                    mesg = chat_client.message_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                if mesg.sender.id() != chat_client.user_info().id() {
                                    if gcmod.is_some() || !mesg.profanity {
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
                                if gcmod.is_none() {
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
                                if joined_client.id() != chat_client.user_info().id() {
                                    //dbg!("forwarding", &joined_client);
                                    wsclient.forward_client(&joined_client).await?;
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
