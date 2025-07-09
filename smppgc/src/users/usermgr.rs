use log::*;
use profanity::{ProfanityFilter, TokenizedMessage};
use rocket::{
    outcome::{try_outcome, Outcome},
    request::FromRequest,
    time::Duration,
};
use rocket_db_pools::Connection;
use sqlx::query;
use std::ops::Deref;
use thiserror::Error;

use crate::{db::Db, wsprotocol::KickReason};

use super::{User, UserConfig, UserId};

#[derive(Error, Debug)]
pub enum NameClaimError {
    #[error("Username contains invalid characters")]
    Invalid,
    #[error("Username too short or long")]
    Length,
    #[error("Username taken")]
    Taken,
    #[error("Username contains profanity")]
    Profanity,
}
impl NameClaimError {
    pub fn into_kickreason(self) -> KickReason {
        match self {
            Self::Profanity => KickReason::UsernameProfanity,
            Self::Taken => KickReason::UsernameTaken,
            Self::Length => KickReason::UsernameInvalidLength,
            Self::Invalid => KickReason::UsernameInvalid,
        }
    }
}

pub struct ClaimedName(String);
impl Deref for ClaimedName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<String> for ClaimedName {
    fn into(self) -> String {
        self.0
    }
}

pub struct Ban {
    reason: Option<String>,
    expiration_time: i32,
}
impl Ban {
    pub fn reason(&self) -> &str {
        match self.reason.as_ref() {
            Some(r) => &*r,
            None => "",
        }
    }

    pub fn into_close_frame(&self) -> rocket_ws::frame::CloseFrame {
        rocket_ws::frame::CloseFrame {
            code: rocket_ws::frame::CloseCode::Normal,
            reason: format!("err_banned:{}:{}", self.expiration_time, self.reason()).into(),
        }
    }
}

pub struct UserManager<'r> {
    con: Connection<Db>,
    prof_filter: &'r tokio::sync::RwLock<ProfanityFilter>,
    max_name_len: usize,
    max_claimed_names: usize,
    max_name_retention: usize,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserManager<'r> {
    type Error = Option<rocket_db_pools::Error<sqlx::Error>>;

    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let con = try_outcome!(req.guard::<Connection<Db>>().await);
        let prof_filter = req
            .rocket()
            .state::<tokio::sync::RwLock<ProfanityFilter>>()
            .expect("Failed to get prof filter");
        let user_config = req
            .rocket()
            .state::<UserConfig>()
            .expect("Failed to get user config");

        Outcome::Success(UserManager {
            con,
            prof_filter,
            max_name_len: user_config.max_username_len,
            max_claimed_names: user_config.max_claimed_names,
            max_name_retention: user_config.max_name_retention,
        })
    }
}

impl<'r> UserManager<'r> {
    fn tokenized_to_normalized(tm: TokenizedMessage) -> String {
        let mut str = String::with_capacity(tm.len());
        for tg in tm.tokens() {
            for token in tg.iter() {
                if let Some(char) = token.to_char() {
                    str.push(char)
                }
            }
        }
        str
    }
    pub async fn claim_name(
        &mut self,
        user: &User,
        name: &str,
    ) -> Result<Result<ClaimedName, NameClaimError>, sqlx::Error> {
        if name.len() > self.max_name_len || name.len() < 2 {
            return Ok(Err(NameClaimError::Length));
        }
        let (norm_name, name) = {
            let filter = self.prof_filter.read().await;
            let (tok, name) = filter.tokenize(name);
            if filter.check(&tok).is_some() {
                return Ok(Err(NameClaimError::Profanity));
            }
            if name.len() > self.max_name_len || name.len() < 2 {
                return Ok(Err(NameClaimError::Invalid));
            }
            (Self::tokenized_to_normalized(tok), name)
        };
        let max_claimed_names = self.max_claimed_names as i32;
        let max_retention = self.max_name_retention as i32;
        let result = query!(
            "SELECT claim_name($1, $2, $3, $4)",
            user.id().to_i32(),
            norm_name,
            max_claimed_names,
            max_retention,
        )
        .fetch_one(&mut **self.con)
        .await?;

        if result.claim_name.is_none() {
            return Ok(Err(NameClaimError::Taken));
        }
        Ok(Ok(ClaimedName(name)))
    }

    pub async fn ban_user(
        &mut self,
        user_id: UserId,
        reason: &str,
        duration: Duration,
    ) -> Result<(), sqlx::Error> {
        query!("DELETE FROM bans WHERE expiration_time-EXTRACT(epoch from now()) < 0")
            .execute(&mut **self.con)
            .await?;

        query!(
            "INSERT INTO bans (user_id, reason, expiration_time) VALUES ($1, $2, EXTRACT(epoch from now())+$3)",
            user_id.to_i32(),
            reason,
            duration.whole_seconds() as i32
        ).execute(&mut **self.con).await?;

        query!(
            "UPDATE users SET ban_count = ban_count + 1 WHERE id=$1",
            user_id.to_i32()
        )
        .execute(&mut **self.con)
        .await?;
        Ok(())
    }

    pub async fn get_ban(&mut self, user_id: UserId) -> Result<Option<Ban>, sqlx::Error> {
        Ok(query!(
            "SELECT reason,expiration_time FROM bans WHERE user_id=$1 AND expiration_time-EXTRACT(epoch from now()) > 0",
            user_id.to_i32()
        ).fetch_optional(&mut **self.con).await?.map(|b|Ban { reason: b.reason, expiration_time: b.expiration_time }))
    }
}
