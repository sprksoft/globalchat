use crate::{Token, TokenizedMessage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfRule {
    tokens: Vec<crate::Token>,
    flags: u8,
}
impl ProfRule {
    pub fn from_match_str(str: &str) -> Option<Self> {
        let me = Self {
            flags: 0,
            tokens: Self::match_str_to_tokens(str, true)?,
        };
        Some(me)
    }
    fn match_str_to_tokens(str: &str, dedup: bool) -> Option<Vec<Token>> {
        let mut tokens = Vec::with_capacity(str.len());
        let mut chars = str.chars();
        loop {
            if let Some(t) = Token::from_iter(&mut chars) {
                if t.is_whitespace() {
                    continue;
                }
                if tokens.last().cloned() != Some(t) || !dedup {
                    tokens.push(t);
                }
            } else {
                break;
            }
        }
        Some(tokens)
    }

    pub fn from_str(str: &str) -> Option<Self> {
        if let Some(sep_index) = str.find(':') {
            let mut me = Self::from_match_str(&str[sep_index + 1..])?;
            let flag_region = &str[..sep_index];
            if flag_region.contains('w') {
                me.set_wordmatch();
            }
            if flag_region.contains('l') {
                me.set_exactlen();
                me.tokens = Self::match_str_to_tokens(&str[sep_index + 1..], false)?;
            }
            Some(me)
        } else {
            Self::from_match_str(&str)
        }
    }
    pub fn to_string(&self) -> String {
        let mut str = String::with_capacity(2 + self.tokens.len());
        if self.wordmatch() {
            str.push('w');
        }
        if self.exactlen() {
            str.push('w');
        }
        str.push(':');
        for token in self.tokens.iter() {
            str.push_str(&token.to_string())
        }

        str
    }
    pub fn exactlen(&self) -> bool {
        self.get_flag(1)
    }
    pub fn set_exactlen(&mut self) {
        self.set_flag(1);
    }
    pub fn wordmatch(&self) -> bool {
        self.get_flag(0)
    }
    pub fn set_wordmatch(&mut self) {
        self.set_flag(0);
    }

    fn get_flag(&self, index: u8) -> bool {
        (self.flags << index) & 0x80 == 0x80
    }

    fn set_flag(&mut self, index: u8) {
        //0x8000... = 0b10000000...
        self.flags |= 0x80 >> index;
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
            if prev_char_check == Some(t_other) && !self.exactlen() {
                continue;
            }
            prev_char_check = Some(t_other);
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
                flags: 0x80,
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
