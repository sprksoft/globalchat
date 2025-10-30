use dashmap::DashMap;

use crate::{IntoWordTagPair, Tag, TokenizedString, WFTime};

#[derive(PartialEq, Eq, Debug)]
struct WordStatEntry {
    last_modified: WFTime,
    count: usize,
    tag: Tag,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug)]
pub struct Stat {
    pub word: Box<str>,
    pub tag: Tag,
    pub count: usize,
    pub last_modified: WFTime,
}
impl<'a> IntoWordTagPair<'a, Tag> for &'a Stat {
    fn into_word_tag_pair(self) -> (&'a str, Tag) {
        (self.word.as_ref(), self.tag)
    }
}

#[derive(Debug)]
pub struct WordFilterStats {
    words: DashMap<Box<str>, WordStatEntry>,
}
impl WordFilterStats {
    const MIN_AGE_MINUTES: u32 = 10080; // 7 days

    pub fn empty() -> Self {
        Self {
            words: DashMap::new(),
        }
    }

    // Removes all old entries with a count of 1
    pub fn purge_stale(&mut self) {
        let now = WFTime::now();
        self.words.retain(|_, e| {
            e.count > 1 || e.last_modified.duration_since(now) < Self::MIN_AGE_MINUTES
        });
    }

    pub fn calc_top(&self, min_count: usize, filter: &[Tag]) -> Vec<Stat> {
        let mut top = Vec::new();
        for kv in self.words.iter() {
            if !filter.contains(&kv.tag) {
                continue;
            }
            if kv.count >= min_count {
                top.push(Stat {
                    word: kv.key().clone(),
                    tag: kv.tag,
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
    pub fn record<const N: usize>(&self, ts: &TokenizedString, recorded_tags: [Tag; N]) -> bool {
        let mut recorded = false;
        for (word, tag) in ts.words() {
            if recorded_tags.contains(&tag) {
                self.record_word((word, tag), 1);
                recorded = true;
            }
        }
        recorded
    }

    pub fn record_word<'a>(&self, word: impl IntoWordTagPair<'a, Tag>, count: usize) {
        if count == 0 {
            return;
        }
        let pair = word.into_word_tag_pair();
        let mut entry = WordStatEntry {
            last_modified: WFTime::now(),
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
            let Some(tag) = get::<Tag>(&mut split) else {
                continue;
            };

            let Some(count) = get::<usize>(&mut split) else {
                continue;
            };

            let Some(epoch) = get::<WFTime>(&mut split) else {
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
            string.push(kv.tag.char());
            string.push(' ');
            string.push_str(&kv.count.to_string());
            string.push(' ');
            string.push_str(&kv.last_modified.to_string());
            string.push('\n');
        }
        string
    }
}
