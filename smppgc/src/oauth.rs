use rocket::time::Duration;

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
use sqlx;
use thiserror::Error;
use url::Url;
use uuid::Uuid;

use crate::db::{self, Db, DbResult};

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

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct SmUserInfo {
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
    #[error("Invalid characters in return parameter")]
    InvalidCharsInRet,
}

impl OAuth {
    pub const STATE_COOKIE_NAME: &'static str = "oauth_state";

    pub fn get_auth_url(&self, ret: &str) -> Result<(Url, String), OAuthError> {
        if ret
            .chars()
            .find(|char| !char.is_ascii_alphanumeric())
            .is_some()
        {
            return Err(OAuthError::InvalidCharsInRet);
        }
        let state = format!("{}+ret:{}", Uuid::new_v4().as_simple(), ret);

        let config = &self.config;

        let url_base = if config.debug {
            &config.redirect_uri
        } else {
            "https://oauth.smartschool.be/OAuth"
        };
        Ok((
            Url::parse_with_params(
                url_base,
                &[
                    ("response_type", "code"),
                    ("client_id", &config.client_id),
                    ("redirect_uri", &config.redirect_uri),
                    ("scope", "userinfo"),
                    ("state", &state),
                ],
            )?,
            state,
        ))
    }

    pub async fn fetch_access_token(
        &self,
        client: &reqwest::Client,
        code: &str,
    ) -> Result<String, OAuthError> {
        if self.config.debug {
            return Ok("debug_access_token".to_string());
        }

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
        if self.config.debug {
            let mut debug_smid =Uuid::new_v4().as_simple().to_string();
            debug_smid.push_str("_debugsmid");
            return Ok(SmUserInfo { user_id: debug_smid name: "Jan", surname: "Jansens" })
        }
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
    UnprocessableEntity(&'static str),
}

#[get("/oauth/start?<ret>")]
fn oauth_start(
    ret: &str,
    oauth: &State<OAuth>,
    cookiejar: &CookieJar<'_>,
) -> Result<OAuthResponse, response::Debug<OAuthError>> {
    let (url, state) = match oauth.get_auth_url(ret) {
        Ok(us) => us,
        Err(OAuthError::InvalidCharsInRet) => {
            return Ok(OAuthResponse::UnprocessableEntity(
                "Invalid value for ret parameter",
            ))
        }
        Err(e) => return Err(response::Debug(e)),
    };
    cookiejar.add(
        Cookie::build((OAuth::STATE_COOKIE_NAME, state))
            .secure(true)
            .max_age(Duration::new(60, 0))
            .http_only(true)
            .same_site(rocket::http::SameSite::Strict)
            .path("/")
            .build(),
    );
    Ok(OAuthResponse::Redirect(Redirect::temporary(
        url.to_string(),
    )))
}

#[get("/oauth/return?<code>&<state>")]
async fn oauth_return(
    code: &str,
    state: &str,
    oauth: &State<OAuth>,
    session_mgr: &State<crate::users::SessionMgr>,
    cookiejar: &CookieJar<'_>,
    mut db: Connection<Db>,
) -> Result<OAuthResponse, response::Debug<OAuthError>> {
    cookiejar.remove(OAuth::STATE_COOKIE_NAME);

    if !cookiejar
        .get(OAuth::STATE_COOKIE_NAME)
        .map(|c| c.value_trimmed() == state)
        .unwrap_or(false)
    {
        return Ok(OAuthResponse::UnprocessableEntity("OAuth state mismatch"));
    }

    let client = reqwest::Client::new();
    let access_token = oauth.fetch_access_token(&client, code).await?;
    let user_info = oauth.fetch_userinfo(&client, &access_token).await?;

    let user: db::models::User = sqlx::query_as!(
        db::models::User,
        "INSERT INTO users (smid) VALUES ($1) ON CONFLICT DO NOTHING RETURNING *;",
        user_info.user_id
    )
    .fetch_one(&mut **db)
    .await
    .map_err(|e| response::Debug(e.into()))?;

    let ses_id = session_mgr.create_session(user).await;

    cookiejar.add(
        Cookie::build(("session", ses_id.to_string()))
            .http_only(true)
            .secure(true)
            .same_site(rocket::http::SameSite::Strict)
            .permanent(),
    );

    let param = state.split_once('+').map(|(_, param)| param);
    match param {
        Some("ret:admin") => Ok(OAuthResponse::Redirect(Redirect::temporary("/admin"))),
        Some("ret:chat") => Ok(OAuthResponse::Redirect(Redirect::temporary("/chat"))),
        _ => Ok(OAuthResponse::UnprocessableEntity(
            "Invalid ret in state parameter",
        )),
    }
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

        r.mount("/", routes![oauth_start, oauth_return])
            .manage(OAuth {
                config,
                client_secret,
            })
    })
}
