use rocket::{
    fairing::AdHoc,
    get,
    http::{hyper::Uri, Cookie, CookieJar, Header, Status},
    request::{self, FromRequest},
    response::Responder,
    routes,
    serde::Deserialize,
    time::{Duration, OffsetDateTime},
    Request, State,
};
use rocket_dyn_templates::{context, tera, Template};

use crate::{auth::GcMod, csp::CSPFrameAncestors, themes::Theme, users::UserConfig, MessageConfig};

mod admin;

#[derive(Responder)]
enum GcPageResponder {
    #[response(status = 200)]
    Ok {
        inner: Template,
        csp: CSPFrameAncestors,
    },
}

#[get("/v1?<placeholder>")]
fn v1(
    theme: Theme,
    placeholder: Option<&str>,
    message_config: &State<MessageConfig>,
    user_config: &State<UserConfig>,
    debug: &State<crate::debug::Debug>,
    gcmod: Option<GcMod>,
    cookiejar: &CookieJar<'_>,
) -> GcPageResponder {
    let placeholder = placeholder.unwrap_or("");
    let theme_string = serde_json::to_string(&theme).expect("Failed to convert theme to json");
    cookiejar.add(
        Cookie::build(("smpptheme", theme_string))
            .expires(OffsetDateTime::now_utc() + Duration::hours(100_000)),
    );

    GcPageResponder::Ok {
        inner: Template::render(
            "v1",
            context! (theme_css:theme.css(),
            placeholder:placeholder,
            debug: debug.debug,
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
        match args.get("path") {
            Some(tera::Value::String(url)) => {
                let mut url: &str = url;
                if url.starts_with("/") {
                    url = &url[1..];
                }
                Ok(tera::Value::String(format!(
                    "{}/{}?ckey={}",
                    self.root_url, url, ver_int
                )))
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

        r.mount("/", routes![v1])
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
