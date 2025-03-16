mod names;
mod userid;
mod userinfo;
pub use names::*;
use rocket::fairing::AdHoc;
pub use userid::*;
pub use userinfo::*;

pub fn stage() -> AdHoc {
    names::stage()
}
