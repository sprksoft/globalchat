use nanotime::snowflake::Snowflake;
use rocket::time::Duration;
use tokio_tungstenite::tungstenite;

use crate::{
    models::{ChatUser, Message, Role},
    wsprotocol::{
        packets::{packet, Packet, PacketC2SId, PacketDecodeError, PacketField, PacketId},
        reader::{self, Reader},
    },
};

pub fn new_setup(id: u16, role: Role, max_stored_messages: u8) -> Packet {
    packet! {
        PacketId::SETUP,
        &crate::VERSION_INT.to_be_bytes(),
        &id.to_be_bytes(),
        role.to_u8(),
        max_stored_messages
    }
}
pub fn new_client_joined(client: &ChatUser, mask_role: bool) -> Packet {
    let packet_id = if client.mod_badge() {
        PacketId::MODJOIN
    } else {
        PacketId::USERJOIN
    };
    let role = if mask_role { Role::User } else { client.role() };
    packet! {
        packet_id,
        &client.local_id().to_be_bytes(),
        role.to_u8(),
        client.username().as_bytes()
    }
}

pub fn new_message_del(snowflake: Snowflake) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_MESSAGE_DEL
    //| Snowflake | message id

    packet!(PacketId::MESSAGE_DEL, &snowflake.to_be_bytes())
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
        PacketId::MESSAGE,
        &mesg.sender.local_id().to_be_bytes(),
        &mesg.id().to_be_bytes()
    )
    .into_data();

    for (word, tag) in mesg.content.words() {
        data.push(tag.char() as u8);
        data.extend_from_slice(&(word.len() as u16).to_be_bytes());
        data.extend_from_slice(word.as_bytes());
    }

    tungstenite::Message::Binary(data)
}
pub fn new_system_message(content: &str) -> Packet {
    packet! {
        PacketId::MESSAGE_SYSTEM,
        &content.as_bytes()
    }
}

pub fn new_update_user_count(user_count: u16) -> Packet {
    packet!(PacketId::USER_COUNT, &user_count.to_be_bytes())
}

pub enum C2SPacket {
    Message(String),
    Report {
        message_id: Snowflake,
        reason: Box<str>,
    },
    ModCmd(ModCmd),
    AdminCmd(AdminCmd),
}

pub enum ModCmd {
    WFMark {
        word: String,
        good: bool,
    },
    DelMsg(Snowflake),
    BanMsgAuthor {
        mesg: Snowflake,
        duration: Duration,
        reason: String,
    },
}
pub enum AdminCmd {
    WFCommit,
    WFLock {
        word: String,
        reason: String,
        locked: bool,
    },
}

pub fn parse_c2s(data: Vec<u8>) -> Result<C2SPacket, PacketDecodeError> {
    let mut reader = Reader::new(&data);
    let packet_id = reader
        .read_u8()
        .map_err(|e| PacketDecodeError::PacketIdRead(e))?;

    let packet_id = PacketC2SId::try_from_backing_type(packet_id as isize)
        .ok_or(PacketDecodeError::InvalidPacketId(packet_id))?;

    Ok(parse_packet(packet_id, &mut reader)
        .map_err(|e| PacketDecodeError::PacketRead(packet_id, e))?)
}

fn parse_packet<'a>(packet: PacketC2SId, reader: &mut Reader<'a>) -> reader::Result<C2SPacket> {
    match packet {
        PacketC2SId::MESSAGE => Ok(C2SPacket::Message(
            reader.read_str(reader.len())?.trim().to_string(),
        )),
        PacketC2SId::DELMSG => {
            let snowflake = reader.read_snowflake()?;
            Ok(C2SPacket::ModCmd(ModCmd::DelMsg(snowflake)))
        }
        PacketC2SId::BANMSGAUTHOR => {
            let snowflake = reader.read_snowflake()?;
            let duration = reader.read_dur()?;
            let reason = reader.read_str(reader.len())?.to_string();
            Ok(C2SPacket::ModCmd(ModCmd::BanMsgAuthor {
                mesg: snowflake,
                duration,
                reason,
            }))
        }
        PacketC2SId::WF_MARKGOOD => {
            let word = reader.read_str(reader.len())?.to_string();
            Ok(C2SPacket::ModCmd(ModCmd::WFMark { word, good: true }))
        }
        PacketC2SId::WF_MARKBAD => {
            let word = reader.read_str(reader.len())?.to_string();
            Ok(C2SPacket::ModCmd(ModCmd::WFMark { word, good: false }))
        }
        PacketC2SId::WF_COMMIT => Ok(C2SPacket::AdminCmd(AdminCmd::WFCommit)),

        PacketC2SId::WF_LOCK => {
            //|    u8     | const PACKET_C2S_WF_LOCK
            //| word_len  | u16
            //|    [u8]   | word
            //|           | reason

            let word_len = reader.read_u16()?;
            let word = reader.read_str(word_len as usize)?.to_string();
            let reason = reader.read_str(reader.len())?.to_string();

            Ok(C2SPacket::AdminCmd(AdminCmd::WFLock {
                word,
                reason,
                locked: true,
            }))
        }
        PacketC2SId::WF_UNLOCK => {
            //|    u8     | const PACKET_C2S_WF_UNLOCK
            //|    [u8]   | word

            let word = reader.read_str(reader.len())?.to_string();
            Ok(C2SPacket::AdminCmd(AdminCmd::WFLock {
                word,
                reason: String::new(),
                locked: false,
            }))
        }
        PacketC2SId::REPORT => {
            let message_id = reader.read_snowflake()?;
            let reason = reader.read_str(reader.len())?.into();

            Ok(C2SPacket::Report { message_id, reason })
        }
    }
}
