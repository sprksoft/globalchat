use rocket::fairing::AdHoc;

pub mod message_limiter_service;
pub mod name_service;
pub mod user_service;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("services", |r| async {
        r.attach(message_limiter_service::stage())
    })
}
