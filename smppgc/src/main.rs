use chat::Chat;
use lmetrics::metrics;
use lmetrics::LMetrics;
use rocket::catch;
use rocket::catchers;
use rocket::get;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket::Responder;
use rocket::{fairing::AdHoc, launch};
use rocket_dyn_templates::context;
use rocket_dyn_templates::Template;
use themes::Theme;
use utils::static_routing;
use utils::CSPFrameAncestors;

mod auth;
mod chat;
mod db;
mod disclaimer;
mod oauth;
mod pages;
mod profanity;
mod ratelimit;
mod snowflake;
mod socket;
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
}

pub type MessageLen = u16;
pub type BadWordLen = u8;

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct MessageConfig {
    pub small_message_len: usize,
    pub max_message_len: MessageLen,
    pub min_message_len: MessageLen,
    pub large_message_penalty: u32,

    pub max_same_message_streak: u32,
    pub same_message_penalty: u32,
}
metrics! {
    pub counter total_500_responses("Total amount of 500 responses");
}

#[derive(Responder)]
struct ErrorResponder {
    inner: Template,
    csp: CSPFrameAncestors<'static>,
}

#[catch(500)]
fn internal_server_error() -> ErrorResponder {
    let theme = themes::DEFAULT_THEME.clone();
    ErrorResponder {
        inner: Template::render(
            "error_page",
            context! { title: "500 Internal Server Error", error: "Oei! Er ging iets mis.", theme_css: theme.css(), internal:"500",},
        ),
        csp: CSPFrameAncestors::SMARTSCHOOL_PLAT,
    }
}

#[get("/err_test")]
fn err_test() -> rocket::response::Debug<()> {
    rocket::response::Debug(())
}

#[get("/version")]
fn server_version() -> String {
    let ver_str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

    format!("{} debug_assertions: {} ", ver_str, cfg!(debug_assertions),)
}

#[launch]
fn rocket() -> _ {
    let mut metrics = LMetrics::new(&[
        &crate::total_500_responses::METRIC,
        &oauth::total_started_oauth_flows::METRIC,
        &oauth::total_failed_oauth_flows::METRIC,
        &oauth::total_logins::METRIC,
        &static_routing::static_req_total::METRIC,
        &chat::joined_total::METRIC,
        &chat::left_total::METRIC,
        &chat::history_events_lost_total::METRIC,
        &socket::messages_total::METRIC,
        &socket::messages_blocked::METRIC,
        &lmetrics::http_errors_total::METRIC,
        &lmetrics::http_req_total::METRIC,
    ]);
    rocket::build()
        .register("/", catchers![internal_server_error])
        .mount("/", routes![server_version, err_test])
        .mount("/metrics", metrics)
        .attach(db::stage())
        .attach(AdHoc::config::<MessageConfig>())
        .attach(ratelimit::stage())
        .attach(static_routing::stage())
        .attach(pages::stage())
        .attach(users::stage())
        .attach(auth::stage())
        .attach(oauth::stage())
        .attach(AdHoc::on_ignite("chat", |r| async {
            let config = r
                .figment()
                .extract::<ChatConfig>()
                .expect("No chat config found");

            r.mount("/", routes![socket::chat_socket])
                .manage(Chat::new(config))
        }))
        .attach(profanity::stage())
}
