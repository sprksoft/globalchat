use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar, SameSite},
    response::Redirect,
    routes,
    time::{Duration, OffsetDateTime},
    State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    chat::MessageLimiter,
    disclaimer::DisclaimerVer,
    oauth::{self, OAuth},
    themes::Theme,
    users::{role::Role, User, UserConfig},
    utils::AllowSmIFrame,
};

mod api;
mod prof;
mod promote;
mod templating;

#[get("/login?<redirect>")]
fn login(
    theme: Theme,
    cookiejar: &CookieJar<'_>,
    redirect: String,
    oauth: &State<OAuth>,
    accepted_disclaimer: DisclaimerVer,
) -> AllowSmIFrame<Template> {
    oauth::set_continue_url_cookie(&cookiejar, redirect);
    AllowSmIFrame(Template::render(
        "pages/login",
        context! {
            oauth_smartschool: oauth.has_provider(oauth::Provider::Smartschool),
            oauth_google: oauth.has_provider(oauth::Provider::Google),
            theme_css:theme.css(),
            accepted_disclaimer:accepted_disclaimer,
            disclaimer_ver:DisclaimerVer::LATEST
        },
    ))
}

#[get("/")]
fn home(theme: Theme, user: Option<User>) -> AllowSmIFrame<Template> {
    let logged_in = user.is_some();
    let role = user.map(|u| u.role()).unwrap_or(Role::User);
    AllowSmIFrame(Template::render(
        "pages/home",
        context! {
            role,
            is_admin: role >= Role::Admin,
            logged_in,
            theme_css:theme.css()
        },
    ))
}

fn set_cookie_theme(cookiejar: &CookieJar<'_>, theme: &Theme) {
    let theme_string = serde_json::to_string(&theme).expect("Failed to convert theme to json");
    cookiejar.add(
        Cookie::build(("smpptheme", theme_string))
            .same_site(SameSite::None)
            .expires(OffsetDateTime::now_utc() + Duration::hours(100_000)),
    );
}

#[get("/v1")]
fn chat(
    theme: Theme,
    user: User,
    message_limiter: &State<MessageLimiter>,
    user_config: &State<UserConfig>,
    cookiejar: &CookieJar<'_>,
) -> AllowSmIFrame<Template> {
    set_cookie_theme(cookiejar, &theme);

    let (min_message_len, max_message_len) = message_limiter.message_size_range();
    AllowSmIFrame(Template::render(
        "pages/chat",
        context! (theme_css:theme.css(),
            readonly: false,
            irl_name: user.irl_name(),
            is_mod: user.role().is_mod(),
            max_username_len: user_config.max_username_len,
            max_message_len: max_message_len,
            min_message_len: min_message_len),
    ))
}

#[get("/rochat")]
fn ro_chat(cookiejar: &CookieJar<'_>, theme: Theme) -> AllowSmIFrame<Template> {
    set_cookie_theme(cookiejar, &theme);

    AllowSmIFrame(Template::render(
        "pages/chat",
        context! (theme_css:theme.css(),
            readonly: true,
            irl_name: "",
            is_mod: false,
            max_username_len: 0,
            max_message_len: 0,
            min_message_len: 0),
    ))
}

#[get("/v1", rank = 0)]
fn chat_noses() -> Redirect {
    Redirect::to("/login?redirect=/v1")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("templates", |r| async {
        r.mount(
            "/",
            routes![login, chat, ro_chat, chat_noses, prof::prof, home],
        )
        .attach(promote::stage())
        .attach(api::stage())
        .attach(Template::custom(move |engines| {
            templating::setup(&mut engines.tera);
        }))
    })
}
