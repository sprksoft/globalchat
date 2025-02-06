use std::fmt::Debug;
use std::fmt::Write;
use std::num::NonZeroU8;

#[macro_export]
macro_rules! tokens {
    [$($c:literal),*] => {
        vec![$(crate::Token::from_char($c).unwrap()),*]
    };
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Token(NonZeroU8);
impl Token {
    pub fn from_str(str: &str) -> Option<Self> {
        Self::from_iter(&mut str.chars())
    }
    pub fn from_iter<I: Iterator<Item = char>>(iter: &mut I) -> Option<Self> {
        let first = iter.next()?;
        if first == '/' {
            let next = iter.next()?;
            if next == '0' {
                Some(Self::new_number())
            } else if next == 'w' {
                Some(Self::new_whitespace())
            } else {
                None
            }
        } else {
            Some(Self::from_char(first)?)
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
    pub fn from_str(str: &str) -> Option<Self> {
        let mut split = str.split(',');
        let mut me = Self::from_single(Token::from_str(split.next()?)?);
        for element in split {
            me.push(Token::from_str(element.trim())?);
        }
        Some(me)
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

// struct GroupTokenItr<'a> {
//     tg: &'a TokenGroup,
//     index: usize,
// }
// impl<'a> Iterator for GroupTokenItr<'a> {
//     type Item = Token;
//     fn next(&mut self) -> Option<Self::Item> {
//         let result = self.tg.0[self.index]; self.index += 1;
//         result
//     }
// }
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct TokenGroup([Option<Token>; 32]);
// impl TokenGroup {
//     pub fn from_single(token: Token) -> Self {
//         let mut arr = [None; 32];
//         arr[0] = Some(token);
//         Self(arr)
//     }
//     pub fn contains(&self, other: Token) -> bool {
//         for item in self.iter() {
//             if item == other {
//                 return true;
//             }
//         }
//         false
//     }
//     fn iter(&self) -> GroupTokenItr<'_> {
//         GroupTokenItr {
//             tg: &self,
//             index: 0,
//         }
//     }
//     pub fn is_whitespace(&self) -> bool {
//         for item in self.iter() {
//             if item.is_whitespace() {
//                 return true;
//             }
//         }
//         false
//     }
// }
// impl From<Vec<Token>> for TokenGroup {
//     fn from(value: Vec<Token>) -> Self {
//         value.as_slice().into()
//     }
// }
// impl From<&[Token]> for TokenGroup {
//     fn from(value: &[Token]) -> Self {
//         let mut array = [None; 32];
//         //SAFETY: Tokens can't be 0 so they can safely be copied into a zeroed array
//         unsafe { array[..value.len()].copy_from_slice(std::mem::transmute(value)) }
//         Self(array)
//     }
// }
// impl From<Token> for TokenGroup {
//     fn from(value: Token) -> Self {
//         Self::from_single(value)
//     }
// }
// impl From<NonZeroU8> for TokenGroup {
//     fn from(value: NonZeroU8) -> Self {
//         Self::from_single(value.into())
//     }
// }
//

// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct TokenGroup(Vec<Token>);
// impl TokenGroup {
//     pub fn from_single(token: Token) -> Self {
//         Self(vec![token])
//     }
//     pub fn contains(&self, other: Token) -> bool {
//         self.0.contains(&other)
//     }
//     pub fn is_whitespace(&self) -> bool {
//         self.0.contains(&Token::new_whitespace())
//     }
// }
// impl From<Vec<Token>> for TokenGroup {
//     fn from(value: Vec<Token>) -> Self {
//         Self(value)
//     }
// }
// impl From<NonZeroU8> for TokenGroup {
//     fn from(value: NonZeroU8) -> Self {
//         Self::from_single(value.into())
//     }
// }
// impl From<Token> for TokenGroup {
//     fn from(value: Token) -> Self {
//         Self::from_single(value)
//     }
// }
