use rocket::{
    async_trait,
    http::Status,
    outcome::{try_outcome, Outcome},
    request::{self, FromRequest},
};
use rocket_db_pools::Connection;
use sqlx::query;

use super::{role::Role, SesId, UserConfig, UserId};
use crate::db::Db;

pub struct User {
    role: Role,
    id: UserId,
    irl_name: Box<str>,
}
impl User {
    pub fn id(&self) -> UserId {
        self.id
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn irl_name(&self) -> &str {
        &self.irl_name
    }
}
pub type UserGuardError = Option<rocket_db_pools::Error<sqlx::Error>>;

#[async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = UserGuardError;
    async fn from_request(req: &'r rocket::Request<'_>) -> request::Outcome<Self, Self::Error> {
        let mut con = try_outcome!(req.guard::<Connection<Db>>().await);
        let ses_id = match req.guard::<SesId>().await {
            Outcome::Success(s) => s,
            Outcome::Forward(f) => return Outcome::Forward(f),
            Outcome::Error(_) => {
                unreachable!("its infallible")
            }
        };
        let user_config = req
            .rocket()
            .state::<UserConfig>()
            .expect("Expected UserConfig to be available");

        match query!("SELECT users.role,users.id,users.irl_name FROM sessions INNER JOIN users ON sessions.user_id = users.id WHERE sessions.id = $1 AND EXTRACT(epoch from now())-sessions.created_at < $2", ses_id.inner(), user_config.max_session_age as f64).fetch_optional(&mut **con).await {
            Ok(Some(u)) => Outcome::Success(User {
                role: Role::from_i32(u.role).unwrap_or(Role::User),
                id: UserId(u.id),
                irl_name: u.irl_name.into(),
            }),
            Ok(None) => Outcome::Forward(Status::Unauthorized),
            Err(e) => Outcome::Error((Status::InternalServerError, Some(rocket_db_pools::Error::Get(e)))),
        }
    }
}

pub struct ModUser(pub User);
#[async_trait]
impl<'r> FromRequest<'r> for ModUser {
    type Error = Option<rocket_db_pools::Error<sqlx::Error>>;
    async fn from_request(req: &'r rocket::Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user = try_outcome!(req.guard::<User>().await);
        match user.role {
            Role::Owner | Role::Admin | Role::Mod => Outcome::Success(ModUser(user)),
            Role::User => Outcome::Forward(Status::Forbidden),
        }
    }
}

pub struct AdminUser(pub User);
#[async_trait]
impl<'r> FromRequest<'r> for AdminUser {
    type Error = Option<rocket_db_pools::Error<sqlx::Error>>;
    async fn from_request(req: &'r rocket::Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user = try_outcome!(req.guard::<User>().await);
        match user.role {
            Role::Owner | Role::Admin => Outcome::Success(AdminUser(user)),
            Role::Mod | Role::User => Outcome::Forward(Status::Forbidden),
        }
    }
}
