use std::collections::HashMap;

#[cfg(feature = "bincode")]
use bincode::{
    error::{DecodeError, EncodeError},
    Decode, Encode,
};
use wordprocessing::NormalizedWord;

//mod stemming;
mod wordprocessing;
pub use wordprocessing::TokenizedString;
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

fn normalize_word(word: &str) -> NormalizedWord {
    Word::from_str(word).normalize()
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
                let word = normalize_word(word);
                context.push(word.into());
            }

            let word = normalize_word(word);
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
    pub fn get_entry(&self, word: &NormalizedWord) -> Option<&WordEntry> {
        match self.hashmap.get(word.root()) {
            Some(entry) => Some(entry),
            None => match self.hashmap.get(word.str()) {
                Some(entry) => Some(entry),
                None => None,
            },
        }
    }

    pub fn train_word(&mut self, word: &str, good: bool) -> TrainResult {
        let word = normalize_word(word);
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
        let ts = TokenizedString::tokenize(data);
        for word in ts.words() {
            self.hashmap.insert(
                word.normalize().into(),
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
    use crate::{CheckResult, TokenizedString, Word, WordFilter};

    #[test]
    fn context() {
        let filter = WordFilter::from_string(
            "f good haar\nk bad ben\nsibe good\nben good\nwacht good\nfuck bad\njij good",
        );
        assert_eq!(
            filter.check(&TokenizedString::tokenize("fucking")),
            CheckResult::Bad(Word::from_str("fucking"))
        );
        assert_eq!(filter.check_str("ben jij"), CheckResult::Good);
        assert_eq!(filter.check_str("f wacht"), CheckResult::Good);
        assert_eq!(filter.check_str("k ben sibe"), CheckResult::Good);
        assert_eq!(filter.check_str("k ben"), CheckResult::Good);
        assert_eq!(filter.check_str("K BEN"), CheckResult::Good);
        assert_eq!(filter.check_str("SIBE"), CheckResult::Good);
        assert_eq!(filter.check_str("k"), CheckResult::Bad(Word::from_str("k")));
    }
}
