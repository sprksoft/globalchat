use rocket::{fairing::AdHoc, get, response::Redirect, routes, State};
use rocket_dyn_templates::{context, Template};

use crate::{
    chat::MessageLimiter,
    themes::Theme,
    users::{role::Role, User, UserConfig},
    utils::{AllowSmFrame, CatchForward},
};

mod api;
mod login;
mod promote;
mod stats;
mod templating;
mod wf;

pub use login::*;

#[get("/")]
fn home(theme: Theme, user: CatchForward<User>) -> AllowSmFrame<Template> {
    let logged_in = user.is_success();
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

#[get("/v1?<glass>")]
fn chat(
    theme: Theme,
    user: User,
    message_limiter: &State<MessageLimiter>,
    user_config: &State<UserConfig>,
    glass: bool,
) -> AllowSmFrame<Template> {
    let (min_message_len, max_message_len) = message_limiter.message_size_range();
    AllowSmFrame(Template::render(
        "pages/chat",
        context! (theme_css:theme.css(),
            glass: glass,
            readonly: false,
            irl_name: user.irl_name(),
            role: user.role().to_i32(),
            is_mod: user.role().is_mod(),
            max_username_len: user_config.max_username_len,
            max_message_len: max_message_len,
            min_message_len: min_message_len),
    ))
}

#[get("/rochat?<glass>")]
fn ro_chat(glass: bool, theme: Theme) -> AllowSmFrame<Template> {
    AllowSmFrame(Template::render(
        "pages/chat",
        context! {
            glass:glass,
            theme_css:theme.css(),
            readonly: true,
            irl_name: "",
            role: Role::User.to_i32(),
            is_mod: false,
            max_username_len: 0,
            max_message_len: 0,
            min_message_len: 0
        },
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
                login::login,
                login::login_complete,
                chat,
                ro_chat,
                chat_noses,
                home
            ],
        )
        .attach(promote::stage())
        .attach(api::stage())
        .attach(stats::stage())
        .attach(wf::stage())
        .attach(Template::custom(move |engines| {
            templating::setup(&mut engines.tera, profile_name.clone());
        }))
    })
}
