use lmetrics::metrics;
use rocket::time::Duration;

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

use crate::themes::{self};
use crate::{db::Db, users::UserConfig};
use crate::{disclaimer::DisclaimerVer, users::SesId};

use self::client::{OAuthError, OAuthProviderConfig};

mod client;
mod jwt;

pub use client::{OAuth, Provider};

const REDIRECT_URL_COOKIE: &'static str = "login_continue_url";

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
}
impl OAuthResponse {
    pub fn fail_flow_404(internal: &str) -> Self {
        let dtheme = themes::DEFAULT_THEME.clone();
        total_failed_oauth_flows::inc(internal);
        Self::NotFound(Template::render(
            "pages/error_page",
            context! {title: "404 Not Found", theme_css: dtheme.css(), error: "Oei! Er ging iets mis tijdens het inloggen.", internal: internal},
        ))
    }
    pub fn fail_flow_403(internal: &str) -> Self {
        let dtheme = themes::DEFAULT_THEME.clone();
        total_failed_oauth_flows::inc(internal);
        Self::Forbidden(Template::render(
            "pages/error_page",
            context! {title: "403 Forbidden", theme_css: dtheme.css(), error: "Oei! Er ging iets mis tijdens het inloggen.", internal: internal},
        ))
    }
    pub fn fail_flow_422(internal: &str) -> Self {
        let dtheme = themes::DEFAULT_THEME.clone();
        total_failed_oauth_flows::inc(internal);
        Self::UnprocessableEntity(Template::render(
            "pages/error_page",
            context! {title: "422 Unprocessable Entity", theme_css: dtheme.css(), error: "Oei! Er ging iets mis tijdens het inloggen.", internal: internal},
        ))
    }
}

#[get("/oauth/start?<provider>")]
fn oauth_start(
    oauth: &State<OAuth>,
    cookiejar: &CookieJar<'_>,
    provider: &str,
    accepted_disclaimer: DisclaimerVer,
) -> Result<OAuthResponse, response::Debug<OAuthError>> {
    total_started_oauth_flows::inc();
    if accepted_disclaimer != DisclaimerVer::LATEST {
        return Ok(OAuthResponse::fail_flow_422(
            "Disclaimer niet geaccepteerd.",
        ));
    }

    match oauth.begin_flow(provider, cookiejar) {
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

#[get("/oauth/return/<provider>?<code>&<state>")]
async fn oauth_return(
    code: &str,
    state: &str,
    provider: Option<&str>,
    oauth: &State<OAuth>,
    cookiejar: &CookieJar<'_>,
    user_config: &State<UserConfig>,
    mut db: Connection<Db>,
) -> Result<OAuthResponse, response::Debug<OAuthError>> {
    if !OAuth::check_state(cookiejar, state) {
        return Ok(OAuthResponse::fail_flow_422(
            "'state' is niet dezelfde als de cookie.",
        ));
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
        "DELETE FROM sessions WHERE EXTRACT(epoch from now())-created_at < $1",
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

    cookiejar.add(
        Cookie::build(("session", ses_id.to_string()))
            .http_only(true)
            .secure(true)
            .same_site(rocket::http::SameSite::Lax)
            .max_age(Duration::seconds(
                user_config.max_session_age.saturating_sub(10) as i64,
            ))
            .path("/"),
    );

    total_logins::inc();

    let redirect_url = cookiejar
        .get(REDIRECT_URL_COOKIE)
        .map(|c| c.value_trimmed().to_string())
        .filter(|url| validate_redirect_url(&url));
    cookiejar.remove(REDIRECT_URL_COOKIE);

    dbg!(&redirect_url);
    Ok(OAuthResponse::Redirect(Redirect::to(
        redirect_url.unwrap_or("/v1".to_string()),
    )))
}

fn validate_redirect_url(url: &str) -> bool {
    if !url.starts_with("/") {
        error!("Invalid redirect url (no starting slash): {}", url);
        return false;
    }
    if url.contains("://") || url.contains("javascript:") {
        error!("Invalid redirect url (contains :// | javascript:): {}", url);
        return false;
    }
    for char in url.chars() {
        if !(char.is_alphanumeric() || ['/', '=', '_', '-', '?', '&'].contains(&char)) {
            error!("Invalid redirect url: '{}' invalid char: {}", url, char);
            return false;
        }
    }
    true
}

pub fn set_continue_url_cookie(cookiejar: &CookieJar<'_>, url: String) {
    cookiejar.add(
        Cookie::build((REDIRECT_URL_COOKIE, url))
            .http_only(true)
            .secure(true)
            .same_site(rocket::http::SameSite::Lax)
            .max_age(Duration::seconds(3600))
            .path("/")
            .build(),
    );
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("oauth", |r| async {
        let config: OAuthConfig = r.figment().extract_inner("oauth").unwrap();

        let mut oauth = OAuth::with_capacity(2);
        oauth.opt_provider(Provider::Smartschool, config.smartschool);
        oauth.opt_provider(Provider::Google, config.google);

        r.mount("/", routes![oauth_start, oauth_return])
            .manage(oauth)
    })
}
