use rocket_db_pools::Connection;
use sqlx::query;
use uuid::Uuid;

use crate::{
    csrf::CSRFProtect,
    db::{Db, DbResult},
    users::{role::Role, AdminUser, User},
};
use rocket::{fairing::AdHoc, get, post, routes, Responder};

#[get("/role")]
fn role(user: Option<User>) -> &'static str {
    match user.map(|u| u.role()) {
        None => "not logged in",
        Some(Role::Owner) => "owner",
        Some(Role::Mod) => "mod",
        Some(Role::Admin) => "admin",
        Some(Role::User) => "user",
    }
}

#[derive(Responder)]
enum NewKeyResponder {
    Ok(String),
    #[response(status = 403)]
    Forbidden(&'static str),
}

#[post("/new_key?<role>")]
async fn new_key(
    admin_user: AdminUser,
    role: Role,
    mut db: Connection<Db>,
    _csrf: CSRFProtect,
) -> DbResult<NewKeyResponder> {
    if !(admin_user.0.role() > role) {
        return Ok(NewKeyResponder::Forbidden(
            "Can't create role higher or equal to yourself",
        ));
    }
    let new_key = Uuid::new_v4().simple().to_string();
    query!(
        "INSERT INTO promote_keys(key,new_role) VALUES ($1, $2)",
        new_key,
        role.to_i32()
    )
    .execute(&mut **db)
    .await?;
    return Ok(NewKeyResponder::Ok(new_key));
}

#[derive(Responder)]
enum DemoteResponder {
    Ok(&'static str),
    #[response(status = 403)]
    Forbidden(&'static str),
}

#[post("/demote?<id>")]
async fn demote(
    user: User,
    id: i32,
    _csrf: CSRFProtect,
    mut db: Connection<Db>,
) -> DbResult<DemoteResponder> {
    let result = query!(
        "UPDATE users SET role=0 WHERE id=$1 AND role < $2",
        id,
        user.role().to_i32()
    )
    .execute(&mut **db)
    .await?;

    if result.rows_affected() == 0 {
        Ok(DemoteResponder::Forbidden("unauthorized"))
    } else {
        Ok(DemoteResponder::Ok("ok"))
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("api", |r| async {
        r.mount("/api", routes![new_key, role, demote])
    })
}
