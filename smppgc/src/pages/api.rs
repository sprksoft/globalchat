use std::sync::Arc;

use rocket_db_pools::Connection;
use sqlx::query;
use uuid::Uuid;

use crate::{
    chat::Chat,
    csrf::CSRFProtect,
    db::{Db, DbResult},
    users::{role::Role, AdminUser, ModUser, User},
    utils::CatchForward,
    wf::Filter,
};
use rocket::{fairing::AdHoc, get, post, routes, Responder, State};

#[get("/role")]
fn role(user: CatchForward<User>) -> &'static str {
    match user.map(|u| u.role()) {
        CatchForward::Forward(_) => "not logged in",
        CatchForward::Success(Role::Owner) => "owner",
        CatchForward::Success(Role::Mod) => "mod",
        CatchForward::Success(Role::Admin) => "admin",
        CatchForward::Success(Role::User) => "user",
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

#[post("/wf/<word>/markgood")]
async fn wf_markgood(
    _mod: ModUser,
    word: &str,
    chat: &State<Chat>,
    filter: &State<Arc<Filter>>,
    _csrf: CSRFProtect,
) {
    filter.mark_word(word, true).await;
    filter.rerun(&chat).await;
}
#[post("/wf/<word>/markbad")]
async fn wf_markbad(
    _mod: ModUser,
    word: &str,
    chat: &State<Chat>,
    filter: &State<Arc<Filter>>,
    _csrf: CSRFProtect,
) {
    filter.mark_word(word, false).await;
    filter.rerun(&chat).await;
}

#[post("/wf/<word>/lock?<reason>")]
async fn wf_lockword(
    word: &str,
    reason: &str,
    _mod: AdminUser,
    filter: &State<Arc<Filter>>,
    _csrf: CSRFProtect,
) {
    filter.lock_word(word, reason.into()).await;
}

#[post("/wf/<word>/unlock")]
async fn wf_unlockword(
    word: &str,
    _mod: AdminUser,
    filter: &State<Arc<Filter>>,
    _csrf: CSRFProtect,
) {
    filter.unlock_word(word).await;
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("api", |r| async {
        r.mount(
            "/api",
            routes![
                new_key,
                role,
                demote,
                wf_markbad,
                wf_markgood,
                wf_lockword,
                wf_unlockword
            ],
        )
    })
}
