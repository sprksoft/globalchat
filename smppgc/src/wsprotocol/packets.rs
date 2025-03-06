use std::ops::Range;

use tokio_tungstenite::tungstenite;

use crate::{
    chat::Message,
    users::{UserInfo, UserSid},
};

pub const PACKET_SETUP: u8 = 0;
pub const PACKET_MESSAGE: u8 = 1;
pub const PACKET_USERJOIN: u8 = 2;
pub const PACKET_PROFANITY_WARN: u8 = 3;

pub fn new_setup<'a, 'b>(
    sid: UserSid,
    id: u16,
    clients: Vec<UserInfo>,
    history: Vec<Message>,
) -> tokio_tungstenite::tungstenite::Message {
    //|    u8    | const PACKET_SETUP
    //| [u8; 3]  | version
    //|    u16   | id
    //| [u8; 33] | key
    //
    //  clients:
    //|    u16   | client count
    //|    u16   | client id
    //|    u8    | username len
    //|    [u8]  | username
    //
    //  hist messages:
    //|    u32   | time (minutes since UNIX_EPOCH)
    //|    u8    | sender username len
    //|    [u8]  | sender username
    //|    u8    | content len
    //|    [u8]  | content

    let key_str = sid.to_string();
    let key_str_bytes = key_str.as_bytes();
    let mut data = Vec::with_capacity(1 + 3 + size_of::<u16>() + key_str_bytes.len());
    data.push(PACKET_SETUP);
    data.extend_from_slice(&crate::VERSION_INT.to_be_bytes());
    data.extend_from_slice(&id.to_be_bytes());
    data.extend_from_slice(key_str_bytes);

    data.extend_from_slice(&(clients.len() as u16).to_be_bytes());
    for client in clients {
        let name_bytes = client.username().as_bytes();
        data.reserve(name_bytes.len() + 3);
        data.extend_from_slice(&client.id().to_be_bytes());
        data.push(name_bytes.len() as u8);
        data.extend_from_slice(name_bytes);
    }
    for message in history {
        let sender_bytes = message.sender.username().as_bytes();
        let content_bytes = message.content.as_bytes();
        data.reserve(sender_bytes.len() + content_bytes.len() + 2 + 8);
        data.extend_from_slice(&message.timestamp.to_be_bytes());
        data.push(sender_bytes.len() as u8);
        data.extend_from_slice(sender_bytes);
        data.push(content_bytes.len() as u8);
        data.extend_from_slice(content_bytes);
    }
    tokio_tungstenite::tungstenite::Message::Binary(data)
}
pub fn new_client_joined(client: &UserInfo) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_USERJOIN
    //| u16  | user id
    //| [u8] | username

    let username_bytes = client.username().as_bytes();
    let mut data = Vec::with_capacity(username_bytes.len() + size_of::<u16>() + 1);
    data.push(PACKET_USERJOIN);
    data.extend_from_slice(&client.id().to_be_bytes());
    data.extend_from_slice(&username_bytes);
    tokio_tungstenite::tungstenite::Message::Binary(data)
}

pub fn new_profanity_warn(
    message: &str,
    bad_word: &str,
    span: Range<crate::MessageLen>,
) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_PROFANITY_WARN
    //| MessageLen | match start
    //| MessageLen | match end
    //| MessageLen | len of message
    //| [u8]  | message
    //| [u8] | bad word

    let mut data =
        Vec::with_capacity(1 + size_of::<crate::MessageLen>() * 3 + message.len() + bad_word.len());
    data.push(PACKET_PROFANITY_WARN);
    data.extend_from_slice(&span.start.to_be_bytes());
    data.extend_from_slice(&span.end.to_be_bytes());
    data.extend_from_slice(&(message.len() as crate::MessageLen).to_be_bytes());
    data.extend_from_slice(message.as_bytes());
    data.extend_from_slice(bad_word.as_bytes());
    tokio_tungstenite::tungstenite::Message::Binary(data)
}

pub fn new_message(mesg: &Message) -> tokio_tungstenite::tungstenite::Message {
    //|  u8  | const PACKET_MESSAGE
    //|  RelSnowflake | sender id
    //|  RelSnowflakeu32 | message id
    //| [u8] | content bytes

    let content_bytes = mesg.content.as_bytes();
    let mut data = Vec::with_capacity(content_bytes.len() + size_of::<u64>() * 2);
    data.extend_from_slice(&mesg.sender.id().to_be_bytes());
    data.extend_from_slice(&mesg.timestamp.to_be_bytes());
    data.extend_from_slice(content_bytes);
    tungstenite::Message::Binary(data)
}
