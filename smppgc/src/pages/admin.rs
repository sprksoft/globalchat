use rocket::{fairing::AdHoc, get, response::Redirect, routes};
use rocket_dyn_templates::{context, Template};

#[get("/prof")]
fn prof() -> Template {
    Template::render("admin/prof", context! {})
}

#[get("/")]
fn index() -> Redirect {
    Redirect::permanent("/admin/prof")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("admin pages", |r| async {
        r.mount("/admin", routes![index, prof])
    })
}
