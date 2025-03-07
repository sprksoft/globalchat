use std::ops::Range;

use tokio_tungstenite::tungstenite;

use crate::{
    chat::Message,
    users::{UserInfo, UserSid},
    Snowflake,
};

pub const PACKET_SETUP: u8 = 0;
pub const PACKET_MESSAGE: u8 = 1;
pub const PACKET_PROFANITY_MESSAGE: u8 = 4;
pub const PACKET_USERJOIN: u8 = 2;
pub const PACKET_PROFANITY_WARN: u8 = 3;

pub fn new_setup<'a, 'b>(sid: UserSid, id: u16) -> tokio_tungstenite::tungstenite::Message {
    //|    u8    | const PACKET_SETUP
    //| [u8; 3]  | version
    //|    u16   | local id
    //| [u8; 33] | static id

    let sid_str = sid.to_string();
    let key_str_bytes = sid_str.as_bytes();
    let mut data = Vec::with_capacity(1 + 3 + size_of::<u16>() + key_str_bytes.len());
    data.push(PACKET_SETUP);
    data.extend_from_slice(&crate::VERSION_INT.to_be_bytes());
    data.extend_from_slice(&id.to_be_bytes());
    data.extend_from_slice(key_str_bytes);

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
    //|  u8  | const PACKET_MESSAGE, const PACKET_PROFANITY_MESSAGE
    //|  u16 | sender id
    //| Snowflake | message id
    //| [u8] | content bytes

    let content_bytes = mesg.content.as_bytes();
    let mut data =
        Vec::with_capacity(1 + size_of::<u16>() + size_of::<Snowflake>() + content_bytes.len());
    if mesg.profanity {
        data.push(PACKET_PROFANITY_MESSAGE)
    } else {
        data.push(PACKET_MESSAGE)
    }
    data.extend_from_slice(&mesg.sender.id().to_be_bytes());
    data.extend_from_slice(&mesg.id().to_be_bytes());
    data.extend_from_slice(content_bytes);
    tungstenite::Message::Binary(data)
}
