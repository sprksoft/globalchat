use std::{
    ops::Deref,
    sync::{Mutex, RwLock},
};

use profanity::{Flags, ProfanityFilter, RuleFlags, Token};
use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar},
    response::{Debug, Redirect},
    routes,
    serde::json::Json,
    time::{Duration, OffsetDateTime},
    Responder, State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    auth::{self, AuthConfig, GcAdmin, GcRole},
    profanity::{LintSet, ProfRuleset},
    themes::Theme,
};

mod api;

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
        "gcadmin/prof",
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

#[get("/")]
fn index(theme: Theme, role: GcRole) -> Template {
    Template::render(
        "gcadmin/gcadmin",
        context! {
            role,
            theme_css:theme.css()
        },
    )
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
        None => Ok(BecomeResponse::Err("Invalid key")),
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
                Ok(BecomeResponse::Redirect(Redirect::temporary("../admin")))
            }
        }
    }
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
    AdHoc::on_ignite("admin pages", |r| async {
        r.mount("/admin", routes![index, prof, become_role, role])
            .mount("/admin/api", routes![])
    })
}
