use log::*;
use rocket::{fairing::AdHoc, form::validate::Len};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{RwLock, RwLockReadGuard};
use wordfilter::{
    stats::{Stat, WordFilterStats},
    FilterMeta, Tag, TokenizedString, TrainResult, WordFilter,
};

use crate::chat::Chat;

#[derive(Deserialize)]
struct WFConfig {
    wordfilter: PathBuf,
    wordfilter_stats: PathBuf,
}

macro_rules! escaping {
    ($($esc:literal:$char:literal),*) => {
        fn escape(str: &str, buffer: &mut String) {
            for char in str.chars() {
                match char {
                    $(
                        $char=>{buffer.push('\\'); buffer.push($esc)}
                    ),*
                    _=> { buffer.push(char)},
                }
            }
        }

        fn unescape(str: &str) -> String {
            let mut string = String::with_capacity(str.len());
            let mut esc = false;
            for char in str.chars() {
                if char == '\\' {
                    esc = true;
                    continue;
                }
                if esc {
                    match char {
                        $(
                            $esc=>{ string.push($char); }
                        ),*
                        _=>{},
                    }
                } else {
                    string.push(char);
                }
            }
            string
        }
    };
}

escaping!(
    '\\':'\\',
    'n':'\n'
);

#[derive(Clone)]
struct Meta {
    locked: bool,
    lock_reason: Arc<str>,
}
impl FilterMeta for Meta {
    fn read(str: &str) -> Self {
        let locked = str.starts_with('L');
        let str = unescape(&str[1..]).into();
        Self {
            locked,
            lock_reason: str,
        }
    }
    fn write(&self, string: &mut String) {
        if self.locked {
            string.push('L');
        } else {
            string.push('l');
        }
        escape(&self.lock_reason, string);
    }
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
    pub fn calc_stats(&self, min_count: usize, filter: &[Tag]) -> Vec<Stat> {
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

        let bgsave_fil = fil.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60 * 60 * 24)).await;
                debug!("starting filter save..");
                bgsave_fil.save_all().await;
            }
        });
        r.manage(fil.clone())
            .attach(AdHoc::on_shutdown("word filter shutdown save", |_| {
                Box::pin(async move {
                    info!("Saving filter+stats because of shutdown...");
                    fil.save_all().await;
                })
            }))
    })
}
