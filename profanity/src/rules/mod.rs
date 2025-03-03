use thiserror::Error;

mod match_rule;
mod rep_rule;
pub use match_rule::*;
pub use rep_rule::*;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum Rule {
    Match(MatchRule),
    Replace(RepRule),
}

#[derive(Debug, Error)]
pub enum ParseRuleError {
    #[error("Error while parsing replace rule: {0}")]
    RepRule(#[from] RepRuleParseError),
    #[error("Error while parsing match rule: {0}")]
    MatchRule(MatchRuleParseError),
}

impl Rule {
    pub fn parse_from_str(line: &str) -> Result<Self, ParseRuleError> {
        Ok(if let Some(_) = line.find("=>") {
            let rule = RepRule::parse_from_str(line)?;
            Self::Replace(rule)
        } else {
            let rule = MatchRule::parse_from_str(line).map_err(|e| ParseRuleError::MatchRule(e))?;
            Self::Match(rule)
        })
    }
}
