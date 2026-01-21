#![allow(dead_code)]

use csrf::CSRFProtect;
use rocket::catch;
use rocket::catchers;
use rocket::get;
use rocket::http::Status;
use rocket::launch;
use rocket::routes;
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
        context! { title: "403 Forbidden", error: "De pagina is te oud of je hebt momenteel geen toegang", theme_css: theme.css(), internal:"403" },
    ))
}

#[catch(503)]
fn service_unavailible() -> AllowSmFrame<Template> {
    let theme = themes::DEFAULT_THEME.clone();
    AllowSmFrame(Template::render(
        "pages/error_page",
        context! { title: "503 Service Unavailable", error:"De chat is tijdelijk overbelast waarschijnlijk door een bug. Sorry voor het ongemak", theme_css: theme.css(), internal:"503"},
    ))
}

#[catch(404)]
fn not_found() -> AllowSmFrame<Template> {
    let theme = themes::DEFAULT_THEME.clone();
    AllowSmFrame(Template::render(
        "pages/error_page",
        context! { title: "404 Not Found", error:"What?!", theme_css: theme.css(), internal:"404"},
    ))
}

#[get("/err_test/<code>")]
fn err_test(code: u16) -> Status {
    Status::new(code)
}

#[get("/csrf_protect_test")]
fn csrf_test(_csrf: CSRFProtect) -> &'static str {
    "200 ok (If you got here without a internal link CSRF protection has failed)"
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
        .register(
            "/",
            catchers![
                internal_server_error,
                forbidden,
                service_unavailible,
                not_found
            ],
        )
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
