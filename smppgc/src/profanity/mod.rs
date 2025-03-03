use std::ops::Range;
use std::path::PathBuf;
use std::sync::Mutex;

use log::*;
use profanity::{ProfanityFilter, TokenizedMessage};
use rocket::fairing::AdHoc;
use rocket::serde::Deserialize;
use thiserror::Error;
use tokio::sync::RwLock;

mod rules;

#[derive(Debug, Error)]
pub enum ProfFileLoadErr {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    ParseError(#[from] rules::ParseError),
}

pub struct ProfRuleset {
    filter_path: PathBuf,
    rules: Vec<rules::Rule>,
}
impl ProfRuleset {
    pub fn new(filter_path: PathBuf) -> Result<Self, ProfFileLoadErr> {
        let file = std::fs::read_to_string(&filter_path)?;

        let rules = rules::parse_from_str(&file)?;

        Ok(Self { filter_path, rules })
    }
    pub fn rules(&self) -> &[rules::Rule] {
        &self.rules
    }

    pub fn build_filter(&self) -> ProfanityFilter {
        let mut filter = ProfanityFilter::empty();
        for rule in self.rules.iter() {
            if rule.enabled {
                filter.insert_rule(rule.inner.clone())
            }
        }
        filter
    }

    // pub async fn load(&self) -> Result<(), ProfFileLoadErr> {
    //     let file = std::fs::read_to_string(&self.filter_path)?;
    //     self.filter.write().await.add_from_parsed(&file)?;
    //     Ok(())
    // }

    // pub async fn filter_string(
    //     &self,
    //     string: &str,
    // ) -> (Result<String, Range<usize>>, TokenizedMessage) {
    //     let filter = self.filter.read().await;
    //     let (tokenized, string) = filter.tokenize(string);
    //     if let Some(r) = filter.check(&tokenized) {
    //         (Err(r.span), tokenized)
    //     } else {
    //         (Ok(string), tokenized)
    //     }
    // }
    // pub async fn contains_profanity_any(&self, strings: impl Iterator<Item = &str>) -> bool {
    //     let filter = self.filter.read().await;
    //     for str in strings {
    //         if filter.contains_profanity(str) {
    //             return true;
    //         }
    //     }
    //     false
    // }
    //
    // pub async fn filter_all(&self, messages: impl Iterator<Item = &mut Message>) {
    //     let filter = self.filter.read().await;
    //     for message in messages {
    //         if filter.contains_profanity(&message.content) {
    //             Self::hide_message(message)
    //         }
    //         if filter.contains_profanity(&message.sender) {
    //             Self::hide_message_sender(message)
    //         }
    //     }
    // }
    //
    // pub async fn filter_message_content(
    //     &self,
    //     message: &mut Message,
    // ) -> (Result<&mut Message, Range<usize>>) {
    //     let filter = self.filter.read().await;
    //
    //     let mut prof = false;
    //     if filter.contains_profanity(&message.content) {
    //         Self::hide_message(message);
    //         prof = true;
    //     }
    //     if filter.contains_profanity(&message.sender) {
    //         Self::hide_message_sender(message);
    //         prof = true;
    //     }
    //     prof
    // }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct ProfConfig {
    prof_filter_file: PathBuf,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("profanity filter", |r| async {
        let config = r
            .figment()
            .extract::<ProfConfig>()
            .expect("No profanity config found");

        let ruleset = ProfRuleset::new(config.prof_filter_file)
            .expect("Failed to load profanity filter rules");
        r.manage(std::sync::RwLock::new(ruleset.build_filter()))
            .manage(Mutex::new(ruleset))
    })
}
