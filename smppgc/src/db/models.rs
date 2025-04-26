use rocket::time::PrimitiveDateTime;
use sqlx::FromRow;

use crate::users::{role::Role, SmId};

#[derive(FromRow)]
pub struct User {
    pub id: i32,
    pub smid: String,
    pub role: i32,
    pub ban_count: i32,
    pub irl_name: String,
    pub ban_release_timestamp: Option<i32>,
}
