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

pub fn get_role_from_key(key: &str, auth_file: &Path) -> std::io::Result<Option<GcRole>> {
    let file_contents = std::fs::read_to_string(auth_file)?;
    for (file_key, role) in file_contents
        .lines()
        .map(|l| l.split_once(":"))
        .filter_map(|l| l)
    {
        if key.trim() == file_key.trim() {
            return Ok(Some(match role {
                "mod" => GcRole::Mod,
                "admin" => GcRole::Admin,
                "user" => GcRole::User,
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum GcRole {
    User,
    Mod,
    Admin,
}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for GcRole {
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
                Ok(Some(role)) => Outcome::Success(role),
                Ok(None) => Outcome::Forward(Status::Unauthorized),
            }
        } else {
            Outcome::Success(GcRole::User)
        }
    }
}

pub struct GcMod;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for GcMod {
    type Error = std::io::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match try_outcome!(request.guard().await) {
            GcRole::Mod => rocket::outcome::Outcome::Success(GcMod),
            GcRole::Admin => rocket::outcome::Outcome::Success(GcMod),
            _ => Outcome::Forward(Status::Forbidden),
        }
    }
}

pub struct GcAdmin;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for GcAdmin {
    type Error = std::io::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match try_outcome!(request.guard().await) {
            GcRole::Admin => rocket::outcome::Outcome::Success(GcAdmin),
            _ => Outcome::Forward(Status::Forbidden),
        }
    }
}

pub fn stage() -> AdHoc {
    AdHoc::config::<AuthConfig>()
}
