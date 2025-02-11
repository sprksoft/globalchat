use rocket::{fairing::AdHoc, get, routes};

#[get("/")]
fn admin() {}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("admin pages", |r| async {
        r.mount("/admin", routes![admin])
    })
}
