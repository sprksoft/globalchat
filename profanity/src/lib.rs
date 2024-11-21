#![cfg_attr(test, feature(test))]
use serde::{Deserialize, Serialize};
use string_tree::StringTree;

#[cfg(test)]
mod test;
#[cfg(test)]
mod wordlist;

#[derive(Serialize, Deserialize)]
pub struct ProfanityFilter {
    wordlist: Vec<Box<str>>,
}
impl ProfanityFilter {
    pub fn from_wordlist(wordlist: &str) -> Self {
        let wordlist = wordlist
            .lines()
            .map(|l| l.trim_matches(['"']).to_lowercase())
            .filter(|i| i.len() > 0)
            .map(|s| Box::from(s))
            .collect();
        //println!("{:?}", words);
        Self { wordlist }
    }
    pub fn add_word(&mut self, word: impl Into<Box<str>>) {
        self.wordlist.push(word.into());
    }

    pub fn contains_profanity(&self, string: &str) -> bool {
        sentence_contains_loop(&self.wordlist, string)
    }
}

fn char_equals_normalized(nchar: u8, cchar: u8) -> bool {
    if nchar == cchar {
        return true;
    }

    let a: &[u8] = match nchar {
        b'!' => &[b'i', b'l'],
        b'i' => &[b'l'],
        b'l' => &[b'i'],

        b'1' => &[b'i', b'l'],
        b'3' => &[b'e'],
        b'4' => &[b'a'],
        b'6' => &[b'g'],

        b'@' => &[b'a', b'e', b'i', b'o', b'u'],
        b'*' => &[b'a', b'e', b'i', b'o', b'u'],
        b'$' => &[b'4', b's'],
        _ => &[],
    };
    a.contains(&cchar)
}

fn matches(sentence: &str, check: &str) -> bool {
    let mut iter_check = check.bytes();
    let mut iter_sentence = sentence.bytes();
    //println!("matching {} {}", sentence, check);
    loop {
        let Some(char_check) = iter_check.next() else {
            return true;
        };
        let Some(mut char_sen) = iter_sentence.next() else {
            return char_check.is_ascii_whitespace();
        };
        loop {
            if char_equals_normalized(char_sen, char_check) {
                // println!(
                //     "normalize check between {}={}",
                //     char::from_u32(char_sen as u32).unwrap(),
                //     char::from_u32(char_check as u32).unwrap()
                // );
                break;
            }

            if !char_sen.is_ascii_alphabetic() {
                char_sen = match iter_sentence.next() {
                    Some(v) => v,
                    None => return char_check.is_ascii_whitespace(),
                };
                continue;
            }
            return false;
        }
    }
}

fn sentence_contains_tree(tree: &StringTree, sentence: &str) -> bool {
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

fn sentence_contains_loop(wordlist: &Vec<Box<str>>, sentence: &str) -> bool {
    let sentence = sentence.to_lowercase();
    for i in 0..sentence.len() {
        if !sentence.is_char_boundary(i) {
            continue;
        }
        for item in wordlist.iter() {
            if matches(&sentence[i..], &item) {
                // println!(
                //     "MATCH '{}' starts with '{}' index: {}",
                //     &sentence[i..],
                //     item,
                //     i
                // );
                return true;
            }
        }
    }
    false
}
