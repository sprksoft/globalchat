use std::ops::Range;

use rocket::time::Duration;
use tokio_tungstenite::tungstenite;

use crate::{
    chat::{ChatUser, Message, MessageChangeType, MessageLen},
    Snowflake,
};
use log::*;

pub const PACKET_MESSAGE: u8 = 0;
pub const PACKET_MESSAGE_PROF: u8 = 1;
pub const PACKET_MESSAGE_SYSTEM: u8 = 2;

pub const PACKET_SETUP: u8 = 3;
pub const PACKET_USERJOIN: u8 = 4;
pub const PACKET_MODJOIN: u8 = 5;
pub const PACKET_PROFANITY_WARN: u8 = 6;
pub const PACKET_MESSAGE_DEL: u8 = 7;
pub const PACKET_MESSAGE_CENSOR: u8 = 8;

//C2S
pub const PACKET_C2S_MESSAGE: u8 = 0;
pub const PACKET_C2S_DELMSG: u8 = 1;
pub const PACKET_C2S_BANMSGAUTHOR: u8 = 2;

pub fn new_setup<'a, 'b>(id: u16) -> tokio_tungstenite::tungstenite::Message {
    //|    u8    | const PACKET_SETUP
    //| [u8; 3]  | version
    //|    u16   | local id

    let mut data = Vec::with_capacity(1 + size_of::<u8>() * 3 + size_of::<u16>());
    data.push(PACKET_SETUP);
    data.extend_from_slice(&crate::VERSION_INT.to_be_bytes());
    data.extend_from_slice(&id.to_be_bytes());

    tokio_tungstenite::tungstenite::Message::Binary(data)
}
pub fn new_client_joined(client: &ChatUser) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_USERJOIN, const PACKET_MODJOIN
    //| u16  | user id
    //| [u8] | username

    let username_bytes = client.username().as_bytes();
    let mut data = Vec::with_capacity(username_bytes.len() + size_of::<u16>() + 1);
    data.push(if client.mod_badge() {
        PACKET_MODJOIN
    } else {
        PACKET_USERJOIN
    });
    data.extend_from_slice(&client.local_id().to_be_bytes());
    data.extend_from_slice(&username_bytes);
    tokio_tungstenite::tungstenite::Message::Binary(data)
}

pub fn new_profanity_warn(
    message: &str,
    bad_word: &str,
    span: Range<MessageLen>,
) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_PROFANITY_WARN
    //| MessageLen | match start
    //| MessageLen | match end
    //| MessageLen | len of message
    //| [u8]  | message
    //| [u8] | bad word

    let mut data =
        Vec::with_capacity(1 + size_of::<MessageLen>() * 3 + message.len() + bad_word.len());
    data.push(PACKET_PROFANITY_WARN);
    data.extend_from_slice(&span.start.to_be_bytes());
    data.extend_from_slice(&span.end.to_be_bytes());
    data.extend_from_slice(&(message.len() as MessageLen).to_be_bytes());
    data.extend_from_slice(message.as_bytes());
    data.extend_from_slice(bad_word.as_bytes());
    tokio_tungstenite::tungstenite::Message::Binary(data)
}

pub fn new_message_change(
    snowflake: Snowflake,
    ty: MessageChangeType,
) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_MESSAGE_DEL,const PACKET_MESSAGE_CENSOR
    //| Snowflake | message id

    let mut data = Vec::with_capacity(1 + size_of::<Snowflake>());
    data.push(match ty {
        MessageChangeType::Censored => PACKET_MESSAGE_CENSOR,
        MessageChangeType::Deleted => PACKET_MESSAGE_DEL,
    });
    data.extend_from_slice(&snowflake.to_be_bytes());
    tokio_tungstenite::tungstenite::Message::Binary(data)
}

pub fn new_message(mesg: &Message) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_MESSAGE, const PACKET_MESSAGE_PROF
    //|  u16 | sender id
    //| Snowflake | message id
    //| [u8] | content bytes

    let content_bytes = mesg.content.as_bytes();
    let mut data =
        Vec::with_capacity(1 + size_of::<u16>() + size_of::<Snowflake>() + content_bytes.len());

    data.push(if mesg.profanity {
        PACKET_MESSAGE_PROF
    } else {
        PACKET_MESSAGE
    });
    data.extend_from_slice(&mesg.sender.local_id().to_be_bytes());
    data.extend_from_slice(&mesg.id().to_be_bytes());
    data.extend_from_slice(content_bytes);
    tungstenite::Message::Binary(data)
}
pub fn new_system_message(content: &str) -> tokio_tungstenite::tungstenite::Message {
    let content_bytes = content.as_bytes();
    let mut data = Vec::with_capacity(1 + content_bytes.len());
    data.push(PACKET_MESSAGE_SYSTEM);
    data.extend_from_slice(content_bytes);
    tungstenite::Message::Binary(data)
}

pub enum C2SPacket {
    Message(String),
    AdminCmd(AdminCmd),
}

pub enum AdminCmd {
    DelMsg(Snowflake),
    BanMsgAuthor {
        mesg: Snowflake,
        duration: Duration,
        reason: String,
    },
}

fn parse_u64(bytes: &[u8]) -> Result<u64, ()> {
    Ok(u64::from_be_bytes(bytes[..8].try_into().map_err(|_| ())?))
}
fn parse_u32(bytes: &[u8]) -> Result<u32, ()> {
    Ok(u32::from_be_bytes(bytes[..4].try_into().map_err(|_| ())?))
}
fn parse_dur(bytes: &[u8]) -> Result<Duration, ()> {
    Ok(Duration::seconds(parse_u32(bytes)? as i64))
}
fn parse_snowflake(bytes: &[u8]) -> Result<Snowflake, ()> {
    Ok(Snowflake::from_u64(parse_u64(bytes)?))
}
fn parse_str(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

pub fn parse_c2s(data: Vec<u8>) -> Result<C2SPacket, ()> {
    let packet_id = data[0];
    let data = &data[1..];
    match packet_id {
        PACKET_C2S_MESSAGE => Ok(C2SPacket::Message(
            String::from_utf8_lossy(data).trim().to_string(),
        )),
        PACKET_C2S_DELMSG => {
            let snowflake = parse_snowflake(&data[..8])?;
            Ok(C2SPacket::AdminCmd(AdminCmd::DelMsg(snowflake)))
        }
        PACKET_C2S_BANMSGAUTHOR => {
            let snowflake = parse_snowflake(&data[..8])?;
            let data = &data[8..];
            let duration = parse_dur(&data[..4])?;
            let data = &data[4..];
            let reason = parse_str(&data);
            Ok(C2SPacket::AdminCmd(AdminCmd::BanMsgAuthor {
                mesg: snowflake,
                duration,
                reason,
            }))
        }
        id => {
            error!("Invalid c2s packet_id: {}", id);
            Err(())
        }
    }
}
