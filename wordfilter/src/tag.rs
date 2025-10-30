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

impl Into<u8> for Tag {
    fn into(self) -> u8 {
        self as u8
    }
}
impl From<u8> for Tag {
    fn from(value: u8) -> Self {
        if value > Self::Whitespace.into() {
            return Self::Unknown;
        }
        unsafe { std::mem::transmute(value) }
    }
}
pub trait TokenTag: Clone + Copy + PartialEq + Eq {}
impl TokenTag for Tag {}

#[cfg(feature = "serde")]
impl serde::Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_char(self.char())
    }
}
