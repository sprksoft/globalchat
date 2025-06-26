use std::{ops::Range, str::CharIndices};

const WHITESPACE: [char; 4] = [' ', '.', '_', '\n'];
const ALLOWED_CHARS: [char; 7] = ['?', '!', ':', '-', '+', '\'', '"'];

pub type Span = Range<usize>;

#[derive(Debug, Clone)]
pub struct Word {
    pub span: Span,
    string: String,
}
impl Word {
    pub fn str(&self) -> &str {
        &self.string
    }
}
impl Into<Box<str>> for Word {
    fn into(self) -> Box<str> {
        self.string.into_boxed_str()
    }
}

#[inline]
fn stem(string: &mut String) {
    if string.ends_with("y") {
        string.pop();
    } else if string.ends_with("en") || string.ends_with("je") {
        string.pop();
        string.pop();
    } else if string.ends_with("ing") || string.ends_with("ers") {
        string.pop();
        string.pop();
        string.pop();
    } else if string.ends_with("baar") || string.ends_with("lijk") {
        string.pop();
        string.pop();
        string.pop();
        string.pop();
    }
}

#[inline]
fn push_normalized(string: &mut String, char: char) {
    if char.is_numeric() {
        return;
    }
    if char.is_ascii_alphanumeric() {
        string.push(char.to_ascii_lowercase());
        return;
    }
    if ALLOWED_CHARS.contains(&char) {
        string.push(char);
        return;
    }
    match char {
        'é' | 'è' => {
            string.push('e');
        }
        _ => {}
    }
}

pub struct ProcessedWordsIter<'a> {
    char_iter: CharIndices<'a>,
}
impl<'a> Iterator for ProcessedWordsIter<'a> {
    type Item = Word;
    fn next(&mut self) -> Option<Self::Item> {
        let (start_index, start_char) = loop {
            let (index, char) = self.char_iter.next()?;
            if !WHITESPACE.contains(&char) {
                break (index, char);
            }
        };
        let mut string = String::new();
        let mut prev_char = start_char;
        push_normalized(&mut string, start_char);
        loop {
            let (index, char) = match self.char_iter.next() {
                Some((index, char)) => (index, char),
                None => {
                    return Some(Word {
                        string,
                        span: start_index..self.char_iter.offset(),
                    })
                }
            };
            if WHITESPACE.contains(&char) {
                stem(&mut string);
                return Some(Word {
                    string,
                    span: start_index..index,
                });
            }
            if prev_char != char {
                push_normalized(&mut string, char);
            }
            prev_char = char;
        }
    }
}

pub fn process_data_to_words<'a>(data: &'a str) -> ProcessedWordsIter<'a> {
    ProcessedWordsIter {
        char_iter: data.char_indices(),
    }
}
