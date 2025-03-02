use profanity::RuleFlags;
use rocket::{fairing::AdHoc, get, response::Redirect, routes, State};
use rocket_dyn_templates::{context, Template};

use crate::{profanity::ProfFilter, themes::Theme};

#[get("/prof")]
fn prof(theme: Theme, filter: &State<ProfFilter>) -> Template {
    Template::render(
        "admin/prof",
        context! {theme_css:theme.css(), flagsinfo: RuleFlags::flags_info()},
    )
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
