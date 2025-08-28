use std::path::PathBuf;

use log::*;
use rocket::fairing::AdHoc;
use serde::Deserialize;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use wordfilter::WordFilter;

#[derive(Deserialize)]
struct WFConfig {
    wordfilter: PathBuf,
}

pub struct FilterWrapper {
    pub wf: WordFilter,
    pub dirty: bool,
}

pub struct Filter {
    path: PathBuf,
    wf: RwLock<FilterWrapper>,
}
impl Filter {
    #[inline]
    pub async fn read(&self) -> RwLockReadGuard<'_, FilterWrapper> {
        self.wf.read().await
    }
    #[inline]
    pub async fn write(&self) -> RwLockWriteGuard<'_, FilterWrapper> {
        self.wf.write().await
    }

    pub async fn save(&self) {
        let lock = self.read().await;
        if !lock.dirty {
            return;
        }
        match std::fs::write(&self.path, lock.wf.save_string()) {
            Err(e) => {
                error!("Failed to save filter: {}", e);
                return;
            }
            Ok(()) => {}
        }
        drop(lock);
        self.write().await.dirty = false;
    }
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
            wf: RwLock::new(FilterWrapper { wf, dirty: false }),
        })
    })
}
