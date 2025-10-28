use log::*;
use rocket::fairing::AdHoc;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{RwLock, RwLockReadGuard};
use wordfilter::{
    stats::{Stat, WordFilterStats},
    Tag, TokenizedString, TrainResult, WordFilter,
};

use crate::chat::Chat;

#[derive(Deserialize)]
struct WFConfig {
    wordfilter: PathBuf,
    wordfilter_stats: PathBuf,
}

//Interior mutable part of the filter
pub struct FilterIMut {
    pub wf: WordFilter,
    pub dirty: bool,
}

pub struct Filter {
    filter_path: PathBuf,
    stats_path: PathBuf,
    stats: WordFilterStats,
    wf: RwLock<FilterIMut>,
}
impl Filter {
    #[inline]
    async fn read(&self) -> RwLockReadGuard<'_, FilterIMut> {
        self.wf.read().await
    }

    #[inline]
    pub async fn check(&self, message: &str) -> TokenizedString {
        let lock = self.wf.read().await;
        let ts = lock.wf.check(message);
        drop(lock);
        self.stats.record(&ts, [Tag::Unknown, Tag::Bad]);
        ts
    }

    #[inline]
    pub fn calc_stats<const N: usize>(&self, min_count: usize, filter: [Tag; N]) -> Vec<Stat> {
        self.stats.calc_top(min_count, filter)
    }

    #[inline]
    pub async fn mark_word(&self, word: &str, good: bool) {
        let mut lock = self.wf.write().await;
        match lock.wf.train_word(&word, good) {
            TrainResult::Unchanged => {}
            _ => {
                lock.dirty = true;
            }
        }
    }

    #[inline]
    pub async fn save_rerun(&self, chat: &Chat) {
        let lock = self.read().await;
        debug!("rerunning filter on chat...");
        chat.run_filter(&lock.wf).await;
    }

    async fn save_all(&self) {
        debug!("saving filter stats...");
        let string = self.stats.save_string();
        match std::fs::write(&self.stats_path, string) {
            Err(e) => {
                error!("Failed to save filter stats: {}", e);
            }
            Ok(()) => {}
        }

        let lock = self.read().await;
        if !lock.dirty {
            return;
        }
        debug!("saving wf filter...");
        let string = lock.wf.save_string();
        drop(lock);
        match std::fs::write(&self.filter_path, string) {
            Err(e) => {
                error!("Failed to save filter: {}", e);
                return;
            }
            Ok(()) => {}
        }
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
        let stats = match std::fs::read_to_string(&config.wordfilter_stats) {
            Ok(data) => WordFilterStats::from_string(&data),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => WordFilterStats::empty(),
            Err(e) => panic!("Failed to load wordfilter file: {}", e),
        };

        let fil = Arc::new(Filter {
            filter_path: config.wordfilter,
            stats_path: config.wordfilter_stats,
            stats,
            wf: RwLock::new(FilterIMut { wf, dirty: false }),
        });

        let r = r.manage(fil.clone());

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                debug!("starting filter save..");
                fil.save_all().await;
            }
        });
        r
    })
}
