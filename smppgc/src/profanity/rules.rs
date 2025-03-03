use rocket::serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Rule {
    pub enabled: bool,
    #[serde(flatten)]
    pub inner: profanity::Rule,
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
