use std::ops::Deref;

use rocket::request::{FromRequest, Outcome};
use rocket::serde::Serialize;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Copy, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DisclaimerVer(usize);
impl DisclaimerVer {
    pub const LATEST: DisclaimerVer = DisclaimerVer(1);
    const COOKIE_NAME: &'static str = "accepted_disclaimer";

    pub fn inner(self) -> usize {
        self.0
    }
}
impl Deref for DisclaimerVer {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DisclaimerVer {
    type Error = std::convert::Infallible;
    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        Outcome::Success(DisclaimerVer(
            request
                .cookies()
                .get(Self::COOKIE_NAME)
                .map(|c| c.value_trimmed().parse().unwrap_or(0))
                .unwrap_or(0),
        ))
    }
}
