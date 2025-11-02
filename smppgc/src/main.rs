#![allow(dead_code)]
use csrf::CSRFProtect;
use rocket::catch;
use rocket::catchers;
use rocket::get;
use rocket::launch;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket_dyn_templates::context;
use rocket_dyn_templates::Template;
use utils::static_routing;
use utils::AllowSmFrame;

mod chat;
mod csrf;
mod db;
mod disclaimer;
mod metrics;
mod oauth;
mod pages;
mod ratelimit;
mod themes;
mod users;
mod utils;
mod version_int;
mod wf;
mod wsprotocol;

pub use version_int::*;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ChatConfig {
    pub max_stored_messages: usize,
    pub max_users: u16,
    pub max_ro_users: usize,
}

#[catch(500)]
fn internal_server_error() -> AllowSmFrame<Template> {
    let theme = themes::DEFAULT_THEME.clone();
    AllowSmFrame(Template::render(
        "pages/error_page",
        context! { title: "500 Internal Server Error", error: "Oei! Er ging iets mis.", theme_css: theme.css(), internal:"500" },
    ))
}

#[catch(403)]
fn forbidden() -> AllowSmFrame<Template> {
    let theme = themes::DEFAULT_THEME.clone();
    AllowSmFrame(Template::render(
        "pages/error_page",
        context! { title: "403 Forbidden", error: "Je hebt geen toegang of je pagina is oud.", theme_css: theme.css(), internal:"403" },
    ))
}

#[get("/err_test")]
fn err_test() -> rocket::response::Debug<()> {
    rocket::response::Debug(())
}

#[get("/csrf_protect_test")]
fn csrf_test(_csrf: CSRFProtect) -> &'static str {
    "200 ok"
}

#[get("/version")]
fn server_version(conf: &rocket::Config) -> String {
    let profile = conf.profile.to_string();
    let ver_str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));
    format!(
        "{} ({}) debug_assertions: {} ",
        ver_str,
        profile,
        cfg!(debug_assertions)
    )
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .register("/", catchers![internal_server_error, forbidden])
        .mount("/", routes![server_version, err_test, csrf_test])
        .attach(metrics::stage())
        .attach(db::stage())
        .attach(static_routing::stage())
        .attach(users::stage())
        .attach(pages::stage())
        .attach(wf::stage())
        .attach(oauth::stage())
        .attach(chat::stage())
        .attach(csrf::stage())
}
