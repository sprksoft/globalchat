#![cfg_attr(test, feature(test))]
use std::{collections::HashMap, ops::Range};

#[cfg(test)]
mod other_impls;

mod rules;
mod tokens;

pub use rules::*;
pub use tokens::*;

#[cfg(feature = "serde")]
mod json;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct TokenizedMessage(Vec<TokenGroup>);
impl TokenizedMessage {
    pub fn tokens(&self) -> std::slice::Iter<'_, TokenGroup> {
        self.0.iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct FilterMatch<'a> {
    pub rule: &'a MatchRule,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub struct ProfanityFilter {
    rules: Vec<MatchRule>,
    char_to_token_map: HashMap<char, TokenGroup>,
}

impl ProfanityFilter {
    pub fn empty() -> Self {
        Self {
            rules: vec![],
            char_to_token_map: HashMap::new(),
        }
    }

    pub fn parse_from_str(str: &str) -> Result<Self, ParseRuleError> {
        let mut me = Self::empty();
        for line in str.lines() {
            let line = &line[..line.find('#').unwrap_or(line.len())];
            if line.is_empty() {
                continue;
            }
            me.insert_rule(Rule::parse_from_str(&line)?);
        }
        Ok(me)
    }

    pub fn insert_rule(&mut self, rule: Rule) {
        match rule {
            Rule::Replace(r) => self.insert_rep_rule(r),
            Rule::Match(m) => self.insert_match_rule(m),
        }
    }

    pub fn insert_rep_rule(&mut self, new_rule: RepRule) {
        for token in new_rule.match_chars.chars() {
            self.char_to_token_map
                .insert(token, new_rule.replace_tg.clone());
        }
    }

    pub fn insert_match_rule(&mut self, new_rule: MatchRule) {
        self.rules.push(new_rule);
    }
    pub fn rule(&self, i: usize) -> &MatchRule {
        &self.rules[i]
    }
    pub fn match_rules(&self) -> &[MatchRule] {
        &self.rules
    }

    pub fn check(&self, msg: &TokenizedMessage) -> Option<FilterMatch> {
        for rule in self.rules.iter() {
            if let Some(span) = rule.filter(&msg) {
                return Some(FilterMatch { rule: rule, span });
            }
        }
        None
    }
    pub fn check_all(&self, msg: &TokenizedMessage) -> Vec<FilterMatch> {
        let mut matches = vec![];
        for rule in self.rules.iter() {
            if let Some(span) = rule.filter(&msg) {
                matches.push(FilterMatch { rule: rule, span });
            }
        }
        matches
    }

    pub fn tokenize_match_rule(&self, rule: &MatchRule) -> TokenizedMessage {
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
            } else {
                tokens.push(TokenGroup::from_single(Token::new_unknown()));
            }
        }
        (TokenizedMessage(tokens), new_str)
    }

    fn char_to_token(&self, char: char) -> Option<TokenGroup> {
        let mut tg = TokenGroup::from_char(char)?;
        if let Some(tgroup) = self.char_to_token_map.get(&char).cloned() {
            tg.extend(tgroup);
        }
        Some(tg)
    }
}

#[cfg(test)]
mod test {
    use crate::{rules::RepRule, tokens::TokenGroup, ProfanityFilter, TokenizedMessage};

    #[test]
    fn char_replace() {
        let mut filter = ProfanityFilter::empty();
        filter.insert_rep_rule(RepRule::parse_from_str("i => ij").unwrap());
        assert_eq!(
            filter.char_to_token_map.get(&'i'),
            Some(TokenGroup::parse_from_str("ij").unwrap()).as_ref()
        )
    }

    #[test]
    fn tokenize_test() {
        let filter = ProfanityFilter::empty();

        assert_eq!(
            filter.tokenize("ik"),
            (
                TokenizedMessage(vec![
                    TokenGroup::parse_from_str("i/k").unwrap(),
                    TokenGroup::parse_from_str("k").unwrap()
                ]),
                "ik".to_string()
            )
        )
    }
}
