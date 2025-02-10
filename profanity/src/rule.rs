use crate::{tokens::TokenParseError, ProfSyntaxErr, Token, TokenizedMessage};
use bitflags::bitflags;
use thiserror::Error;

macro_rules! flags {
    ($vis:vis flags $name:ident : $type:ty{$($flagname:ident:$bits:literal:$char:literal;)*}) => {
        bitflags! {
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub struct $name: u8 {
                const NONE = 0b00000000;
                $(const $flagname = $bits;)*
            }
        }

        impl $name {
            pub fn set_from_char(&mut self, char: char) -> bool {
                match char {
                    $($char => {*self |= Self::$flagname; true},)*
                    _ => {false}
                }
            }
            pub fn append_to_string(&self, str: &mut String) {
                $(
                    if self.contains(Self::$flagname) {
                        str.push($char)
                    }
                )*
            }
        }
    };
}

flags! {
    flags ProfRuleFlags : u8 {
        WORDMATCH:0b00000001: 'w';
        NO_DEDUP: 0b00000010: 'd';
    }
}

#[derive(Clone, Debug, Error)]
pub enum RuleParseError {
    #[error("Unknown flag '{0}'")]
    UnknownFlag(char),
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
pub struct ProfRule {
    pub(super) tokens: Vec<crate::Token>,
    pub(super) flags: ProfRuleFlags,
}
impl ProfRule {
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
    pub fn from_match_str(str: &str) -> Result<Self, RuleParseError> {
        Ok(Self {
            flags: ProfRuleFlags::NONE,
            tokens: Token::parse_multiple(str, true)?,
        })
    }

    pub fn parse_from_str(str: &str) -> Result<Self, RuleParseError> {
        if let Some(sep_index) = str.find(':') {
            let mut me = Self::from_match_str(&str[sep_index + 1..])?;
            let flag_region = &str[..sep_index];
            for char in flag_region.chars() {
                if !me.flags.set_from_char(char) {
                    return Err(RuleParseError::UnknownFlag(char));
                }
            }
            if me.nodedup() {
                me.tokens = Token::parse_multiple(&str[sep_index + 1..], false)?;
            }
            Ok(me)
        } else {
            Self::from_match_str(&str)
        }
    }
    pub fn to_string(&self) -> String {
        let mut str = String::with_capacity(2 + self.tokens.len());
        self.flags.append_to_string(&mut str);
        str.push(':');
        for token in self.tokens.iter() {
            str.push_str(&token.to_string())
        }

        str
    }
    pub fn nodedup(&self) -> bool {
        self.flags.contains(ProfRuleFlags::NO_DEDUP)
    }
    pub fn wordmatch(&self) -> bool {
        self.flags.contains(ProfRuleFlags::WORDMATCH)
    }

    #[inline]
    pub fn matches(&self, other: &TokenizedMessage) -> bool {
        //println!("{:?}", self);
        let mut iter_other = other.tokens();
        let mut prev_char_check = None;

        let mut match_index = 0;
        let mut t_me = self.tokens[match_index];
        loop {
            let Some(t_other) = iter_other.next() else {
                return false;
            };
            if t_other.is_whitespace() && !self.wordmatch() {
                continue;
            }
            if prev_char_check == Some(t_other) && !self.nodedup() {
                continue;
            }
            prev_char_check = Some(t_other);
            //println!("{:?} {:?}", t_me, t_other);
            if t_other.contains(t_me) {
                //println!("rule: {:?} token: {:?}", t_me, t_other);
                match_index += 1;
                if match_index == self.tokens.len() {
                    return true;
                }
                t_me = self.tokens[match_index];
            } else {
                match_index = 0;
                t_me = self.tokens[match_index];
                //println!("reset");
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        rule::{ProfRule, ProfRuleFlags},
        tokens_ar, ProfanityFilter2,
    };

    #[test]
    fn parse() {
        let rule = ProfRule::parse_from_str("w:abcd").unwrap();
        assert_eq!(
            rule,
            ProfRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: ProfRuleFlags::WORDMATCH
            },
            "wordmatch flag"
        );

        let rule = ProfRule::parse_from_str(":abcd").unwrap();
        assert_eq!(
            rule,
            ProfRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: ProfRuleFlags::NONE
            },
            "no wordmatch flag"
        );
    }

    #[test]
    fn wordmatch() {
        let mut filter = ProfanityFilter2::empty();
        let rule = ProfRule::parse_from_str("w:abcd").unwrap();
        filter.insert_rule(rule);
        assert!(
            filter
                .filter(filter.tokenize("Hi word a.b cd end words").0)
                .is_none(),
            "expected word flag to not match whitespace delimited"
        );

        let mut filter = ProfanityFilter2::empty();
        let rule = ProfRule::parse_from_str("abcd").unwrap();
        filter.insert_rule(rule);
        assert!(
            filter
                .filter(filter.tokenize("Hi word a.b cd end words").0)
                .is_some(),
            "expected non word flag to match whitespace delimited"
        );
    }

    #[test]
    fn dedup_test() {
        let rule = ProfRule::parse_from_str(":abbbcd").unwrap();
        assert_eq!(
            rule,
            ProfRule {
                tokens: tokens_ar!['a', 'b', 'c', 'd'],
                flags: ProfRuleFlags::NONE
            }
        );
    }

    #[test]
    fn dont_dedup_wordmatch() {
        let rule = ProfRule::parse_from_str("d:dedddup").unwrap();
        assert_eq!(
            rule,
            ProfRule {
                tokens: tokens_ar!['d', 'e', 'd', 'd', 'd', 'u', 'p'],
                flags: ProfRuleFlags::NO_DEDUP
            }
        )
    }

    #[test]
    fn to_str() {
        let rule = ProfRule {
            flags: ProfRuleFlags::NONE,
            tokens: tokens_ar!['a', 'b', 'c', 'd'],
        };
        assert!(!rule.wordmatch(), "wordmatch flag");
        assert_eq!(rule.to_string(), ":abcd".to_string())
    }
}
