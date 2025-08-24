#![feature(test)]

pub mod bench;
pub mod other_impls;
pub mod test_data;

pub const PROFANITY_V2: &'static str = include_str!("profanity_v2.filter");
pub const WF: &'static str = include_str!("wf.filter.txt");

pub const WORD_LIST: &'static str = include_str!("wordlist.txt");

pub fn gen_wordlist<T: From<String> + std::cmp::Ord>() -> Vec<T> {
    let mut list: Vec<T> = WORD_LIST
        .lines()
        .map(|w| w.trim_matches('"').to_lowercase())
        .filter(|w| w.len() > 0)
        .map(|w| w.into())
        .collect();
    list.sort();
    list
}
