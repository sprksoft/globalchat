use lazy_static::lazy_static;
use log::*;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize},
    Request,
};

lazy_static! {
    pub static ref DEFAULT_THEME: Theme<'static> = {
        let mut colors = Vec::with_capacity(Theme::DEFAULTS.len());
        for (name, col) in Theme::DEFAULTS {
            colors.push(CssColorVar::new(name, col).expect("Invalid color in default theme"));
        }
        Theme { colors }
    };
}

fn validate_hex_color(str: &str) -> bool {
    for char in str.chars() {
        if !char.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}
fn validate_oklch_color(str: &str) -> bool {
    if !str.starts_with("oklch") {
        return false;
    };
    let str = &str[5..];
    for char in str.chars() {
        if char.is_whitespace() {
            continue;
        }
        if !char.is_digit(10) && !['%', '(', ')', '.'].contains(&char) {
            return false;
        }
    }
    true
}

#[derive(Clone)]
pub struct CssColorVar<'n, 'v> {
    name: &'n str,
    value: &'v str,
}
impl<'n, 'v> CssColorVar<'n, 'v> {
    pub fn new(name: &'n str, value: &'v str) -> Result<Self, ()> {
        let mut me = Self { name, value };
        if !name.starts_with("--") {
            return Err(());
        }
        let _ = me.set_value(value);

        Ok(me)
    }
    pub fn set_value(&mut self, mut value: &'v str) -> Result<(), ()> {
        if value.len() == 6 || value.len() == 8 {
            if value.starts_with('#') {
                value = &value[1..];
            }
            if !validate_hex_color(value) {
                return Err(());
            }
        } else if value.starts_with("oklch") {
            if !validate_oklch_color(value) {
                return Err(());
            }
        } else {
            return Err(());
        }
        self.value = value;
        Ok(())
    }
    pub fn name(&self) -> &str {
        self.name
    }
    pub fn value(&self) -> &str {
        self.value
    }
    pub fn css(&self) -> String {
        let mut out = String::with_capacity(self.name().len() + self.value().len() + 3);
        out.push_str(self.name());
        out.push(':');
        if !self.value().starts_with("oklch") {
            out.push('#');
        }
        out.push_str(self.value());
        out.push(';');
        out
    }
}
impl<'n, 'v> TryFrom<(&'n str, &'v str)> for CssColorVar<'n, 'v> {
    type Error = ();
    fn try_from(value: (&'n str, &'v str)) -> Result<Self, Self::Error> {
        Self::new(value.0, value.1)
    }
}

#[macro_export]
macro_rules! css_color_vars {
    ($($var_name:literal: $var_value:literal),*) => {
        [$(($var_name, $var_value)),*]
    };
}

#[derive(Clone)]
pub struct Theme<'v> {
    colors: Vec<CssColorVar<'static, 'v>>,
}

impl<'a> Theme<'a> {
    const DEFAULTS: [(&'static str, &'static str); 6] = css_color_vars! {
        "--color-accent": "oklch(90% 0.069 70)",
        "--color-text": "oklch(80% 0.004 90)",
        "--color-base00": "oklch(15% 0.005 70)",
        "--color-base01": "oklch(20% 0.005 70)",
        "--color-base02": "oklch(24% 0.005 70)",
        "--color-base03": "oklch(30% 0.005 70)"
    };

    pub fn css(&self) -> String {
        let mut out = String::new();
        out.push_str("html {");
        for col in &self.colors {
            out.push_str(&col.css());
        }
        out.push_str("}");
        out
    }
    pub fn set_value(&mut self, value: CssColorVar<'static, 'a>) {
        for col in self.colors.iter_mut() {
            if col.name() == value.name() {
                col.value = value.value;
            }
        }
    }
}

struct ThemeVisitor;
impl<'de> Visitor<'de> for ThemeVisitor {
    type Value = Theme<'de>;
    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("a theme")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: rocket::serde::de::MapAccess<'de>,
    {
        let mut theme: Theme<'de> = DEFAULT_THEME.clone();

        while let Some((key, value)) = map.next_entry::<&str, &str>()? {
            if let Some(col) = theme.colors.iter_mut().find(|c| c.name() == key) {
                let _ = col.set_value(value);
            }
        }

        Ok(theme)
    }
}
impl<'de> Deserialize<'de> for Theme<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: rocket::serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ThemeVisitor)
    }
}
impl<'a> Serialize for Theme<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: rocket::serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.colors.len()))?;
        for col in &self.colors {
            map.serialize_entry(col.name(), col.value())?;
        }
        map.end()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Theme<'r> {
    type Error = String;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut theme = req
            .cookies()
            .get("smpptheme")
            .map(|c| match serde_json::from_str(c.value_trimmed()) {
                Ok(t) => t,
                Err(e) => {
                    error!(
                        "Failed to deserialize theme: '{}' error: {}",
                        c.value_trimmed(),
                        e
                    );
                    DEFAULT_THEME.clone()
                }
            })
            .unwrap_or_else(|| DEFAULT_THEME.clone());

        for (color_name, _) in Theme::DEFAULTS {
            let Some(value) = req.query_value(&color_name[2..]).map(|r| r.ok()).flatten() else {
                continue;
            };
            match CssColorVar::new(color_name, value) {
                Ok(colvar) => theme.set_value(colvar),
                Err(_) => {
                    return Outcome::Error((
                        Status::BadRequest,
                        format!("Invalid color value for {}", color_name),
                    ))
                }
            }
        }

        Outcome::Success(theme)
    }
}
