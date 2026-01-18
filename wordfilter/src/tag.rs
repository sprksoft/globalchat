use std::{
    fmt::{Display, Write},
    str::FromStr,
};

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Tag {
    #[default]
    Unknown = 0,
    Good = 1,
    Bad = 2,
    Whitespace = 3,
}
impl Tag {
    pub fn good(self) -> bool {
        match self {
            Tag::Good | Tag::Whitespace => true,
            _ => false,
        }
    }
    pub fn unknown(self) -> bool {
        match self {
            Tag::Unknown => true,
            _ => false,
        }
    }
    pub fn bad(self) -> bool {
        match self {
            Tag::Bad => true,
            _ => false,
        }
    }

    pub fn char(self) -> char {
        match self {
            Tag::Unknown => 'u',
            Tag::Good => 'g',
            Tag::Bad => 'b',
            Tag::Whitespace => 'w',
        }
    }
    pub fn from_char(char: char) -> Option<Self> {
        match char {
            'u' => Some(Self::Unknown),
            'g' => Some(Self::Good),
            'b' => Some(Self::Bad),
            'w' => Some(Self::Whitespace),
            _ => None,
        }
    }
}
impl FromStr for Tag {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_char(s.chars().next().ok_or(())?).ok_or(())
    }
}
impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(self.char())
    }
}

impl From<Tag> for u8 {
    fn from(value: Tag) -> Self {
        value as u8
    }
}
impl From<u8> for Tag {
    fn from(value: u8) -> Self {
        if value > u8::from(Self::Whitespace) {
            return Self::Unknown;
        }
        unsafe { std::mem::transmute(value) }
    }
}
pub trait TokenTag<M>: Clone + Copy + PartialEq + Eq {
    fn from_entry(good: bool, meta: &M) -> Self;
    fn whitespace() -> Self;
    fn unknown() -> Self;
    fn good() -> Self;
    fn is_whitespace(self) -> bool;
    fn is_good_or_ws(self) -> bool;
}
impl<M> TokenTag<M> for Tag {
    fn whitespace() -> Self {
        Self::Whitespace
    }
    fn good() -> Self {
        Self::Good
    }
    fn unknown() -> Self {
        Self::Unknown
    }
    fn from_entry(good: bool, _: &M) -> Self {
        if good {
            Self::Good
        } else {
            Self::Bad
        }
    }
    fn is_whitespace(self) -> bool {
        self == Self::Whitespace
    }
    fn is_good_or_ws(self) -> bool {
        self == Self::Good || self == Self::Whitespace
    }
}

pub trait ColoredTokenTag {
    fn ansii_color(self) -> &'static str;
}
impl ColoredTokenTag for Tag {
    fn ansii_color(self) -> &'static str {
        use crate::ansii::*;
        match self {
            Tag::Good => COLOR_GREEN,
            Tag::Bad => COLOR_RED,
            Tag::Unknown => COLOR_GRAY,
            Tag::Whitespace => "",
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_char(self.char())
    }
}
