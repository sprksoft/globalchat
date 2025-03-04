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

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RulesetLint {
    pub importance: LintImportance,
    pub affected_rule: usize,
    pub second_affected_rule: Option<usize>,
    pub message: &'static str,
}

#[derive(Debug, Error)]
pub enum ProfFileLoadErr {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    ParseError(#[from] rules::ParseError),
}

pub struct ProfRuleset {
    filter_path: Option<PathBuf>,
    rules: Vec<rules::Rule>,
}
impl ProfRuleset {
    pub fn new(filter_path: PathBuf) -> Result<Self, ProfFileLoadErr> {
        let file = std::fs::read_to_string(&filter_path)?;

        let rules = rules::parse_from_str(&file)?;

        Ok(Self {
            filter_path: Some(filter_path),
            rules,
        })
    }
    pub fn from_str(string: &str) -> Result<Self, ProfFileLoadErr> {
        Ok(Self {
            filter_path: None,
            rules: rules::parse_from_str(string)?,
        })
    }
    pub fn from_rules(rules: Vec<rules::Rule>) -> Self {
        Self {
            filter_path: None,
            rules,
        }
    }
    pub fn rules(&self) -> &[rules::Rule] {
        &self.rules
    }

    pub fn lint(&self, filter: &ProfanityFilter) -> Vec<RulesetLint> {
        let mut lints = Vec::with_capacity(self.rules.len());
        for (i, rule) in self.rules.iter().enumerate() {
            for (importance, message) in rule.lint() {
                lints.push(RulesetLint {
                    affected_rule: i,
                    second_affected_rule: None,
                    message,
                    importance,
                })
            }
            if let Some(other_i) = self
                .rules()
                .iter()
                .enumerate()
                .find(|(ii, r)| *ii != i && r.inner == rule.inner)
                .map(|(i, _)| i)
            {
                lints.push(RulesetLint {
                    affected_rule: i,
                    second_affected_rule: Some(other_i),
                    message: "Duplicated rule found",
                    importance: LintImportance::Error,
                });
                continue;
            }
            match &rule.inner {
                profanity::Rule::Match(rule) => {
                    let tm = filter.tokenize_match_rule(&rule);
                    let matches = filter.check_all(&tm);
                    if let Some(other_match) = matches.iter().find(|m| m.rule != rule) {
                        let other_index = self
                            .rules
                            .iter()
                            .enumerate()
                            .find(|(_, r)| match &r.inner {
                                profanity::Rule::Match(r) => r == other_match.rule,
                                _ => false,
                            })
                            .map(|(i, _)| i)
                            .unwrap();

                        lints.push(RulesetLint {
                            affected_rule: i,
                            second_affected_rule: Some(other_index),
                            message: "Possible double match between 2 rules",
                            importance: LintImportance::Notify,
                        });
                    }
                }
                profanity::Rule::Replace(_rule) => {}
            }
        }

        lints
    }

    pub fn build_filter(&self) -> ProfanityFilter {
        let mut filter = ProfanityFilter::empty();
        for rule in self.rules.iter() {
            if rule.enabled {
                filter.insert_rule(rule.inner.clone())
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
    use crate::profanity::{LintImportance, ProfRuleset, RulesetLint};

    #[test]
    fn double_match_lint() {
        let ruleset = ProfRuleset::from_str("sexy\nsex").unwrap();
        let filter = ruleset.build_filter();
        assert_eq!(
            ruleset.lint(&filter),
            vec![RulesetLint {
                affected_rule: 0,
                second_affected_rule: Some(1),
                importance: LintImportance::Notify,
                message: "Possible double match between 2 rules"
            }]
        );
    }

    #[test]
    fn double_rule_lint() {
        let ruleset = ProfRuleset::from_str("sex\nsomething\nsex").unwrap();
        let filter = ruleset.build_filter();
        assert_eq!(
            ruleset.lint(&filter),
            vec![
                RulesetLint {
                    affected_rule: 0,
                    second_affected_rule: Some(2),
                    importance: LintImportance::Error,
                    message: "Duplicated rule found"
                },
                RulesetLint {
                    affected_rule: 2,
                    second_affected_rule: Some(0),
                    importance: LintImportance::Error,
                    message: "Duplicated rule found"
                }
            ]
        );
    }
}
