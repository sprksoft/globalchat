use std::ops::Deref;

use chat::Chat;
use lmetrics::LMetrics;
use rocket::response::Redirect;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket::{fairing::AdHoc, launch};
use rocket::{get, State};
use utils::static_routing;

#[cfg(test)]
mod test;

pub mod chat;
mod csp;
mod debug;
mod mesg_filter;
pub mod names;
mod pages;
pub mod profanity;
pub mod ratelimit;
pub mod socket;
mod userinfo;
mod utils;
mod wsprotocol;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ChatConfig {
    pub max_stored_messages: usize,
    pub max_users: u16,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct MessageConfig {
    pub small_message_len: usize,
    pub max_message_len: usize,
    pub large_message_penalty: u32,

    pub max_same_message_streak: u32,
    pub same_message_penalty: u32,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct RootUrl {
    pub root_url: String,
}

impl Default for RootUrl {
    fn default() -> Self {
        Self {
            root_url: String::new(),
        }
    }
}
impl Deref for RootUrl {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.root_url
    }
}

#[get("/version")]
fn server_version(debug: &State<debug::Debug>) -> String {
    let ver_str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

    format!(
        "{} debug_assertions: {} debug: {} ",
        ver_str,
        debug.debug,
        cfg!(debug_assertions)
    )
}

#[get("/")]
fn index(debug: &State<debug::Debug>) -> Redirect {
    if debug.debug {
        Redirect::permanent("/v1")
    } else {
        Redirect::permanent("/smpp/gc/v1")
    }
}

#[launch]
fn rocket() -> _ {
    let mut metrics = LMetrics::new(&[
        &static_routing::static_req_total::METRIC,
        &chat::joined_total::METRIC,
        &chat::left_total::METRIC,
        &chat::client_left_events_lost_total::METRIC,
        &chat::history_messages_lost_total::METRIC,
        &socket::messages_total::METRIC,
        &socket::profanity_messages_total::METRIC,
        &socket::messages_blocked::METRIC,
        &socket::new_users::METRIC,
        &lmetrics::http_errors_total::METRIC,
        &lmetrics::http_req_total::METRIC,
    ]);
    metrics.on_before_handle(|| {});
    rocket::build()
        .mount("/", routes![index, server_version])
        .mount("/metrics", metrics)
        .attach(AdHoc::config::<MessageConfig>())
        .attach(ratelimit::stage())
        .attach(static_routing::stage())
        .attach(pages::stage())
        .attach(names::stage())
        .attach(AdHoc::config::<RootUrl>())
        .attach(AdHoc::on_ignite("chat", |r| async {
            let config = r
                .figment()
                .extract::<ChatConfig>()
                .expect("No chat config found");

            r.mount("/", routes![socket::socket_v1])
                .manage(Chat::new(config))
        }))
        .attach(profanity::stage())
        .attach(debug::stage())
}
