use crate::{tokens::TokenParseError, TokenGroup};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum RepRuleParseError {
    #[error("Couldn't find => in replace rule")]
    ArrowRequired,
    #[error("{0}")]
    TokenParseError(#[from] TokenParseError),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RepRule {
    pub match_chars: String,
    pub replace_tg: TokenGroup,
}

impl RepRule {
    pub fn parse_from_str(str: &str) -> Result<Self, RepRuleParseError> {
        let Some(arrow_pos) = str.find("=>") else {
            return Err(RepRuleParseError::ArrowRequired);
        };
        let mut match_chars = String::with_capacity(str[..arrow_pos].len());
        for char in str[..arrow_pos].chars() {
            if char.is_whitespace() {
                continue;
            }
            match_chars.push(char);
        }
        let replace_tg = TokenGroup::parse_from_str(&str[arrow_pos + 2..])?;

        Ok(Self {
            match_chars,
            replace_tg,
        })
    }
}
