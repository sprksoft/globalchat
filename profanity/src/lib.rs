#![cfg_attr(test, feature(test))]
use std::{collections::HashMap, num::NonZeroU8, rc::Rc};

use serde::{Deserialize, Serialize};
use string_tree::StringTree;

#[cfg(test)]
mod test;
#[cfg(test)]
mod wordlist;

mod rule;
mod tokens;
use rule::ProfRule;
use tokens::{Token, TokenGroup};

pub struct TokenizedMessage(Vec<TokenGroup>);
impl TokenizedMessage {
    pub fn tokens(&self) -> std::slice::Iter<'_, TokenGroup> {
        self.0.iter()
    }
}

#[derive(Debug)]
pub struct ProfSyntaxErr {
    linenum: usize,
    message: String,
}
#[derive(Debug)]
pub struct ProfanityFilter2 {
    rules: Vec<ProfRule>,
    char_to_token_map: HashMap<char, TokenGroup>,
}
impl ProfanityFilter2 {
    pub fn empty() -> Self {
        Self {
            rules: vec![],
            char_to_token_map: HashMap::new(),
        }
    }
    pub fn from_str(str: &str) -> Result<Self, ProfSyntaxErr> {
        let mut char_replacements = true;
        let mut me = Self::empty();
        for (i, line) in str.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[RULES]" {
                char_replacements = false;
                continue;
            }
            if char_replacements {
                me.insert_char_replace_from_str(line)
                    .map_err(|e| ProfSyntaxErr {
                        linenum: i + 1,
                        message: e,
                    })?;
            } else {
                me.insert_rule(ProfRule::from_str(line).ok_or(ProfSyntaxErr {
                    linenum: i + 1,
                    message: "invalid rule".to_string(),
                })?);
            }
        }

        Ok(me)
    }
    fn insert_char_replace_from_str(&mut self, str: &str) -> Result<(), String> {
        let mut chars = str.char_indices();
        let (_, char) = chars.next().ok_or(format!("Expected char replace rule"))?;
        let (char_index, colon_char) = chars.next().ok_or(format!(
            "Expected a : after the target character in a char replace rule"
        ))?;
        if colon_char != ':' {
            return Err(format!(
                "Expected a : at the start of the char replace statement got '{}'",
                colon_char
            ));
        }
        self.char_to_token_map.insert(
            char,
            TokenGroup::from_str(&str[char_index + 1..])
                .ok_or(format!("Invalid token group '{}'", &str[char_index + 1..]))?,
        );
        Ok(())
    }
    pub fn insert_char_replace(&mut self, char: char, tg: TokenGroup) {
        self.char_to_token_map.insert(char, tg);
    }

    pub fn insert_rule(&mut self, new_rule: ProfRule) -> bool {
        let insert_index = self.rules.len();
        for rule in self.rules.iter() {
            if rule == &new_rule {
                return false;
            }
        }

        self.rules.insert(insert_index, new_rule);

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

    fn char_to_token(&self, char: char) -> Option<TokenGroup> {
        if let Some(tgroup) = self.char_to_token_map.get(&char).cloned() {
            return Some(tgroup);
        }

        TokenGroup::from_char(char)
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

#[inline]
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
