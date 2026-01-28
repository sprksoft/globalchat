use std::sync::Arc;

use rocket::{fairing::AdHoc, form::Form, get, post, routes, serde::json::Json, FromForm, State};
use rocket_dyn_templates::{context, Template};

use crate::{
    chat::Chat,
    csrf::CSRFProtect,
    themes::Theme,
    users::{AdminUser, ModUser},
    wf::{Filter, WFTag, WordInfo},
};

#[get("/wf")]
fn wf(mod_user: ModUser, theme: Theme<'_>) -> Template {
    Template::render(
        "pages/wf",
        context! { theme_css: theme.css(), role: mod_user.0.role().to_i32()},
    )
}

#[derive(FromForm)]
struct WFSearchForm {
    message: String,
}

#[post("/wf", data = "<form>")]
async fn wf_search(
    mod_user: ModUser,
    form: Form<WFSearchForm>,
    theme: Theme<'_>,
    filter: &State<Arc<Filter>>,
) -> Template {
    let ts = filter.check(&form.message).await;
    let words: Vec<(&str, WFTag)> = ts.words().collect();
    Template::render(
        "pages/wf",
        context! { theme_css:theme.css(), words: words, role: mod_user.0.role().to_i32() },
    )
}

#[post("/wf/<word>/markgood")]
async fn wf_markgood(
    _mod: ModUser,
    word: &str,
    chat: &State<Chat>,
    filter: &State<Arc<Filter>>,
    _csrf: CSRFProtect,
) {
    filter.mark_word(word, true).await;
    filter.rerun(&chat).await;
}
#[post("/wf/<word>/markbad")]
async fn wf_markbad(
    _mod: ModUser,
    word: &str,
    chat: &State<Chat>,
    filter: &State<Arc<Filter>>,
    _csrf: CSRFProtect,
) {
    filter.mark_word(word, false).await;
    filter.rerun(&chat).await;
}

#[post("/wf/<word>/lock?<reason>")]
async fn wf_lockword(
    word: &str,
    reason: &str,
    _admin: AdminUser,
    filter: &State<Arc<Filter>>,
    chat: &State<Chat>,
    _csrf: CSRFProtect,
) {
    if filter.lock_word(word, reason.into()).await {
        filter.rerun(&chat).await;
    }
}

#[post("/wf/<word>/unlock")]
async fn wf_unlockword(
    word: &str,
    _admin: AdminUser,
    filter: &State<Arc<Filter>>,
    chat: &State<Chat>,
    _csrf: CSRFProtect,
) {
    if filter.unlock_word(word).await {
        filter.rerun(&chat).await;
    }
}

#[get("/wf/<word>")]
async fn wf_wordinfo(
    word: &str,
    _mod: ModUser,
    filter: &State<Arc<Filter>>,
) -> Option<Json<WordInfo>> {
    filter.word_info(word).await.map(|wi| Json(wi))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("wf pages", async |r| {
        r.mount("/", routes![wf, wf_search]).mount(
            "/api",
            routes![
                wf_markbad,
                wf_markgood,
                wf_lockword,
                wf_unlockword,
                wf_wordinfo
            ],
        )
    })
}
