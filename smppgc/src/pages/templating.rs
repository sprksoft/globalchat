use rocket_dyn_templates::{
    context,
    tera::{self, Tera},
    Template,
};
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

pub fn setup(tera: &mut Tera) {
    tera.register_function("version_int", VersionIntFunction);
    tera.register_function("url", UrlFunction);
}
