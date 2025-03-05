use std::path::PathBuf;
use std::sync::Mutex;

use log::*;
use profanity::ProfanityFilter;
use rocket::fairing::AdHoc;
use rocket::serde::{Deserialize, Serialize};
use thiserror::Error;

mod rules;
pub use rules::*;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum LintImportance {
    Error,
    Notify,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Lint {
    pub importance: LintImportance,
    pub affected_rule: usize,
    pub second_affected_rule: Option<usize>,
    pub message: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LintSet {
    match_lints: Vec<Lint>,
    rep_lints: Vec<Lint>,
    #[serde(skip)]
    has_errors: bool,
}
impl LintSet {
    pub fn has_errors(&self) -> bool {
        self.has_errors
    }
    pub fn rep_lints(&self) -> &[Lint] {
        &self.rep_lints
    }
    pub fn match_lints(&self) -> &[Lint] {
        &self.match_lints
    }
}

#[derive(Debug, Error)]
#[error("{linenum}: {err}")]
pub struct ParseError {
    linenum: usize,
    err: profanity::ParseRuleError,
}

#[derive(Debug, Error)]
pub enum RulesetError {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    ParseError(#[from] ParseError),

    #[error("No filter path set on this ruleset")]
    NoFilterPath,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct ProfRuleset {
    #[serde(skip)]
    filter_path: Option<PathBuf>,
    rep_rules: Vec<rules::RepRule>,
    match_rules: Vec<rules::MatchRule>,
}
impl ProfRuleset {
    pub fn new(filter_path: PathBuf) -> Result<Self, RulesetError> {
        let file = std::fs::read_to_string(&filter_path)?;

        let mut me = Self::parse_from_str(&file)?;
        me.filter_path = Some(filter_path);

        Ok(me)
    }

    pub fn parse_from_str(str: &str) -> Result<Self, ParseError> {
        let mut match_rules = Vec::new();
        let mut rep_rules = Vec::new();

        for (i, line) in str.lines().enumerate() {
            let line = &line[..line.find('#').unwrap_or(line.len())];
            if line.is_empty() {
                continue;
            }
            let (enabled, rule_str) = if line.starts_with('*') {
                (false, &line[1..])
            } else {
                (true, line)
            };

            match profanity::Rule::parse_from_str(rule_str).map_err(|err| ParseError {
                err,
                linenum: i + 1,
            })? {
                profanity::Rule::Match(m) => match_rules.push(MatchRule { enabled, inner: m }),
                profanity::Rule::Replace(r) => rep_rules.push(RepRule { enabled, inner: r }),
            }
        }

        Ok(Self {
            rep_rules,
            match_rules,
            filter_path: None,
        })
    }

    pub fn from_rules(rep_rules: Vec<rules::RepRule>, match_rules: Vec<rules::MatchRule>) -> Self {
        Self {
            filter_path: None,
            rep_rules,
            match_rules,
        }
    }
    pub fn rep_rules(&self) -> &[rules::RepRule] {
        &self.rep_rules
    }
    pub fn match_rules(&self) -> &[rules::MatchRule] {
        &self.match_rules
    }
    pub fn replace_from(&mut self, other: ProfRuleset) {
        self.rep_rules = other.rep_rules;
        self.match_rules = other.match_rules;
    }

    pub fn to_string(&self) -> String {
        let mut string = String::new();

        for rule in self.rep_rules() {
            if !rule.enabled {
                string.push('*');
            }
            string.push_str(&rule.inner.to_string());
            string.push('\n');
        }
        for rule in self.match_rules() {
            if !rule.enabled {
                string.push('*');
            }
            string.push_str(&rule.inner.to_string());
            string.push('\n');
        }
        string
    }

    pub fn save(&self) -> Result<(), RulesetError> {
        let filter_path = self
            .filter_path
            .as_ref()
            .ok_or(RulesetError::NoFilterPath)?;

        std::fs::write(&filter_path, self.to_string())?;
        Ok(())
    }

    pub fn lint(&self, filter: &ProfanityFilter) -> LintSet {
        let mut rep_lints = Vec::new();
        let mut has_errors = false;
        for (i, rule) in self.rep_rules.iter().enumerate() {
            for (importance, message) in rule.lint() {
                rep_lints.push(Lint {
                    affected_rule: i,
                    second_affected_rule: None,
                    message,
                    importance,
                })
            }

            for (ii, other_rule) in self.rep_rules().iter().enumerate() {
                if ii == i {
                    continue;
                }
                if other_rule.inner.matches(rule.inner.match_chars.chars()) {
                    rep_lints.push(Lint {
                        affected_rule: i,
                        second_affected_rule: Some(ii),
                        message: "Found 2 replace rules that replace the same character",
                        importance: LintImportance::Error,
                    });
                    has_errors = true;
                }
            }
        }

        let mut match_lints = Vec::new();
        for (i, rule) in self.match_rules.iter().enumerate() {
            for (importance, message) in rule.lint() {
                match_lints.push(Lint {
                    affected_rule: i,
                    second_affected_rule: None,
                    message,
                    importance,
                })
            }
            if let Some(other_i) = self
                .match_rules()
                .iter()
                .enumerate()
                .find(|(ii, r)| *ii != i && r.inner == rule.inner)
                .map(|(i, _)| i)
            {
                match_lints.push(Lint {
                    affected_rule: i,
                    second_affected_rule: Some(other_i),
                    message: "Duplicated match rule found",
                    importance: LintImportance::Error,
                });
                has_errors = true;
                continue;
            }
            let tm = filter.tokenize_match_rule(&rule.inner);
            let matches = filter.check_all(&tm);
            if let Some(other_match) = matches.iter().find(|m| *m.rule != rule.inner) {
                let other_index = self
                    .match_rules()
                    .iter()
                    .enumerate()
                    .find(|(_, r)| r.inner == *other_match.rule)
                    .map(|(i, _)| i)
                    .unwrap();

                match_lints.push(Lint {
                    affected_rule: i,
                    second_affected_rule: Some(other_index),
                    message: "Possible double match between 2 rules",
                    importance: LintImportance::Notify,
                });
            }
        }

        LintSet {
            match_lints,
            rep_lints,
            has_errors,
        }
    }

    pub fn build_filter(&self) -> ProfanityFilter {
        let mut filter = ProfanityFilter::empty();
        for rule in self.rep_rules.iter() {
            if rule.enabled {
                filter.insert_rep_rule(rule.inner.clone())
            }
        }
        for rule in self.match_rules.iter() {
            if rule.enabled {
                filter.insert_match_rule(rule.inner.clone())
            }
        }
        filter
    }

    // pub async fn load(&self) -> Result<(), ProfFileLoadErr> {
    //     let file = std::fs::read_to_string(&self.filter_path)?;
    //     self.filter.write().await.add_from_parsed(&file)?;
    //     Ok(())
    // }

    // pub async fn filter_string(
    //     &self,
    //     string: &str,
    // ) -> (Result<String, Range<usize>>, TokenizedMessage) {
    //     let filter = self.filter.read().await;
    //     let (tokenized, string) = filter.tokenize(string);
    //     if let Some(r) = filter.check(&tokenized) {
    //         (Err(r.span), tokenized)
    //     } else {
    //         (Ok(string), tokenized)
    //     }
    // }
    // pub async fn contains_profanity_any(&self, strings: impl Iterator<Item = &str>) -> bool {
    //     let filter = self.filter.read().await;
    //     for str in strings {
    //         if filter.contains_profanity(str) {
    //             return true;
    //         }
    //     }
    //     false
    // }
    //
    // pub async fn filter_all(&self, messages: impl Iterator<Item = &mut Message>) {
    //     let filter = self.filter.read().await;
    //     for message in messages {
    //         if filter.contains_profanity(&message.content) {
    //             Self::hide_message(message)
    //         }
    //         if filter.contains_profanity(&message.sender) {
    //             Self::hide_message_sender(message)
    //         }
    //     }
    // }
    //
    // pub async fn filter_message_content(
    //     &self,
    //     message: &mut Message,
    // ) -> (Result<&mut Message, Range<usize>>) {
    //     let filter = self.filter.read().await;
    //
    //     let mut prof = false;
    //     if filter.contains_profanity(&message.content) {
    //         Self::hide_message(message);
    //         prof = true;
    //     }
    //     if filter.contains_profanity(&message.sender) {
    //         Self::hide_message_sender(message);
    //         prof = true;
    //     }
    //     prof
    // }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct ProfConfig {
    prof_filter_file: PathBuf,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("profanity filter", |r| async {
        let config = r
            .figment()
            .extract::<ProfConfig>()
            .expect("No profanity config found");

        let ruleset = ProfRuleset::new(config.prof_filter_file)
            .expect("Failed to load profanity filter rules");
        let filter = ruleset.build_filter();
        r.manage(std::sync::RwLock::new(filter))
            .manage(Mutex::new(ruleset))
    })
}

#[cfg(test)]
mod test {
    use crate::profanity::{Lint, LintImportance, LintSet, ProfRuleset};

    #[test]
    fn json_test() {
        let orig =
            ProfRuleset::parse_from_str("aeéè => xbd/k/w\nî=>lji\npotato\nsomething").unwrap();
        let json = rocket::serde::json::to_string(&orig).unwrap();

        assert_eq!(
            rocket::serde::json::from_str::<ProfRuleset>(&json).unwrap(),
            orig
        )
    }

    #[test]
    fn double_match_lint() {
        let ruleset = ProfRuleset::parse_from_str("sexy\nsex").unwrap();
        let filter = ruleset.build_filter();
        assert_eq!(
            ruleset.lint(&filter),
            LintSet {
                match_lints: vec![Lint {
                    affected_rule: 0,
                    second_affected_rule: Some(1),
                    importance: LintImportance::Notify,
                    message: "Possible double match between 2 rules"
                }],
                rep_lints: vec![],
                has_errors: false,
            }
        );
    }

    #[test]
    fn double_rule_lint() {
        let ruleset = ProfRuleset::parse_from_str("sex\nsomething\nsex").unwrap();
        let filter = ruleset.build_filter();
        assert_eq!(
            ruleset.lint(&filter),
            LintSet {
                match_lints: vec![
                    Lint {
                        affected_rule: 0,
                        second_affected_rule: Some(2),
                        importance: LintImportance::Error,
                        message: "Duplicated match rule found"
                    },
                    Lint {
                        affected_rule: 2,
                        second_affected_rule: Some(0),
                        importance: LintImportance::Error,
                        message: "Duplicated match rule found"
                    }
                ],
                rep_lints: vec![],
                has_errors: true,
            }
        );
    }
}
