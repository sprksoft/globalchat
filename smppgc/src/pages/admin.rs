use std::sync::Mutex;

use profanity::{Flags, RuleFlags, Token};
use rocket::{fairing::AdHoc, get, response::Redirect, routes, State};
use rocket_dyn_templates::{context, Template};

use crate::{profanity::ProfRuleset, themes::Theme};

#[get("/prof")]
fn prof(theme: Theme, ruleset: &State<Mutex<ProfRuleset>>) -> Template {
    let ruleset = ruleset.lock().expect("Prof ruleset lock poisoned");
    Template::render(
        "admin/prof",
        context! {theme_css:theme.css(), flagsinfo: RuleFlags::flags_info(), rules: ruleset.rules(), tokeninfo: Token::token_info()},
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
