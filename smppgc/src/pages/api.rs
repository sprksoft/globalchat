use rocket_db_pools::Connection;
use sqlx::query;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    chat::Chat,
    db::{Db, DbResult},
    profanity::{ProfRuleset, RuleLintSet, RulesetChanges, RulesetError},
    users::{role::Role, AdminUser, User},
};
use profanity::ProfanityFilter;
use rocket::{
    fairing::AdHoc, get, post, response::Debug, routes, serde::json::Json, Responder, State,
};

#[derive(Responder)]
enum SyncResponse {
    #[response(status = 200)]
    Ok(Json<RuleLintSet>),
    #[response(status = 422)]
    Error(Json<RuleLintSet>),
}

#[post("/prof/ruleset", data = "<changes>")]
async fn sync_ruleset(
    _ses: AdminUser,
    changes: Json<RulesetChanges>,
    global_ruleset: &State<Mutex<ProfRuleset>>,
    global_filter: &State<RwLock<ProfanityFilter>>,
    chat: &State<Chat>,
) -> Result<SyncResponse, Debug<RulesetError>> {
    let lock = global_ruleset.lock().await;
    let mut ruleset: ProfRuleset = lock.clone();
    drop(lock);

    ruleset.apply(changes.into_inner());
    ruleset.sort();
    let mut filter = ruleset.build_filter();
    let lints = ruleset.lint(&filter);

    if lints.has_errors() {
        for rep_lint in lints.rep_lints() {
            ruleset.disable_rep(rep_lint.affected_rule);
        }
        for match_lint in lints.match_lints() {
            ruleset.disable_match(match_lint.affected_rule);
        }
        filter = ruleset.build_filter();
    }
    {
        let mut lock = global_ruleset.lock().await;
        lock.replace_from(ruleset.clone());
        lock.save()?;
    }
    chat.run_filter(&filter).await;
    {
        let mut lock = global_filter.write().await;
        *lock = filter;
    }

    Ok(SyncResponse::Ok(Json(RuleLintSet {
        lints,
        rules: ruleset,
    })))
}

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
enum DemoteResponder {
    Ok(&'static str),
    #[response(status = 401)]
    Unauthorized(&'static str),
}

#[post("/new_key?<role>")]
async fn new_key(_admin: AdminUser, role: Role, mut db: Connection<Db>) -> DbResult<String> {
    let new_key = Uuid::new_v4().simple().to_string();
    query!(
        "INSERT INTO promote_keys(key,new_role) VALUES ($1, $2)",
        new_key,
        role.to_i32()
    )
    .execute(&mut **db)
    .await?;
    return Ok(new_key);
}

#[post("/demote?<id>")]
async fn demote(user: User, id: i32, mut db: Connection<Db>) -> DbResult<DemoteResponder> {
    let result = query!(
        "UPDATE users SET role=0 WHERE id=$1 AND role < $2",
        id,
        user.role().to_i32()
    )
    .execute(&mut **db)
    .await?;

    if result.rows_affected() == 0 {
        Ok(DemoteResponder::Unauthorized("unauthorized"))
    } else {
        Ok(DemoteResponder::Ok("ok"))
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("api", |r| async {
        r.mount("/api", routes![new_key, role, sync_ruleset, demote])
    })
}
