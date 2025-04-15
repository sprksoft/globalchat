use rocket::{
    fairing::AdHoc,
    get,
    http::{hyper::Uri, Cookie, CookieJar, Header, SameSite, Status},
    request::{self, FromRequest},
    response::{Redirect, Responder},
    routes,
    serde::Deserialize,
    time::{Duration, OffsetDateTime},
    Request, State,
};
use rocket_dyn_templates::{context, tera, Template};

use crate::{
    auth::GcMod, csp::CSPFrameAncestors, oauth, themes::Theme, users::UserConfig, MessageConfig,
};

mod admin;

const DISCLAIMER_VER: usize = 1;

#[derive(Responder)]
enum GcPageResponder {
    #[response(status = 200)]
    Ok {
        inner: Template,
        csp: CSPFrameAncestors,
    },
    Redirect(Redirect),
}

#[get("/v1")]
fn landing_page(
    theme: Theme,
    oauth_config: &State<oauth::OAuthConfig>,
    cookiejar: &CookieJar<'_>,
) -> GcPageResponder {
    let accepted_disclaimer: usize = cookiejar
        .get("accepted_disclaimer")
        .map(|c| c.value_trimmed().parse().unwrap_or(0))
        .unwrap_or(0);

    GcPageResponder::Ok {
        inner: Template::render(
            "landing",
            context! { theme_css:theme.css(), accepted_disclaimer:accepted_disclaimer, disclaimer_ver:DISCLAIMER_VER, oauth_url:oauth::get_auth_url(&oauth_config)},
        ),
        csp: CSPFrameAncestors {
            frame_ancestors: "*.smartschool.be".to_string(),
        },
    }
}

#[get("/chat?<code>")]
fn chat(
    theme: Theme,
    code: &str,
    message_config: &State<MessageConfig>,
    user_config: &State<UserConfig>,
    gcmod: Option<GcMod>,
    cookiejar: &CookieJar<'_>,
) -> GcPageResponder {
    let theme_string = serde_json::to_string(&theme).expect("Failed to convert theme to json");
    cookiejar.add(
        Cookie::build(("smpptheme", theme_string))
            .same_site(SameSite::None)
            .expires(OffsetDateTime::now_utc() + Duration::hours(100_000)),
    );

    let fullname = "";

    GcPageResponder::Ok {
        inner: Template::render(
            "chat",
            context! (theme_css:theme.css(),
            placeholder:fullname,
            is_mod: gcmod.is_some(),
            max_username_len: user_config.max_username_len,
            max_message_len: message_config.max_message_len,
            min_message_len: message_config.min_message_len),
        ),
        csp: CSPFrameAncestors {
            frame_ancestors: "*.smartschool.be".to_string(),
        },
    }
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct RootUrl {
    pub root_url: String,
}

struct UrlFunction {
    root_url: String,
}
impl tera::Function for UrlFunction {
    fn call(
        &self,
        args: &std::collections::HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        if let Some(tera::Value::Bool(true)) = args.get("root") {
            return Ok(tera::Value::String(self.root_url.clone()));
        }
        let ver_int: u16 = *crate::VERSION_INT;

        let static_res = match args.get("static") {
            Some(tera::Value::Bool(true)) => Ok(true),
            Some(tera::Value::Bool(false)) => Ok(false),
            None => Ok(true),
            _ => Err("Invalid value for static parameter"),
        }?;

        match args.get("path") {
            Some(tera::Value::String(url)) => {
                let mut url: &str = url;
                if url.starts_with("/") {
                    url = &url[1..];
                }
                if static_res {
                    Ok(tera::Value::String(format!(
                        "{}/{}?ckey={}",
                        self.root_url, url, ver_int
                    )))
                } else {
                    Ok(tera::Value::String(format!("{}/{}", self.root_url, url)))
                }
            }
            _ => Err("url filter requires a parameter 'path' of type string.".into()),
        }
    }
}
struct VersionIntFunction();
impl tera::Function for VersionIntFunction {
    fn call(
        &self,
        _: &std::collections::HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        Ok(tera::Value::String(crate::VERSION_INT.to_string()))
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("templates", |r| async {
        let root_url = r
            .figment()
            .extract::<RootUrl>()
            .expect("No root_url field found in config");

        r.mount("/", routes![landing_page, chat])
            .attach(admin::stage())
            .attach(Template::custom(move |engines| {
                let tera = &mut engines.tera;
                tera.register_function("version_int", VersionIntFunction());
                tera.register_function(
                    "url",
                    UrlFunction {
                        root_url: root_url.root_url.clone(),
                    },
                );
            }))
    })
}
