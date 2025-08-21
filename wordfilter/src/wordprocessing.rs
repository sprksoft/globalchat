use std::{ ops::{Range, Deref}, str::CharIndices};

use crate::{WordFilter, WordEntry};

// Characters that do not indicate a word boundery
const NO_WORD_BOUNDERY_CHARS: [char; 5] = ['*', '.', '!', '?', '\''];
// Always allowed in any context.
const GHOST_CHARS: [char; 37] = [
    '💔', '🩷', '💕', '💖', '💙', '🔥', '✅', '🥹', '😭', '🙄', '😉', '😆', '😢', '🤔', '😁', '😅',
    '😂', '🤣', '🫠', '😊', '💪', '👌', '🫶', '👏', '👍', '🙏', '🦲', '👴', '✨', '⭐', '🎉', '💀',
    '👀', '🚀', '🌋', '🥔', '🪽',
];

pub type Span = Range<usize>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WFTag {
    Unknown,
    Good,
    Bad,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Word<'a>(WFTag, &'a str);
impl<'a> Word<'a> {
    pub fn from_str(str: &'a str) -> Self {
        Self(WFTag::Unknown, str)
    }

    pub fn str(&self) -> &str {
        self.1
    }

    pub fn normalize(self) -> NormalizedWord {
        let mut word = String::with_capacity(self.1.len());
        let mut prev_char = None;
        for char in self.chars() {
            if let Some(char) = push_normalized(&mut word, prev_char, char) {
                prev_char = Some(char);
            }
        }


        let mut word = NormalizedWord {
            root_end: self.len(),
            word
        }

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
}
impl<'a> From<&'a str> for Word<'a> {
    fn from(value: &'a str) -> Self {
        Self::from_str(value)
    }
}
impl<'a> Deref for Word<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.str()
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

pub struct NormalizedWord{
        word: String,
        root_end: usize,
    }
impl NormalizedWord {
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
pub struct TokenizedString<'a>(Vec<Word<'a>>);
impl<'a> TokenizedString<'a> {
    pub fn tokenize(str: &'a str) -> TokenizedString<'a> {
        let mut words = Vec::new();

        let mut start_index = 0;
        let mut prev_char = '\0';
        for (index, char) in str.char_indices() {
            if is_word_boundery(char) {
                if !is_word_boundery(prev_char) {
                    words.push(Word::from_str(&str[start_index..index]));
                }
                start_index = index;
                prev_char = char;
            }
        }
        words.push(Word::from_str(&str[start_index..]));

        TokenizedString(words)
    }

    pub fn update_tags(&mut self, filter: &WordFilter) {
        for window in self.0.as_mut_slice().windows(2) {
            let mut word = window[0];
            let next_word = window[1];
            let Some(entry) = filter.get_entry(&word) else {
                word.0 = WFTag::Unknown;
                continue;
            };
            let next_norm = next_word.normalize();
            if entry.forward_ctx
                    .iter()
                    .find(|c| c.as_ref() == next_norm.root() || c.as_ref() == next_norm.str())
                    .is_some() {

                word.0 = if entry.good { WFTag::Bad} else { WFTag::Good };
            }else{
                word.0 = if entry.good { WFTag::Good} else { WFTag::Bad };
            }

        }
    }

    pub fn words(&self) -> impl Iterator<Item = &Word> {
        self.0.iter()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        wordprocessing::{is_word_boundery, TokenizedString},
        Word,
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
            TokenizedString(vec![
                Word::from_str("ik"),
                Word::from_str("ben"),
                Word::from_str("sibe")
            ])
        );
    }
    
    #[test]
    fn normalized_word() {
        assert_eq!(Box::<str>::from(Word::from_str("fuckery").normalize()), "fuck".into());
        assert_eq!(Word::from_str("fuckery").normalize().root(), "fuck");
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
