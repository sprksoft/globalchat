use std::collections::HashMap;

#[cfg(feature = "bincode")]
use bincode::{
    error::{DecodeError, EncodeError},
    Decode, Encode,
};

//mod stemming;
mod charprocessing;
mod tag;
mod wftime;
mod wordprocessing;
pub use charprocessing::*;
pub use tag::*;
pub use wftime::*;
pub use wordprocessing::*;
mod ansii;
pub mod stats;

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
    NormalizedWord::normalize(word)
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
                context.push(word.into());
            }

            if good_bad == "good" || good_bad == "g" {
                hashmap.insert(
                    word.into(),
                    WordEntry {
                        good: true,
                        forward_ctx: context,
                    },
                );
            } else if good_bad == "bad" || good_bad == "b" {
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
                string.push_str(" g");
            } else {
                string.push_str(" b");
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
    pub(crate) fn get_entry(&self, word: &NormalizedWord) -> Option<&WordEntry> {
        match self.hashmap.get(word.root()) {
            Some(entry) => Some(entry),
            None => match self.hashmap.get(word.str()) {
                Some(entry) => Some(entry),
                None => None,
            },
        }
    }

    pub fn check(&self, message: &str) -> TokenizedString {
        let mut ts = TokenizedString::tokenize(message);
        ts.recheck(self);
        ts
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
        for (_, _, norm_word) in ts.norm_words() {
            self.hashmap.insert(
                norm_word.clone().into(),
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
    use crate::{IntoWordTagPair, Tag, TokenizedString, WordFilter};

    fn filter() -> WordFilter {
        WordFilter::from_string(
            "f good haar\nk bad ben\nsibe good\nben good\nwacht good\nfuck bad\njij good\nldev good\nsmppgc good\n❤️ good\nu good\ni good",
        )
    }
    fn ts<'a>(words: impl IntoIterator<Item = impl IntoWordTagPair<'a, Tag>>) -> TokenizedString {
        TokenizedString::from_words(words)
    }

    #[test]
    fn check() {
        let filter = filter();

        assert_eq!(filter.check("ldev234"), ts([("ldev234", Tag::Unknown)]));
        assert_eq!(filter.check("fucking"), ts([("fucking", Tag::Bad)]));
        assert_eq!(
            filter.check("fucking\n"),
            ts([("fucking", Tag::Bad), ("\n", Tag::Whitespace)])
        );
        assert_eq!(
            filter.check("ben jij"),
            ts([
                ("ben", Tag::Good),
                (" ", Tag::Whitespace),
                ("jij", Tag::Good)
            ])
        );
    }

    #[test]
    fn context() {
        let filter = filter();

        assert_eq!(
            filter.check("la     la\n"),
            ts([
                ("la", Tag::Unknown),
                ("     ", Tag::Whitespace),
                ("la", Tag::Unknown),
                ("\n", Tag::Whitespace),
            ])
        );
        assert_eq!(filter.check("0x1d3f"), ts([("0x1d3f", Tag::Unknown)]));

        assert_eq!(
            filter.check("f wacht"),
            ts([
                ("f", Tag::Good),
                (" ", Tag::Whitespace),
                ("wacht", Tag::Good)
            ])
        );
        assert_eq!(
            filter.check("k ben sibe"),
            ts([
                ("k", Tag::Good),
                (" ", Tag::Whitespace),
                ("ben", Tag::Good),
                (" ", Tag::Whitespace),
                ("sibe", Tag::Good)
            ])
        );
        assert_eq!(
            filter.check("k ben"),
            ts([("k", Tag::Good), (" ", Tag::Whitespace), ("ben", Tag::Good)])
        );
        assert_eq!(
            filter.check("K BEN"),
            ts([("K", Tag::Good), (" ", Tag::Whitespace), ("BEN", Tag::Good)])
        );
        assert_eq!(filter.check("SIBE"), ts([("SIBE", Tag::Good)]));
        assert_eq!(filter.check("k"), ts([("k", Tag::Bad)]));
    }

    // #[test]
    // fn ghost_chars() {
    //     let filter = filter();
    //
    //     assert_eq!(filter.check(":smppgc:"), ts([(":smppgc:", Tag::Good),]));
    // }

    #[test]
    fn emoji() {
        let filter = filter();

        //❤️ is multiple characters
        assert_eq!(
            filter.check("i❤️u"),
            ts([("i", Tag::Good), ("❤️", Tag::Good), ("u", Tag::Good)])
        );
    }

    #[test]
    fn emoji_query() {
        let filter = WordFilter::from_string("❤️ g");
        let norm = crate::NormalizedWord::normalize("❤️");
        dbg!(&filter, &norm);
        assert!(filter.get_entry(&norm).is_some(),);
    }
}
