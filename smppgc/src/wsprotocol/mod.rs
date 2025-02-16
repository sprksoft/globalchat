use std::borrow::Cow;

use futures_util::SinkExt;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    result::Result,
    stream::DuplexStream,
};
use thiserror::Error;
use tokio_tungstenite::tungstenite;

use crate::{
    chat::Message,
    users::{UserInfo, UserSid},
    Timestamp,
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
        IpRateLimit(Policy, "err_ipratelimit"),
        TooManyUsers(Policy, "err_toomanyusers"),
        RateLimit(Policy,"err_ratelimit"),
        ServerShutdown(Away,"err_shutdown"),
        ChatFull(Again,"err_full"),
        UsernameProfanity(Error,"err_username_prof"),
        UsernameTaken(Error,"err_username_taken"),
        UsernameInvalid(Error,"err_username_invalid")
    }
}

pub struct RecievedMessage {
    pub content: String,
    pub timestamp: Timestamp,
}
impl RecievedMessage {
    #[inline]
    pub fn len(&self) -> usize {
        self.content.len()
    }
}

pub struct WsClient {
    ws: DuplexStream,
    user_info: UserInfo,
}
impl WsClient {
    pub async fn new(
        mut ws: DuplexStream,
        user_info: UserInfo,
        clients: Vec<UserInfo>,
        history: Vec<Message>,
    ) -> Result<Self> {
        ws.send(packets::new_setup(
            user_info.static_id(),
            user_info.id(),
            clients,
            history,
        ))
        .await?;

        Ok(Self { ws, user_info })
    }

    pub async fn kick(&mut self, reason: KickReason) -> Result<()> {
        self.ws.close(Some(reason.into_close_frame())).await
    }

    pub async fn forward_client(&mut self, client: &UserInfo) -> Result<()> {
        self.ws.send(packets::new_client_joined(client)).await?;
        Ok(())
    }
    pub async fn forward_all_clients(
        &mut self,
        clients: impl Iterator<Item = &UserInfo>,
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
    pub async fn forward_all(&mut self, messages: impl Iterator<Item = &Message>) -> Result<()> {
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
        if message.is_close() {
            return Ok(None);
        }
        if !message.is_text() {
            error!("Closing connection because: Received non text message");
            self.ws
                .close(Some(CloseFrame {
                    code: CloseCode::Unsupported,
                    reason: Cow::Borrowed("INT: No non text messages."),
                }))
                .await?;
            return Ok(None);
        }
        let content = String::from_utf8_lossy(&message.into_data()).to_string();

        Ok(Some(RecievedMessage {
            timestamp: Timestamp::now(),
            content: content,
        }))
    }
}
