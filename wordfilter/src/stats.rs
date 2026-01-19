use std::str::FromStr;

use dashmap::DashMap;
use nanotime::NanoTime;

use crate::{IntoWordTagPair, TokenTag, TokenizedString};

#[derive(PartialEq, Eq, Debug)]
struct WordStatEntry<T> {
    last_modified: NanoTime,
    count: usize,
    tag: T,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug)]
pub struct Stat<T> {
    pub word: Box<str>,
    pub tag: T,
    pub count: usize,
    pub last_modified: NanoTime,
}
impl<'a, T: Clone> IntoWordTagPair<'a, T> for &'a Stat<T> {
    fn into_word_tag_pair(self) -> (&'a str, T) {
        (self.word.as_ref(), self.tag.clone())
    }
}

#[derive(Debug)]
pub struct WordFilterStats<T> {
    words: DashMap<Box<str>, WordStatEntry<T>>,
}
impl<T: Eq + Clone + TokenTag> WordFilterStats<T> {
    // const MIN_AGE_MINUTES: u32 = 10080; // 7 days

    pub fn empty() -> Self {
        Self {
            words: DashMap::new(),
        }
    }

    // // Removes all old entries with a count of 1
    // pub fn purge_stale(&mut self) {
    //     let now = NanoTime::now();
    //     self.words.retain(|_, e| {
    //         e.count > 1 || e.last_modified.duration_since(now) < Self::MIN_AGE_MINUTES
    //     });
    // }

    pub fn calc_top(&self, min_count: usize, filter: &[T]) -> Vec<Stat<T>> {
        let mut top = Vec::new();
        for kv in self.words.iter() {
            if !filter.contains(&kv.tag) {
                continue;
            }
            if kv.count >= min_count {
                top.push(Stat {
                    word: kv.key().clone(),
                    tag: kv.tag.clone(),
                    count: kv.count,
                    last_modified: kv.last_modified,
                });
            }
        }
        top.sort_by_key(|e| std::cmp::Reverse(e.count));
        top
    }

    // Record all the words that have a tag that appears in the recorded_tags array
    // Returns true when stats have been recorded
    pub fn record<const N: usize>(&self, ts: &TokenizedString<T>, recorded_tags: [T; N]) -> bool {
        let mut recorded = false;
        for (word, tag) in ts.words() {
            if recorded_tags.contains(&tag) {
                self.record_word((word, tag), 1);
                recorded = true;
            }
        }
        recorded
    }

    pub fn record_word<'a>(&self, word: impl IntoWordTagPair<'a, T>, count: usize) {
        if count == 0 {
            return;
        }
        let pair = word.into_word_tag_pair();
        let mut entry = WordStatEntry {
            last_modified: NanoTime::now(),
            count,
            tag: pair.1,
        };

        match self.words.get_mut(pair.0) {
            Some(mut e) => {
                entry.count += e.count;
                *e = entry;
            }
            None => {
                self.words.insert(pair.0.into(), entry);
            }
        }
    }
}

impl<T: FromStr + Into<char> + Clone> WordFilterStats<T> {
    pub fn from_string(str: &str) -> Self {
        fn get<'a, T: std::str::FromStr>(mut iter: impl Iterator<Item = &'a str>) -> Option<T> {
            iter.next().map(|i| i.parse::<T>().ok()).flatten()
        }

        let words = DashMap::new();
        for line in str.split('\n') {
            let mut split = line.split(" ");
            let Some(word) = split.next() else {
                continue;
            };
            let Some(tag) = get::<T>(&mut split) else {
                continue;
            };

            let Some(count) = get::<usize>(&mut split) else {
                continue;
            };

            let Some(epoch) = get::<NanoTime>(&mut split) else {
                continue;
            };

            words.insert(
                word.into(),
                WordStatEntry {
                    tag,
                    last_modified: epoch,
                    count,
                },
            );
        }
        Self { words }
    }

    pub fn save_string(&self) -> String {
        let mut string = String::new();
        for kv in self.words.iter() {
            string.push_str(kv.key());
            string.push(' ');
            string.push(kv.tag.clone().into());
            string.push(' ');
            string.push_str(&kv.count.to_string());
            string.push(' ');
            string.push_str(&kv.last_modified.to_string());
            string.push('\n');
        }
        string
    }
}
