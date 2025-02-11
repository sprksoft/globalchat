use std::fmt::Debug;
use std::num::NonZeroU8;

use thiserror::Error;

#[macro_export]
macro_rules! tokens_ar {
    [$($c:literal),*] => {
        vec![$(crate::Token::from_char($c).unwrap()),*]
    };
}

#[derive(Clone, Debug, Error)]
pub enum TokenParseError {
    #[error("Expected a character after /")]
    ExpectedCharAfterEscapeChar,
    #[error("Invalid escape sequence: '/{0}'")]
    InvalidEscapedToken(char),
    #[error("Invalid token: '{0}'")]
    InvalidToken(char),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Token(NonZeroU8);
impl Token {
    fn from_iter<I: Iterator<Item = char>>(iter: &mut I) -> Result<Option<Self>, TokenParseError> {
        let Some(first) = iter.next() else {
            return Ok(None);
        };
        if first == '/' {
            let next = iter
                .next()
                .ok_or(TokenParseError::ExpectedCharAfterEscapeChar)?;
            if next == '0' {
                Ok(Some(Self::new_number()))
            } else if next == 'w' {
                Ok(Some(Self::new_whitespace()))
            } else {
                Err(TokenParseError::InvalidEscapedToken(next))
            }
        } else {
            Ok(Some(
                Self::from_char(first).ok_or(TokenParseError::InvalidToken(first))?,
            ))
        }
    }
    pub fn parse_one(str: &str) -> Result<Option<Self>, TokenParseError> {
        Self::from_iter(&mut str.chars())
    }
    pub fn parse_multiple(str: &str, dedup: bool) -> Result<Vec<Self>, TokenParseError> {
        let mut vec = Vec::with_capacity(str.len());
        let mut iter = str.chars();
        loop {
            let Some(t) = Self::from_iter(&mut iter)? else {
                return Ok(vec);
            };
            if vec.last() != Some(&t) || !dedup {
                vec.push(t);
            }
        }
    }

    pub fn from_char(char: char) -> Option<Token> {
        if char.is_whitespace() {
            return Some(Self::new_whitespace());
        }
        if char.is_ascii_alphanumeric() {
            let char_byte = char as u8;
            return Some(Self(
                NonZeroU8::new(char_byte.to_ascii_lowercase()).unwrap(),
            ));
        }
        None
    }
    pub fn new_whitespace() -> Token {
        Token(unsafe { NonZeroU8::new_unchecked(' ' as u8) })
    }
    pub fn new_number() -> Token {
        Token(unsafe { NonZeroU8::new_unchecked(1) })
    }
    pub fn is_whitespace(&self) -> bool {
        self.0.get() == ' ' as u8
    }
    pub fn to_string(self) -> String {
        match self.0.get() {
            1 => "/0".to_string(),
            c => (c as char).to_string(),
        }
    }
    pub fn to_u8(self) -> u8 {
        self.0.get()
    }
}
impl From<NonZeroU8> for Token {
    fn from(value: NonZeroU8) -> Self {
        Self(value)
    }
}
impl Into<u8> for Token {
    fn into(self) -> u8 {
        self.to_u8()
    }
}
impl Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenGroup {
    Single(Token),
    Multiple(Vec<Token>),
}
impl TokenGroup {
    pub fn parse_from_str(str: &str) -> Result<Option<Self>, TokenParseError> {
        let mut split = str.split(',');
        let Some(first) = split.next() else {
            return Ok(None);
        };
        let Some(token) = Token::parse_one(first.trim())? else {
            return Ok(None);
        };
        let mut me = Self::from_single(token);
        for element in split {
            let Some(token) = Token::parse_one(element.trim())? else {
                break;
            };
            me.push(token);
        }
        Ok(Some(me))
    }
    pub fn from_char(char: char) -> Option<Self> {
        if char.is_whitespace() {
            return Some(Token::new_whitespace().into());
        }

        if char.is_ascii_alphanumeric() {
            //SAFETY: \0 is not ascii alphanumeric so it will not go here
            let mut tg: TokenGroup =
                unsafe { NonZeroU8::new_unchecked((char as u8).to_ascii_lowercase()) }.into();
            if char.is_ascii_digit() {
                tg.push(Token::new_number());
            }
            return Some(tg);
        }
        None
    }
    pub fn from_single(token: Token) -> Self {
        Self::Single(token)
    }
    pub fn contains(&self, other: Token) -> bool {
        match self {
            Self::Single(t) => *t == other,
            Self::Multiple(t) => t.contains(&other),
        }
    }
    pub fn is_whitespace(&self) -> bool {
        match self {
            Self::Single(t) => t.is_whitespace(),
            Self::Multiple(t) => t.contains(&Token::new_whitespace()),
        }
    }
    pub fn push(&mut self, new: Token) {
        match self {
            Self::Single(t) => {
                let new_tg = Self::Multiple(vec![*t, new]);
                let _ = std::mem::replace::<TokenGroup>(self, new_tg);
            }
            Self::Multiple(t) => t.push(new),
        }
    }
}
impl From<Vec<Token>> for TokenGroup {
    fn from(value: Vec<Token>) -> Self {
        Self::Multiple(value)
    }
}
impl From<NonZeroU8> for TokenGroup {
    fn from(value: NonZeroU8) -> Self {
        Self::from_single(value.into())
    }
}
impl From<Token> for TokenGroup {
    fn from(value: Token) -> Self {
        Self::from_single(value)
    }
}
