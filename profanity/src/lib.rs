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

macro_rules! char_match_table {
    ($char1:ident, $char2:ident, $($char:literal=>[$($match_option:literal),*]),*) => {
        match $char1{
        $($char => $($match_option == $char2 ||)* true,)*
        _=>false
        }
    };
}

fn char_equals_normalized(char: u8, other: u8) -> bool {
    if char == other {
        return true;
    }
    char_match_table!(char, other,
        b'!' => [b'i', b'l'],
        b'i' => [b'l'],
        b'l' => [b'i'],
        b'1' => [b'i', b'l'],

        b'3' => [b'e'],
        b'$' => [b'4']
    )
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
        for item in wordlist.iter() {
            //println!("check '{}' starts with '{}'", &sentence[i..], item);
            if matches(&sentence[i..], &item) {
                //println!("MATCH '{}' starts with '{}'", &sentence[i..], item);
                return true;
            }
        }
    }
    false
}
