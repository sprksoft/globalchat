use std::convert::Infallible;

use rocket::{
    async_trait,
    fairing::AdHoc,
    http::{Cookie, SameSite, Status},
    request::{FromRequest, Outcome},
    time::Duration,
};
use uuid::Uuid;

pub struct CSRFProtect;

#[async_trait]
impl<'r> FromRequest<'r> for CSRFProtect {
    type Error = Infallible;
    async fn from_request(req: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        if req
            .cookies()
            .get("csrf-protect")
            .map(|c| {
                req.headers()
                    .get_one("X-CSRF-Protect")
                    .map(|header| header == c.value_trimmed())
            })
            .flatten()
            .unwrap_or(false)
        {
            Outcome::Success(CSRFProtect)
        } else {
            Outcome::Forward(Status::Forbidden)
        }
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_request("csrf protect", |req, _| {
        Box::pin(async move {
            req.cookies().add(
                Cookie::build(("csrf-protect", Uuid::new_v4().simple().to_string()))
                    .max_age(Duration::days(365))
                    .partitioned(true)
                    .same_site(SameSite::None)
                    .secure(true),
            )
        })
    })
}
