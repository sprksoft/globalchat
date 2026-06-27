use crate::{
    chat::Chat,
    models::{Ban, ChatUser, Message, Role},
    wsprotocol::protocol::{self, C2SPacket},
};
use futures_util::SinkExt;
use log::*;
use nanotime::snowflake::Snowflake;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    result::Result,
    stream::DuplexStream,
};
use std::borrow::Cow;
use thiserror::Error;
use tokio_tungstenite::tungstenite;
use ts_import::import;

import!({ pub ProtoError } from "../../client/gcapi/protoerr.ts");

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

impl ProtoError {
    pub fn into_close_frame(self) -> CloseFrame<'static> {
        let code = match self {
            Self::Ok => CloseCode::Normal,
            Self::Protocol => CloseCode::Protocol,
            Self::Disclaimer
            | Self::AlreadyInChat
            | Self::NoSession
            | Self::IPRateLimit
            | Self::RateLimit
            | Self::Kick => CloseCode::Policy,

            Self::ChatFull => CloseCode::Again,
            Self::Shutdown => CloseCode::Away,

            _ => CloseCode::Error,
        };
        CloseFrame {
            code: code,
            reason: Cow::Borrowed(self.to_backing_type()),
        }
    }
}

pub struct WsClient {
    ws: DuplexStream,
    ro: bool,
    role: Role,
}
impl WsClient {
    pub fn role(&self) -> Role {
        self.role
    }
    async fn send_setup_packets(
        mut ws: DuplexStream,
        chat: &Chat,
        starting_snowflake: Snowflake,
        profanity: bool,
        local_id: u16,
        role: Role,
    ) -> Result<DuplexStream> {
        let history = chat.history(starting_snowflake, profanity).await;
        let clients = chat.users().await;

        ws.feed(protocol::new_setup(
            local_id,
            role,
            chat.config().max_stored_messages as u8,
        ))
        .await?;

        for client in clients {
            let mask_role = !role.is_mod() && !client.mod_badge();
            ws.feed(protocol::new_client_joined(&client, mask_role))
                .await?;
        }

        for msg in history {
            ws.feed(protocol::new_message(&msg)).await?;
        }

        ws.flush().await?;
        Ok(ws)
    }
    pub async fn new(
        ws: DuplexStream,
        chat: &Chat,
        user: &ChatUser,
        starting_snowflake: Snowflake,
    ) -> Result<Self> {
        Ok(Self {
            ws: Self::send_setup_packets(
                ws,
                chat,
                starting_snowflake,
                user.role().is_mod(),
                user.local_id(),
                user.role(),
            )
            .await?,
            ro: false,
            role: user.role(),
        })
    }
    pub async fn new_ro(
        ws: DuplexStream,
        chat: &Chat,
        starting_snowflake: Snowflake,
    ) -> Result<Self> {
        Ok(Self {
            ws: Self::send_setup_packets(ws, chat, starting_snowflake, false, 0, Role::User)
                .await?,
            ro: true,
            role: Role::User,
        })
    }

    async fn close(&mut self, frame: CloseFrame<'static>) -> Result<()> {
        self.ws
            .send(tokio_tungstenite::tungstenite::Message::Close(Some(frame)))
            .await?;
        Ok(())
    }

    pub async fn disconnect(&mut self, reason: ProtoError) -> Result<()> {
        self.close(reason.into_close_frame()).await
    }
    pub async fn ban(&mut self, ban: Ban) -> Result<()> {
        self.close(ban.into_close_frame()).await
    }
    pub async fn system_message(&mut self, content: &str) -> Result<()> {
        self.ws.send(protocol::new_system_message(content)).await
    }

    pub async fn update_user_count(&mut self, user_count: u16) -> Result<()> {
        self.ws
            .send(protocol::new_update_user_count(user_count))
            .await?;
        Ok(())
    }

    pub async fn forward_message_del(&mut self, message_id: Snowflake) -> Result<()> {
        self.ws.send(protocol::new_message_del(message_id)).await?;
        Ok(())
    }

    pub async fn forward_user(&mut self, client: &ChatUser) -> Result<()> {
        let mask_role = !self.role.is_mod() && !client.mod_badge();
        self.ws
            .send(protocol::new_client_joined(client, mask_role))
            .await?;
        Ok(())
    }
    pub async fn forward_multiple_users(
        &mut self,
        clients: impl Iterator<Item = &ChatUser>,
    ) -> Result<()> {
        for client in clients {
            let mask_role = !self.role.is_mod() && !client.mod_badge();
            self.ws
                .feed(protocol::new_client_joined(client, mask_role))
                .await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn forward(&mut self, mesg: &Message) -> Result<()> {
        self.ws.send(protocol::new_message(mesg)).await?;
        Ok(())
    }
    pub async fn forward_multiple(
        &mut self,
        messages: impl Iterator<Item = &Message>,
    ) -> Result<()> {
        for message in messages {
            self.ws.feed(protocol::new_message(message)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn try_recv(&mut self) -> Result<C2SPacket> {
        loop {
            let Some(message) = futures_util::StreamExt::next(&mut self.ws).await else {
                return Err(rocket_ws::result::Error::ConnectionClosed);
            };
            let message = message?;
            if self.ro {
                continue;
            }

            if message.is_binary() {
                match protocol::parse_c2s(message.into_data()) {
                    Ok(p) => return Ok(p),
                    Err(e) => {
                        error!(
                            "Closing connection because invalid data: Can't parse c2s packet: {}",
                            e
                        );
                        self.close(CloseFrame {
                            code: CloseCode::Invalid,
                            reason: Cow::Borrowed("INT: invalid packet"),
                        })
                        .await?;
                    }
                }
            }
        }
    }
}
