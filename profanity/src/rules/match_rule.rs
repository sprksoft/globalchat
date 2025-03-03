use std::ops::Range;

use crate::{tokens::TokenParseError, Token, TokenizedMessage};
use bitflags::bitflags;
use thiserror::Error;

pub trait Flags {
    fn none() -> Self;
    fn set_from_str(&mut self, str: &str) -> bool;
    fn append_to_string(&self, string: &mut String);
    fn flags_info() -> &'static [(&'static str, u8, &'static str)];
}

macro_rules! flags {
    ($vis:vis flags $name:ident{$($flagname:ident:$flagstring:literal:$flagbits:literal:$flagdesc:literal;)*}) => {
        bitflags! {
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub struct $name: u8 {
                const NONE = 0b00000000;
                $(
                    const $flagname = $flagbits;
                )*
            }
        }
        impl Flags for $name {
            fn none() -> Self {
                Self::NONE
            }
            fn flags_info() -> &'static [(&'static str, u8, &'static str)]{
                &[
                    $(
                        ($flagstring, $flagbits, $flagdesc)
                    ),*
                ]
            }
            fn set_from_str(&mut self, str: &str) -> bool {
                match str {
                    $(
                    $flagstring => {
                        self.insert(Self::$flagname);
                        true
                    }
                    ),*
                    _ => false,
                }
            }
            #[allow(unused_assignments)]
            fn append_to_string(&self, string: &mut String) {
                let mut first = true;
                $(
                    if self.contains(Self::$flagname) {
                        if !first {
                            string.push(',');
                        }
                        string.push_str($flagstring);
                        first = false;
                    }
                )*
            }
        }
        #[cfg(feature="serde")]
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeSeq;
                let mut seq_ser = serializer.serialize_seq(None)?;
                $(
                    if (self.contains(Self::$flagname)){
                        seq_ser.serialize_element($flagstring)?;
                    }
                )*
                seq_ser.end()
            }
        }
        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_seq(crate::json::FlagsVisitor(std::marker::PhantomData::<Self>::default()))
            }
        }
    };
}

flags! {
    pub flags RuleFlags {
        WORD:"word":0b00000001:"Only match words that are separated by whitespace. 'ass' will not match 'password' when this is on";
        NO_DEDUP:"no_dedup":0b00000010:"Don't deduplicate characters. 'potato' will not match 'pottttttato' when this is on";
    }
}

#[derive(Clone, Debug, Error)]
pub enum MatchRuleParseError {
    #[error("Unknown flag '{0}'")]
    UnknownFlag(Box<str>),
    #[error("{0}")]
    TokenParseError(#[from] TokenParseError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuleLint {
    #[error(
        "Rule ends in -en. Ex. godverdomen will not match godverdome. (Replace -en suffix with -e)"
    )]
    En,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MatchRule {
    pub tokens: Vec<crate::Token>,
    pub flags: RuleFlags,
}
impl MatchRule {
    pub fn lint(&self) -> Vec<RuleLint> {
        let mut lints = Vec::new();
        if self.tokens.ends_with(&[
            Token::from_char('e').unwrap(),
            Token::from_char('n').unwrap(),
        ]) {
            lints.push(RuleLint::En)
        }
        lints
    }
    pub fn from_match_str(str: &str) -> Result<Self, MatchRuleParseError> {
        Ok(Self {
            flags: RuleFlags::NONE,
            tokens: Token::parse_multiple(str, true)?,
        })
    }

    pub fn parse_from_str<'a>(str: &'a str) -> Result<Self, MatchRuleParseError> {
        if let Some(sep_index) = str.find(':') {
            let mut me = Self::from_match_str(&str[..sep_index])?;
            //dbg!(str);
            let flag_region = &str[sep_index + 1..];
            //dbg!(flag_region);
            for flag in flag_region.split(',') {
                let flag = flag.trim();
                if flag.len() == 0 {
                    continue;
                }
                if !me.flags.set_from_str(flag) {
                    return Err(MatchRuleParseError::UnknownFlag(flag.into()));
                }
            }
            if me.flags.contains(RuleFlags::NO_DEDUP) {
                me.tokens = Token::parse_multiple(&str[..sep_index], false)?;
            }
            Ok(me)
        } else {
            Self::from_match_str(&str)
        }
    }
    pub fn to_string(&self) -> String {
        let mut str = String::with_capacity(2 + self.tokens.len());
        for token in self.tokens.iter() {
            str.push_str(&token.to_string())
        }
        str.push(':');
        self.flags.append_to_string(&mut str);

        str
    }

    #[inline]
    pub fn filter(&self, other: &TokenizedMessage) -> Option<Range<usize>> {
        //println!("{:?}", self);
        let mut prev_char_check = None;
        let mut match_index = 0;
        let mut t_me = self.tokens[match_index];
        let mut start_index = 0;
        for (index, t_other) in other.tokens().enumerate() {
            if t_other.is_unknown() {
                continue;
            }
            if t_other.is_whitespace() && !self.flags.contains(RuleFlags::WORD) {
                continue;
            }
            if prev_char_check == Some(t_other) && !self.flags.contains(RuleFlags::NO_DEDUP) {
                continue;
            }
            prev_char_check = Some(t_other);
            //println!("{:?} {:?}", t_me, t_other);
            if t_other.contains(t_me) {
                //println!("rule: {:?} token: {:?}", t_me, t_other);
                match_index += 1;
                if match_index == self.tokens.len() {
                    return Some(start_index..index + 1);
                }
                t_me = self.tokens[match_index];
            } else {
                start_index = index;
                match_index = 0;
                t_me = self.tokens[match_index];
                if t_other.contains(t_me) {
                    match_index += 1;
                    if match_index == self.tokens.len() {
                        return Some(start_index..index + 1);
                    }
                    t_me = self.tokens[match_index];
                }

                //println!("reset");
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use crate::{
        rules::{MatchRule, RuleFlags},
        tokens_ar, ProfanityFilter,
    };

    #[test]
    fn parse() {
        let rule = MatchRule::parse_from_str("abcd:word").unwrap();
        assert_eq!(
            rule,
            MatchRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: RuleFlags::WORD
            },
            "wordmatch flag"
        );

        let rule = MatchRule::parse_from_str("abcd:").unwrap();
        assert_eq!(
            rule,
            MatchRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: RuleFlags::NONE
            },
            "no wordmatch flag"
        );
    }

    #[test]
    fn match_after_half_match() {
        let mut filter = ProfanityFilter::empty();
        let rule = MatchRule::parse_from_str("abcd").unwrap();
        filter.insert_match_rule(rule);

        assert!(
            filter.check(&filter.tokenize("ab abcd qa").0).is_some(),
            "match after half match failed"
        );
    }

    #[test]
    fn wordmatch() {
        let mut filter = ProfanityFilter::empty();
        let rule = MatchRule::parse_from_str("abcd:word").unwrap();
        filter.insert_match_rule(rule);
        assert!(
            filter
                .check(&filter.tokenize("Hi word a.b cd end words").0)
                .is_none(),
            "expected word flag to not match whitespace delimited"
        );

        let mut filter = ProfanityFilter::empty();
        let rule = MatchRule::parse_from_str("abcd").unwrap();
        filter.insert_match_rule(rule);
        assert!(
            filter
                .check(&filter.tokenize("Hi word a.b cd end words").0)
                .is_some(),
            "expected non word flag to match whitespace delimited"
        );
    }

    #[test]
    fn span() {
        let mut filter = ProfanityFilter::empty();
        let rule = MatchRule::parse_from_str("abcd").unwrap();
        filter.insert_match_rule(rule);

        let result = filter.check(&filter.tokenize("abcd dq").0);
        assert_eq!(result.unwrap().span, 0..4, "Span is wrong");

        let result = filter.check(&filter.tokenize("ab a.a_bc d").0);
        assert_eq!(result.unwrap().span, 3..11, "End span is wrong");

        let result = filter.check(&filter.tokenize(".a_bc d aaaaa").0);
        assert_eq!(result.unwrap().span, 0..7, "Start span is wrong");

        let result = filter.check(&filter.tokenize("Hi word a.b cd end words").0);
        assert_eq!(result.unwrap().span, 6..14, "Center span is wrong");
    }

    #[test]
    fn dedup_test() {
        let rule = MatchRule::parse_from_str("abbbcd").unwrap();
        assert_eq!(
            rule,
            MatchRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: RuleFlags::NONE
            }
        );
    }

    #[test]
    fn dont_dedup_wordmatch() {
        let rule = MatchRule::parse_from_str("dedddup:no_dedup").unwrap();
        assert_eq!(
            rule,
            MatchRule {
                tokens: tokens_ar!['d', 'e', 'd', 'd', 'd', 'u', 'p'],
                flags: RuleFlags::NO_DEDUP
            }
        )
    }

    #[test]
    fn to_str() {
        let mut rule = MatchRule {
            flags: RuleFlags::NONE,
            tokens: tokens_ar!['a', 'b', 'c', 'd'],
        };
        assert_eq!(rule.to_string(), "abcd:".to_string());

        rule.flags.insert(RuleFlags::WORD);
        assert_eq!(rule.to_string(), "abcd:word".to_string());

        rule.flags.insert(RuleFlags::NO_DEDUP);
        assert_eq!(rule.to_string(), "abcd:word,no_dedup".to_string());

        rule.flags.remove(RuleFlags::WORD);
        assert_eq!(rule.to_string(), "abcd:no_dedup".to_string());
    }
}
