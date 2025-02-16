use rocket::{
    fairing::AdHoc,
    get,
    http::{hyper::Uri, Header, Status},
    request::{self, FromRequest},
    response::Responder,
    routes,
    serde::Deserialize,
    Request, State,
};
use rocket_dyn_templates::{context, tera, Template};

use crate::{csp::CSPFrameAncestors, users::UserConfig, MessageConfig};

mod admin;

macro_rules! css_var {
    ($name:ident, $($alpha:literal),*) => {
        concat!($(
                "--", stringify!($name), "-", $alpha, ": #{}", $alpha, ";"
        ),*)
    };
    ($name:ident) => {
        concat!("--", stringify!($name), ": #{};")
    }
}

fn string_void<'a>(string: &'a str, _void: &'static str) -> &'a str {
    string
}
macro_rules! theme {
    ($vis:vis $name:ident{$($param:ident:$default_value:literal:[$($alpha:literal),*]),*}) => {
        $vis struct $name {
            pub $($param:String),*
        }
        impl $name {
            pub fn css(&self) -> String {
                format!(concat!("body{{", $(css_var!($param), css_var!($param, $($alpha),*)),*, "}}"), $(self.$param, $(string_void(&self.$param, $alpha),)* )*)
            }
        }

        #[rocket::async_trait]
        impl<'r> FromRequest<'r> for $name {
            type Error = ();

            async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
                $(
                    let $param = match req .query_value::<String>(stringify!($param)) .unwrap_or(Ok($default_value.to_string()))
                    {
                        Ok(value) => {
                            if value.len() > 8{
                                return request::Outcome::Error((Status::BadRequest, ()));
                            }
                            for character in value.chars(){
                                if !character.is_alphanumeric(){
                                    return request::Outcome::Error((Status::BadRequest, ()));
                                }
                            }
                            value
                        },
                        Err(_) => return request::Outcome::Error((Status::BadRequest, ())),
                    };
                )*
                request::Outcome::Success($name {
                    $($param),*
                })
            }
        }
    };
}

theme! {
    SmppTheme{
        color_text:"c2bab2":[],
        color_base00:"191817":[],
        color_base01:"232020":["b0"],
        color_base02:"2b2828":[],
        color_base03:"353232":[],
        color_base04:"3f3c3c":[],
        color_base05:"4a4747":[],
        color_accent:"ffd5a0":[]
    }
}

#[derive(Responder)]
enum GcPageResponder {
    #[response(status = 200)]
    Ok {
        inner: Template,
        csp: CSPFrameAncestors,
    },
}

#[get("/v1?<skip_login>&<placeholder>")]
fn v1(
    theme: SmppTheme,
    placeholder: Option<&str>,
    skip_login: Option<bool>,
    message_config: &State<MessageConfig>,
    user_config: &State<UserConfig>,
    debug: &State<crate::debug::Debug>,
) -> GcPageResponder {
    let placeholder = placeholder.unwrap_or("");

    GcPageResponder::Ok {
        inner: Template::render(
            "v1",
            context! (theme_css:theme.css(),
            placeholder:placeholder,
            debug: debug.debug,
            skip_login: skip_login.unwrap_or(false),
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
