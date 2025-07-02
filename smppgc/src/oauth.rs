use lmetrics::metrics;
use rocket::form::Form;
use rocket::time::Duration;
use rocket::{post, FromForm};

use log::*;
use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar},
    response::{self, Redirect},
    routes,
    serde::{Deserialize, Serialize},
    Responder, State,
};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use sqlx;
use thiserror::Error;
use url::Url;
use uuid::Uuid;

use crate::db::Db;
use crate::disclaimer::DisclaimerVer;
use crate::themes::{self, Theme};
use crate::users::{SesId, UserConfig};

pub const REDIRECT_URL_COOKIE: &'static str = "login_continue_url";

metrics!(
    pub counter total_started_oauth_flows("Total count of started oauth flows");
    pub counter total_failed_oauth_flows("Total count of failed oauth flows", [reason]);

    pub counter total_logins("Total amount of logins");
);

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: usize,
}

struct OAuth {
    config: OAuthConfig,
    client_secret: String,
}

#[derive(Deserialize, Serialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct SmUserInfo {
    #[serde(rename = "userID")]
    pub user_id: String,
    #[serde(rename = "actualUserName")]
    pub name: String,
    #[serde(rename = "actualUserSurname")]
    pub surname: String,
}

#[derive(Debug, Error)]
enum OAuthError {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("{0}")]
    Url(#[from] url::ParseError),
}

impl OAuth {
    pub const STATE_COOKIE_NAME: &'static str = "oauth_state";

    pub fn get_auth_url(&self, state: &str) -> Result<Url, OAuthError> {
        let config = &self.config;
        Ok(Url::parse_with_params(
            "https://oauth.smartschool.be/OAuth",
            &[
                ("response_type", "code"),
                ("client_id", &config.client_id),
                ("redirect_uri", &config.redirect_uri),
                ("scope", "userinfo"),
                ("state", state),
            ],
        )?)
    }

    pub async fn fetch_access_token(
        &self,
        client: &reqwest::Client,
        code: &str,
    ) -> Result<String, OAuthError> {
        let url = Url::parse_with_params(
            "https://oauth.smartschool.be/OAuth/index/token",
            &[
                ("grant_type", "authorization_code"),
                ("redirect_uri", &self.config.redirect_uri),
                ("client_id", &self.config.client_id),
                ("client_secret", &self.client_secret),
                ("code", code),
            ],
        )?;
        let res = client.post(url).send().await?.error_for_status()?;
        let json: OAuthTokenResponse = res.json().await?;
        Ok(json.access_token)
    }
    pub async fn fetch_userinfo(
        &self,
        client: &reqwest::Client,
        access_token: &str,
    ) -> Result<SmUserInfo, OAuthError> {
        let res = client
            .get(Url::parse_with_params(
                "https://oauth.smartschool.be/Api/V1/userinfo",
                &[("access_token", access_token)],
            )?)
            .send()
            .await?
            .error_for_status()?;
        Ok(res.json().await?)
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct OAuthConfig {
    #[serde(default)]
    debug: bool,
    redirect_uri: String,
    client_id: String,
    client_secret_file: String,
}

#[derive(Responder)]
enum OAuthResponse {
    Redirect(Redirect),

    #[response(status = 422)]
    UnprocessableEntity(Template),

    #[response(status = 422)]
    Forbidden(Template),
}
impl OAuthResponse {
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

    let state = Uuid::new_v4().as_simple().to_string();
    let mut url = match oauth.get_auth_url(&state) {
        Ok(us) => us,
        Err(e) => {
            total_failed_oauth_flows::inc("get_auth_url: OAuth error");
            return Err(response::Debug(e));
        }
    };
    if oauth.config.debug {
        url.set_host(Some("localhost")).unwrap();
        url.set_scheme("http").unwrap();
    }
    cookiejar.add(
        Cookie::build((OAuth::STATE_COOKIE_NAME, state))
            .secure(true)
            .max_age(Duration::new(300, 0))
            .http_only(true)
            .same_site(rocket::http::SameSite::Strict)
            .path("/")
            .build(),
    );
    Ok(OAuthResponse::Redirect(Redirect::temporary(
        url.to_string(),
    )))
}

#[get("/OAuth")]
fn oauth_debug(theme: Theme) -> Template {
    Template::render("pages/oauth_debug", context! {theme_css: theme.css()})
}
#[post("/OAuth?<redirect_uri>&<state>", data = "<smuserinfo>")]
fn oauth_debug_post(redirect_uri: &str, state: &str, smuserinfo: Form<SmUserInfo>) -> Redirect {
    let code: &str = &serde_json::to_string(&smuserinfo.into_inner())
        .expect("Failed to serialize sm_userinfo to json (oauth debug)");

    Redirect::to(
        Url::parse_with_params(&redirect_uri, &[("code", code), ("state", state)])
            .unwrap()
            .to_string(),
    )
}

#[get("/oauth/return?<code>&<state>")]
async fn oauth_return(
    code: &str,
    state: &str,
    oauth: &State<OAuth>,
    cookiejar: &CookieJar<'_>,
    user_config: &State<UserConfig>,
    mut db: Connection<Db>,
) -> Result<OAuthResponse, response::Debug<OAuthError>> {
    cookiejar.remove(OAuth::STATE_COOKIE_NAME);

    if !cookiejar
        .get(OAuth::STATE_COOKIE_NAME)
        .map(|c| c.value_trimmed() == state)
        .unwrap_or(false)
    {
        return Ok(OAuthResponse::fail_flow_422(
            "'state' is niet dezelfde als de cookie.",
        ));
    }

    let sm_uinfo = if oauth.config.debug {
        serde_json::from_str(code)
            .expect("Failed to deserialize sm_userinfo from json (oauth debug)")
    } else {
        let client = reqwest::Client::new();
        let access_token = oauth.fetch_access_token(&client, code).await?;
        oauth.fetch_userinfo(&client, &access_token).await?
    };

    let mut irl_name = sm_uinfo.name;
    irl_name.push(' ');
    irl_name.push_str(&sm_uinfo.surname);

    let user = sqlx::query!(
        "INSERT INTO users (smid, irl_name) VALUES ($1, $2) ON CONFLICT (smid) DO UPDATE SET irl_name = $2 RETURNING *;",
        sm_uinfo.user_id,
        irl_name,
    )
    .fetch_one(&mut **db)
    .await
    .map_err(|e| response::Debug(e.into()))?;

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
        user.id
    )
    .execute(&mut **db)
    .await
    .map_err(|e| response::Debug(e.into()))?;

    cookiejar.add(
        Cookie::build(("session", ses_id.to_string()))
            .http_only(true)
            .secure(true)
            .same_site(rocket::http::SameSite::Strict)
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

    Ok(OAuthResponse::Redirect(Redirect::to(
        redirect_url.unwrap_or("/v1".to_string()),
    )))
}

fn validate_redirect_url(url: &str) -> bool {
    if !url.starts_with("/") {
        error!("invalid redirect url (no starting slash): {}", url);
        return false;
    }
    if url.contains("://") {
        error!("invalid redirect url (contains ://): {}", url);
        return false;
    }
    for char in url.chars() {
        if !(char.is_alphanumeric() || ['/', '=', '_', '-', '?', '&'].contains(&char)) {
            error!("invalid redirect url: '{}' invalid char: {}", url, char);
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
            .same_site(rocket::http::SameSite::Strict)
            .max_age(Duration::seconds(3600))
            .path("/")
            .build(),
    );
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("oauth", |r| async {
        let config: OAuthConfig = r.figment().extract_inner("oauth").unwrap();

        let client_secret = if config.debug {
            String::new()
        } else {
            std::fs::read_to_string(&config.client_secret_file)
                .expect("Failed to load client_secret_file")
        };

        r.mount(
            "/",
            routes![oauth_start, oauth_return, oauth_debug, oauth_debug_post],
        )
        .manage(OAuth {
            config,
            client_secret,
        })
    })
}
