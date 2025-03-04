use profanity::Token;
use rocket::serde::{Deserialize, Serialize};
use thiserror::Error;

use super::LintImportance;

#[derive(Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Rule {
    pub enabled: bool,
    #[serde(flatten)]
    pub inner: profanity::Rule,
}
impl Rule {
    pub fn lint(&self) -> Vec<(LintImportance, &'static str)> {
        let mut lints = Vec::new();
        match &self.inner {
            profanity::Rule::Match(rule) => {
                if rule.tokens.ends_with(&[
                    Token::from_char('e').unwrap(),
                    Token::from_char('n').unwrap(),
                ]) {
                    lints.push((
                            LintImportance::Error,
                            "Rule ends in -en. Ex. 'aaien' will not match 'aaie'. (Replace -en suffix with -e)",
                    ));
                }
            }
            profanity::Rule::Replace(_rule) => {
                //TODO: check for double match chars
            }
        }
        lints
    }
}

#[derive(Debug, Error)]
#[error("{linenum}: {err}")]
pub struct ParseError {
    linenum: usize,
    err: profanity::ParseRuleError,
}

pub fn parse_from_str(str: &str) -> Result<Vec<Rule>, ParseError> {
    let mut rules = Vec::new();

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
        rules.push(Rule {
            enabled,
            inner: profanity::Rule::parse_from_str(rule_str).map_err(|err| ParseError {
                err,
                linenum: i + 1,
            })?,
        });
    }

    Ok(rules)
}

#[cfg(test)]
mod test {
    use super::{LintImportance, Rule};
    use profanity::MatchRule;

    #[test]
    fn lint() {
        let rule = Rule {
            inner: profanity::Rule::Match(MatchRule::parse_from_str("aaien").unwrap()),
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
