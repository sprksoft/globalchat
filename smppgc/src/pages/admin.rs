use std::sync::{Mutex, RwLock};

use profanity::{Flags, ProfanityFilter, RuleFlags, Token};
use rocket::{
    fairing::AdHoc,
    get, post,
    response::{self, Redirect},
    routes,
    serde::json::Json,
    Responder, State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    profanity::{LintImportance, ProfRuleset, Rule, RulesetLint},
    themes::Theme,
};

#[get("/prof")]
fn prof(
    theme: Theme,
    ruleset: &State<Mutex<ProfRuleset>>,
    filter: &State<RwLock<ProfanityFilter>>,
) -> Template {
    let ruleset = ruleset.lock().expect("Prof ruleset lock poisoned");
    let lints = { ruleset.lint(&filter.read().expect("profanity filter lock poisoned")) };
    Template::render(
        "admin/prof",
        context! {theme_css:theme.css(), flagsinfo: RuleFlags::flags_info(), rules: ruleset.rules(), tokeninfo: Token::token_info(), lints:lints},
    )
}

#[derive(Responder)]
enum RulesetWriteResponse {
    ErroredLints()

    #[response(status = 200)]
    Ok(Json<Vec<RulesetLint>>),
}

#[post("/prof/ruleset?<check_only>", data = "<rules>")]
fn prof_check(rules: Json<Vec<Rule>>, check_only: bool) -> RulesetWriteResponse {
    let ruleset = ProfRuleset::from_rules(rules.0);
    let filter = ruleset.build_filter();
    let lints = ruleset.lint(&filter);

    if check_only {
        return RulesetWriteResponse::Ok(Json(lints));
    }

    if lints
        .iter()
        .find(|l| l.importance == LintImportance::Error)
        .is_some()
    {

    }
}

#[get("/")]
fn index() -> Redirect {
    Redirect::permanent("/admin/prof")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("admin pages", |r| async {
        r.mount("/admin", routes![index, prof, prof_check])
    })
}
