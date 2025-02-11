#![cfg_attr(test, feature(test))]

#[cfg(test)]
mod other_impls;

mod rule;
mod tokens;
use rule::{ProfRule, RuleLint};
use std::collections::HashMap;
use thiserror::Error;
use tokens::{Token, TokenGroup};

#[derive(Debug, Eq, PartialEq)]
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
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FilterLint {
    #[error("rule: {0} {1}")]
    Rule(usize, RuleLint),
    #[error("Possible dubble match between rule: {0} and {1}")]
    PossibleDubbleRule(usize, usize),
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
    pub fn parse_from_str(str: &str) -> Result<Self, ProfSyntaxErr> {
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
                me.parse_insert_char_replace_rule(line)
                    .map_err(|e| ProfSyntaxErr {
                        linenum: i + 1,
                        message: e,
                    })?;
            } else {
                me.insert_rule(ProfRule::parse_from_str(line).map_err(|e| ProfSyntaxErr {
                    linenum: i + 1,
                    message: format!("Error parsing rule: {}", e),
                })?);
            }
        }

        Ok(me)
    }
    fn parse_insert_char_replace_rule(&mut self, str: &str) -> Result<(), String> {
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

        let Some(tg) = TokenGroup::parse_from_str(&str[char_index + 1..]).map_err(|e| {
            format!(
                "Error parsing token group '{}': {}",
                &str[char_index + 1..],
                e
            )
        })?
        else {
            return Ok(());
        };
        self.char_to_token_map.insert(char, tg);
        Ok(())
    }
    pub fn insert_char_replace_rule(&mut self, char: char, tg: TokenGroup) {
        self.char_to_token_map.insert(char, tg);
    }

    pub fn insert_rule(&mut self, new_rule: ProfRule) {
        self.rules.push(new_rule);
    }
    pub fn lint(&self) -> Vec<FilterLint> {
        let mut lints = Vec::with_capacity(self.rules.len());
        for (i, rule) in self.rules.iter().enumerate() {
            for rule in rule.lint() {
                lints.push(FilterLint::Rule(i, rule));
            }
            let tm = self.tokenize_rule(rule);
            for (ii, rule2) in self.rules.iter().enumerate() {
                if ii != i && rule2.matches(&tm) {
                    lints.push(FilterLint::PossibleDubbleRule(i, ii));
                }
            }
        }

        lints
    }
    pub fn rule(&self, i: usize) -> &ProfRule {
        &self.rules[i]
    }

    pub fn filter(&self, msg: TokenizedMessage) -> Option<&ProfRule> {
        for rule in self.rules.iter() {
            if rule.matches(&msg) {
                return Some(rule);
            }
        }
        None
    }

    fn tokenize_rule(&self, rule: &ProfRule) -> TokenizedMessage {
        TokenizedMessage(
            rule.tokens
                .iter()
                .map(|t| TokenGroup::from_single(*t))
                .collect(),
        )
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

#[cfg(test)]
mod test {
    use crate::{
        rule::{ProfRule, ProfRuleFlags},
        tokens::{self, TokenGroup},
        tokens_ar, FilterLint, ProfanityFilter2, TokenizedMessage,
    };

    #[test]
    fn char_replace() {
        let mut filter = ProfanityFilter2::empty();
        filter.parse_insert_char_replace_rule("i: i, j").unwrap();
        assert_eq!(
            filter.char_to_token_map.get(&'i'),
            Some(TokenGroup::parse_from_str("i, j").unwrap().unwrap()).as_ref()
        )
    }
    #[test]
    fn lints() {
        let mut filter = ProfanityFilter2::empty();
        filter.insert_rule(ProfRule {
            tokens: tokens_ar!['s', 'e', 'x', 'y'],
            flags: ProfRuleFlags::NONE,
        });
        filter.insert_rule(ProfRule {
            tokens: tokens_ar!['s', 'e', 'x'],
            flags: ProfRuleFlags::NONE,
        });

        assert_eq!(filter.lint(), vec![FilterLint::PossibleDubbleRule(0, 1)]);
    }

    #[test]
    fn tokenize_test() {
        let filter = ProfanityFilter2::empty();

        assert_eq!(
            filter.tokenize("ik"),
            (
                TokenizedMessage(vec![
                    TokenGroup::parse_from_str("i").unwrap().unwrap(),
                    TokenGroup::parse_from_str("k").unwrap().unwrap()
                ]),
                "ik".to_string()
            )
        )
    }
}
