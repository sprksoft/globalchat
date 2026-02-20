use crate::{
    disclaimer::DisclaimerVer,
    oauth::{self, OAuth, PendingSessionStore, PendingSessionType},
    themes::Theme,
    utils::{AllowSmFrame, InIframe},
};
use rocket::{get, State};
use rocket_dyn_templates::{context, Template};

#[get("/login?<redirect>&<delayed>")]
pub fn login(
    theme: Theme,
    redirect: String,
    delayed: Option<bool>,
    oauth: &State<OAuth>,
    ses_store: &State<PendingSessionStore>,
    accepted_disclaimer: DisclaimerVer,
    in_iframe: InIframe,
) -> AllowSmFrame<Template> {
    let pses_type = match delayed {
        Some(true) => PendingSessionType::Delayed,
        Some(false) => PendingSessionType::Immediate,
        None => match in_iframe {
            InIframe::Yes => PendingSessionType::Delayed,
            InIframe::No => PendingSessionType::Immediate,
            InIframe::Unknown => PendingSessionType::Delayed,
        },
    };
    let id = ses_store.new_pending(redirect.into(), pses_type).simple();
    AllowSmFrame(Template::render(
        "pages/login",
        context! {
            pses_type: pses_type.str(),
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
