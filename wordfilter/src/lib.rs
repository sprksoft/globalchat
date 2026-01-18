use std::collections::HashMap;

//mod stemming;
mod charprocessing;
mod tag;
mod wordprocessing;
pub use charprocessing::*;
pub use tag::*;
pub use wordprocessing::*;
mod ansii;
pub mod stats;

pub trait FilterMeta: Clone {
    fn read(str: &str) -> Self;
    fn write(&self, string: &mut String);
}
impl FilterMeta for () {
    fn write(&self, _: &mut String) {}
    fn read(_: &str) -> Self {
        ()
    }
}

#[derive(Clone, Debug)]
struct WordEntry<M> {
    good: bool,
    forward_ctx: Vec<Box<str>>,
    meta: M,
}
impl<M> WordEntry<M> {
    pub fn merge(&mut self, other: WordEntry<M>) {
        self.good = self.good && other.good
    }
}

#[derive(Debug)]
pub struct WordFilter<M = ()> {
    hashmap: HashMap<Box<str>, WordEntry<M>>,
}

pub enum TrainResult {
    New,
    Changed,
    Unchanged,
}

impl<M: FilterMeta> WordFilter<M> {
    pub fn empty() -> Self {
        Self {
            hashmap: HashMap::new(),
        }
    }
    pub fn from_string(str: &str) -> Self {
        let mut hashmap = HashMap::new();
        for line in str.split('\n') {
            let (line, meta) = line.split_once('\0').unwrap_or((line, ""));
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
            let meta = M::read(meta);
            if good_bad == "good" || good_bad == "g" {
                hashmap.insert(
                    word.into(),
                    WordEntry {
                        good: true,
                        forward_ctx: context,
                        meta,
                    },
                );
            } else if good_bad == "bad" || good_bad == "b" {
                hashmap.insert(
                    word.into(),
                    WordEntry {
                        good: false,
                        forward_ctx: context,
                        meta,
                    },
                );
            }
        }
        Self { hashmap }
    }

    pub fn merge(&mut self, other: WordFilter<M>) {
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
            string.push('\0');
            entry.meta.write(&mut string);

            string.push('\n');
        }
        string
    }

    #[inline]
    pub(crate) fn get_entry(&self, word: &NormalizedWord) -> Option<&WordEntry<M>> {
        match self.hashmap.get(word.root()) {
            Some(entry) => Some(entry),
            None => match self.hashmap.get(word.str()) {
                Some(entry) => Some(entry),
                None => None,
            },
        }
    }

    pub fn check<T: TokenTag<M>>(&self, message: &str) -> TokenizedString<T> {
        let mut ts = TokenizedString::<T>::tokenize(message);
        ts.recheck(self);
        ts
    }

    pub fn meta<W: IntoNormalizedWord>(&self, word: &W) -> Option<&M> {
        self.get_entry(&word.into_normalized_word())
            .map(|e| &e.meta)
    }

    /// Returns an error if the word was not found.
    pub fn edit_meta<W: IntoNormalizedWord>(
        &mut self,
        word: &W,
        edit_fn: impl FnOnce(&mut M),
    ) -> Result<(), ()> {
        let word = word.into_normalized_word();
        let (key, mut entry) = self.hashmap.remove_entry(word.root()).ok_or(())?;
        edit_fn(&mut entry.meta);
        self.hashmap.insert(key, entry);
        Ok(())
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
impl<M: FilterMeta + Default> WordFilter<M> {
    pub fn train_good<T: TokenTag<M>>(&mut self, data: &str) {
        let ts = TokenizedString::<T>::tokenize(data);
        for (_, _, norm_word) in ts.norm_words() {
            self.hashmap.insert(
                norm_word.clone().into(),
                WordEntry {
                    good: true,
                    forward_ctx: vec![],
                    meta: M::default(),
                },
            );
        }
    }

    pub fn train_word<W: IntoNormalizedWord + Sized>(
        &mut self,
        word: &W,
        good: bool,
    ) -> TrainResult {
        let word = word.into_normalized_word();
        let word = word.root().into();

        let meta = self
            .hashmap
            .remove::<Box<str>>(&word)
            .map(|e| e.meta)
            .unwrap_or_else(|| M::default());

        match self.hashmap.insert(
            word,
            WordEntry {
                good,
                forward_ctx: vec![],
                meta,
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
}

impl<M: Default + FilterMeta> Default for WordFilter<M> {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod test {
    use crate::{IntoWordTagPair, Tag, TokenizedString, WordFilter};

    fn filter() -> WordFilter {
        WordFilter::from_string(
            "f good haar\nk bad ben\nsibe good\nben good\nwacht good\nfuck bad\njij good\nldev good\nsmppgc good\n❤️ good\nu good\ni good\n69 bad",
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
    fn number_check() {
        let filter = filter();

        assert_eq!(filter.check("123.5%"), ts([("123.5%", Tag::Good)]));
        assert_eq!(filter.check("5€"), ts([("5€", Tag::Good)]));
        assert_eq!(filter.check("69"), ts([("69", Tag::Bad)]));
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
