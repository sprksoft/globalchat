use lmetrics::metrics;
use rocket::{http::SameSite, time::Duration};

use log::*;
use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar},
    response::{self, Redirect},
    routes,
    serde::Deserialize,
    Responder, State,
};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use sqlx;
use uuid::Uuid;

use crate::{config::UserConfig, db::Db};
use crate::{models::SesId, oauth::pses_store::CompletionResult};
use crate::{
    oauth::client::StateCheckError,
    themes::{self},
};

use self::client::{OAuthError, OAuthProviderConfig};

mod client;
mod jwt;
mod pses_store;

pub use client::{OAuth, Provider};
pub use pses_store::{PendingSessionStore, PendingSessionType};

metrics!(
    pub counter total_started_oauth_flows("Total count of started oauth flows");
    pub counter total_failed_oauth_flows("Total count of failed oauth flows", [reason]);

    pub counter total_logins("Total amount of logins");
);

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct OAuthConfig {
    google: Option<OAuthProviderConfig>,
    smartschool: Option<OAuthProviderConfig>,
}

#[derive(Responder)]
enum OAuthResponse {
    Redirect(Redirect),

    #[response(status = 422)]
    UnprocessableEntity(Template),

    #[response(status = 403)]
    Forbidden(Template),

    #[response(status = 404)]
    NotFound(Template),

    #[response(status = 500)]
    InternalServerError(Template),
}
impl OAuthResponse {
    fn template(error: &str, internal: &str) -> Template {
        total_failed_oauth_flows::inc(internal);
        let dtheme = themes::DEFAULT_THEME.clone();
        Template::render(
            "pages/login_complete",
            context! {error: error, theme_css: dtheme.css(), internal: internal},
        )
    }

    pub fn fail_flow_404(internal: &str) -> Self {
        Self::NotFound(Self::template("404 Not Found", internal))
    }
    pub fn fail_flow_403(internal: &str) -> Self {
        Self::Forbidden(Self::template("403 Forbidden", internal))
    }
    pub fn fail_flow_422(internal: &str) -> Self {
        Self::UnprocessableEntity(Self::template("422 Unprocessable Entity", internal))
    }
    pub fn fail_flow_500(internal: &str) -> Self {
        error!(
            "InternalServerError inside oauth_return_error: {}",
            internal
        );
        Self::InternalServerError(Self::template("500 Internal Server Error", internal))
    }
}

#[get("/oauth/start?<provider>&<pending_id>")]
fn oauth_start(
    pending_id: &str,
    oauth: &State<OAuth>,
    cookiejar: &CookieJar<'_>,
    provider: &str,
) -> Result<OAuthResponse, response::Debug<OAuthError>> {
    total_started_oauth_flows::inc();

    match oauth.begin_flow(pending_id.to_string(), provider, cookiejar) {
        Ok(r) => Ok(OAuthResponse::Redirect(r)),
        Err(OAuthError::ProviderNotFound(p)) => {
            error!("oauth provider '{}' not found", p);
            Ok(OAuthResponse::fail_flow_404("provider not found"))
        }
        Err(e) => {
            total_failed_oauth_flows::inc("begin_flow: OAuth error");
            Err(e.into())
        }
    }
}

#[get("/oauth/return/<provider>?<error>&<state>")]
async fn oauth_return_error(
    error: &str,
    state: &str,
    provider: Option<&str>,
    ses_store: &State<PendingSessionStore>,
) -> OAuthResponse {
    // We don't check the state cookie because this endpoint will never result in a successful
    // login
    let provider = provider.unwrap_or("smartschool");

    let mut error_message = match error {
        "access_denied" => "Flow canceled".to_string(),
        e => format!("{} error code returned by oauth provider {}", e, provider),
    };

    let Ok(pending_id) = Uuid::parse_str(state) else {
        error_message.push_str(", Invalid 'state'");
        return OAuthResponse::fail_flow_422(&error_message);
    };
    let session = match ses_store.session(pending_id) {
        Some(s) => s,
        None => {
            error_message.push_str(", No pending session found");
            return OAuthResponse::fail_flow_422(&error_message);
        }
    };
    session.abort();

    OAuthResponse::fail_flow_422(&error_message)
}

#[get("/oauth/return/<provider>?<code>&<state>")]
async fn oauth_return(
    code: &str,
    state: &str,
    provider: Option<&str>,
    oauth: &State<OAuth>,
    cookiejar: &CookieJar<'_>,
    ses_store: &State<PendingSessionStore>,
    user_config: &State<UserConfig>,
    mut db: Connection<Db>,
) -> Result<OAuthResponse, response::Debug<OAuthError>> {
    let Ok(pending_id) = Uuid::parse_str(state) else {
        return Ok(OAuthResponse::fail_flow_422("Invalid 'state'"));
    };
    let session = match ses_store.session(pending_id) {
        Some(s) => s,
        None => {
            //TODO: maybe we should return an error for security?
            error!(
                "oauth_return: No pending session found. Falling back to creating a generic pending session for immediate login to /v1."
            );
            ses_store.new_pending_and_get("/v1".to_string(), PendingSessionType::Immediate)
        }
    };
    match OAuth::check_state(cookiejar, state) {
        Err(StateCheckError::CookieNotFound) => {
            return Ok(OAuthResponse::fail_flow_422(
                "'state' cookie werd niet gevonden.",
            ));
        }
        Err(StateCheckError::CookieDoesntMatch) => {
            return Ok(OAuthResponse::fail_flow_422(
                "'state' is niet dezelfde als de cookie.",
            ));
        }
        Ok(()) => {}
    }

    let user_info = oauth
        .fetch_userinfo(provider.unwrap_or("smartschool"), code)
        .await?;

    let user_id = match user_info.provider {
        Provider::Smartschool => {
            sqlx::query!(
                "INSERT INTO users (smid, irl_name) VALUES ($1, $2) ON CONFLICT (smid) DO UPDATE SET irl_name = $2 RETURNING *;",
                user_info.id,
                user_info.irl_name,
            )
            .fetch_one(&mut **db)
            .await
            .map_err(|e| response::Debug(e.into()))?.id
        }
        Provider::Google => {
            sqlx::query!(
                "INSERT INTO users (googleid, irl_name) VALUES ($1, $2) ON CONFLICT (googleid) DO UPDATE SET irl_name = $2 RETURNING *;",
                user_info.id,
                user_info.irl_name,
            )
            .fetch_one(&mut **db)
            .await
            .map_err(|e| response::Debug(e.into()))?.id
        }
    };

    // Cleanup sessions
    sqlx::query!(
        "DELETE FROM sessions WHERE EXTRACT(epoch from now())-created_at > $1",
        user_config.max_session_age as i64
    )
    .execute(&mut **db)
    .await
    .map_err(|e| response::Debug(e.into()))?;

    let ses_id = SesId::new();
    sqlx::query!(
        "INSERT INTO sessions (id, user_id) VALUES ($1, $2)",
        ses_id.inner(),
        user_id
    )
    .execute(&mut **db)
    .await
    .map_err(|e| response::Debug(e.into()))?;

    set_ses_id(cookiejar, ses_id.clone(), &user_config);
    total_logins::inc();

    Ok(OAuthResponse::Redirect(match session.complete(ses_id) {
        CompletionResult::Delayed => Redirect::to("/login_complete"),
        CompletionResult::Immediate(c) => Redirect::to(c.redirect),
    }))
}

fn set_ses_id(cookiejar: &CookieJar<'_>, ses_id: SesId, user_config: &UserConfig) {
    cookiejar.add(
        Cookie::build(("session", ses_id.to_string()))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::None)
            .partitioned(true)
            .max_age(Duration::seconds(
                user_config.max_session_age.saturating_sub(10) as i64,
            ))
            .path("/"),
    );
}

#[get("/setup_ses/<id>")]
fn setup_session(
    id: &str,
    cookiejar: &CookieJar<'_>,
    user_config: &State<UserConfig>,
    ses_store: &State<PendingSessionStore>,
) -> OAuthResponse {
    let Ok(id) = Uuid::parse_str(id) else {
        return OAuthResponse::fail_flow_422("setup_session: invalid pses_id");
    };
    match ses_store.consume_delayed_session(id) {
        Some(c) => {
            set_ses_id(cookiejar, c.ses_id, &user_config);
            OAuthResponse::Redirect(Redirect::to(c.redirect))
        }
        None => OAuthResponse::Redirect(Redirect::to("/login?redirect=/v1")),
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("oauth", |r| async {
        let config: OAuthConfig = r.figment().extract_inner("oauth").unwrap();

        let mut oauth = OAuth::with_capacity(2);
        oauth.opt_provider(Provider::Smartschool, config.smartschool);
        oauth.opt_provider(Provider::Google, config.google);

        r.mount("/", routes![oauth_start, oauth_return, setup_session])
            .manage(oauth)
            .attach(pses_store::stage())
    })
}
