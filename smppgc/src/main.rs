#![allow(dead_code)]
use csrf::CSRFProtect;
use lmetrics::metrics;
use lmetrics::LMetrics;
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
mod oauth;
mod pages;
mod profanity;
mod ratelimit;
mod snowflake;
mod themes;
mod users;
mod utils;
mod version_int;
mod wsprotocol;

pub use snowflake::*;
pub use version_int::*;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ChatConfig {
    pub max_stored_messages: usize,
    pub max_users: u16,
    pub max_ro_users: usize,
}

metrics! {
    pub counter total_500_responses("Total amount of 500 responses");
}

#[catch(500)]
fn internal_server_error() -> AllowSmFrame<Template> {
    total_500_responses::inc();
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
    let metrics = LMetrics::new(&[
        &crate::total_500_responses::METRIC,
        &oauth::total_started_oauth_flows::METRIC,
        &oauth::total_failed_oauth_flows::METRIC,
        &oauth::total_logins::METRIC,
        &static_routing::static_req_total::METRIC,
        &chat::joined_total::METRIC,
        &chat::left_total::METRIC,
        &chat::ro_joined_total::METRIC,
        &chat::ro_left_total::METRIC,
        &chat::history_events_lost_total::METRIC,
        &chat::socket::messages_total::METRIC,
        &chat::socket::messages_blocked::METRIC,
        &lmetrics::http_errors_total::METRIC,
        &lmetrics::http_req_total::METRIC,
    ]);
    rocket::build()
        .register("/", catchers![internal_server_error, forbidden])
        .mount("/", routes![server_version, err_test, csrf_test])
        .mount("/metrics", metrics)
        .attach(db::stage())
        .attach(static_routing::stage())
        .attach(users::stage())
        .attach(pages::stage())
        .attach(oauth::stage())
        .attach(profanity::stage())
        .attach(chat::stage())
        .attach(csrf::stage())
        .attach(csrf::stage())
}
