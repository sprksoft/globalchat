use std::{
    ops::Deref,
    sync::{Mutex, RwLock},
};

use profanity::{Flags, ProfanityFilter, RuleFlags, Token};
use rocket::{
    fairing::AdHoc,
    get, post,
    response::{Debug, Redirect},
    routes,
    serde::json::Json,
    Responder, State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    chat::Chat,
    profanity::{LintSet, ProfRuleset, RulesetError},
    themes::Theme,
};

#[get("/prof")]
fn prof(
    theme: Theme,
    ruleset: &State<Mutex<ProfRuleset>>,
    filter: &State<RwLock<ProfanityFilter>>,
) -> Template {
    let ruleset = ruleset.lock().expect("Prof ruleset lock poisoned");
    let lints = { ruleset.lint(&filter.read().expect("profanity filter lock poisoned")) };
    Template::render(
        "admin/prof",
        context! {theme_css:theme.css(), flagsinfo: RuleFlags::flags_info(), ruleset: ruleset.deref(), tokeninfo: Token::token_info(), lints:lints},
    )
}

#[derive(rocket::serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct RulesetSaveError {
    message: &'static str,
    lints: LintSet,
}

#[derive(Responder)]
enum RulesetWriteResponse {
    #[response(status = 422)]
    Error(Json<RulesetSaveError>),

    #[response(status = 200)]
    Ok(Json<LintSet>),
}

#[post("/prof/ruleset", data = "<ruleset>")]
async fn prof_ruleset_save(
    ruleset: Json<ProfRuleset>,
    global_ruleset: &State<Mutex<ProfRuleset>>,
    global_filter: &State<RwLock<ProfanityFilter>>,
    chat: &State<Chat>,
) -> Result<RulesetWriteResponse, Debug<RulesetError>> {
    let filter = ruleset.build_filter();
    let lints = ruleset.lint(&filter);

    if lints.has_errors() {
        Ok(RulesetWriteResponse::Error(Json(RulesetSaveError {
            message: "Ruleset contains errors",
            lints,
        })))
    } else {
        {
            let mut lock = global_ruleset
                .lock()
                .expect("Profanity ruleset lock poisoned");
            lock.replace_from(ruleset.0);
            lock.save()?;
        }
        chat.run_filter(&filter).await;
        {
            let mut lock = global_filter
                .write()
                .expect("profanity filter lock poisoned");
            *lock = filter;
        }

        Ok(RulesetWriteResponse::Ok(Json(lints)))
    }
}

#[get("/")]
fn index() -> Redirect {
    Redirect::permanent("/admin/prof")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("admin pages", |r| async {
        r.mount("/admin", routes![index, prof, prof_ruleset_save])
    })
}
