use std::{convert::Infallible, sync::Arc};

use nanotime::snowflake::Snowflake;
use rocket::{
    outcome::{try_outcome, Outcome},
    request::FromRequest,
    time::Duration,
};
use thiserror::Error;

use crate::{
    config::UserConfig,
    models::{Ban, Role, User, UserId},
    repositories::{BanError, UserRepo},
    wf::Filter,
};

#[derive(Error, Debug)]
pub enum EnterChatError {
    #[error("The user is banned")]
    Banned(Ban),
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub struct UserService {
    filter: Arc<Filter>,
    repo: UserRepo,
    config: UserConfig,
}
impl UserService {
    pub async fn enter_chat(&self, user: &User) -> Result<(), EnterChatError> {
        if !user.role().is_mod() {
            if let Some(ban) = self.repo.get_ban(user.id()).await? {
                return Err(EnterChatError::Banned(ban));
            }
        }
        Ok(())
    }

    pub async fn ban_user(
        &self,
        user_id: UserId,
        banner_role: Role,
        reason: &str,
        duration: Duration,
    ) -> Result<(), BanError> {
        self.repo
            .ban_user(user_id, banner_role, reason, duration)
            .await
    }

    pub async fn report_message(
        &self,
        reporter_id: UserId,
        message_id: Snowflake,
        reason: Box<str>,
    ) -> Result<(), sqlx::Error> {
        self.repo
            .report_message(message_id, reporter_id, reason)
            .await
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserService {
    type Error = Infallible;

    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let filter = req
            .rocket()
            .state::<Arc<Filter>>()
            .expect("Failed to get word filter");
        let config = req
            .rocket()
            .state::<UserConfig>()
            .expect("Failed to get user config")
            .clone();

        let repo = try_outcome!(req.guard::<UserRepo>().await);

        Outcome::Success(UserService {
            filter: filter.clone(),
            repo,
            config,
        })
    }
}
