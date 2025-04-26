use std::time::SystemTime;

use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar, SameSite},
    response::{self, Redirect, Responder},
    routes,
    time::{Duration, OffsetDateTime},
    State,
};
use rocket_dyn_templates::{context, tera, Template};

use crate::{
    auth::{GcMod, GcRole},
    disclaimer::DisclaimerVer,
    themes::Theme,
    users::{Session, UserConfig},
    utils::CSPFrameAncestors,
    MessageConfig,
};

mod api;
mod prof;
mod promote;
mod templating;

#[derive(Responder)]
enum GcPageResponder {
    #[response(status = 200)]
    Ok {
        inner: Template,
        csp: CSPFrameAncestors<'static>,
    },
    Redirect(Redirect),
}

#[get("/login")]
fn login(
    theme: Theme,
    cookiejar: &CookieJar<'_>,
    accepted_disclaimer: DisclaimerVer,
) -> GcPageResponder {
    GcPageResponder::Ok {
        inner: Template::render(
            "pages/login",
            context! {
                theme_css:theme.css(),
                accepted_disclaimer:accepted_disclaimer,
                disclaimer_ver:DisclaimerVer::LATEST
            },
        ),
        csp: CSPFrameAncestors::SMARTSCHOOL_PLAT,
    }
}

#[get("/")]
fn home(theme: Theme, role: GcRole) -> Template {
    Template::render(
        "pages/home",
        context! {
            role,
            theme_css:theme.css()
        },
    )
}

#[get("/chat")]
fn chat(
    theme: Theme,
    session: Session,
    message_config: &State<MessageConfig>,
    user_config: &State<UserConfig>,
    cookiejar: &CookieJar<'_>,
) -> GcPageResponder {
    let theme_string = serde_json::to_string(&theme).expect("Failed to convert theme to json");
    cookiejar.add(
        Cookie::build(("smpptheme", theme_string))
            .same_site(SameSite::None)
            .expires(OffsetDateTime::now_utc() + Duration::hours(100_000)),
    );

    let user_info = session.user_info;

    GcPageResponder::Ok {
        inner: Template::render(
            "pages/chat",
            context! (theme_css:theme.css(),
            irl_name: &user_info.irl_name,
            is_mod: user_info.role.has_mod(),
            max_username_len: user_config.max_username_len,
            max_message_len: message_config.max_message_len,
            min_message_len: message_config.min_message_len),
        ),
        csp: CSPFrameAncestors::SMARTSCHOOL_PLAT,
    }
}

#[get("/chat", rank = 0)]
fn chat_noses() -> Redirect {
    Redirect::to("/login")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("templates", |r| async {
        r.mount(
            "/",
            routes![login, chat, chat_noses, prof::prof, home, promote::promote],
        )
        .attach(api::stage())
        .attach(Template::custom(move |engines| {
            templating::setup(&mut engines.tera);
        }))
    })
}
