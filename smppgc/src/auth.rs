use std::path::{Path, PathBuf};

use log::*;
use rocket::{
    fairing::AdHoc,
    http::Status,
    outcome::try_outcome,
    request::{FromRequest, Outcome},
    serde::{Deserialize, Serialize},
    Request,
};

pub fn get_role_from_key(key: &str, auth_file: &Path) -> std::io::Result<Option<PermRole>> {
    let file_contents = std::fs::read_to_string(auth_file)?;
    for (key, role) in file_contents
        .lines()
        .map(|l| l.split_once(":"))
        .filter_map(|l| l)
    {
        if key.trim() == key {
            return Ok(Some(match role {
                "mod" => PermRole::Mod,
                "admin" => PermRole::Admin,
                _ => continue,
            }));
        }
    }
    Ok(None)
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AuthConfig {
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
            match get_role_from_key(auth.value_trimmed(), &conf.auth_file) {
                Err(e) => {
                    error!("Failed to load auth file: {}", e);
                    return Outcome::Error((Status::InternalServerError, e));
                }
                Ok(Some(role)) => Outcome::Success(GcAuth(role)),
                Ok(None) => Outcome::Forward(Status::Unauthorized),
            }
        } else {
            Outcome::Forward(Status::Unauthorized)
        }
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
