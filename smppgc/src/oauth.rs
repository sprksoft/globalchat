use rocket::{fairing::AdHoc, serde::Deserialize};

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct OAuthConfig {
    smartschool_platform: String,
    oauth_client_id: String,
    oauth_client_secret_file: String,
    oauth_redirect_uri: String,
}

pub fn get_auth_url(conf: &OAuthConfig) -> String {
    let mut encoded = url::form_urlencoded::Serializer::new(format!(
        "https://{}/OAuth",
        conf.smartschool_platform
    ));
    encoded.append_pair("response_type", "code");
    encoded.append_pair("client_id", &conf.oauth_client_id);
    encoded.append_pair("redirect_uri", &conf.oauth_redirect_uri);
    encoded.append_pair("scope", "userinfo");
    encoded.finish()
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("oauth", |r| async {
        r.attach(AdHoc::config::<OAuthConfig>())
    })
}
