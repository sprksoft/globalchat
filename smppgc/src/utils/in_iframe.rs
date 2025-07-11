use std::convert::Infallible;

use rocket::{
    async_trait,
    request::{FromRequest, Outcome},
};

#[derive(Clone, Copy)]
pub enum InIframe {
    Yes,
    No,
    Unknown,
}

#[async_trait]
impl<'r> FromRequest<'r> for InIframe {
    type Error = Infallible;
    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        match req.headers().get_one("Sec-Fetch-Dest") {
            Some("iframe") => Outcome::Success(InIframe::Yes),
            None => Outcome::Success(InIframe::Unknown),
            _ => Outcome::Success(InIframe::No),
        }
    }
}
