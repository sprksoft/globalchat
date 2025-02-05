#![cfg_attr(test, feature(test))]
use std::{collections::HashMap, num::NonZeroU8};

use serde::{Deserialize, Serialize};
use string_tree::StringTree;

#[cfg(test)]
mod test;
#[cfg(test)]
mod wordlist;

mod rule;
use rule::ProfRule;

#[macro_export]
macro_rules! tokens {
    [$($c:literal),*] => {
        vec![$(crate::Token::from_char($c).unwrap()),*]
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Token(NonZeroU8);
impl Token {
    pub fn from_char(char: char) -> Option<Token> {
        if char.is_ascii_alphanumeric() {
            return Some(Self(NonZeroU8::new(char as u8).unwrap()));
        }
        None
    }
    pub fn new_whitespace() -> Token {
        Token(unsafe { NonZeroU8::new_unchecked(1) })
    }
    pub fn is_whitespace(&self) -> bool {
        self.0.get() == 1
    }
    pub fn to_char(self) -> char {
        self.0.get() as char
    }
}

pub struct TokenizedMessage(Vec<Token>);
impl TokenizedMessage {
    pub fn tokens(&self) -> std::slice::Iter<'_, Token> {
        self.0.iter()
    }
}

pub struct ProfanityFilter2 {
    rules: Vec<ProfRule>,
    char_to_token_map: HashMap<char, Token>,
}
impl ProfanityFilter2 {
    pub fn empty() -> Self {
        Self {
            rules: vec![],
            char_to_token_map: HashMap::new(),
        }
    }

    pub fn insert_rule(&self, new_rule: ProfRule) -> bool {
        for rule in self.rules.iter() {
            if rule == &new_rule {
                return false;
            }
        }

        true
    }

    pub fn find_matching(&self, msg: TokenizedMessage) -> Option<&ProfRule> {
        for rule in self.rules.iter() {
            if rule.matches(&msg) {
                return Some(rule);
            }
        }
        None
    }

    pub fn tokenize(&self, str: &str) -> (TokenizedMessage, String) {
        let mut new_str = String::with_capacity(str.len());
        let mut tokens = Vec::with_capacity(str.len());
        for char in str.chars() {
            if let Some(t) = self.char_to_token(char) {
                new_str.push(char);
                tokens.push(t);
            }
        }
        (TokenizedMessage(tokens), new_str)
    }

    fn char_to_token(&self, char: char) -> Option<Token> {
        if char.is_whitespace() {
            return Some(Token::new_whitespace());
        }
        if let Some(token) = self.char_to_token_map.get(&char) {
            return Some(*token);
        }

        if char.is_ascii_alphanumeric() {
            let mut u8_char = char as u8;
            if u8_char <= 90 && u8_char >= 65 {
                u8_char += 32;
            }
            //SAFETY: not null checks are above
            return Some(Token(unsafe { NonZeroU8::new_unchecked(u8_char) }));
        }

        None
    }
}

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
        //println!("{:?}", wordlist);
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
        b'!' => &[b'i', b'l', b'j'],
        b'i' => &[b'l', b'j'],
        b'l' => &[b'i', b'j'],
        b'j' => &[b'i', b'j'],

        b'1' => &[b'i', b'l', b'j'],
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
    let mut prev_char_check = None;
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

            if !char_sen.is_ascii_alphabetic()
                || prev_char_check
                    .map(|prev_char_check| char_equals_normalized(char_sen, prev_char_check))
                    .unwrap_or(false)
            {
                char_sen = match iter_sentence.next() {
                    Some(v) => v,
                    None => return char_check.is_ascii_whitespace(),
                };
                continue;
            }
            return false;
        }
        prev_char_check = Some(char_check);
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
