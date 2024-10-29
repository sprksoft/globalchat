use std::path::{Path, PathBuf};

use log::*;
use profanity::ProfanityFilter;
use rocket::fairing::AdHoc;
use rocket::serde::Deserialize;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use crate::chat::Message;

pub struct ProfFilter {
    //cache_file: String,
    wordlist_file: PathBuf,
    filter: RwLock<ProfanityFilter>,
}
impl ProfFilter {
    // async fn load_from_cache(cache_file: &str) -> Result<ProfanityFilter, bincode::Error> {
    //     let data = tokio::fs::read(cache_file).await?;
    //     bincode::deserialize(&data)
    // }
    pub async fn new(wordlist_file: PathBuf) -> Result<Self, bincode::Error> {
        // let mut cache_file = std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| {
        //     let mut str = std::env::var("HOME").expect("home env var not set");
        //     str.push_str("/.cache");
        //     str
        // });
        // cache_file.push_str("/smppgc");
        // cache_file.push_str("/profanity_tree");
        // tokio::fs::create_dir_all(&cache_file).await?;

        // let filter = match Self::load_from_cache(&cache_file).await {
        //     Ok(f) => f,
        //     Err(e) => {
        //         error!("Failed to load profanity tree from cache path. Generating a new one from wordlist ('{:?}'): {}", &wordlist_file, e);
        //         let wordlist = std::fs::read_to_string(&wordlist_file)?;
        //         let filter = ProfanityFilter::from_wordlist(&wordlist);
        //         tokio::fs::write(&cache_file, bincode::serialize(&filter)?).await?;
        //         filter
        //     }
        // };

        let wordlist = std::fs::read_to_string(&wordlist_file)?;
        let filter = ProfanityFilter::from_wordlist(&wordlist);
        Ok(Self {
            filter: filter.into(),
            wordlist_file,
            //cache_file,
        })
    }

    pub async fn add_word(&self, word: impl Into<Box<str>>) -> Result<(), bincode::Error> {
        let word = word.into();
        {
            let mut wordlist_file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(&self.wordlist_file)
                .await?;
            let string = format!("\"{}\"\n", word.clone());
            wordlist_file.write_all(string.as_bytes()).await?;
        }

        {
            self.filter.write().await.add_word(word)
        }
        // let filter = self.filter.read().await;
        // tokio::fs::write(&self.cache_file, bincode::serialize(&*filter)?).await?;
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
        }
    }
    fn hide_message(message: &mut Message) {
        message.content = "#".repeat(message.content.len()).into();
    }

    pub async fn filter(&self, message: &mut Message) {
        if self.contains_profanity(&message.content).await {
            Self::hide_message(message)
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct ProfConfig {
    prof_wordlist: PathBuf,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("profanity filter", |r| async {
        let config = r
            .figment()
            .extract::<ProfConfig>()
            .expect("No profanity config found");
        let mut wordlist = config.prof_wordlist;
        if wordlist.is_relative() {
            wordlist = Path::new(env!("CARGO_MANIFEST_DIR")).join(wordlist);
        }
        r.manage(
            ProfFilter::new(wordlist)
                .await
                .expect("Failed to load profanity filter"),
        )
    })
}
