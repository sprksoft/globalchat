use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar, SameSite},
    response::{self, Redirect, Responder},
    routes,
    time::{Duration, OffsetDateTime},
    State,
};
use rocket_dyn_templates::{context, tera, Template};

use crate::{
    auth::GcMod, disclaimer::DisclaimerVer, themes::Theme, users::UserConfig,
    utils::CSPFrameAncestors, MessageConfig,
};

mod admin;

#[derive(Responder)]
enum GcPageResponder {
    #[response(status = 200)]
    Ok {
        inner: Template,
        csp: CSPFrameAncestors<'static>,
    },
    Redirect(Redirect),
}

#[get("/?<ret>")]
fn index(
    theme: Theme,
    ret: Option<&str>,
    cookiejar: &CookieJar<'_>,
    accepted_disclaimer: DisclaimerVer,
) -> GcPageResponder {
    GcPageResponder::Ok {
        inner: Template::render(
            "index",
            context! {
                theme_css:theme.css(),
                accepted_disclaimer:accepted_disclaimer,
                disclaimer_ver:DisclaimerVer::LATEST
            },
        ),
        csp: CSPFrameAncestors::SMARTSCHOOL_PLAT,
    }
}

#[get("/chat")]
fn chat(
    theme: Theme,
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
        csp: CSPFrameAncestors::SMARTSCHOOL_PLAT,
    }
}

struct UrlFunction;
impl tera::Function for UrlFunction {
    fn call(
        &self,
        args: &std::collections::HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        let ver_int: u16 = *crate::VERSION_INT;

        match args.get("path") {
            Some(tera::Value::String(url)) => {
                let url: &str = url;
                if url.contains('?') {
                    Ok(tera::Value::String(format!("{}&ckey={}", url, ver_int)))
                } else {
                    Ok(tera::Value::String(format!("{}?ckey={}", url, ver_int)))
                }
            }
            _ => Err("url function requires a parameter 'path' of type string.".into()),
        }
    }
}
struct VersionIntFunction;
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
        r.mount("/", routes![index, chat])
            .attach(admin::stage())
            .attach(Template::custom(move |engines| {
                let tera = &mut engines.tera;
                tera.register_function("version_int", VersionIntFunction);
                tera.register_function("url", UrlFunction);
            }))
    })
}
