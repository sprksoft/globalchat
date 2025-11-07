use std::str::Chars;

pub enum NormCharsIter<'a> {
    Single(char),
    Multiple(Chars<'a>),
    Empty,
}
impl<'a> Iterator for NormCharsIter<'a> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(char) => {
                let char = *char;
                *self = Self::Empty;
                Some(char)
            }
            Self::Multiple(chars) => chars.next(),
            Self::Empty => None,
        }
    }
}

pub fn normalize_char(char: char) -> NormCharsIter<'static> {
    let char = char.to_ascii_lowercase();
    let decode = unidecode::unidecode_char(char).trim();
    if decode == "[?]" || decode == "" {
        return NormCharsIter::Single(char);
    }
    NormCharsIter::Multiple(decode.chars())
}

pub fn is_emoji(char: char) -> bool {
    match char {
        // dingbats
        '\u{2700}'..='\u{27BF}' => true,
        // emoticons
        '\u{1F600}'..='\u{1F64F}' => true,
        // Symbols and Pictographs Extended-A
        '\u{1FA70}'..='\u{1FAFF}' => true,
        // Miscellaneous Symbols
        '\u{2600}'..='\u{26FF}' => true,
        // Miscellaneous Symbols and Pictographs
        '\u{1F300}'..='\u{1F5FF}' => true,
        // Supplemental Symbols and Pictographs
        '\u{1F900}'..='\u{1F9FF}' => true,
        _ => false,
    }
}

pub fn is_void(char: char) -> bool {
    match char {
        //variation selector
        '\u{FE00}'..='\u{FE0F}' => true,
        // skintone modifiers
        '\u{1F3FB}'..='\u{1F3FF}' => true,
        _ => false,
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum CharType {
    Normal,
    Whitespace,
    Emoji,
}
impl CharType {
    pub fn new(char: char) -> Self {
        for char in normalize_char(char) {
            if is_emoji(char) {
                return CharType::Emoji;
            }
            if char.is_whitespace() {
                return CharType::Whitespace;
            }
        }
        CharType::Normal
    }
}

#[cfg(test)]
mod test {
    use crate::CharType;

    #[test]
    fn is_emoji() {
        assert_eq!(super::is_emoji('a'), false);
        assert_eq!(super::is_emoji('✅'), true);
        assert_eq!(super::is_emoji('😭'), true);

        assert_eq!(super::is_emoji('🩲'), true);
        assert_eq!(super::is_emoji('🫸'), true);
        assert_eq!(super::is_emoji('🙅'), true);
    }

    #[test]
    fn char_type() {
        assert_eq!(super::CharType::new('i'), CharType::Normal);
        assert_eq!(super::CharType::new('k'), CharType::Normal);
        assert_eq!(super::CharType::new('a'), CharType::Normal);
        assert_eq!(super::CharType::new('A'), CharType::Normal);
        assert_eq!(super::CharType::new('し'), CharType::Normal);

        assert_eq!(super::CharType::new('1'), CharType::Normal);
        assert_eq!(super::CharType::new('!'), CharType::Normal);
        assert_eq!(super::CharType::new('*'), CharType::Normal);
        assert_eq!(super::CharType::new(')'), CharType::Normal);
        assert_eq!(super::CharType::new(' '), CharType::Whitespace);
        assert_eq!(super::CharType::new('\t'), CharType::Whitespace);

        assert_eq!(super::CharType::new('😐'), CharType::Emoji);
        assert_eq!(super::CharType::new('🙅'), CharType::Emoji);
        assert_eq!(super::CharType::new('卐'), CharType::Normal);
        assert_eq!(super::CharType::new('à'), CharType::Normal);
        assert_eq!(super::CharType::new('€'), CharType::Normal);
        assert_eq!(super::CharType::new('Œ'), CharType::Normal);
    }
}
