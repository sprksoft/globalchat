use chat::Chat;
use lmetrics::LMetrics;
use rocket::get;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket::{fairing::AdHoc, launch};
use utils::static_routing;

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

#[get("/version")]
fn server_version() -> String {
    let ver_str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

    format!("{} debug_assertions: {} ", ver_str, cfg!(debug_assertions),)
}

#[launch]
fn rocket() -> _ {
    let mut metrics = LMetrics::new(&[
        &static_routing::static_req_total::METRIC,
        &chat::joined_total::METRIC,
        &chat::left_total::METRIC,
        &chat::history_events_lost_total::METRIC,
        &socket::messages_total::METRIC,
        &socket::messages_blocked::METRIC,
        &socket::new_users::METRIC,
        &lmetrics::http_errors_total::METRIC,
        &lmetrics::http_req_total::METRIC,
    ]);
    rocket::build()
        .mount("/", routes![server_version])
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
