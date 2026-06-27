use std::convert::Infallible;

use nanotime::snowflake::Snowflake;
use rocket::{
    async_trait,
    request::{FromRequest, Outcome},
    time::Duration,
};
use rocket_db_pools::Database;
use sqlx::query;
use thiserror::Error;

use crate::{
    db::Db,
    models::{Ban, Role, UserId},
};

#[derive(Debug, Error)]
pub enum BanError {
    #[error("Permission denied")]
    PermissionDenied,
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
}

pub struct UserRepo {
    db: Db,
}

impl UserRepo {
    pub async fn ban_user(
        &self,
        user_id: UserId,
        banner_role: Role,
        reason: &str,
        duration: Duration,
    ) -> Result<(), BanError> {
        let mut con = self.db.acquire().await?;
        query!("DELETE FROM bans WHERE expiration_time-EXTRACT(epoch from now()) < 0")
            .execute(&mut *con)
            .await?;

        let role = Role::from_i32(
            query!("SELECT role FROM users WHERE id=$1", user_id.to_i32())
                .fetch_one(&mut *con)
                .await?
                .role,
        )
        .unwrap_or(Role::User);
        if role >= banner_role {
            return Err(BanError::PermissionDenied);
        }

        query!(
            "INSERT INTO bans (user_id, reason, expiration_time) VALUES ($1, $2, EXTRACT(epoch from now())+$3)",
            user_id.to_i32(),
            reason,
            duration.whole_seconds() as i32
        ).execute(&mut *con).await?;

        query!(
            "UPDATE users SET ban_count = ban_count + 1 WHERE id=$1",
            user_id.to_i32()
        )
        .execute(&mut *con)
        .await?;
        Ok(())
    }

    pub async fn get_ban(&self, user_id: UserId) -> Result<Option<Ban>, sqlx::Error> {
        Ok(query!(
            "SELECT reason,expiration_time FROM bans WHERE user_id=$1 AND expiration_time-EXTRACT(epoch from now()) > 0",
            user_id.to_i32()
        ).fetch_optional( &*self.db).await?.map(|b|Ban { reason: b.reason, expiration_time: b.expiration_time }))
    }

    pub async fn report_message(
        &self,
        message_id: Snowflake,
        reporter_id: UserId,
        reason: Box<str>,
    ) -> Result<(), sqlx::Error> {
        let mut con = self.db.acquire().await?;
        query!(
            "INSERT INTO reports(message_snowflake, reporter_id, reason) VALUES($1, $2, $3)",
            message_id.to_u64().cast_signed(),
            reporter_id.to_i32(),
            &reason
        )
        .execute(&mut *con)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for UserRepo {
    type Error = Infallible;
    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let db = Db::fetch(req.rocket()).unwrap().clone();

        Outcome::Success(Self { db })
    }
}
