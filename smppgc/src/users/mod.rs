mod ids;
pub mod role;
mod user;
mod usermgr;
pub use ids::*;
use rocket::{fairing::AdHoc, serde::Deserialize};
pub use user::*;
pub use usermgr::*;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UserConfig {
    pub max_username_len: usize,

    pub max_claimed_names: usize,
    pub max_name_retention: usize,
    pub max_session_age: usize,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("users", |r| async {
        r.attach(AdHoc::config::<UserConfig>())
    })
}
