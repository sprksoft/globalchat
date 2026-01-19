use std::str::FromStr;

use wordfilter::{FromEntry, TokenTag};

use crate::wf::WFMeta;

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WFTag {
    #[default]
    Unknown,
    Good,
    Bad,
    GoodLocked,
    BadLocked,
    Whitespace,
}
impl WFTag {
    pub fn char(self) -> char {
        match self {
            Self::Unknown => 'u',
            Self::Good => 'g',
            Self::Bad => 'b',
            Self::Whitespace => 'w',
            Self::GoodLocked => 'G',
            Self::BadLocked => 'B',
        }
    }
    pub fn from_char(char: char) -> Option<Self> {
        match char {
            'u' => Some(Self::Unknown),
            'g' => Some(Self::Good),
            'b' => Some(Self::Bad),
            'G' => Some(Self::GoodLocked),
            'B' => Some(Self::BadLocked),
            'w' => Some(Self::Whitespace),
            _ => None,
        }
    }
}
impl FromStr for WFTag {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let char = s.chars().next().ok_or(())?;
        WFTag::from_char(char).ok_or(())
    }
}
impl TryFrom<char> for WFTag {
    type Error = ();
    fn try_from(value: char) -> Result<Self, Self::Error> {
        WFTag::from_char(value).ok_or(())
    }
}
impl From<WFTag> for char {
    fn from(value: WFTag) -> Self {
        value.char()
    }
}

impl TokenTag for WFTag {
    fn whitespace() -> Self {
        Self::Whitespace
    }
    fn good() -> Self {
        Self::Good
    }
    fn bad() -> Self {
        Self::Bad
    }
    fn unknown() -> Self {
        Self::Unknown
    }
    fn is_whitespace(self) -> bool {
        self == Self::Whitespace
    }
    fn is_good(self) -> bool {
        self == Self::Good || self == Self::GoodLocked
    }
    fn is_bad(self) -> bool {
        self == Self::Bad || self == Self::BadLocked
    }
    fn is_unknown(self) -> bool {
        self == Self::Unknown
    }
}
impl FromEntry<WFMeta> for WFTag {
    fn from_matched(good: bool, meta: &WFMeta) -> Self {
        if meta.locked {
            if good {
                Self::GoodLocked
            } else {
                Self::BadLocked
            }
        } else {
            if good {
                Self::Good
            } else {
                Self::Bad
            }
        }
    }
}

impl serde::Serialize for WFTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_char(self.char())
    }
}
