mod ids;
mod names;
pub mod role;
mod session;
mod userinfo;
pub use ids::*;
pub use names::*;
use rocket::fairing::AdHoc;
pub use session::*;
pub use userinfo::*;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("users", |r| async {
        r.attach(names::stage()).attach(session::stage())
    })
}
