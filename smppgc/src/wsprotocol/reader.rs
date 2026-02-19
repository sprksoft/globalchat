use std::borrow::Cow;

use nanotime::snowflake::Snowflake;
use rocket::time::Duration;
use thiserror::Error;

pub type Result<T, E = ReadError> = std::result::Result<T, E>;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ReadError {
    #[error("Unexpected end of stream")]
    UnexpectedEndOfStream,
}

pub struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self(buffer)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn read_dyn(&mut self, len: usize) -> Result<&[u8]> {
        if self.0.len() < len {
            return Err(ReadError::UnexpectedEndOfStream);
        }
        let result = &self.0[..len];
        self.0 = &self.0[len..];
        Ok(result)
    }

    pub fn read<const LEN: usize>(&mut self) -> Result<[u8; LEN]> {
        Ok(self.read_dyn(LEN)?.try_into().unwrap())
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read::<1>()?[0])
    }
    pub fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.read::<2>()?))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.read::<4>()?))
    }
    pub fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.read::<8>()?))
    }
    pub fn read_snowflake(&mut self) -> Result<Snowflake> {
        Ok(Snowflake::from_u64(self.read_u64()?))
    }
    pub fn read_dur(&mut self) -> Result<Duration> {
        Ok(Duration::seconds(self.read_u32()? as i64))
    }
    pub fn read_str(&mut self, len: usize) -> Result<Cow<'_, str>> {
        let array = self.read_dyn(len)?;
        Ok(String::from_utf8_lossy(array))
    }
}

#[cfg(test)]
mod test {
    use crate::wsprotocol::reader::{ReadError, Reader};

    #[test]
    fn reader_unexpected_end_of_stream() {
        let mut reader = Reader::new(&[0x00, 0x01, 0x0D, 0x88]);
        assert_eq!(reader.read_u64(), Ok(69000));

        let mut reader = Reader::new(&[0x00, 0x01, 0x0D]);
        assert_eq!(reader.read_u64(), Err(ReadError::UnexpectedEndOfStream));
    }

    #[test]
    fn reader() {
        let mut reader = Reader::new(&[2, 0x00, 0x01, 0x0D, 0x88]);
        assert_eq!(reader.len(), 5);
        assert_eq!(reader.read_u8(), Ok(2));
        assert_eq!(reader.read_u64(), Ok(69000));

        let mut reader = Reader::new(&[1, 2, 3, 4, 5]);
        assert_eq!(reader.read_dyn(2), Ok([1, 2].as_slice()));
        assert_eq!(reader.read::<3>(), Ok([3, 4, 5]));
    }
}
