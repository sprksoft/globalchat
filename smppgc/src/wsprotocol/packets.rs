use thiserror::Error;
use ts_import::import;

import!({ pub PacketId, pub PacketC2SId } from "../../client/gcapi/packets.ts");

pub type Packet = tokio_tungstenite::tungstenite::Message;
pub type Error = PacketDecodeError;

#[derive(Debug, Error)]
pub enum PacketDecodeError {
    #[error("Couldn't read packet id: {0}")]
    PacketIdRead(ReadError),
    #[error("Couldn't read packet {0:?}: {1}")]
    PacketRead(PacketC2SId, ReadError),
    #[error("Invalid packet id {0}")]
    InvalidPacketId(u8),
}

pub trait PacketField {
    fn extend_bytes(self, data: &mut Vec<u8>);
    fn size(&self) -> usize;
}
impl PacketField for u8 {
    #[inline]
    fn extend_bytes(self, data: &mut Vec<u8>) {
        data.push(self);
    }
    #[inline]
    fn size(&self) -> usize {
        1
    }
}
impl PacketField for &[u8] {
    #[inline]
    fn extend_bytes(self, data: &mut Vec<u8>) {
        data.extend_from_slice(self);
    }
    fn size(&self) -> usize {
        self.len()
    }
}
impl<const N: usize> PacketField for [u8; N] {
    fn extend_bytes(self, data: &mut Vec<u8>) {
        data.extend_from_slice(&self);
    }
    fn size(&self) -> usize {
        self.len()
    }
}
impl PacketField for PacketId {
    fn extend_bytes(self, data: &mut Vec<u8>) {
        data.push(self.to_backing_type() as u8);
    }
    fn size(&self) -> usize {
        1
    }
}

macro_rules! packet_impl {
    ($($expr:expr),*) => {
        {
            let size = 0 $(+$expr.size())*;
            let mut data = Vec::with_capacity(size);
            $(
            $expr.extend_bytes(&mut data);
            )*
            tokio_tungstenite::tungstenite::Message::Binary(data)
        }
    };
}
pub(super) use packet_impl as packet;

use crate::wsprotocol::reader::ReadError;
