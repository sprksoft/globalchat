#![cfg_attr(test, feature(test))]
use serde::{Deserialize, Serialize};
use string_tree::StringTree;

#[cfg(test)]
mod test;
#[cfg(test)]
mod wordlist;

#[derive(Serialize, Deserialize)]
pub struct ProfanityFilter {
    tree: StringTree,
}
impl ProfanityFilter {
    pub fn from_wordlist(wordlist: &str) -> Self {
        let words = wordlist.lines().map(|l| l.to_lowercase()).collect();
        Self {
            tree: StringTree::from_vec(words),
        }
    }
    pub fn add_word(&mut self, word: impl Into<Box<str>>) {
        self.tree.add(word.into(), 0);
    }

    pub fn contains_profanity(&self, string: &str) -> bool {
        sentence_contains(&self.tree, string)
    }
}

fn matches(sentence: &str, check: &str) -> bool {
    if check.len() > sentence.len() {
        return false;
    }
    let mut iter_check = check.bytes();
    let mut iter_sentence = sentence.bytes();
    loop {
        let Some(char_check) = iter_check.next() else {
            return true;
        };
        let Some(mut char_sen) = iter_sentence.next() else {
            return true;
        };
        loop {
            if char_sen == char_check {
                break;
            }
            if char_sen.is_ascii_whitespace() {
                char_sen = match iter_sentence.next() {
                    Some(v) => v,
                    None => return true,
                };
                continue;
            }
            return false;
        }
    }
}

fn sentence_contains(tree: &StringTree, sentence: &str) -> bool {
    let mut index = 0;
    let sentence = sentence.to_lowercase();
    loop {
        let result = tree.contains(&|check: &str, i| matches(&sentence[index + i..], &check[i..]));
        if result.0 {
            return true;
        }
        index += result.1 + 1;
        if index >= sentence.len() {
            return false;
        }
    }
}

fn sentence_contains_naive(tree: &Vec<String>, sentence: &str) -> bool {
    let sentence = sentence.to_lowercase();
    for i in 0..sentence.len() {
        for item in tree.iter() {
            //println!("check '{}' starts with '{}'", &sentence[i..], item);
            if matches(&sentence[i..], &item) {
                //println!("MATCH '{}' starts with '{}'", &sentence[i..], item);
                return true;
            }
        }
    }
    false
}
