use rocket::serde;

use super::OAuthConfig;

pub enum Provider {
    Google,
    Smartschool,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct OAuthProviderConfig {
    redirect_uri: String,
    client_id: String,
    client_secret_file: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct OpenIdResponse {
    #[serde(flatten)]
    oauth: OpenIdResponse,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: usize,
}

pub struct OAuthClient {
    provider: Provider,
    config: OAuthConfig,
    client_secret: String,
}

impl OAuthClient {
    pub const STATE_COOKIE_NAME: &'static str = "oauth_state";
    pub fn new(provider: Provider, config: OAuthProviderConfig) -> Self {
        let client_secret = std::fs::read_to_string(&config.client_secret_file).expect(format!(
            "Failed to read client_secret_file: ({})",
            &config.client_secret_file
        ));
        Self {
            provider,
            config,
            client_secret,
        }
    }
}
