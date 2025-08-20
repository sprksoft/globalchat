use std::{ops::Range, str::CharIndices};

// Characters that do not indicate a word boundery
const NO_WORD_BOUNDERY_CHARS: [char; 5] = ['*', '.', '!', '?', '\''];
// Always allowed in any context.
const GHOST_CHARS: [char; 37] = [
    '💔', '🩷', '💕', '💖', '💙', '🔥', '✅', '🥹', '😭', '🙄', '😉', '😆', '😢', '🤔', '😁', '😅',
    '😂', '🤣', '🫠', '😊', '💪', '👌', '🫶', '👏', '👍', '🙏', '🦲', '👴', '✨', '⭐', '🎉', '💀',
    '👀', '🚀', '🌋', '🥔', '🪽',
];

pub type Span = Range<usize>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub span: Span,
    root_end: usize,
    string: String,
}
impl Word {
    #[inline]
    pub fn new_stemmed(string: String, span: Span) -> Self {
        let mut word = Word {
            span,
            root_end: string.len(),
            string,
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
    pub fn root(&self) -> &str {
        &self.string[..self.root_end]
    }
    pub fn str(&self) -> &str {
        &self.string
    }
}
impl Into<Box<str>> for Word {
    fn into(self) -> Box<str> {
        self.string.into_boxed_str()
    }
}

fn suffix(word: &mut Word, suffix: &'static str) {
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

pub struct NormWordsIter<'a> {
    char_iter: CharIndices<'a>,
}
impl<'a> Iterator for NormWordsIter<'a> {
    type Item = Word;
    fn next(&mut self) -> Option<Self::Item> {
        let (start_index, start_char) = loop {
            let (index, char) = self.char_iter.next()?;
            if !is_word_boundery(char) {
                break (index, char);
            }
        };
        let mut string = String::new();
        let mut prev_char = push_normalized(&mut string, None, start_char);

        let span = loop {
            let (index, char) = match self.char_iter.next() {
                Some((index, char)) => (index, char),
                None => {
                    break start_index..self.char_iter.offset();
                }
            };
            if is_word_boundery(char) {
                break start_index..index;
            }

            if let Some(char) = push_normalized(&mut string, prev_char, char) {
                prev_char = Some(char);
            }
        };

        return if string.len() == 0 {
            None
        } else {
            Some(Word::new_stemmed(string, span))
        };
    }
}

pub fn normalize_words<'a>(data: &'a str) -> NormWordsIter<'a> {
    NormWordsIter {
        char_iter: data.char_indices(),
    }
}

#[cfg(test)]
mod test {
    use crate::wordprocessing::is_word_boundery;

    #[test]
    fn word_boundery() {
        assert!(!is_word_boundery('🍆'));
        assert!(!is_word_boundery('💕'));
        assert!(!is_word_boundery('.'));
        assert!(is_word_boundery('-'));
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
