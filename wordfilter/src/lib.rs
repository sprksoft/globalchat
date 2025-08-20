use std::collections::HashMap;

#[cfg(feature = "bincode")]
use bincode::{
    error::{DecodeError, EncodeError},
    Decode, Encode,
};
use wordprocessing::normalize_words;

//mod stemming;
mod wordprocessing;
pub use wordprocessing::Word;

#[cfg_attr(feature = "bincode", derive(Encode, Decode))]
#[derive(Clone, Debug)]
struct WordEntry {
    good: bool,
    forward_ctx: Vec<Box<str>>,
}
impl WordEntry {
    pub fn merge(&mut self, other: WordEntry) {
        self.good = self.good && other.good
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckResult {
    Good,
    Unknown(Word),
    Bad(Word),
}

#[cfg_attr(feature = "bincode", derive(Encode, Decode))]
#[derive(Debug)]
pub struct WordFilter {
    hashmap: HashMap<Box<str>, WordEntry>,
}

pub enum TrainResult {
    New,
    Changed,
    Unchanged,
}

impl WordFilter {
    pub fn empty() -> Self {
        Self {
            hashmap: HashMap::new(),
        }
    }
    pub fn from_string(str: &str) -> Self {
        let mut hashmap = HashMap::new();
        for line in str.split('\n') {
            let mut split = line.split(" ");
            let Some(word) = split.next() else {
                continue;
            };
            let Some(good_bad) = split.next() else {
                continue;
            };

            let mut context = Vec::new();
            for word in split {
                let Some(word) = normalize_words(word).next() else {
                    continue;
                };
                context.push(word.into());
            }

            let Some(word) = normalize_words(word).next() else {
                continue;
            };
            if good_bad == "good" {
                hashmap.insert(
                    word.into(),
                    WordEntry {
                        good: true,
                        forward_ctx: context,
                    },
                );
            } else if good_bad == "bad" {
                hashmap.insert(
                    word.into(),
                    WordEntry {
                        good: false,
                        forward_ctx: context,
                    },
                );
            }
        }
        Self { hashmap }
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

    #[cfg(feature = "bincode")]
    pub fn append_bin(&mut self, data: &[u8]) -> Result<(), DecodeError> {
        let (other, _) = bincode::decode_from_slice(data, bincode::config::standard())?;
        self.merge(other);
        Ok(())
    }
    pub fn save_string(&self) -> String {
        let mut string = String::new();
        for (word, entry) in self.hashmap.iter() {
            string.push_str(word);
            if entry.good {
                string.push_str(" good");
            } else {
                string.push_str(" bad");
            }
            for context in &entry.forward_ctx {
                string.push(' ');
                string.push_str(&context);
            }
            string.push('\n');
        }
        string
    }

    #[cfg(feature = "bincode")]
    pub fn save_bin(&self) -> Result<Vec<u8>, EncodeError> {
        bincode::encode_to_vec(&self, bincode::config::standard())
    }

    #[inline]
    fn get_entry(&self, word: &Word) -> Option<&WordEntry> {
        match self.hashmap.get(word.root()) {
            Some(entry) => Some(entry),
            None => match self.hashmap.get(word.str()) {
                Some(entry) => Some(entry),
                None => None,
            },
        }
    }

    pub fn check(&self, data: &str) -> CheckResult {
        let mut prev_entry: Option<(&WordEntry, Word)> = None;
        for word in normalize_words(data) {
            let Some(entry) = self.get_entry(&word) else {
                return CheckResult::Unknown(word);
            };
            if let Some((prev_entry, prev_word)) = prev_entry {
                if prev_entry
                    .forward_ctx
                    .iter()
                    .find(|c| c.as_ref() == word.root() || c.as_ref() == word.str())
                    .is_some()
                {
                    if prev_entry.good {
                        return CheckResult::Bad(prev_word);
                    }
                } else {
                    if !prev_entry.good {
                        return CheckResult::Bad(prev_word);
                    }
                }
            }
            prev_entry = Some((entry, word));
        }
        if let Some((prev_entry, prev_word)) = prev_entry {
            if !prev_entry.good {
                return CheckResult::Bad(prev_word);
            }
        }
        CheckResult::Good
    }

    pub fn train_word(&mut self, word: &str, good: bool) -> TrainResult {
        let Some(word) = normalize_words(word).next() else {
            return TrainResult::Unchanged;
        };
        match self.hashmap.insert(
            word.root().into(),
            WordEntry {
                good,
                forward_ctx: vec![],
            },
        ) {
            Some(old) => {
                if old.good == good {
                    TrainResult::Unchanged
                } else {
                    TrainResult::Changed
                }
            }
            None => TrainResult::New,
        }
    }

    pub fn train_good(&mut self, data: &str) {
        for word in normalize_words(data) {
            self.hashmap.insert(
                word.root().into(),
                WordEntry {
                    good: true,
                    forward_ctx: vec![],
                },
            );
        }
    }

    pub fn short_words(&self) -> Vec<(Box<str>, bool)> {
        let mut short_words = Vec::new();
        for (word, entry) in self.hashmap.iter() {
            if word.len() < 3 {
                short_words.push((word.clone(), entry.good))
            }
        }
        short_words
    }
}
impl Default for WordFilter {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod test {
    use crate::{CheckResult, Word, WordFilter};

    #[test]
    fn context() {
        let filter = WordFilter::from_string(
            "f good haar\nk bad ben\nsibe good\nben good\nwacht good\nfuck bad\njij good",
        );
        assert_eq!(
            filter.check("fucking"),
            CheckResult::Bad(Word::new_stemmed("fucking".to_string(), 0..7))
        );
        assert_eq!(filter.check("ben jij"), CheckResult::Good);
        assert_eq!(filter.check("f wacht"), CheckResult::Good);
        assert_eq!(filter.check("k ben sibe"), CheckResult::Good);
        assert_eq!(filter.check("k ben"), CheckResult::Good);
        assert_eq!(filter.check("K BEN"), CheckResult::Good);
        assert_eq!(filter.check("SIBE"), CheckResult::Good);
        assert_eq!(
            filter.check("k"),
            CheckResult::Bad(Word::new_stemmed("k".to_string(), 0..1))
        );
    }
}
