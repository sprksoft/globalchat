use rocket::{
    http::{Cookie, CookieJar},
    response::Redirect,
    serde::Deserialize,
    time::Duration,
    FromForm,
};
use thiserror::Error;
use url::Url;

use super::jwt;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Google,
    Smartschool,
}
impl Provider {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "google" => Some(Self::Google),
            "smartschool" => Some(Self::Smartschool),
            _ => None,
        }
    }

    fn redirect_url_base(self) -> &'static str {
        match self {
            Provider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Provider::Smartschool => "https://oauth.smartschool.be/OAuth",
        }
    }

    fn token_url(self) -> &'static str {
        match self {
            Provider::Smartschool => "https://oauth.smartschool.be/OAuth/index/token",
            Provider::Google => "https://oauth2.googleapis.com/token",
        }
    }

    fn scopes(self) -> &'static str {
        match self {
            Provider::Smartschool => "userinfo",
            Provider::Google => "openid profile",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct OAuthProviderConfig {
    redirect_uri: String,
    client_id: String,
    client_secret_file: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct OpenIdResponse {
    id_token: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: usize,
}

#[derive(Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
struct SmUserInfo {
    #[serde(rename = "userID")]
    user_id: String,
    #[serde(rename = "actualUserName")]
    name: String,
    #[serde(rename = "actualUserSurname")]
    surname: String,
}
impl SmUserInfo {
    fn to_userinfo(self) -> UserInfo {
        let mut irl_name = self.name;
        irl_name.push(' ');
        irl_name.push_str(&self.surname);
        UserInfo {
            irl_name,
            id: self.user_id,
            provider: Provider::Smartschool,
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct GoogleJwt {
    sub: String,
    name: Option<String>,
}
impl GoogleJwt {
    fn to_userinfo(self) -> UserInfo {
        UserInfo {
            irl_name: self.name.unwrap_or("".to_string()),
            id: self.sub,
            provider: Provider::Google,
        }
    }
}

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("OAuth http error: {0}\n\nbody:\n{1}")]
    Status(String, reqwest::StatusCode, String),

    #[error("'{0}' was not and enabled oauth provider")]
    ProviderNotFound(Box<str>),

    #[error("OAuth reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("OAuth error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("OAuth error: {0}")]
    Url(#[from] url::ParseError),

    #[error("OAuth error: {0}")]
    Jwt(#[from] jwt::Error),
}
impl OAuthError {
    async fn check_res(res: reqwest::Response) -> Result<reqwest::Response, OAuthError> {
        if res.status().is_server_error() || res.status().is_client_error() {
            Err(Self::from_request(res).await)
        } else {
            Ok(res)
        }
    }
    async fn from_request(res: reqwest::Response) -> OAuthError {
        let url = res.url().to_string();
        let status = res.status();
        match res.text().await {
            Ok(body) => OAuthError::Status(url, status, body),
            Err(_) => OAuthError::Status(url, status, "<error reading body>".to_string()),
        }
    }
}

pub struct UserInfo {
    pub irl_name: String,
    pub id: String,
    pub provider: Provider,
}

struct OAuthClient {
    provider: Provider,
    config: OAuthProviderConfig,
    client_secret: String,
}

impl OAuthClient {
    pub fn new(provider: Provider, config: OAuthProviderConfig) -> Self {
        let mut client_secret =
            std::fs::read_to_string(&config.client_secret_file).expect(&format!(
                "Failed to read client_secret_file: ({})",
                &config.client_secret_file
            ));
        client_secret.truncate(client_secret.trim_end().len());
        Self {
            provider,
            config,
            client_secret,
        }
    }
}

pub struct OAuth {
    clients: Vec<OAuthClient>,
}
impl OAuth {
    const STATE_COOKIE_NAME: &'static str = "oauth_state";

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            clients: Vec::with_capacity(capacity),
        }
    }
    pub fn opt_provider(&mut self, provider: Provider, config: Option<OAuthProviderConfig>) {
        if let Some(config) = config {
            self.clients.push(OAuthClient::new(provider, config));
        }
    }

    fn get_client(&self, provider: &str) -> Result<&OAuthClient, OAuthError> {
        Provider::from_str(provider)
            .map(|p| self.clients.iter().find(|c| c.provider == p))
            .flatten()
            .ok_or_else(|| OAuthError::ProviderNotFound(provider.to_string().into()))
    }

    pub fn has_provider(&self, provider: Provider) -> bool {
        self.clients
            .iter()
            .find(|c| c.provider == provider)
            .is_some()
    }

    /// Begins the flow by setting the state cookie and returning the redirect.
    pub(super) fn begin_flow(
        &self,
        state: String,
        provider: &str,
        cookiejar: &CookieJar,
    ) -> Result<Redirect, OAuthError> {
        let client = self.get_client(provider)?;
        let url = Url::parse_with_params(
            client.provider.redirect_url_base(),
            &[
                ("response_type", "code"),
                ("client_id", &client.config.client_id),
                ("redirect_uri", &client.config.redirect_uri),
                ("scope", client.provider.scopes()),
                ("state", &state),
            ],
        )?;

        cookiejar.add(
            Cookie::build((Self::STATE_COOKIE_NAME, state))
                .secure(true)
                .max_age(Duration::new(300, 0))
                .http_only(true)
                .same_site(rocket::http::SameSite::Lax)
                .partitioned(true)
                .path("/")
                .build(),
        );

        Ok(Redirect::to(url.to_string()))
    }

    pub(super) fn check_state(cookiejar: &CookieJar, state: &str) -> bool {
        cookiejar.remove(Self::STATE_COOKIE_NAME);
        cookiejar
            .get(Self::STATE_COOKIE_NAME)
            .map(|c| c.value_trimmed() == state.trim())
            .unwrap_or(false)
    }

    pub(super) async fn fetch_userinfo(
        &self,
        provider: &str,
        code: &str,
    ) -> Result<UserInfo, OAuthError> {
        let client = self.get_client(provider)?;
        let http_client = reqwest::Client::new();

        let form = &[
            ("grant_type", "authorization_code"),
            ("redirect_uri", &client.config.redirect_uri.trim()),
            ("client_id", &client.config.client_id.trim()),
            ("client_secret", &client.client_secret.trim()),
            ("code", code),
        ];
        let res = OAuthError::check_res(
            http_client
                .post(client.provider.token_url())
                .form(&form)
                .send()
                .await?,
        )
        .await?;
        match client.provider {
            Provider::Smartschool => {
                let json: OAuthTokenResponse = res.json().await?;
                let res = OAuthError::check_res(
                    http_client
                        .get(Url::parse_with_params(
                            "https://oauth.smartschool.be/Api/V1/userinfo",
                            &[("access_token", json.access_token)],
                        )?)
                        .send()
                        .await?,
                )
                .await?;
                let sm_uinfo: SmUserInfo = res.json().await?;
                Ok(sm_uinfo.to_userinfo())
            }
            Provider::Google => {
                let json: OpenIdResponse = res.json().await?;
                let jwt = jwt::decode_payload_insecure::<GoogleJwt>(&json.id_token)?;
                Ok(jwt.to_userinfo())
            }
        }
    }
}
