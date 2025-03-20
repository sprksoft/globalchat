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
            #[inline]
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

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MatchRule {
    pub tokens: Vec<crate::Token>,
    pub flags: RuleFlags,
}
impl Ord for MatchRule {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tokens.cmp(&other.tokens)
    }
}
impl PartialOrd for MatchRule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.tokens.partial_cmp(&other.tokens)
    }
}
impl MatchRule {
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
    pub fn to_string_friendly(&self) -> String {
        let mut str = String::with_capacity(self.tokens.len());
        for t in &self.tokens {
            if let Some(c) = t.to_friendly_char() {
                str.push(c);
            }
        }
        str
    }

    #[inline]
    pub fn filter(&self, other: &TokenizedMessage) -> Option<Range<usize>> {
        //println!("{:?}", self);
        let mut prev_char_check = None;
        let mut match_index = 0;
        if self.tokens[match_index].is_whitespace() {
            match_index += 1;
        }
        let mut t_me = self.tokens[match_index];
        let mut start_index = 0;
        for (index, t_other) in other.tokens().enumerate() {
            if !t_me.is_whitespace() && (t_other.is_unknown() || t_other.is_whitespace()) {
                continue;
            }
            if prev_char_check == Some(t_other) && !self.flags.contains(RuleFlags::NO_DEDUP) {
                continue;
            }
            prev_char_check = Some(t_other);
            //dbg!(index, start_index, match_index, t_other);
            if !t_other.contains(t_me) {
                match_index = 0;
                t_me = self.tokens[match_index];
            };

            if t_other.contains(t_me) {
                if match_index == 0 {
                    start_index = index;
                }
                match_index += 1;
                if match_index == self.tokens.len() {
                    //dbg!("match");
                    return Some(start_index..index + 1);
                }
                t_me = self.tokens[match_index];
            }
        }
        if match_index == self.tokens.len() - 1 && self.tokens[match_index].is_whitespace() {
            return Some(start_index..other.len());
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
        let rule = MatchRule::parse_from_str("abcd:no_dedup").unwrap();
        assert_eq!(
            rule,
            MatchRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: RuleFlags::NO_DEDUP
            },
            "no_dedup flag"
        );

        let rule = MatchRule::parse_from_str("abcd:").unwrap();
        assert_eq!(
            rule,
            MatchRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: RuleFlags::NONE
            },
            "no no_dedup flag"
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
    fn whitespace_non_character_match_test() {
        let mut filter = ProfanityFilter::empty();
        let rule = MatchRule::parse_from_str("/wabcd/w").unwrap();
        filter.insert_match_rule(rule);
        assert!(
            filter.check(&filter.tokenize("abcd").0).is_some(),
            "expected whitespace to match non characters"
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
        assert_eq!(result.unwrap().span, 1..7, "Start span is wrong");

        let result = filter.check(&filter.tokenize("Hi word a.b cd end words").0);
        assert_eq!(result.unwrap().span, 8..14, "Center span is wrong");
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

        rule.flags.insert(RuleFlags::NO_DEDUP);
        assert_eq!(rule.to_string(), "abcd:no_dedup".to_string());

        // rule.flags.insert(RuleFlags::NO_DEDUP);
        // assert_eq!(rule.to_string(), "abcd:word,no_dedup".to_string());
        //
        // rule.flags.remove(RuleFlags::WORD);
        // assert_eq!(rule.to_string(), "abcd:no_dedup".to_string());
    }
}
