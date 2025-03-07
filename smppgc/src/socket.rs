use lmetrics::metrics;
use profanity::ProfanityFilter;
use rocket::{
    get,
    request::{FromRequest, Outcome},
    Request, Responder, Shutdown, State,
};
use std::{convert::Infallible, net::IpAddr, sync::RwLock};

use log::*;
use rocket_ws::{Channel, WebSocket};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    auth::GcMod,
    chat::{Chat, Message, NewClientError},
    ipcountry::IpCountry,
    ratelimit::RateLimitIpPenalty,
    ratelimit::{MesgIpRateLimiters, MesgRateLimiters, NewUserIpRateLimiters},
    users::{UserConfig, UserSid, UsernameManager},
    wsprotocol::{KickReason, WsClient},
    MessageConfig, Snowflake,
};

#[derive(Responder)]
pub enum SocketV1Responder<'a> {
    #[response(status = 500)]
    Error(&'static str),
    #[response(status = 200)]
    Channel(Channel<'a>),
}
impl<'a> SocketV1Responder<'a> {
    pub fn ws_close(ws: WebSocket, reason: KickReason) -> SocketV1Responder<'a> {
        SocketV1Responder::Channel(ws.channel(move |mut stream| {
            Box::pin(async move { stream.close(Some(reason.into_close_frame())).await })
        }))
    }
}

metrics! {
    pub counter messages_total("Total count of sent messages");
    pub counter messages_blocked("Total count of blocked messages", [reason]);
    pub counter blocked_newusers("Total amount of blocked new user creation requests");
    pub counter new_users("Total count of new sid's being generated");
}

#[get("/socket/v1?<username>&<key>&<start_time>")]
pub async fn socket_v1<'a>(
    username: &str,
    key: Option<&str>,
    start_time: Option<Snowflake>,
    ws: WebSocket,
    mesg_limits: &State<MessageConfig>,
    user_config: &State<UserConfig>,
    ip_limits: &State<RateLimitIpPenalty>,
    mesg_ratelimiters: &'a State<MesgRateLimiters>,
    ip_ratelimiters: &'a State<MesgIpRateLimiters>,
    user_ratelimiting: &State<NewUserIpRateLimiters>,
    prof_filter: &'a State<RwLock<ProfanityFilter>>,
    chat: &'a State<Chat>,
    usrnamemgr: &State<UsernameManager>,
    mut shutdown: Shutdown,
    addr: IpAddr,
    country: IpCountry,
    gcmod: Option<GcMod>,
) -> SocketV1Responder<'a> {
    if !ip_ratelimiters.0.update(addr, 0) {
        return SocketV1Responder::ws_close(ws, KickReason::IpRateLimit);
    }
    let ip_penalty_multiplier = if country.is_be() {
        1
    } else if country.is_unknown() {
        ip_limits.xx_penalty
    } else {
        ip_limits.not_be_penalty
    };
    let start_time = start_time.unwrap_or(Snowflake::ZERO);
    let (new_user, sid) = key
        .map(|sid| UserSid::parse_str(sid))
        .flatten()
        .map(|sid| (false, sid))
        .unwrap_or_else(|| (true, UserSid::new()));
    if new_user {
        if !user_ratelimiting.0.update(addr, ip_penalty_multiplier) {
            blocked_newusers::inc();
            return SocketV1Responder::ws_close(ws, KickReason::TooManyUsers);
        }
        new_users::inc();
    }

    let name_lease = match usrnamemgr
        .claim_name(
            username,
            sid.clone(),
            user_config.max_username_len,
            &prof_filter,
        )
        .await
    {
        Ok(name_lease) => name_lease,
        Err(e) => {
            return SocketV1Responder::ws_close(ws, e.into_kickreason());
        }
    };

    let mut chat_client = match chat.new_client(sid.clone(), name_lease).await {
        Ok(c) => c,
        Err(e) => {
            info!("Closing connection: {:?}", e);
            match e {
                NewClientError::MaxConcurrentUserCount => {
                    return SocketV1Responder::ws_close(ws, KickReason::ChatFull)
                }
            }
        }
    };
    let mesg_limits = mesg_limits.inner().clone();
    SocketV1Responder::Channel(ws.channel(move |stream| {
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
                        if mesg.len() > mesg_limits.max_message_len as usize && mesg.len() < mesg_limits.min_message_len as usize{
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
                            let lock = prof_filter.read().expect("Profanity filter lock poisoned");
                            let (tokenized_mesg, content) = lock.tokenize(&mesg.content);

                            (lock.check(&tokenized_mesg).map(|m|(m.span, m.rule.to_string_friendly())), content)
                        };
                        if content.len() > mesg_limits.max_message_len as usize && content.len() < mesg_limits.min_message_len as usize{
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
                                if mesg.sender.id() != chat_client.user_info().id(){
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
                    joined_client = chat_client.join_receiver.recv() => {
                        match joined_client{
                            Ok(joined_client) => {
                                if !joined_client.id() == chat_client.user_info().id() {
                                    info!("user join {}", joined_client.id());
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
    }))
}
