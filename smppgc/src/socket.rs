use rocket::{get, Responder, State};
use std::{borrow::Cow, sync::Arc};

use log::*;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    Channel, WebSocket,
};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    chat::{Chat, Message, NewClientError},
    mesg_filter::{self, Cmd, FilterResult},
    names::{UserId, UsernameManager},
    profanity::ProfFilter,
    ratelimit::{RateLimitConfig, RateLimiter, SpamLimiter},
    wsprotocol::{KickReason, WsClient},
    MaxLengthConfig, OfflineConfig,
};

#[derive(Responder)]
pub enum SocketV1Responder<'a> {
    #[response(status = 503)]
    Offline(&'static str),
    #[response(status = 500)]
    Error(&'static str),
    #[response(status = 200)]
    Channel(Channel<'a>),
}
impl<'a> SocketV1Responder<'a> {
    pub fn ws_close(ws: WebSocket, frame: CloseFrame<'a>) -> SocketV1Responder {
        SocketV1Responder::Channel(
            ws.channel(move |mut stream| Box::pin(async move { stream.close(Some(frame)).await })),
        )
    }
}

#[get("/socket/v1?<username>&<key>")]
pub async fn socket_v1<'a>(
    username: &str,
    key: Option<&str>,
    ws: WebSocket,
    offline_config: &State<OfflineConfig>,
    maxlen_config: &State<MaxLengthConfig>,
    ratelimit_config: &State<RateLimitConfig>,
    prof_filter: &'a State<ProfFilter>,
    chat: &'a State<Chat>,
    usrnamemgr: &State<UsernameManager>,
) -> SocketV1Responder<'a> {
    if offline_config.offline {
        return SocketV1Responder::Offline("smppgc offline");
    }

    let static_user_id = match key {
        Some(key) => match UserId::parse_str(key) {
            Some(sui) => sui,
            None => {
                return SocketV1Responder::ws_close(
                    ws,
                    CloseFrame {
                        code: CloseCode::Error,
                        reason: Cow::Borrowed("INT: Ongeldige statische gebruikers id."),
                    },
                );
            }
        },
        None => UserId::new(),
    };

    let name_lease = match usrnamemgr.claim_name(
        username,
        static_user_id.clone(),
        maxlen_config.max_username_len,
    ) {
        Ok(name_lease) => name_lease,
        Err(e) => {
            return SocketV1Responder::ws_close(
                ws,
                CloseFrame {
                    code: CloseCode::Error,
                    reason: Cow::Owned(e.to_string()),
                },
            );
        }
    };

    let mut chat_client = match chat.new_client(name_lease).await {
        Ok(c) => c,
        Err(e) => {
            info!("Closing connection: {:?}", e);
            match e {
                NewClientError::MaxConcurrentUserCount => {
                    return SocketV1Responder::ws_close(
                        ws,
                        CloseFrame {
                            code: CloseCode::Again,
                            reason: Cow::Borrowed("De chat zit vol"),
                        },
                    )
                }
            }
        }
    };
    let mut rate_limiter = RateLimiter::new(ratelimit_config.inner().clone());
    let max_message_len = maxlen_config.max_message_len;
    SocketV1Responder::Channel(ws.channel(move |stream| {
        Box::pin(async move {
            let mut wsclient = WsClient::new(
                stream,
                static_user_id,
                chat_client.user_info(),
                chat.clients().await,
                chat.history().await
            )
                .await?;

            let mut spam_limiter = SpamLimiter::new();
            loop {
                tokio::select! {
                    mesg = wsclient.try_recv() => {
                        let Some(mesg) = mesg? else { continue; };
                        match on_message(mesg, &mut wsclient, &prof_filter, &mut rate_limiter, &mut spam_limiter, max_message_len).await?{
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
                                error!("{} Messages lost", count);
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
    wsclient: &mut WsClient,
    prof_filter: &ProfFilter,
    rate_limiter: &mut RateLimiter,
    smap_limiter: &mut SpamLimiter<Arc<str>>,
    max_message_len: usize,
) -> Result<Option<Message>, tokio_tungstenite::tungstenite::Error> {
    if !rate_limiter.update() {
        wsclient.kick(KickReason::RateLimit).await?;
        return Ok(None);
    }
    if !smap_limiter.update(message.content.clone()) {
        wsclient.kick(KickReason::Spam).await?;
        return Ok(None);
    }
    match mesg_filter::filter(message.clone(), max_message_len) {
        FilterResult::Cmd(Cmd::BlockMe) => {}
        FilterResult::Cmd(Cmd::KillMe) => {
            return Ok(None);
        }
        FilterResult::Invalid => {}
        FilterResult::Message(mut filtered_mesg) => {
            if prof_filter.contains_profanity(&filtered_mesg.content).await {
                filtered_mesg.content = "#".repeat(filtered_mesg.content.len()).into();
            }
            trace!(
                "got message from {}: {}",
                filtered_mesg.sender,
                filtered_mesg.content
            );
            wsclient.forward(&message).await?;
            return Ok(Some(filtered_mesg));
        }
    }
    Ok(None)
}
