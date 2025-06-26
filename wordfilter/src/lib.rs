use std::collections::HashMap;

use bincode::{
    error::{DecodeError, EncodeError},
    Decode, Encode,
};
use wordprocessing::{process_data_to_words, Word};

//mod stemming;
mod wordprocessing;

#[derive(Clone, Debug, Encode, Decode)]
struct WordEntry {
    good: bool,
}
impl WordEntry {
    pub fn merge(&mut self, other: WordEntry) {
        self.good = self.good && other.good
    }
}

#[derive(Debug, Clone)]
pub enum CheckResult {
    Good,
    Unknown(Word),
    Bad(Word),
}

#[derive(Debug, Encode, Decode)]
pub struct WordFilter {
    hashmap: HashMap<Box<str>, WordEntry>,
}

impl WordFilter {
    pub fn empty() -> Self {
        Self {
            hashmap: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: WordFilter) {
        for (word, entry) in other.hashmap {
            self.hashmap
                .entry(word)
                .and_modify(|e| e.merge(entry.clone()))
                .or_insert_with(|| entry);
        }
    }

    pub fn entry_count(&self) -> usize {
        self.hashmap.len()
    }

    pub fn append_bin(&mut self, data: &[u8]) -> Result<(), DecodeError> {
        let (other, _) = bincode::decode_from_slice(data, bincode::config::standard())?;
        self.merge(other);
        Ok(())
    }
    pub fn save_string(&self) -> String {
        let mut string = "word;bad or good\n".to_string();
        for (word, entry) in self.hashmap.iter() {
            string.push_str(word);
            if entry.good {
                string.push_str(";good");
            } else {
                string.push_str(";bad");
            }
            string.push('\n');
        }
        string
    }
    pub fn save_bin(&self) -> Result<Vec<u8>, EncodeError> {
        bincode::encode_to_vec(&self, bincode::config::standard())
    }

    pub fn check(&self, data: &str) -> CheckResult {
        for word in process_data_to_words(data) {
            let Some(entry) = self.hashmap.get::<str>(word.str()) else {
                return CheckResult::Unknown(word);
            };
            if !entry.good {
                return CheckResult::Bad(word);
            }
        }
        CheckResult::Good
    }

    pub fn train(&mut self, good: bool, data: &str) {
        for word in process_data_to_words(data) {
            if !good {
                if let Some(entry) = self.hashmap.get(word.str()) {
                    if entry.good {
                        continue;
                    }
                }
            }
            self.hashmap.insert(word.into(), WordEntry { good });
        }
    }
}
impl Default for WordFilter {
    fn default() -> Self {
        Self::empty()
    }
}
