use std::{borrow::Cow, ops::Range};

use futures_util::SinkExt;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    result::Result,
    stream::DuplexStream,
};
use thiserror::Error;
use tokio_tungstenite::tungstenite;

use crate::{
    chat::{ChatUser, Message, MessageChangeType, MessageLen},
    users::Ban,
    Snowflake,
};

use log::*;

mod packets;

#[derive(Debug, Error)]
pub enum PacketsError {
    #[error("Invalid packets or protocol error")]
    Invalid,
    #[error("Client Disconnected")]
    Disconected,
}
impl From<tungstenite::Error> for PacketsError {
    fn from(err: tungstenite::Error) -> Self {
        match err {
            tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
                PacketsError::Disconected
            }
            _ => PacketsError::Invalid,
        }
    }
}

macro_rules! kick_reason {
    ($vis:vis $ty:ident{$($enum:ident($code:ident, $proto_mesg:literal)),*}) => {
        #[derive(Clone)]
        $vis enum $ty {
            $(
                $enum
            ),*
        }

        impl $ty{
            pub fn into_close_frame(self) -> CloseFrame<'static>{
                match self{
                    $(
                        Self::$enum => CloseFrame{
                            code: CloseCode::$code,
                            reason: Cow::Borrowed($proto_mesg)
                        }
                    ),*
                }
            }
        }
    };
}
kick_reason! {
    pub KickReason{
        Hard(Policy,""),
        Cmd(Abnormal,"err_cmd"),
        NoSession(Policy, "err_no_session"),
        AlreadyInChat(Policy, "err_already_in_chat"),
        IpRateLimit(Policy, "err_ipratelimit"),
        RateLimit(Policy,"err_ratelimit"),
        ServerShutdown(Away,"err_shutdown"),
        ChatFull(Again,"err_full"),
        UsernameProfanity(Error,"err_username_prof"),
        UsernameTaken(Error,"err_username_taken"),
        UsernameInvalid(Error,"err_username_invalid"),
        UsernameInvalidLength(Error,"err_username_length")
    }
}

pub struct RecievedMessage {
    pub content: String,
}
impl RecievedMessage {
    #[inline]
    pub fn len(&self) -> usize {
        self.content.len()
    }
}

pub struct WsClient {
    ws: DuplexStream,
}
impl WsClient {
    pub async fn new(
        mut ws: DuplexStream,
        clients: Vec<ChatUser>,
        history: Vec<Message>,
        user_info: &ChatUser,
    ) -> Result<Self> {
        ws.feed(packets::new_setup(user_info.local_id())).await?;

        for client in clients {
            ws.feed(packets::new_client_joined(&client)).await?;
        }

        for msg in history {
            ws.feed(packets::new_message(&msg)).await?;
        }

        ws.flush().await?;
        Ok(Self { ws })
    }

    pub async fn disconnect(&mut self, reason: KickReason) -> Result<()> {
        self.ws.close(Some(reason.into_close_frame())).await
    }
    pub async fn ban(&mut self, ban: Ban) -> Result<()> {
        self.ws.close(Some(ban.into_close_frame())).await
    }

    pub async fn profanity_warning(
        &mut self,
        message: &str,
        bad_word: &str,
        span: Range<MessageLen>,
    ) -> Result<()> {
        self.ws
            .send(packets::new_profanity_warn(message, bad_word, span))
            .await?;
        Ok(())
    }

    pub async fn forward_message_change(
        &mut self,
        message_id: Snowflake,
        ty: MessageChangeType,
    ) -> Result<()> {
        self.ws
            .send(packets::new_message_change(message_id, ty))
            .await?;
        Ok(())
    }

    pub async fn forward_user(&mut self, client: &ChatUser) -> Result<()> {
        self.ws.send(packets::new_client_joined(client)).await?;
        Ok(())
    }
    pub async fn forward_multiple_users(
        &mut self,
        clients: impl Iterator<Item = &ChatUser>,
    ) -> Result<()> {
        for client in clients {
            self.ws.feed(packets::new_client_joined(client)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn forward(&mut self, mesg: &Message) -> Result<()> {
        self.ws.send(packets::new_message(mesg)).await?;
        Ok(())
    }
    pub async fn forward_multiple(
        &mut self,
        messages: impl Iterator<Item = &Message>,
    ) -> Result<()> {
        for message in messages {
            self.ws.feed(packets::new_message(message)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn try_recv(&mut self) -> Result<Option<RecievedMessage>> {
        let Some(message) = futures_util::StreamExt::next(&mut self.ws).await else {
            return Err(rocket_ws::result::Error::ConnectionClosed);
        };
        let message = message?;

        if message.is_text() {
            let content = String::from_utf8_lossy(&message.into_data())
                .trim()
                .to_string();

            Ok(Some(RecievedMessage { content: content }))
        } else if message.is_binary() {
            error!("Closing connection because: Received binary message");
            self.ws
                .close(Some(CloseFrame {
                    code: CloseCode::Unsupported,
                    reason: Cow::Borrowed("INT: No binary messages."),
                }))
                .await?;
            Ok(None)
        } else {
            Ok(None)
        }
    }
}
