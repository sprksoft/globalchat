use std::sync::Arc;

use rocket::{fairing::AdHoc, form::Form, get, post, routes, FromForm, State};
use rocket_dyn_templates::{context, Template};
use wordfilter::Tag;

use crate::{themes::Theme, users::AdminUser, wf::Filter};

#[get("/wf")]
fn wf(_admin: AdminUser, theme: Theme<'_>) -> Template {
    Template::render("pages/wf", context! { theme_css: theme.css()})
}

#[derive(FromForm)]
struct WFSearchForm {
    message: String,
}

#[post("/wf", data = "<form>")]
async fn wf_search(
    _admin: AdminUser,
    form: Form<WFSearchForm>,
    theme: Theme<'_>,
    filter: &State<Arc<Filter>>,
) -> Template {
    let ts = filter.check(&form.message).await;
    let words: Vec<(&str, Tag)> = ts.words().collect();
    Template::render("pages/wf", context! { theme_css:theme.css(), words: words })
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("wf pages", async |r| r.mount("/", routes![wf, wf_search]))
}
