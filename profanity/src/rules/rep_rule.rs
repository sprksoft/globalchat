use crate::{tokens::TokenParseError, TokenGroup};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum RepRuleParseError {
    #[error("Couldn't find => in replace rule")]
    ArrowRequired,
    #[error("{0}")]
    TokenParseError(#[from] TokenParseError),
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

    pub fn matches(&self, chars: impl Iterator<Item = char>) -> bool {
        for char in chars {
            if self.match_chars.contains(char) {
                return true;
            }
        }
        false
    }

    pub fn to_string(&self) -> String {
        let mut string = String::with_capacity(self.match_chars.len() + 2 + self.replace_tg.len());
        let mut last_equals = false;
        for char in self.match_chars.chars() {
            if last_equals && char == '>' {
                string.push(' '); // Escape a => combination by adding a space inbetween
            }
            string.push(char);
            last_equals = false;
            if char == '=' {
                last_equals = true;
            }
        }
        string.push('=');
        string.push('>');
        for token in self.replace_tg.iter() {
            string.push_str(&token.to_string());
        }

        string
    }
}
