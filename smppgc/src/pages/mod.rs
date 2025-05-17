use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar, SameSite},
    response::{Redirect, Responder},
    routes,
    time::{Duration, OffsetDateTime},
    State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    chat::MessageLimiter,
    disclaimer::DisclaimerVer,
    themes::Theme,
    users::{role::Role, Session, UserConfig},
    utils::{AllowSmIFrame, CSPFrameAncestors},
};

mod api;
mod prof;
mod promote;
mod templating;

#[get("/login")]
fn login(theme: Theme, accepted_disclaimer: DisclaimerVer) -> AllowSmIFrame<Template> {
    AllowSmIFrame(Template::render(
        "pages/login",
        context! {
            theme_css:theme.css(),
            accepted_disclaimer:accepted_disclaimer,
            disclaimer_ver:DisclaimerVer::LATEST
        },
    ))
}

#[get("/")]
fn home(theme: Theme, ses: Option<Session>) -> AllowSmIFrame<Template> {
    let logged_in = ses.is_some();
    let role = ses.map(|s| s.user_info.role).unwrap_or(Role::User);
    AllowSmIFrame(Template::render(
        "pages/home",
        context! {
            role,
            logged_in,
            theme_css:theme.css()
        },
    ))
}

#[get("/chat")]
fn chat(
    theme: Theme,
    session: Session,
    message_limiter: &State<MessageLimiter>,
    user_config: &State<UserConfig>,
    cookiejar: &CookieJar<'_>,
) -> AllowSmIFrame<Template> {
    let theme_string = serde_json::to_string(&theme).expect("Failed to convert theme to json");
    cookiejar.add(
        Cookie::build(("smpptheme", theme_string))
            .same_site(SameSite::None)
            .expires(OffsetDateTime::now_utc() + Duration::hours(100_000)),
    );

    let user_info = session.user_info;
    let (max_message_len, min_message_len) = message_limiter.message_size_range();
    AllowSmIFrame(Template::render(
        "pages/chat",
        context! (theme_css:theme.css(),
            irl_name: &user_info.irl_name,
            is_mod: user_info.role.is_mod(),
            max_username_len: user_config.max_username_len,
            max_message_len: max_message_len,
            min_message_len: min_message_len),
    ))
}

#[get("/chat", rank = 0)]
fn chat_noses() -> Redirect {
    Redirect::to("/login")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("templates", |r| async {
        r.mount(
            "/",
            routes![
                login,
                chat,
                chat_noses,
                prof::prof,
                home,
                promote::promote,
                promote::mods
            ],
        )
        .attach(api::stage())
        .attach(Template::custom(move |engines| {
            templating::setup(&mut engines.tera);
        }))
    })
}
