use rocket::{get, Responder, Shutdown, State};
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
    ratelimit::{RateLimitConfig, RateLimiter},
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
    pub fn ws_close(ws: WebSocket, frame: CloseFrame<'a>) -> SocketV1Responder<'a> {
        SocketV1Responder::Channel(
            ws.channel(move |mut stream| Box::pin(async move { stream.close(Some(frame)).await })),
        )
    }
}

#[get("/socket/v1?<username>&<key>&<start_time>")]
pub async fn socket_v1<'a>(
    username: &str,
    key: Option<&str>,
    start_time: Option<u32>,
    ws: WebSocket,
    offline_config: &State<OfflineConfig>,
    maxlen_config: &State<MaxLengthConfig>,
    ratelimit_config: &State<RateLimitConfig>,
    prof_filter: &'a State<ProfFilter>,
    chat: &'a State<Chat>,
    usrnamemgr: &State<UsernameManager>,
    mut shutdown: Shutdown,
) -> SocketV1Responder<'a> {
    if offline_config.offline {
        return SocketV1Responder::Offline("smppgc offline");
    }
    let start_time = start_time.unwrap_or(0);
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

    let name_lease = match usrnamemgr
        .claim_name(
            username,
            static_user_id.clone(),
            maxlen_config.max_username_len,
            &prof_filter,
        )
        .await
    {
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
            let chat_hist = chat.history(start_time).await;
            let mut wsclient = WsClient::new(
                stream,
                static_user_id,
                chat_client.user_info(),
                chat.clients().await,
                chat_hist

            )
                .await?;

            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        return wsclient.kick(KickReason::ServerShutdown).await;
                    }
                    mesg = wsclient.try_recv() => {
                        let Some(mesg) = mesg? else { continue; };
                        match on_message(mesg, &chat, &mut wsclient, &prof_filter, &mut rate_limiter, max_message_len).await?{
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
    chat: &Chat,
    wsclient: &mut WsClient,
    prof_filter: &ProfFilter,
    rate_limiter: &mut RateLimiter,
    max_message_len: usize,
) -> Result<Option<Message>, tokio_tungstenite::tungstenite::Error> {
    if !rate_limiter.update() {
        wsclient.kick(KickReason::RateLimit).await?;
        return Ok(None);
    }
    match mesg_filter::filter(message, max_message_len) {
        FilterResult::Cmd(message, Cmd::Invalid) => {
            wsclient.forward(&message).await?;
            wsclient
                .forward(&Message::new_response(&message, "invalid command".into()))
                .await?;
            return Ok(None);
        }
        FilterResult::Cmd(_, Cmd::KickMe) => {
            wsclient.kick(KickReason::Cmd).await?;
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
        FilterResult::Invalid => {}
        FilterResult::Message(mut filtered_mesg) => {
            prof_filter.filter(&mut filtered_mesg).await;
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
