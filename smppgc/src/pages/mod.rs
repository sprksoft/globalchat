use rocket::{fairing::AdHoc, get, http::CookieJar, response::Redirect, routes, State};
use rocket_dyn_templates::{context, Template};

use crate::{
    chat::MessageLimiter,
    disclaimer::DisclaimerVer,
    oauth::{self, OAuth},
    themes::Theme,
    users::{role::Role, User, UserConfig},
    utils::AllowSmFrame,
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
) -> AllowSmFrame<Template> {
    oauth::set_continue_url_cookie(&cookiejar, redirect);
    AllowSmFrame(Template::render(
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

#[get("/login_complete")]
fn login_complete(theme: Theme) -> Template {
    Template::render(
        "pages/login_complete",
        context! {
            theme_css: theme.css(),
        },
    )
}

#[get("/")]
fn home(theme: Theme, user: Option<User>) -> AllowSmFrame<Template> {
    let logged_in = user.is_some();
    let role = user.map(|u| u.role()).unwrap_or(Role::User);
    AllowSmFrame(Template::render(
        "pages/home",
        context! {
            role,
            is_admin: role >= Role::Admin,
            logged_in,
            theme_css:theme.css()
        },
    ))
}

#[get("/v1")]
fn chat(
    theme: Theme,
    user: User,
    message_limiter: &State<MessageLimiter>,
    user_config: &State<UserConfig>,
) -> AllowSmFrame<Template> {
    let (min_message_len, max_message_len) = message_limiter.message_size_range();
    AllowSmFrame(Template::render(
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
fn ro_chat(theme: Theme) -> AllowSmFrame<Template> {
    AllowSmFrame(Template::render(
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
fn chat_noses(_theme: Theme) -> Redirect {
    Redirect::to("/login?redirect=/v1")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("templates", |r| async {
        let profile_name = r.figment().profile().to_string();
        r.mount(
            "/",
            routes![
                login,
                login_complete,
                chat,
                ro_chat,
                chat_noses,
                prof::prof,
                home
            ],
        )
        .attach(promote::stage())
        .attach(api::stage())
        .attach(Template::custom(move |engines| {
            templating::setup(&mut engines.tera, profile_name.clone());
        }))
    })
}
