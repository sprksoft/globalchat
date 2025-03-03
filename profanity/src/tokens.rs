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

macro_rules! special_tokens {
    (impl $name:ident{$($escapecode:literal:$number:literal:$desc:literal;)*}) => {
        impl $name {
            pub fn token_info() -> &'static [(char, u8, &'static str)] {
                &[
                    $(
                        ($escapecode, $number, $desc)
                    ),*
                ]
            }

            fn from_escapecode(char: char) -> Option<Self> {
                match char{
                    $(
                        $escapecode => Some(Self(unsafe{NonZeroU8::new_unchecked($number)})),
                    )*
                    _=>None
                }
            }

            pub fn to_string(self) -> String {
                let mut string = String::with_capacity(2);
                match self.0.get() {
                    $(
                        $number => {string.push('/'); string.push($escapecode)},
                    )*
                    c => string.push(c as char),
                }
                string
            }
        }

    };
}
special_tokens! {
    impl Token {
        '/':b'/':"Match a literal / character";
        'w':b' ':"Match any whitespace character. (space, tab, enter, ...)";
        '0':1:"Match any number (0-9)";
        'k':3:"Match any vowel. (a,e,i,o,u,...)";
        '?':2:"Match any unknown character. (Unknown characters are characters that don't appear in a replace rule and aren't a-z)";
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token(NonZeroU8);
impl Token {
    pub fn parse_multiple(str: &str, dedup: bool) -> Result<Vec<Self>, TokenParseError> {
        let mut vec = Vec::with_capacity(str.len());

        let mut escape = false;
        for char in str.chars() {
            if char.is_whitespace() {
                continue;
            }
            if char == '/' && !escape {
                escape = true;
                continue;
            }
            let t = if escape {
                escape = false;
                Self::from_escapecode(char).ok_or(TokenParseError::InvalidEscapedToken(char))?
            } else {
                Self::from_char(char).ok_or(TokenParseError::InvalidToken(char))?
            };

            if vec.last() != Some(&t) || !dedup {
                vec.push(t);
            }
        }
        if escape {
            return Err(TokenParseError::ExpectedCharAfterEscapeChar);
        }
        Ok(vec)
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
    pub fn new_vowel() -> Token {
        Token(unsafe { NonZeroU8::new_unchecked(3) })
    }
    pub fn new_unknown() -> Token {
        Token(unsafe { NonZeroU8::new_unchecked(2) })
    }
    pub fn is_whitespace(&self) -> bool {
        self.0.get() == ' ' as u8
    }
    pub fn is_unknown(&self) -> bool {
        self.0.get() == 2
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
pub enum TokenGroupIter<'a> {
    Multiple(std::slice::Iter<'a, Token>),
    Single(Option<&'a Token>),
}

impl<'a> Iterator for TokenGroupIter<'a> {
    type Item = &'a Token;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(t) => t.take(),
            Self::Multiple(v) => v.next(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TokenGroup {
    Single(Token),
    Multiple(Vec<Token>),
}
impl TokenGroup {
    pub fn parse_from_str(str: &str) -> Result<Self, TokenParseError> {
        let tokens = Token::parse_multiple(str, false)?;
        if tokens.len() == 1 {
            Ok(Self::Single(tokens[0]))
        } else {
            Ok(Self::Multiple(tokens))
        }
    }
    pub fn from_char(char: char) -> Option<Self> {
        if char.is_whitespace() {
            return Some(Token::new_whitespace().into());
        }

        if char.is_ascii_alphanumeric() {
            let char_u8 = (char as u8).to_ascii_lowercase();
            //SAFETY: \0 is not ascii alphanumeric so it will not go here
            let mut tg: TokenGroup = unsafe { NonZeroU8::new_unchecked(char_u8) }.into();
            if char.is_ascii_digit() {
                tg.push(Token::new_number());
            }
            if [b'a', b'e', b'i', b'o', b'u'].contains(&char_u8) {
                tg.push(Token::new_vowel());
            }
            return Some(tg);
        }
        None
    }
    pub fn iter(&self) -> TokenGroupIter<'_> {
        match self {
            Self::Single(t) => TokenGroupIter::Single(Some(t)),
            Self::Multiple(v) => TokenGroupIter::Multiple(v.iter()),
        }
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
    pub fn is_unknown(&self) -> bool {
        match self {
            Self::Single(t) => t.is_unknown(),
            _ => false,
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

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TokenGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(crate::json::TokenGroupVisitor)
    }
}
#[cfg(feature = "serde")]
impl serde::Serialize for TokenGroup {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let len = match self {
            Self::Single(_) => 1,
            Self::Multiple(v) => v.len(),
        };
        let mut seq_ser = serializer.serialize_seq(Some(len))?;
        for token in self.iter() {
            seq_ser.serialize_element(token)?;
        }
        seq_ser.end()
    }
}
