use crate::{
    disclaimer::DisclaimerVer,
    oauth::{self, LoginType, OAuth, PendingSessionStore},
    themes::Theme,
    utils::{AllowSmFrame, InIframe},
};
use rocket::{get, State};
use rocket_dyn_templates::{context, Template};

#[get("/login?<redirect>&<external>")]
pub fn login(
    theme: Theme,
    redirect: String,
    external: Option<bool>,
    oauth: &State<OAuth>,
    ses_store: &State<PendingSessionStore>,
    accepted_disclaimer: DisclaimerVer,
    in_iframe: InIframe,
) -> AllowSmFrame<Template> {
    let login_type = match external {
        Some(true) => LoginType::External,
        Some(false) => LoginType::Internal,
        None => match in_iframe {
            InIframe::Yes => LoginType::External,
            InIframe::No => LoginType::Internal,
            InIframe::Unknown => LoginType::External,
        },
    };
    let id = ses_store.new_pending(redirect.into(), login_type).simple();
    AllowSmFrame(Template::render(
        "pages/login",
        context! {
            internal_login: login_type.is_internal(),
            pending_id: id,
            oauth_smartschool: oauth.has_provider(oauth::Provider::Smartschool),
            oauth_google: oauth.has_provider(oauth::Provider::Google),
            theme_css:theme.css(),
            accepted_disclaimer:accepted_disclaimer,
            disclaimer_ver:DisclaimerVer::LATEST
        },
    ))
}

#[get("/login_complete")]
pub fn login_complete(theme: Theme) -> Template {
    Template::render(
        "pages/login_complete",
        context! {
            theme_css: theme.css(),
        },
    )
}
