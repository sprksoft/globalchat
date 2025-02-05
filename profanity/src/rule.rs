use std::num::NonZeroU8;

use crate::{Token, TokenizedMessage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfRule {
    tokens: Vec<crate::Token>,
    flags: u32,
}
impl ProfRule {
    pub fn from_match_str(str: &str) -> Option<Self> {
        let me = Self {
            flags: 0,
            tokens: Vec::with_capacity(str.len()),
        };
        for char in &str.chars() {
            let t = Token::from_char(char)?;
            if me.tokens.last() != Some(&t) {
                me.tokens.push(t);
            }
        }
        Some(me)
    }
    pub fn from_str(str: &str) -> Option<Self> {
        let mut me = Self {
            flags: 0,
            tokens: Vec::with_capacity(str.len()),
        };

        let flag_sep_index = str.find(':')?;
        let flag_region = &str[..flag_sep_index];
        if flag_region.contains('w') {
            me.set_wordmatch();
        }

        for char in (&str[flag_sep_index + 1..]).chars() {
            let t = Token::from_char(char)?;
            if me.tokens.last() != Some(&t) {
                me.tokens.push(t);
            }
        }
        Some(me)
    }
    pub fn to_string(&self) -> String {
        let mut str = String::with_capacity(2 + self.tokens.len());
        if self.wordmatch() {
            str.push('w');
        }
        str.push(':');
        for token in self.tokens.iter() {
            str.push(token.to_char());
        }

        str
    }
    pub fn wordmatch(&self) -> bool {
        self.get_flag(0)
    }
    pub fn set_wordmatch(&mut self) {
        self.set_flag(0);
    }

    fn get_flag(&self, index: u8) -> bool {
        (self.flags << index) & 0x80000000 == 0x80000000
    }

    fn set_flag(&mut self, index: u8) {
        //0x8000... = 0b10000000...
        self.flags |= 0x80000000 >> index;
    }

    pub fn matches(&self, other: &TokenizedMessage) -> bool {
        let mut iter_me = self.tokens.iter();
        let mut iter_other = other.tokens();
        let mut prev_char_check = None;
        loop {
            let Some(t_me) = iter_me.next() else {
                return true;
            };
            loop {
                let Some(t_other) = iter_other.next() else {
                    return false;
                };
                if prev_char_check == Some(t_other) {
                    continue;
                }
                prev_char_check = Some(t_other);
                if t_other.is_whitespace() && !self.wordmatch() {
                    continue;
                }
                if t_me == t_other {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{rule::ProfRule, tokens};

    #[test]
    fn from_str() {
        let rule = ProfRule::from_str("w:abcd");
        assert_eq!(
            rule.clone().map(|r| r.wordmatch()),
            Some(true),
            "wordmatch flag"
        );
        assert_eq!(
            rule,
            Some(ProfRule {
                flags: 0x80000000,
                tokens: tokens!['a', 'b', 'c', 'd'],
            })
        );
    }

    #[test]
    fn optimize_test() {
        let rule = ProfRule::from_str(":abbbcd");
        assert_eq!(
            rule,
            Some(ProfRule {
                tokens: tokens!['a', 'b', 'c', 'd'],
                flags: 0
            })
        );
    }

    #[test]
    fn to_str() {
        let rule = ProfRule {
            flags: 0,
            tokens: tokens!['a', 'b', 'c', 'd'],
        };
        assert!(!rule.wordmatch(), "wordmatch flag");
        assert_eq!(rule.to_string(), ":abcd".to_string())
    }
}
