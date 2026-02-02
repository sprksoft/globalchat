use crate::{
    ansii::*, is_void, normalize_char, CharType, ColoredTokenTag, FilterMeta, FromEntry, Tag,
    TokenTag,
};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    ops::Range,
    sync::OnceLock,
};

use crate::WordFilter;

#[inline]
fn should_modify(word: &mut NormalizedWord, fix: &'static str) -> bool {
    word.root().len() >= 4 + fix.len()
}

#[inline]
fn suffix(word: &mut NormalizedWord, suffix: impl IntoIterator<Item = &'static str>) -> bool {
    for suffix in suffix.into_iter() {
        if should_modify(word, suffix) && word.root().ends_with(suffix) {
            word.root_end -= suffix.len();
            return true;
        }
    }
    false
}

#[inline]
fn prefix(word: &mut NormalizedWord, prefix: impl IntoIterator<Item = &'static str>) -> bool {
    for prefix in prefix.into_iter() {
        if should_modify(word, prefix) && word.root().starts_with(prefix) {
            word.root_start += prefix.len();
            return true;
        }
    }
    false
}
macro_rules! ret {
    ($expr:expr) => {{
        if $expr {
            return true;
        }
    }};
}

#[derive(Clone, Hash, Debug, PartialEq, Eq)]
pub struct NormalizedWord {
    root_end: usize,
    root_start: usize,
    word: String,
}
impl NormalizedWord {
    pub fn normalize(str: &str) -> NormalizedWord {
        let str = Self::strip_punct(str);
        let mut word = String::with_capacity(str.len());
        let mut prev_char = None;
        let mut rep_count = 0;
        for char in str.chars() {
            for char in normalize_char(char) {
                let char = char.to_ascii_lowercase();
                if char.is_whitespace() {
                    continue;
                }
                if prev_char.map(|prev| prev == char).unwrap_or(false) {
                    rep_count += 1;
                } else {
                    rep_count = 0;
                }
                if rep_count >= 2 {
                    continue;
                }
                word.push(char);
                prev_char = Some(char);
            }
        }

        let mut word = NormalizedWord {
            root_end: word.len(),
            root_start: 0,
            word,
        };

        if !Self::stem_dutch(&mut word) {
            Self::stem_english(&mut word);
        }

        word
    }

    fn strip_punct(mut str: &str) -> &str {
        // 2 passes to allow for: ???!!!
        for _ in 0..2 {
            for punct in ['?', '!', '.', ','] {
                let ends_with = str.ends_with(punct);
                str = str.trim_end_matches(punct);
                if ends_with {
                    break;
                }
            }
        }

        str
    }

    #[inline]
    fn stem_english(word: &mut NormalizedWord) -> bool {
        // VD
        ret!(suffix(word, ["ed"]));

        // verklein woorden
        ret!(suffix(word, ["y"]));

        // compare
        ret!(suffix(word, ["est", "er"]));

        // Continuous tense
        ret!(suffix(word, ["ing"]));

        // meervoud
        ret!(suffix(word, ["ies", "s"]));

        false
    }
    #[inline]
    fn stem_dutch(word: &mut NormalizedWord) -> bool {
        // voltooid deel woord
        if prefix(word, ["ge"]) {
            suffix(word, ["d", "t"]);
            return true;
        }

        // verkleinwoorden
        ret!(suffix(word, ["jes", "tje", "pje", "etje", "je"]));

        // adjectief
        ret!(suffix(word, ["baar", "lijk", "ig", "achtig"]));

        // meervoud
        ret!(suffix(word, ["en", "e"]));

        false
    }
    pub fn str(&self) -> &str {
        &self.word
    }
    pub fn root(&self) -> &str {
        &self.word[self.root_start..self.root_end]
    }

    pub fn is_number(&self) -> bool {
        let word = Self::remove_fixes(
            &self.word,
            ["eur"],
            [
                "%", "$", "eur", /* € */
                "y=",  /* ¥ */
                "ps",  /* £ */
                "km", "kg", "m", "l", "kb", "mb", "gb", "tb",
            ],
        );

        for char in word.chars() {
            if !(char.is_digit(10) || char == ',' || char == '.') {
                return false;
            }
        }
        true
    }

    fn remove_fixes<'a, const PC: usize, const SC: usize>(
        str: &'a str,
        prefixes: [&'static str; PC],
        suffixes: [&'static str; SC],
    ) -> &'a str {
        for m in prefixes {
            if str.starts_with(m) {
                return &str[m.len()..];
            }
        }

        for m in suffixes {
            if str.ends_with(m) {
                return &str[..str.len() - m.len()];
            }
        }
        str
    }
}
impl From<NormalizedWord> for Box<str> {
    fn from(mut value: NormalizedWord) -> Self {
        value.word.truncate(value.root_end);
        value.word.into_boxed_str()
    }
}
pub trait IntoNormalizedWord {
    fn into_normalized_word<'a>(&'a self) -> Cow<'a, NormalizedWord>;
}
impl IntoNormalizedWord for &str {
    fn into_normalized_word<'a>(&'a self) -> Cow<'a, NormalizedWord> {
        Cow::Owned(NormalizedWord::normalize(self))
    }
}
impl IntoNormalizedWord for NormalizedWord {
    fn into_normalized_word<'a>(&'a self) -> Cow<'a, NormalizedWord> {
        Cow::Borrowed(self)
    }
}

#[derive(Clone)]
struct Token<T> {
    tag: T,
    span: Range<usize>,
    norm: OnceLock<NormalizedWord>,
}
impl<T: Eq> Eq for Token<T> {}
impl<T: Eq> PartialEq for Token<T> {
    fn eq(&self, other: &Self) -> bool {
        self.span == other.span && self.tag == other.tag
    }
}
impl<T: Debug> Debug for Token<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        self.span.fmt(f)?;
        f.write_str(", ")?;
        self.tag.fmt(f)?;
        f.write_str(")")?;
        Ok(())
    }
}

impl<T> Token<T> {
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
pub struct TokenizedString<T: Eq = Tag>(Box<str>, Vec<Token<T>>);
impl<T: TokenTag> TokenizedString<T> {
    pub fn from_words<'a>(words: impl IntoIterator<Item = impl IntoWordTagPair<'a, T>>) -> Self {
        let iter = words.into_iter();

        let mut tokens: Vec<Token<T>> = Vec::with_capacity(iter.size_hint().1.unwrap_or(1));

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

    pub fn tokenize(str: impl Into<Box<str>>) -> Self {
        let str = str.into();
        let mut tokens = Vec::new();

        let mut start_index = 0;
        let mut prev_char = None;
        for (index, char) in str.char_indices() {
            if is_void(char) {
                continue;
            }
            if let Some(prev_char) = prev_char {
                let ty = CharType::new(char);
                let prev_ty = CharType::new(prev_char);

                // Split tokens on char type boundaries
                if ty != prev_ty {
                    // Assign whitespace tag if the char type is whitespace.
                    let tag = if prev_ty == CharType::Whitespace {
                        T::whitespace()
                    } else {
                        T::unknown() // other types of tags will be assigned during checking.
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
            T::whitespace()
        } else {
            T::unknown() // other types of tags will be assigned during checking.
        };
        tokens.push(Token::new(start_index..str.len(), tag));

        TokenizedString(str, tokens.into())
    }

    fn get_token(tokens: &Vec<Token<T>>, mut i: usize) -> Option<&Token<T>> {
        loop {
            let t = tokens.get(i)?;
            if !t.tag.is_whitespace() {
                return Some(t);
            }
            i += 1;
        }
    }

    /// Returns true if a tag is changed
    pub fn recheck<M: FilterMeta>(&mut self, filter: &WordFilter<M>) -> bool
    where
        T: FromEntry<M>,
    {
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

        for i in 0..tokens.len() {
            let token = &mut tokens[i];
            if token.tag.is_whitespace() {
                continue;
            }
            let norm = token.norm(&self.0);
            if norm.str().trim().len() == 0 {
                set!(token.tag, T::good());
                continue;
            }
            let Some(entry) = filter.get_entry(norm) else {
                if norm.is_number() {
                    set!(token.tag, T::good());
                } else {
                    set!(token.tag, T::unknown());
                }
                continue;
            };

            let mut tag = None;
            if let Some(next_token) = Self::get_token(tokens, i + 1) {
                if entry
                    .forward_ctx
                    .iter()
                    .find(|c| {
                        c.as_ref() == next_token.norm(&self.0).root()
                            || c.as_ref() == next_token.norm(&self.0).str()
                    })
                    .is_some()
                {
                    // when a word is marked good matching the forward context will make it bad.
                    // and vice versa. that's why we invert the tag here.
                    tag = Some(T::from_matched(!entry.good, &entry.meta)); // inverted tag
                }
            }
            set!(
                tokens[i].tag,
                tag.unwrap_or(T::from_matched(entry.good, &entry.meta))
            );
        }
        changed
    }

    pub fn good(&self) -> bool {
        self.words()
            .find(|(_, tag)| !(tag.is_good() || tag.is_whitespace()))
            .is_none()
    }

    pub fn words(&self) -> impl Iterator<Item = (&str, T)> {
        self.1
            .iter()
            .map(|token| (&self.0[token.span.clone()], token.tag))
    }

    pub fn norm_words(&self) -> impl Iterator<Item = (&str, T, &NormalizedWord)> {
        self.1
            .iter()
            .map(|token| (&self.0[token.span.clone()], token.tag, token.norm(&self.0)))
    }

    pub fn to_string(&self) -> String {
        let mut str = String::new();
        for (word, _) in self.words() {
            str.push_str(word);
        }
        str
    }
}
impl<T: TokenTag + ColoredTokenTag> TokenizedString<T> {
    pub fn colored<'a>(&'a self) -> ColoredFmt<'a, T> {
        ColoredFmt(&self)
    }
}

pub struct ColoredFmt<'a, T: TokenTag>(&'a TokenizedString<T>);
impl<'a, T: TokenTag + ColoredTokenTag> Display for ColoredFmt<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (word, tag) in self.0.words() {
            f.write_str(tag.ansii_color())?;
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
            TokenizedString::tokenize("strawberry <3"),
            TokenizedString::from_words([
                ("strawberry", Tag::Unknown),
                (" ", Tag::Whitespace),
                ("<3", Tag::Unknown),
            ])
        );

        assert_eq!(
            TokenizedString::tokenize("\nstuf"),
            TokenizedString::from_words([("\n", Tag::Whitespace), ("stuf", Tag::Unknown)])
        );

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
            TokenizedString::<Tag>::tokenize("ik ben sibe")
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

    // #[test]
    // fn ghost_chars() {
    //     assert_eq!(
    //         TokenizedString::tokenize(":word:"),
    //         TokenizedString::from_words([(":word:", Tag::Unknown),])
    //     );
    // }

    #[test]
    fn stemming_test() {
        assert_eq!(
            Box::<str>::from(NormalizedWord::normalize("fucken")),
            "fuck".into()
        );
        assert_eq!(NormalizedWord::normalize("fucken").root(), "fuck");
        assert_eq!(NormalizedWord::normalize("10kg").root(), "10kg");

        assert_eq!(NormalizedWord::normalize("fucking").root(), "fuck");
        assert_eq!(NormalizedWord::normalize("studies").root(), "stud");
        assert_eq!(NormalizedWord::normalize("smppgc").root(), "smppgc");
        assert_eq!(NormalizedWord::normalize("filters").root(), "filter");

        assert_eq!(NormalizedWord::normalize("persen").root(), "pers");
        assert_eq!(NormalizedWord::normalize("pers").root(), "pers");
        assert_eq!(NormalizedWord::normalize("taller").root(), "tall");
        assert_eq!(NormalizedWord::normalize("tallest").root(), "tall");
        assert_eq!(NormalizedWord::normalize("ben").root(), "ben");
        assert_eq!(NormalizedWord::normalize("are").root(), "are");

        assert_eq!(NormalizedWord::normalize("卍").root(), "wan");
        assert_eq!(NormalizedWord::normalize("™").root(), "tm");

        assert_eq!(NormalizedWord::normalize("nee").root(), "nee");
        assert_eq!(NormalizedWord::normalize("neee").root(), "nee");
        assert_eq!(NormalizedWord::normalize("hiiii").root(), "hii");
        assert_eq!(
            NormalizedWord::normalize("niiiiggggaaaaaa").root(),
            "niiggaa"
        );

        assert_eq!(NormalizedWord::normalize("123").root(), "123");
    }
    #[test]
    fn punct_test() {
        assert_eq!(NormalizedWord::normalize("hello?").root(), "hello");
        assert_eq!(NormalizedWord::normalize("hello,").root(), "hello");
        assert_eq!(NormalizedWord::normalize("yow!").root(), "yow");
        assert_eq!(NormalizedWord::normalize("yow!!!!!!").root(), "yow");
        assert_eq!(NormalizedWord::normalize("zin.").root(), "zin");
        assert_eq!(NormalizedWord::normalize("zin...").root(), "zin");
        assert_eq!(NormalizedWord::normalize("zin?!!!!").root(), "zin");
        assert_eq!(NormalizedWord::normalize("zin!????").root(), "zin");
        assert_eq!(NormalizedWord::normalize("zin!,.!?").root(), "zin!,.");
        assert_eq!(NormalizedWord::normalize("z.i.n").root(), "z.i.n");
    }

    #[test]
    fn is_number() {
        assert!(NormalizedWord::normalize("123").is_number());
        assert!(NormalizedWord::normalize("123,0").is_number());
        assert!(NormalizedWord::normalize("12.3").is_number());
        assert!(NormalizedWord::normalize("12.3%").is_number());
        assert!(NormalizedWord::normalize("12.3€").is_number());
        assert!(NormalizedWord::normalize("€12.3").is_number());
        assert!(NormalizedWord::normalize("12$").is_number());
        assert!(NormalizedWord::normalize("50.3%").is_number());
        assert!(NormalizedWord::normalize(".50").is_number());

        assert!(NormalizedWord::normalize("100¥").is_number());
        assert!(NormalizedWord::normalize("100£").is_number());
        assert!(NormalizedWord::normalize("10kg").is_number());
        assert!(NormalizedWord::normalize("10KG").is_number());
        assert!(NormalizedWord::normalize("10l").is_number());

        assert!(!NormalizedWord::normalize("123a").is_number());
        assert!(!NormalizedWord::normalize("1$50").is_number());
        assert!(!NormalizedWord::normalize("1%50").is_number());
    }
}
