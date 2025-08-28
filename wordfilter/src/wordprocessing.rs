use std::{
    fmt::{Debug, Display},
    ops::Range,
    sync::OnceLock,
};

use crate::{ansii::*, is_ignored, is_void, normalize_char, CharType};

use crate::WordFilter;

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(usize)]
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
        unsafe { std::mem::transmute(value as usize) }
    }
}
pub trait TokenTag: Clone + Copy + PartialEq + Eq {}
impl TokenTag for Tag {}

#[inline]
fn push_normalized(string: &mut String, mut prev_char: Option<char>, char: char) -> Option<char> {
    let mut last_pushed = None;
    for char in normalize_char(char) {
        if !is_ignored(char) && prev_char.map(|prev| prev != char).unwrap_or(true) {
            string.push(char);
            prev_char = Some(char);
            last_pushed = Some(char);
        }
    }
    last_pushed
}

#[inline]
fn suffix(word: &mut NormalizedWord, suffix: &'static str) {
    if word.root().len() >= 3 + suffix.len() && word.root().ends_with(suffix) {
        word.root_end -= suffix.len();
    }
}

#[derive(Clone, Hash, Debug, PartialEq, Eq)]
pub struct NormalizedWord {
    root_end: usize,
    word: String,
}
impl NormalizedWord {
    pub fn normalize(str: &str) -> NormalizedWord {
        let mut word = String::with_capacity(str.len());
        let mut prev_char = None;
        for char in str.chars() {
            if let Some(char) = push_normalized(&mut word, prev_char, char) {
                prev_char = Some(char);
            }
        }

        let mut word = NormalizedWord {
            root_end: word.len(),
            word,
        };

        // EN
        suffix(&mut word, "y");
        suffix(&mut word, "est");
        suffix(&mut word, "er");
        suffix(&mut word, "ing");
        // NL

        // meervoud
        suffix(&mut word, "en");
        suffix(&mut word, "je");
        suffix(&mut word, "jes");

        // adjectief
        suffix(&mut word, "baar");
        suffix(&mut word, "lijk");

        suffix(&mut word, "e");
        word
    }
    pub fn str(&self) -> &str {
        &self.word
    }
    pub fn root(&self) -> &str {
        &self.word[..self.root_end]
    }
}
impl From<NormalizedWord> for Box<str> {
    fn from(mut value: NormalizedWord) -> Self {
        value.word.truncate(value.root_end);
        value.word.into_boxed_str()
    }
}

#[derive(Clone)]
struct Token<T: TokenTag> {
    tag: T,
    span: Range<usize>,
    norm: OnceLock<NormalizedWord>,
}
impl<T: TokenTag> Eq for Token<T> {}
impl<T: TokenTag> PartialEq for Token<T> {
    fn eq(&self, other: &Self) -> bool {
        self.span == other.span && self.tag == other.tag
    }
}
impl<T: TokenTag + Debug> Debug for Token<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        self.span.fmt(f)?;
        f.write_str(", ")?;
        self.tag.fmt(f)?;
        f.write_str(")")?;
        Ok(())
    }
}

impl<T: TokenTag> Token<T> {
    pub fn new(span: Range<usize>, tag: T) -> Self {
        Token {
            tag,
            norm: OnceLock::default(),
            span,
        }
    }

    pub fn norm(&self, s: &str) -> &NormalizedWord {
        self.norm
            .get_or_init(|| NormalizedWord::normalize(&s[self.span.clone()]))
    }
}

pub trait IntoWordTagPair<'a, T> {
    fn into_word_tag_pair(self) -> (&'a str, T);
}
impl<'a, T: Default> IntoWordTagPair<'a, T> for &'a str {
    fn into_word_tag_pair(self) -> (&'a str, T) {
        (self, T::default())
    }
}
impl<'a, T> IntoWordTagPair<'a, T> for (&'a str, T) {
    fn into_word_tag_pair(self) -> (&'a str, T) {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenizedString(Box<str>, Vec<Token<Tag>>);
impl TokenizedString {
    pub fn from_words<'a>(words: impl IntoIterator<Item = impl IntoWordTagPair<'a, Tag>>) -> Self {
        let iter = words.into_iter();

        let mut tokens: Vec<Token<Tag>> = Vec::with_capacity(iter.size_hint().1.unwrap_or(1));

        let mut string = String::new();
        for word in iter {
            let (word, tag) = word.into_word_tag_pair();
            tokens.push(Token::new(string.len()..string.len() + word.len(), tag));
            string.push_str(word);
        }
        Self(string.into(), tokens.into())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn tokenize(str: impl Into<Box<str>>) -> TokenizedString {
        let str = str.into();
        let mut tokens = Vec::new();

        let mut start_index = 0;
        let mut prev_char = None;
        for (index, char) in str.char_indices() {
            if is_void(char) {
                continue;
            }
            let ty = CharType::new(char);
            if let Some(prev_char) = prev_char {
                let prev_ty = CharType::new(prev_char);
                if ty != prev_ty {
                    let tag = if prev_ty == CharType::Whitespace {
                        Tag::Whitespace
                    } else {
                        Tag::default()
                    };
                    tokens.push(Token::new(start_index..index, tag));
                    start_index = index;
                }
            }
            prev_char = Some(char);
        }

        let tag = if prev_char
            .map(|c| CharType::new(c) == CharType::Whitespace)
            .unwrap_or(true)
        {
            Tag::Whitespace
        } else {
            Tag::default()
        };
        tokens.push(Token::new(start_index..str.len(), tag));

        TokenizedString(str, tokens.into())
    }

    /// Returns true if a tag is changed
    pub fn recheck(&mut self, filter: &WordFilter) -> bool {
        let tokens = &mut self.1;
        let mut changed = false;

        macro_rules! set {
            ($tag:expr, $new:expr) => {
                let new = $new;
                if $tag != new {
                    changed = true;
                }
                $tag = new;
            };
        }

        fn get_token(tokens: &Vec<Token<Tag>>, mut i: usize) -> Option<&Token<Tag>> {
            loop {
                let t = tokens.get(i)?;
                if t.tag != Tag::Whitespace {
                    return Some(t);
                }
                i += 1;
            }
        }

        for i in 0..tokens.len() {
            let token = &mut tokens[i];
            if token.tag == Tag::Whitespace {
                continue;
            }
            let norm = token.norm(&self.0);
            if norm.str().trim().len() == 0 {
                set!(token.tag, Tag::Good);
                continue;
            }
            let Some(entry) = filter.get_entry(norm) else {
                set!(token.tag, Tag::Unknown);
                continue;
            };

            let mut tag = None;
            if let Some(next_token) = get_token(tokens, i + 1) {
                if entry
                    .forward_ctx
                    .iter()
                    .find(|c| {
                        c.as_ref() == next_token.norm(&self.0).root()
                            || c.as_ref() == next_token.norm(&self.0).str()
                    })
                    .is_some()
                {
                    tag = Some(if entry.good { Tag::Bad } else { Tag::Good }); // inverted tag
                }
            }
            set!(
                tokens[i].tag,
                tag.unwrap_or(if entry.good { Tag::Good } else { Tag::Bad })
            );
        }
        changed
    }

    pub fn good(&self) -> bool {
        self.words().find(|(_, tag)| !tag.good()).is_none()
    }

    pub fn words(&self) -> impl Iterator<Item = (&str, Tag)> {
        self.1
            .iter()
            .map(|token| (&self.0[token.span.clone()], token.tag))
    }

    pub fn norm_words(&self) -> impl Iterator<Item = (&str, Tag, &NormalizedWord)> {
        self.1
            .iter()
            .map(|token| (&self.0[token.span.clone()], token.tag, token.norm(&self.0)))
    }

    pub fn colored<'a>(&'a self) -> ColoredFmt<'a> {
        ColoredFmt(&self)
    }
}

pub struct ColoredFmt<'a>(&'a TokenizedString);
impl<'a> Display for ColoredFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (word, tag) in self.0.words() {
            match tag {
                Tag::Good => {
                    f.write_str(COLOR_GREEN)?;
                }
                Tag::Bad => {
                    f.write_str(COLOR_RED)?;
                }
                Tag::Unknown => {
                    f.write_str(COLOR_GRAY)?;
                }
                Tag::Whitespace => {}
            }
            f.write_str(word)?;
            f.write_str(RESET)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{wordprocessing::TokenizedString, NormalizedWord, Tag};

    #[test]
    fn tokenize_test() {
        assert_eq!(
            TokenizedString::tokenize("ik ben sibe"),
            TokenizedString::from_words([
                ("ik", Tag::Unknown),
                (" ", Tag::Whitespace),
                ("ben", Tag::Unknown),
                (" ", Tag::Whitespace),
                ("sibe", Tag::Unknown)
            ])
        );

        assert_eq!(
            TokenizedString::tokenize("ik ben sibe")
                .words()
                .map(|(w, _)| w)
                .collect::<Vec<&str>>(),
            vec!["ik", " ", "ben", " ", "sibe"]
        );

        assert_eq!(
            TokenizedString::tokenize("test with a newline\n"),
            TokenizedString::from_words([
                ("test", Tag::Unknown),
                (" ", Tag::Whitespace),
                ("with", Tag::Unknown),
                (" ", Tag::Whitespace),
                ("a", Tag::Unknown),
                (" ", Tag::Whitespace),
                ("newline", Tag::Unknown),
                ("\n", Tag::Whitespace),
            ])
        );

        assert_eq!(
            TokenizedString::tokenize("❤️✅"),
            TokenizedString::from_words([("❤️✅", Tag::Unknown)])
        );
    }

    #[test]
    fn normalized_word() {
        assert_eq!(
            Box::<str>::from(NormalizedWord::normalize("fuckery")),
            "fuck".into()
        );
        assert_eq!(NormalizedWord::normalize("fuckery").root(), "fuck");
    }

    #[test]
    fn push_normalized() {
        let mut string = "a".to_string();
        assert_eq!(super::push_normalized(&mut string, Some('a'), 'a'), None);
        assert_eq!(string, "a");

        let mut string = "t".to_string();
        assert_eq!(
            super::push_normalized(&mut string, Some('t'), '™'),
            Some('m')
        );
        assert_eq!(string, "tm");

        let mut string = "m".to_string();
        assert_eq!(
            super::push_normalized(&mut string, Some('m'), '™'),
            Some('m')
        );
        assert_eq!(string, "mtm");

        let mut string = "".to_string();
        assert_eq!(super::push_normalized(&mut string, None, '™'), Some('m'));
        assert_eq!(string, "tm");
    }
}
