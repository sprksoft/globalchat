#[derive(Serialize, Deserialize)]
pub struct ProfanityFilter {
    tree: StringTree,
}
impl ProfanityFilter {
    pub fn from_wordlist(wordlist: &str) -> Self {
        let words = wordlist
            .lines()
            .map(|l| l.trim_matches(['"']).to_lowercase())
            .filter(|i| i.len() > 0)
            .collect();
        //println!("{:?}", words);
        Self {
            tree: StringTree::from_vec(words),
        }
    }
    pub fn add_word(&mut self, word: impl Into<Box<str>>) {
        self.tree.add(word.into(), 0);
    }

    pub fn contains_profanity(&self, string: &str) -> bool {
        sentence_contains(&self.tree, string)
    }
}
