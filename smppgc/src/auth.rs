use std::path::PathBuf;

use log::*;
use rocket::{
    fairing::AdHoc,
    http::Status,
    outcome::try_outcome,
    request::{FromRequest, Outcome},
    serde::{Deserialize, Serialize},
    Request,
};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct AuthConfig {
    pub auth_file: PathBuf,
}

pub enum PermRole {
    Mod,
    Admin,
}

pub struct GcAuth(PermRole);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for GcAuth {
    type Error = std::io::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(conf) = request.rocket().state::<AuthConfig>() else {
            error!("No Auth config");
            return Outcome::Forward(Status::Unauthorized);
        };

        if let Some(auth) = request.cookies().get("SMPPGC-Auth") {
            let file_contents = match std::fs::read_to_string(&conf.auth_file) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to load auth file: {}", e);
                    return Outcome::Error((Status::InternalServerError, e));
                }
            };
            for (key, role) in file_contents
                .lines()
                .map(|l| l.split_once(":"))
                .filter_map(|l| l)
            {
                if key.trim() == auth.value_trimmed() {
                    return Outcome::Success(GcAuth(match role {
                        "mod" => PermRole::Mod,
                        "admin" => PermRole::Admin,
                        _ => continue,
                    }));
                }
            }
        }

        Outcome::Forward(Status::Unauthorized)
    }
}

pub struct GcMod;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for GcMod {
    type Error = std::io::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match try_outcome!(request.guard().await) {
            GcAuth(PermRole::Mod) => rocket::outcome::Outcome::Success(GcMod),
            GcAuth(PermRole::Admin) => rocket::outcome::Outcome::Success(GcMod),
        }
    }
}

pub struct GcAdmin;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for GcAdmin {
    type Error = std::io::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match try_outcome!(request.guard().await) {
            GcAuth(PermRole::Mod) => rocket::outcome::Outcome::Success(GcAdmin),
            _ => Outcome::Forward(Status::Forbidden),
        }
    }
}

pub fn stage() -> AdHoc {
    AdHoc::config::<AuthConfig>()
}
