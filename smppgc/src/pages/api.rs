use tokio::sync::{Mutex, RwLock};

use crate::{
    auth::{GcAdmin, GcRole},
    chat::Chat,
    profanity::{ProfRuleset, RuleLintSet, RulesetChanges, RulesetError},
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
    _gcadmin: GcAdmin,
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
fn role(auth: Option<GcRole>) -> &'static str {
    match auth {
        None => "invalid key",
        Some(GcRole::Mod) => "mod",
        Some(GcRole::Admin) => "admin",
        Some(GcRole::User) => "normal user",
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("api", |r| async {
        r.mount("/api", routes![role, sync_ruleset])
    })
}
