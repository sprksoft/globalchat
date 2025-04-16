use rocket::{fairing::AdHoc, get, response::Redirect, routes, serde::Deserialize, State};

pub struct OAuth {
    config: OAuthConfig,
    client_secret: String,
}
impl OAuth {
    pub fn get_auth_url(&self) -> String {
        let config = &self.config;
        if config.debug {
            return config.redirect_uri.clone();
        }
        let mut encoded =
            url::form_urlencoded::Serializer::new("https://oauth.smartschool.be/OAuth".to_string());
        encoded.append_pair("response_type", "code");
        encoded.append_pair("client_id", &config.client_id);
        encoded.append_pair("redirect_uri", &config.redirect_uri);
        encoded.append_pair("scope", "userinfo");
        encoded.finish()
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

#[get("/oauthlogin?<code>&<state>")]
fn oauth_login(
    code: &str,
    oauth: &State<OAuth>,
    state: Option<&str>,
    session_mgr: &State<crate::users::SessionMgr>,
) -> Redirect {
    //TODO: validate oauth code and set session
    match state {
        Some("redirect_to_admin") => Redirect::temporary("admin"),
        _ => Redirect::temporary("chat"),
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

        r.mount("/", routes![oauth_login]).manage(OAuth {
            config,
            client_secret,
        })
    })
}
