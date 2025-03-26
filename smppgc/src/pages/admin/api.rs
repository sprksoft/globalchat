use std::sync::{Mutex, RwLock};

use crate::{
    auth::GcAdmin,
    chat::Chat,
    profanity::{ProfRuleset, RulesetChanges},
};
use profanity::ProfanityFilter;
use rocket::{post, serde::json::Json, State};

#[post("/prof/ruleset", data = "<ruleset>")]
async fn post_ruleset_changes(
    _gcadmin: GcAdmin,
    mut ruleset: Json<RulesetChanges>,
    global_ruleset: &State<Mutex<ProfRuleset>>,
    global_filter: &State<RwLock<ProfanityFilter>>,
    chat: &State<Chat>,
) -> Result<RulesetWriteResponse, Debug<RulesetError>> {
    let global_ruleset = global_ruleset.lock().expect("Global ruleset poisoned");
    ruleset.merge(&mut global_ruleset);
    ruleset.sort();
    let filter = ruleset.build_filter();
    let lints = ruleset.lint(&filter);
    let rule_lint_set = RuleLintSet {
        lints,
        rules: ruleset.into_inner(),
    };

    if lints.has_errors() {
        Ok(RulesetWriteResponse::Error(Json(rule_lint_set)))
    } else {
        {
            let mut lock = global_ruleset
                .lock()
                .expect("Profanity ruleset lock poisoned");
            lock.replace_from(ruleset.0);
            lock.save()?;
        }
        chat.run_filter(&filter).await;
        {
            let mut lock = global_filter
                .write()
                .expect("profanity filter lock poisoned");
            *lock = filter;
        }

        Ok(RulesetWriteResponse::Ok(Json(lints)))
    }
}
