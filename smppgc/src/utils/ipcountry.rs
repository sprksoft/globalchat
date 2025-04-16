use std::convert::Infallible;

use rocket::{
    request::{FromRequest, Outcome},
    Request,
};

pub struct IpCountry {
    code: [u8; 2],
}
impl IpCountry {
    pub fn unknown() -> Self {
        Self { code: [b'X'; 2] }
    }
    pub fn parse(str: &str) -> Option<Self> {
        let mut chars = str.chars();
        let first = chars.next()?;
        let second = chars.next()?;
        if !first.is_ascii() || !second.is_ascii() {
            return None;
        }
        Some(Self {
            code: [first as u8, second as u8],
        })
    }

    pub fn is_be(&self) -> bool {
        self.code == [b'B', b'E']
    }
    pub fn is_unknown(&self) -> bool {
        self.code == [b'X', b'X']
    }
    pub fn is_tor(&self) -> bool {
        self.code == [b'T', b'1']
    }
}
//CF-IPCountry
#[rocket::async_trait]
impl<'r> FromRequest<'r> for IpCountry {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Infallible> {
        Outcome::Success(
            request
                .headers()
                .get_one("CF-IPCountry")
                .map(|cc| IpCountry::parse(cc))
                .flatten()
                .unwrap_or(IpCountry::unknown()),
        )
    }
}
