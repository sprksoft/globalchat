use std::path::PathBuf;

use rocket::fairing::AdHoc;
use serde::Deserialize;
use tokio::sync::RwLock;
use wordfilter::WordFilter;

#[derive(Deserialize)]
struct WFConfig {
    wordfilter: PathBuf,
}

pub struct Filter {
    path: PathBuf,
    wf: RwLock<WordFilter>,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("word filter", |r| async {
        let config = r
            .figment()
            .extract::<WFConfig>()
            .expect("No wordfilter config found");

        let wf = match std::fs::read_to_string(&config.wordfilter) {
            Ok(data) => WordFilter::from_string(&data),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => WordFilter::empty(),
            Err(e) => panic!("Failed to load wordfilter file: {}", e),
        };

        r.manage(Filter {
            path: config.wordfilter,
            wf: wf.into(),
        })
    })
}
