use std::{iter::FlatMap, str::Lines};

const WHITESPACE: [char; 4] = [' ', '-', '_', ':'];

fn data_to_words(data: &str, mut f: impl FnMut(String)) {
    for line in data.split('\n') {
        for word in line.split(&Self::WHITESPACE) {
            f(Self::norm_word(word))
        }
    }
}

fn norm_word(word: &str) -> String {
    let mut prev_char = None;
    let mut str_word = String::with_capacity(word.len());

    let word = if word.ends_with("en") || word.ends_with("je") || word.ends_with("ing") {
        &word[..word.len() - 2]
    } else if word.ends_with("e") || word.ends_with("y") {
        &word[..word.len() - 1]
    } else {
        word
    };

    for char in word.chars() {
        if Some(char) == prev_char || char.is_numeric() {
            prev_char = Some(char);
            continue;
        }
        if char == 'é' || char == 'è' {
            str_word.push('e');
        } else {
            for char in char.to_lowercase() {
                str_word.push(char);
            }
        }
        prev_char = Some(char);
    }

    str_word
}

fn process_data_to_words<'a>(data: &'a str) -> FlatMap<Lines> {
    data.lines()
        .flat_map(|l| l.split(&WHITESPACE).map(|w| norm_word(w)))
}
