use std::{collections::HashMap, time::SystemTime};

use crate::Tag;

#[derive(PartialEq, Eq, Debug)]
struct WordStatEntry {
    created_epoch_minutes: u32,
    count: usize,
}

#[derive(PartialEq, Eq, Debug)]
pub struct FilterStats {
    words: HashMap<(Box<str>, Tag), WordStatEntry>,
}
impl FilterStats {
    const MIN_AGE_MINUTES: u32 = 10080; // 7 days

    pub fn new() -> Self {
        Self {
            words: HashMap::new(),
        }
    }

    fn now_minutes() -> u32 {
        (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time older than unix_epoch")
            .as_secs()
            * 60) as u32
    }

    // Removes all old entries with a count of 1
    pub fn purge_stale(&mut self) {
        let now_minutes = Self::now_minutes();
        self.words.retain(|_, e| {
            e.count > 1 || now_minutes - e.created_epoch_minutes < Self::MIN_AGE_MINUTES
        })
    }

    pub fn calc_top(&self, tag: Tag, min_count: usize) -> Vec<(&str, usize)> {
        let mut top = Vec::new();
        for ((word, wtag), e) in self.words.iter() {
            if tag != *wtag {
                continue;
            }
            if e.count >= min_count {
                top.push((word.as_ref(), e.count));
            }
        }
        top.sort_by_key(|e| e.1);
        top
    }

    pub fn increment<'a>(&mut self, key: impl Into<(Box<str>, Tag)>, amount: usize) {
        if amount == 0 {
            return;
        }
        let key = key.into();
        let now_minutes = Self::now_minutes();
        self.words
            .entry(key)
            .and_modify(|e| e.count += amount)
            .or_insert(WordStatEntry {
                created_epoch_minutes: now_minutes,
                count: amount,
            });
    }

    pub fn from_string(str: &str) -> Self {
        fn get<'a, T: std::str::FromStr>(mut iter: impl Iterator<Item = &'a str>) -> Option<T> {
            iter.next().map(|i| i.parse::<T>().ok()).flatten()
        }

        let mut hashmap = HashMap::new();
        for line in str.split('\n') {
            let mut split = line.split(" ");
            let Some(word) = split.next() else {
                continue;
            };
            let Some(tag) = split
                .next()
                .map(|t| t.chars().next())
                .flatten()
                .map(|t| Tag::from_char(t))
                .flatten()
            else {
                continue;
            };

            let Some(count) = get::<usize>(&mut split) else {
                continue;
            };

            let Some(epoch) = get::<u32>(&mut split) else {
                continue;
            };

            hashmap.insert(
                (word.into(), tag),
                WordStatEntry {
                    created_epoch_minutes: epoch,
                    count,
                },
            );
        }
        Self { words: hashmap }
    }

    pub fn save_string(&self) -> String {
        let mut string = String::new();
        for ((word, tag), e) in self.words.iter() {
            string.push_str(word);
            string.push(' ');
            string.push(tag.char());
            string.push(' ');
            string.push_str(&e.count.to_string());
            string.push(' ');
            string.push_str(&e.created_epoch_minutes.to_string());
            string.push('\n');
        }
        string
    }
}

#[cfg(test)]
mod test {
    use crate::Tag;

    use super::FilterStats;

    #[test]
    fn to_from_str() {
        let mut stats = FilterStats::new();
        stats.increment(("word1".into(), Tag::Unknown), 1);
        stats.increment(("word2".into(), Tag::Unknown), 2);
        stats.increment(("word3".into(), Tag::Bad), 3);
        stats.increment(("word4".into(), Tag::Good), 4);

        let string = stats.save_string();
        let loaded = FilterStats::from_string(&string);

        assert_eq!(stats, loaded);
    }
}
