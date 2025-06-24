use std::collections::HashMap;

//mod stemming;
mod wordprocessing;

struct Word {}

pub struct WordFilter {
    hashmap: HashMap<Box<str>, Word>,
}

impl WordFilter {
    pub fn empty() -> Self {
        Self {
            hashmap: HashMap::new(),
        }
    }

    pub fn check(&self, data: &str) -> bool {
        let mut good = true;
        Self::data_to_words(data, |w| {
            if self.hashmap.get(w.into_boxed_str().as_ref()).is_none() {
                good = false;
            }
        });
        good
    }

    pub fn train(&mut self, good: bool, data: &str) {
        if good {
            Self::data_to_words(data, |w| {
                self.hashmap.insert(w.into(), Word {});
            });
        }
    }
}
impl Default for WordFilter {
    fn default() -> Self {
        Self::empty()
    }
}
