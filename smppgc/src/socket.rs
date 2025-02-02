use lmetrics::metrics;
use rocket::{
    get,
    request::{FromRequest, Outcome},
    Request, Responder, Shutdown, State,
};
use std::{convert::Infallible, net::IpAddr};

use log::*;
use rocket_ws::{Channel, WebSocket};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    chat::{Chat, Message, NewClientError},
    mesg_filter::{self, Cmd, FilterResult},
    names::{UserConfig, UserSid, UsernameManager},
    profanity::ProfFilter,
    ratelimit::RateLimitIpPenalty,
    ratelimit::{MesgIpRateLimiters, MesgRateLimiters, NewUserIpRateLimiters},
    wsprotocol::{KickReason, WsClient},
    MessageConfig,
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
    pub counter profanity_messages_total("Total count of sent messages that contain profanity");
    pub counter messages_blocked("Total count of blocked messages", [reason]);
    pub counter blocked_newusers("Total amount of blocked new user creation requests");
    pub counter new_users("Total count of new sid's being generated");
}

pub struct IpCountry {
    code: [u8; 2],
}
impl IpCountry {
    pub fn unknown() -> Self {
        Self { code: [b'X'; 2] }
    }
    pub fn parse(str: &str) -> Option<Self> {
        let mut chars = str.chars();
        let first = chars.next()?;
        let second = chars.next()?;
        if !first.is_ascii() || !second.is_ascii() {
            return None;
        }
        Some(Self {
            code: [first as u8, second as u8],
        })
    }

    pub fn is_be(&self) -> bool {
        self.code == [b'B', b'E']
    }
    pub fn is_unknown(&self) -> bool {
        self.code == [b'X', b'X']
    }
    pub fn is_tor(&self) -> bool {
        self.code == [b'T', b'1']
    }
}
//CF-IPCountry
#[rocket::async_trait]
impl<'r> FromRequest<'r> for IpCountry {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Infallible> {
        Outcome::Success(
            request
                .headers()
                .get_one("CF-IPCountry")
                .map(|cc| IpCountry::parse(cc))
                .flatten()
                .unwrap_or(IpCountry::unknown()),
        )
    }
}

#[get("/socket/v1?<username>&<key>&<start_time>")]
pub async fn socket_v1<'a>(
    username: &str,
    key: Option<&str>,
    start_time: Option<u32>,
    ws: WebSocket,
    mesg_limits: &State<MessageConfig>,
    user_config: &State<UserConfig>,
    ip_limits: &State<RateLimitIpPenalty>,
    mesg_ratelimiters: &'a State<MesgRateLimiters>,
    ip_ratelimiters: &'a State<MesgIpRateLimiters>,
    user_ratelimiting: &State<NewUserIpRateLimiters>,
    prof_filter: &'a State<ProfFilter>,
    chat: &'a State<Chat>,
    usrnamemgr: &State<UsernameManager>,
    mut shutdown: Shutdown,
    addr: IpAddr,
    country: IpCountry,
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
    let start_time = start_time.unwrap_or(0);
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

    let mut chat_client = match chat.new_client(name_lease).await {
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
            let chat_hist = chat.history(start_time).await;
            let mut wsclient = WsClient::new(
                stream,
                sid.clone(),
                chat_client.user_info(),
                chat.clients().await,
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
                        if mesg.len() > mesg_limits.max_message_len{
                            messages_blocked::inc("msgtoobig");
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

                        match on_message(mesg, &chat, &mut wsclient, &prof_filter).await?{
                            Some(mesg) => {
                                chat_client.send(mesg);
                            },
                            None=>{}
                        }
                    }
                    mesg = chat_client.message_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                if mesg.sender_id != chat_client.user_info().id(){
                                    wsclient.forward(&mesg).await?;
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
                                info!("user join {}", joined_client.id());
                                wsclient.forward_client(&joined_client).await?;
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

async fn on_message(
    message: Message,
    chat: &Chat,
    wsclient: &mut WsClient,
    prof_filter: &ProfFilter,
) -> Result<Option<Message>, tokio_tungstenite::tungstenite::Error> {
    messages_total::inc();
    match mesg_filter::filter(message) {
        FilterResult::Cmd(message, Cmd::Invalid) => {
            wsclient.forward(&message).await?;
            wsclient
                .forward(&Message::new_response(&message, "invalid command".into()))
                .await?;
            return Ok(None);
        }
        FilterResult::Cmd(_, Cmd::KickMe { hard }) => {
            if hard {
                wsclient.kick(KickReason::Hard).await?;
            } else {
                wsclient.kick(KickReason::Cmd).await?;
            }
        }

        FilterResult::Cmd(message, Cmd::BanWord(word)) => {
            wsclient.forward(&message).await?;
            if prof_filter.contains_profanity(&word).await {
                wsclient
                    .forward(&Message::new_response(
                        &message,
                        "profanity filter already catches that one".into(),
                    ))
                    .await?;
                return Ok(None);
            }
            match prof_filter.add_word(word).await {
                Ok(()) => {
                    chat.filter_history_async(prof_filter).await;
                    wsclient
                        .forward(&Message::new_response(
                            &message,
                            "word added to bad word list".into(),
                        ))
                        .await?;
                }
                Err(e) => {
                    error!("failed to add to profanity list: {}", e);
                    wsclient
                        .forward(&Message::new_response(
                            &message,
                            "failed to add to bad word list".into(),
                        ))
                        .await?;
                }
            }
            return Ok(None);
        }
        FilterResult::Invalid => {
            messages_blocked::inc("invalid");
        }
        FilterResult::Message(mut filtered_mesg) => {
            if prof_filter.filter(&mut filtered_mesg).await {
                profanity_messages_total::inc();
            }
            trace!(
                "got message from {}: {}",
                filtered_mesg.sender,
                filtered_mesg.content
            );
            wsclient.forward(&filtered_mesg).await?;
            return Ok(Some(filtered_mesg));
        }
    }
    Ok(None)
}
