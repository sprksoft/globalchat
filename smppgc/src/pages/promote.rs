use profanity::{Flags, RuleFlags, Token};
use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar},
    response::{Debug, Redirect},
    routes,
    time::{Duration, OffsetDateTime},
    Responder, State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    auth::{self, AuthConfig, GcAdmin, GcRole},
    themes::Theme,
};

#[derive(Responder)]
enum PromoteResponse {
    Redirect(Redirect),
    #[response(status = 400)]
    Err(&'static str),
    #[response(status = 200)]
    Ok(&'static str),
}

#[get("/promote?<key>&<no_redirect>")]
pub fn promote(
    key: &str,
    cookie_jar: &CookieJar,
    auth_config: &State<AuthConfig>,
    no_redirect: bool,
) -> Result<PromoteResponse, Debug<std::io::Error>> {
    match auth::get_role_from_key(key, &auth_config.auth_file)? {
        None => Ok(PromoteResponse::Err("Invalid key")),
        _ => {
            cookie_jar.add(
                Cookie::build(("SMPPGC-Auth", key.to_string()))
                    .http_only(true)
                    .secure(true)
                    .expires(OffsetDateTime::now_utc() + Duration::hours(100_000)),
            );
            if no_redirect {
                Ok(PromoteResponse::Ok("ok"))
            } else {
                Ok(PromoteResponse::Redirect(Redirect::temporary("/home")))
            }
        }
    }
}
