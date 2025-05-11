use sqlx::FromRow;

#[derive(FromRow)]
pub struct User {
    pub id: i32,
    pub smid: String,
    pub role: i32,
    pub irl_name: String,
    pub ban_count: i32,
}
#[derive(FromRow)]
pub struct PromoteKey {
    pub key: String,
    pub new_role: i32,
    pub used_by: Option<i32>,
}
