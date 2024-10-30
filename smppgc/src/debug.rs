use rocket::{fairing::AdHoc, get, response::Redirect, routes};

#[get("/reload_js")]
fn reload_js() -> Redirect {
    std::process::Command::new("smppgc/gen_js.sh")
        .spawn()
        .unwrap();
    Redirect::temporary("/v1")
}

pub struct Debug {
    pub debug: bool,
}
pub fn stage() -> AdHoc {
    AdHoc::on_ignite("try_stage_debug", |r| async {
        let debug = r.figment().extract_inner("debug").unwrap_or(false);
        let r = r.manage(Debug { debug });
        if !debug {
            return r;
        }
        r.mount("/debug", routes![reload_js])
    })
}
