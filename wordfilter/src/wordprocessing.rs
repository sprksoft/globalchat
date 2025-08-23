use std::{
    cell::{Cell, OnceCell},
    ops::Range,
    sync::Arc,
};

use crate::WordFilter;

// Characters that do not indicate a word boundery
const NO_WORD_BOUNDERY_CHARS: [char; 5] = ['*', '.', '!', '?', '\''];
// Always allowed in any context.
const GHOST_CHARS: [char; 37] = [
    '💔', '🩷', '💕', '💖', '💙', '🔥', '✅', '🥹', '😭', '🙄', '😉', '😆', '😢', '🤔', '😁', '😅',
    '😂', '🤣', '🫠', '😊', '💪', '👌', '🫶', '👏', '👍', '🙏', '🦲', '👴', '✨', '⭐', '🎉', '💀',
    '👀', '🚀', '🌋', '🥔', '🪽',
];

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tag {
    #[default]
    Unknown,
    Good,
    Bad,
}
impl Tag {
    pub fn good(self) -> bool {
        match self {
            Tag::Good => true,
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

fn suffix(word: &mut NormalizedWord, suffix: &'static str) {
    if word.root().len() >= 3 + suffix.len() && word.root().ends_with(suffix) {
        word.root_end -= suffix.len();
    }
}

#[inline]
fn push_normalized(string: &mut String, mut prev_char: Option<char>, char: char) -> Option<char> {
    let char = char.to_ascii_lowercase();
    if GHOST_CHARS.contains(&char) {
        return None;
    }
    let decode = unidecode::unidecode_char(char);
    if decode == "[?]" || decode == "" {
        if prev_char.map(|prev| prev != char).unwrap_or(true) {
            string.push(char);
            return Some(char);
        }
    }
    let mut last_pushed = None;
    for char in decode.chars().map(|c| c.to_ascii_lowercase()) {
        if char.is_control() || char.is_numeric() {
            continue;
        }
        if char.is_alphabetic() {
            if prev_char.map(|prev| prev != char).unwrap_or(true) {
                string.push(char);
                prev_char = Some(char);
                last_pushed = Some(char);
            }
        }
    }
    last_pushed
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

#[inline]
fn is_word_boundery(char: char) -> bool {
    let char = char.to_ascii_lowercase();
    let decode = unidecode::unidecode_char(char);
    if decode == "[?]" || decode == "" {
        return false;
    }

    if GHOST_CHARS.contains(&char) || NO_WORD_BOUNDERY_CHARS.contains(&char) {
        return false;
    }
    for char in decode.chars() {
        if !char.is_alphabetic() {
            return true;
        }
    }
    false
}

#[derive(Debug, PartialEq, Eq)]
struct Token {
    tag: Cell<Tag>,
    span: Range<usize>,
    norm: OnceCell<NormalizedWord>,
}
impl Token {
    pub fn new(span: Range<usize>, tag: Tag) -> Self {
        Token {
            tag: tag.into(),
            norm: OnceCell::default(),
            span,
        }
    }

    pub fn norm(&self, s: &str) -> &NormalizedWord {
        self.norm
            .get_or_init(|| NormalizedWord::normalize(&s[self.span.clone()]))
    }
}

pub trait IntoWordTagPair<'a> {
    fn into_word_tag_pair(self) -> (&'a str, Tag);
}
impl<'a> IntoWordTagPair<'a> for &'a str {
    fn into_word_tag_pair(self) -> (&'a str, Tag) {
        (self, Tag::Unknown)
    }
}
impl<'a> IntoWordTagPair<'a> for (&'a str, Tag) {
    fn into_word_tag_pair(self) -> (&'a str, Tag) {
        self
    }
}

fn slideing_windows<I>(slice: &[I]) -> impl Iterator<Item = (&I, Option<&I>)> {
    (0..slice.len())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenizedString(Arc<str>, Arc<[Token]>);
impl TokenizedString {
    pub fn from_words<'a>(words: impl IntoIterator<Item = impl IntoWordTagPair<'a>>) -> Self {
        let iter = words.into_iter();

        let mut tokens: Vec<Token> = Vec::with_capacity(iter.size_hint().1.unwrap_or(1));

        let mut string = String::new();
        for word in iter {
            let (word, tag) = word.into_word_tag_pair();
            tokens.push(Token::new(string.len()..string.len() + word.len(), tag));
            string.push_str(word);
        }
        Self(string.into(), tokens.into())
    }

    pub fn tokenize(str: impl Into<Arc<str>>) -> TokenizedString {
        let str = str.into();
        let mut tokens = Vec::new();

        let mut start_index = 0;
        let mut prev_char = '\0';
        for (index, char) in str.char_indices() {
            if is_word_boundery(char) {
                if !is_word_boundery(prev_char) {
                    tokens.push(Token::new(start_index..index, Tag::default()));
                }
                start_index = index;
            }
            prev_char = char;
        }
        tokens.push(Token::new(start_index..str.len(), Tag::default()));

        TokenizedString(str, tokens.into())
    }

    pub fn recheck(&mut self, filter: &WordFilter) {
        for window in self.1.windows(2) {
            let token = &window[0];
            let next_token = &window[1];
            let Some(entry) = filter.get_entry(token.norm(&self.0)) else {
                token.tag.set(Tag::Unknown);
                continue;
            };

            if entry
                .forward_ctx
                .iter()
                .find(|c| {
                    c.as_ref() == next_token.norm(&self.0).root()
                        || c.as_ref() == next_token.norm(&self.0).str()
                })
                .is_some()
            {
                token.tag.set(if entry.good { Tag::Bad } else { Tag::Good });
            } else {
                token.tag.set(if entry.good { Tag::Good } else { Tag::Bad });
            }
        }
    }

    pub fn good(&self) -> bool {
        self.words().find(|(_, tag)| !tag.good()).is_none()
    }

    pub fn words(&self) -> impl Iterator<Item = (&str, Tag)> {
        self.1
            .iter()
            .map(|token| (&self.0[token.span.clone()], token.tag.get()))
    }

    pub fn norm_words(&self) -> impl Iterator<Item = (&str, Tag, &NormalizedWord)> {
        self.1.iter().map(|token| {
            (
                &self.0[token.span.clone()],
                token.tag.get(),
                token.norm(&self.0),
            )
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        wordprocessing::{is_word_boundery, TokenizedString},
        NormalizedWord,
    };

    #[test]
    fn word_boundery() {
        assert!(!is_word_boundery('🍆'));
        assert!(!is_word_boundery('💕'));
        assert!(!is_word_boundery('.'));
        assert!(is_word_boundery('-'));
    }

    #[test]
    fn tokenize_test() {
        assert_eq!(
            TokenizedString::tokenize("ik ben sibe"),
            TokenizedString::from_words(["ik", " ben", " sibe"])
        );

        assert_eq!(
            TokenizedString::tokenize("ik ben sibe")
                .words()
                .map(|(w, _)| w)
                .collect::<Vec<&str>>(),
            vec!["ik", " ben", " sibe"]
        )
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
