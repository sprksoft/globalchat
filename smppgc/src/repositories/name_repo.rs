use crate::{
    db::Db,
    models::UserId,
    services::name_service::{InvalidNameError, NameClaimError},
};
use rocket::{
    async_trait,
    request::{FromRequest, Outcome},
};
use rocket_db_pools::Database;
use sqlx::query;
use std::convert::Infallible;

pub struct NameRepo {
    db: Db,
}
impl NameRepo {
    pub async fn claim_name(
        &self,
        user_id: UserId,
        norm_name: &str,
        max_claimed_names: usize,
        max_retention: usize,
    ) -> Result<(), NameClaimError> {
        let result = query!(
            "SELECT claim_name($1, $2, $3, $4)",
            user_id.to_i32(),
            norm_name,
            max_claimed_names as i32,
            max_retention as i32,
        )
        .fetch_one(&*self.db)
        .await?;

        if result.claim_name.is_none() {
            return Err(InvalidNameError::Taken.into());
        }
        Ok(())
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for NameRepo {
    type Error = Infallible;
    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let db = Db::fetch(req.rocket()).unwrap().clone();

        Outcome::Success(Self { db })
    }
}
