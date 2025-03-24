use std::{
    ops::Deref,
    sync::{Mutex, RwLock},
};

use profanity::{Flags, ProfanityFilter, RuleFlags, Token};
use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar},
    post,
    response::{Debug, Redirect},
    routes,
    serde::json::Json,
    time::{Duration, OffsetDateTime},
    Responder, State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    auth::{self, AuthConfig, GcAdmin, GcAuth},
    chat::Chat,
    profanity::{LintSet, ProfRuleset, RulesetError},
    themes::Theme,
};

#[get("/prof")]
fn prof(
    _gcadmin: GcAdmin,
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
struct RuleLintSet {
    lints: LintSet,
    rules: ProfRuleset,
}

#[derive(Responder)]
enum RulesetWriteResponse {
    #[response(status = 422)]
    Error(Json<RuleLintSet>),

    #[response(status = 200)]
    Ok(Json<RuleLintSet>),
}

#[post("/prof/ruleset", data = "<ruleset>")]
async fn prof_ruleset_save(
    _gcadmin: GcAdmin,
    mut ruleset: Json<ProfRuleset>,
    global_ruleset: &State<Mutex<ProfRuleset>>,
    global_filter: &State<RwLock<ProfanityFilter>>,
    chat: &State<Chat>,
) -> Result<RulesetWriteResponse, Debug<RulesetError>> {
    let global_ruleset = global_ruleset.lock().expect("Global ruleset poisoned");
    ruleset.merge(&mut global_ruleset);
    ruleset.sort();
    let filter = ruleset.build_filter();
    let lints = ruleset.lint(&filter);
    let rule_lint_set = RuleLintSet {
        lints,
        rules: ruleset.into_inner(),
    };

    if lints.has_errors() {
        Ok(RulesetWriteResponse::Error(Json(rule_lint_set)))
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
fn index(_gcadmin: GcAdmin) -> Redirect {
    Redirect::permanent("prof")
}

#[derive(Responder)]
enum BecomeResponse {
    Redirect(Redirect),
    #[response(status = 400)]
    Err(&'static str),
    #[response(status = 200)]
    Ok(&'static str),
}

#[get("/become?<key>&<no_redirect>")]
fn become_role(
    key: &str,
    cookie_jar: &CookieJar,
    auth_config: &State<AuthConfig>,
    no_redirect: bool,
) -> Result<BecomeResponse, Debug<std::io::Error>> {
    match auth::get_role_from_key(key, &auth_config.auth_file)? {
        GcAuth::InvalidKey => Ok(BecomeResponse::Err("Invalid key")),
        _ => {
            cookie_jar.add(
                Cookie::build(("SMPPGC-Auth", key.to_string()))
                    .http_only(true)
                    .secure(true)
                    .expires(OffsetDateTime::now_utc() + Duration::hours(100_000)),
            );
            if no_redirect {
                Ok(BecomeResponse::Ok("ok"))
            } else {
                Ok(BecomeResponse::Redirect(Redirect::temporary("../")))
            }
        }
    }
}

#[get("/role")]
fn role(auth: Option<GcAuth>) -> &'static str {
    match auth {
        None => "normal user",
        Some(GcAuth::Mod) => "mod",
        Some(GcAuth::Admin) => "admin",
        Some(GcAuth::InvalidKey) => "normal user with invalid key",
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("admin pages", |r| async {
        r.mount(
            "/admin",
            routes![index, prof, prof_ruleset_save, become_role, role],
        )
    })
}
