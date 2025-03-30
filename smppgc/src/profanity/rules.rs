use profanity::Token;
use rocket::serde::{Deserialize, Serialize};

use super::LintImportance;

pub trait Rule {
    fn lint(&self) -> Vec<(LintImportance, &'static str)>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
#[serde(crate = "rocket::serde")]
pub struct MatchRule {
    pub enabled: bool,

    #[serde(flatten)]
    pub inner: profanity::MatchRule,
}
impl Rule for MatchRule {
    fn lint(&self) -> Vec<(LintImportance, &'static str)> {
        let mut lints = Vec::new();
        if self.inner.tokens.ends_with(&[
            Token::from_char('e').unwrap(),
            Token::from_char('n').unwrap(),
        ]) {
            lints.push((
                LintImportance::Notify,
                "Rule ends in -en. Ex. 'aaien' will not match 'aaie'. (Replace -en suffix with -e)",
            ));
        }
        if !self.inner.flags.contains(profanity::RuleFlags::NO_DEDUP) {
            let mut prev_token = None;
            for token in self.inner.tokens.iter() {
                if Some(token) == prev_token {
                    lints.push((
                        LintImportance::Error,
                        "Rule matches duplicated characters but no_dedup is turned off. This causes the rule to never match. (turn no_dedup on or remove duplicated character)",
                    ));
                    break;
                }
                prev_token = Some(token)
            }
        }
        lints
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct RepRule {
    pub enabled: bool,
    #[serde(flatten)]
    pub inner: profanity::RepRule,
}
impl Rule for RepRule {
    fn lint(&self) -> Vec<(LintImportance, &'static str)> {
        let mut vec = Vec::new();
        'outer: for (i, char) in self.inner.match_chars.chars().enumerate() {
            for (ii, char2) in self.inner.match_chars.chars().enumerate() {
                if char == char2 && i != ii {
                    vec.push((
                        LintImportance::Error,
                        "Character 2 times in match part of the replace rule",
                    ));
                    break 'outer;
                }
            }
        }

        'outer: for (i, token) in self.inner.replace_tg.iter().enumerate() {
            for (ii, token2) in self.inner.replace_tg.iter().enumerate() {
                if token == token2 && i != ii {
                    vec.push((
                        LintImportance::Error,
                        "Token 2 times in replace part of the replace rule",
                    ));
                    break 'outer;
                }
            }
        }

        vec
    }
}

#[cfg(test)]
mod test {
    use super::{LintImportance, MatchRule, RepRule, Rule};

    #[test]
    fn rep_rule_lint() {
        let rule = RepRule {
            inner: profanity::RepRule::parse_from_str("$'$_é&ö=>s/k").unwrap(),
            enabled: true,
        };
        assert_eq!(
            rule.lint(),
            vec![(
                LintImportance::Error,
                "Character 2 times in match part of the replace rule"
            )]
        );

        let rule = RepRule {
            inner: profanity::RepRule::parse_from_str("$'_é&ö=>/ks/k").unwrap(),
            enabled: true,
        };
        assert_eq!(
            rule.lint(),
            vec![(
                LintImportance::Error,
                "Token 2 times in replace part of the replace rule"
            )]
        );

        let rule = RepRule {
            inner: profanity::RepRule::parse_from_str("$'_é&ö=>/ks").unwrap(),
            enabled: true,
        };
        assert_eq!(rule.lint(), vec![])
    }

    #[test]
    fn match_rule_lint() {
        let rule = MatchRule {
            inner: profanity::MatchRule::parse_from_str("aaien").unwrap(),
            enabled: true,
        };
        assert_eq!(
            rule.lint(),
            vec![(
                LintImportance::Error,
                "Rule ends in -en. Ex. 'aaien' will not match 'aaie'. (Replace -en suffix with -e)"
            )]
        )
    }
}
