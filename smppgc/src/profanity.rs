use std::path::{Path, PathBuf};

use log::*;
use profanity::{ProfSyntaxErr, ProfanityFilter};
use rocket::fairing::AdHoc;
use rocket::serde::Deserialize;
use thiserror::Error;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use crate::chat::Message;

#[derive(Debug, Error)]
enum ProfFileLoadErr {
    IO(#[from] std::io::Error),
    Syntax(#[from] profanity::ProfSyntaxErr),
}

pub struct ProfFilter {
    filter_path: PathBuf,
    filter: RwLock<ProfanityFilter>,
}
impl ProfFilter {
    pub async fn new(filter_path: PathBuf) -> Result<Self, ProfFileLoadErr> {
        let file = tokio::fs::read_to_string(&filter_path).await?;
        let filter = ProfanityFilter::parse_from_str(&file)?;
        Ok(Self {
            filter_path,
            filter: filter.into(),
        })
    }

    pub async fn load(&self) -> Result<(), ProfFileLoadErr> {
        let file = tokio::fs::read_to_string(&self.filter_path).await?;
        &mut self.filter.write().await.add_from_parsed(&file)?;
        Ok(())
    }

    pub async fn contains_profanity(&self, string: &str) -> bool {
        self.filter.read().await.contains_profanity(string)
    }
    pub async fn contains_profanity_any(&self, strings: impl Iterator<Item = &str>) -> bool {
        let filter = self.filter.read().await;
        for str in strings {
            if filter.contains_profanity(str) {
                return true;
            }
        }
        false
    }

    pub async fn filter_all(&self, messages: impl Iterator<Item = &mut Message>) {
        let filter = self.filter.read().await;
        for message in messages {
            if filter.contains_profanity(&message.content) {
                Self::hide_message(message)
            }
            if filter.contains_profanity(&message.sender) {
                Self::hide_message_sender(message)
            }
        }
    }
    fn hide_message(message: &mut Message) {
        message.content = "#".repeat(message.content.len()).into();
    }
    fn hide_message_sender(message: &mut Message) {
        message.sender = "#".repeat(message.sender.len()).into();
    }

    pub async fn filter(&self, message: &mut Message) -> bool {
        let filter = self.filter.read().await;

        let mut prof = false;
        if filter.contains_profanity(&message.content) {
            Self::hide_message(message);
            prof = true;
        }
        if filter.contains_profanity(&message.sender) {
            Self::hide_message_sender(message);
            prof = true;
        }
        prof
    }
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
        let filter_path = config.prof_filter_file;
        r.manage(
            ProfFilter::new(filter_path)
                .await
                .expect("Failed to load profanity filter"),
        )
    })
}
