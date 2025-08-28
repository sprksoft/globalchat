use std::ops::Range;

use rocket::time::Duration;
use tokio_tungstenite::tungstenite;

use crate::{
    chat::{ChatUser, Message, MessageLen},
    users::role::Role,
    Snowflake,
};
use log::*;

pub const PACKET_MESSAGE: u8 = 0;
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
pub const PACKET_C2S_WF_MARKGOOD: u8 = 3;
pub const PACKET_C2S_WF_MARKBAD: u8 = 4;
pub const PACKET_C2S_WF_COMMIT: u8 = 5;

macro_rules! packet {
    ($($expr:expr),*) => {
        {
            let size = 0 $(+size_of_val($expr))*;
            let mut data = Vec::with_capacity(size);
            $(
            data.extend_from_slice($expr);
            )*
            tokio_tungstenite::tungstenite::Message::Binary(data)
        }
    };
}

type Packet = tokio_tungstenite::tungstenite::Message;

pub fn new_setup(id: u16) -> Packet {
    packet! {
        &[PACKET_SETUP],
        &crate::VERSION_INT.to_be_bytes(),
        &id.to_be_bytes()
    }
}
pub fn new_client_joined(client: &ChatUser, mask_role: bool) -> Packet {
    let packet_id = if client.mod_badge() {
        PACKET_MODJOIN
    } else {
        PACKET_USERJOIN
    };
    let role = if mask_role { Role::User } else { client.role() };
    packet! {
        &[packet_id],
        &client.local_id().to_be_bytes(),
        &[role.to_u8()],
        client.username().as_bytes()
    }
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

pub fn new_message_del(snowflake: Snowflake) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_MESSAGE_DEL
    //| Snowflake | message id

    packet!(&[PACKET_MESSAGE_DEL], &snowflake.to_be_bytes())
}

pub fn new_message(mesg: &Message) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_MESSAGE
    //|  u16 | sender id
    //| Snowflake | message id
    //| [Word] | content
    //
    // Word:
    //| u8  | tag
    //| u16 | len
    //| [u8]| data

    let mut data = packet!(
        &[PACKET_MESSAGE],
        &mesg.sender.local_id().to_be_bytes(),
        &mesg.id().to_be_bytes()
    )
    .into_data();

    for (word, tag) in mesg.content.words() {
        data.push(tag.into());
        data.extend_from_slice(&(word.len() as u16).to_be_bytes());
        data.extend_from_slice(word.as_bytes());
    }

    tungstenite::Message::Binary(data)
}
pub fn new_system_message(content: &str) -> Packet {
    packet! {
        &[PACKET_MESSAGE_SYSTEM],
        &content.as_bytes()
    }
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
    WFMark {
        word: String,
        good: bool,
    },
    WFCommit,
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
            //|    u8     | const PACKET_C2S_BANMSGAUTHOR
            //| Snowflake | message id
            //|    u32    | duration (seconds)
            //|           | reason

            let snowflake = parse_snowflake(&data[..8])?;
            let data = &data[8..];
            let duration = parse_dur(&data[..4])?;
            let data = &data[4..];
            let reason = parse_str(&data);
            if reason.len() >= 1000 {
                error!("Invalid PACKET_C2S_BANMSGAUTHOR packet: Reason field too large");
                return Err(());
            }
            Ok(C2SPacket::AdminCmd(AdminCmd::BanMsgAuthor {
                mesg: snowflake,
                duration,
                reason,
            }))
        }
        PACKET_C2S_WF_MARKGOOD => {
            let word = parse_str(&data);
            Ok(C2SPacket::AdminCmd(AdminCmd::WFMark { word, good: true }))
        }
        PACKET_C2S_WF_MARKBAD => {
            let word = parse_str(&data);
            Ok(C2SPacket::AdminCmd(AdminCmd::WFMark { word, good: false }))
        }
        PACKET_C2S_WF_COMMIT => Ok(C2SPacket::AdminCmd(AdminCmd::WFCommit)),
        id => {
            error!("Invalid c2s packet_id: {}", id);
            Err(())
        }
    }
}
