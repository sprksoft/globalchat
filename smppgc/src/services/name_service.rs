use std::{convert::Infallible, sync::Arc};

use rocket::{
    outcome::{try_outcome, Outcome},
    request::FromRequest,
};
use thiserror::Error;

use crate::{
    config::NameConfig,
    models::{ClaimedName, UserId},
    repositories::NameRepo,
    wf::{Filter, TokenizedString},
    wsprotocol::ProtoError,
};

#[derive(Error, Debug)]
pub enum InvalidNameError {
    #[error("Username too short or long")]
    Length,
    #[error("Username taken")]
    Taken,
    #[error("Username contains profanity")]
    Profanity,
}

impl InvalidNameError {
    pub fn into_kickreason(self) -> ProtoError {
        match self {
            Self::Profanity => ProtoError::UsernameProf,
            Self::Taken => ProtoError::UsernameTaken,
            Self::Length => ProtoError::UsernameLength,
        }
    }
}

#[derive(Error, Debug)]
pub enum NameClaimError {
    #[error("invalid: '{0}'")]
    Invalid(#[from] InvalidNameError),
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub struct NameService {
    filter: Arc<Filter>,
    repo: NameRepo,
    max_name_len: usize,
    max_claimed_names: usize,
    max_name_retention: usize,
}

impl NameService {
    fn tokenized_to_normalized(ts: TokenizedString) -> String {
        let mut output = String::new();
        for (_, _, word) in ts.norm_words() {
            output.push_str(word.str());
            output.push(' ');
        }
        output
    }
    pub async fn claim_name(
        &self,
        user_id: UserId,
        name: &str,
    ) -> Result<ClaimedName, NameClaimError> {
        let name = name.trim();
        if name.len() > self.max_name_len || name.len() < 2 {
            return Err(InvalidNameError::Length.into());
        }
        let (norm_name, name) = {
            let ts = self.filter.check(name).await;

            if !ts.good() {
                return Err(InvalidNameError::Profanity.into());
            }
            (Self::tokenized_to_normalized(ts), name)
        };

        self.repo
            .claim_name(
                user_id,
                &norm_name,
                self.max_claimed_names,
                self.max_name_retention,
            )
            .await?;

        Ok(ClaimedName::new(name))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NameService {
    type Error = Infallible;

    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let filter = req
            .rocket()
            .state::<Arc<Filter>>()
            .expect("Failed to get word filter");
        let name_config = req
            .rocket()
            .state::<NameConfig>()
            .expect("Failed to name config");

        let repo = try_outcome!(req.guard::<NameRepo>().await);

        Outcome::Success(NameService {
            filter: filter.clone(),
            repo,
            max_name_len: name_config.max_username_len,
            max_claimed_names: name_config.max_claimed_names,
            max_name_retention: name_config.max_name_retention,
        })
    }
}
