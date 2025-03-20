use super::UserSid;
use dashmap::DashMap;
use log::*;
use profanity::{ProfanityFilter, TokenizedMessage};
use rocket::{fairing::AdHoc, serde::Deserialize};
use std::{
    collections::VecDeque,
    ops::Deref,
    sync::{Arc, RwLock},
};
use thiserror::Error;

use crate::wsprotocol::KickReason;

#[derive(Error, Debug)]
pub enum NameClaimError {
    #[error("Username contains invalid characters")]
    Invalid,
    #[error("Username too short or long")]
    Length,
    #[error("Username taken")]
    Taken,
    #[error("Username contains profanity")]
    Profanity,
}
impl NameClaimError {
    pub fn into_kickreason(self) -> KickReason {
        match self {
            Self::Profanity => KickReason::UsernameProfanity,
            Self::Taken => KickReason::UsernameTaken,
            Self::Length => KickReason::UsernameInvalidLength,
            Self::Invalid => KickReason::UsernameInvalid,
        }
    }
}

struct NameSlot {
    name: Arc<str>,
    owner: Option<UserSid>,
}

pub struct UsernameManager {
    max_reserved: u16,
    names: DashMap<TokenizedMessage, NameSlot>,
    claims: DashMap<UserSid, VecDeque<TokenizedMessage>>,
}
impl UsernameManager {
    pub fn new(max_reserved: u16) -> Self {
        Self {
            max_reserved,
            claims: DashMap::default(),
            names: DashMap::default(),
        }
    }

    pub async fn claim_name(
        &self,
        name: &str,
        user_id: UserSid,
        max_name_len: usize,
        prof_filter: &RwLock<ProfanityFilter>,
    ) -> Result<ClaimedName, NameClaimError> {
        let name = name.trim();
        if name.len() > max_name_len || name.len() < 2 {
            return Err(NameClaimError::Length);
        }
        let (name, tokenized_name) = {
            let lock = prof_filter
                .read()
                .expect("Profanity filter lock has been poisoned");

            let (tokenized_name, name) = lock.tokenize(name);
            if lock.check(&tokenized_name).is_some() {
                return Err(NameClaimError::Profanity);
            }
            (Arc::<str>::from(name), tokenized_name)
        };

        if name.len() > max_name_len || name.len() < 2 {
            return Err(NameClaimError::Invalid);
        }

        {
            let mut slot = self
                .names
                .entry(tokenized_name.clone())
                .or_insert_with(|| NameSlot {
                    owner: Some(user_id.clone()),
                    name: name.clone(),
                });
            if slot.owner.as_ref().map(|o| *o != user_id).unwrap_or(false) {
                return Err(NameClaimError::Taken);
            }
            slot.owner = Some(user_id.clone());
            slot.name = name.clone();
        }

        let mut claimed_names = self
            .claims
            .entry(user_id)
            .or_insert(VecDeque::with_capacity(self.max_reserved as usize));

        if claimed_names.len() == self.max_reserved as usize {
            if let Some(name) = claimed_names.pop_back() {
                if name != tokenized_name {
                    self.names.remove(&name);
                }
            }
        }
        claimed_names.push_front(tokenized_name);

        Ok(ClaimedName(name))
    }
}

pub struct ClaimedName(Arc<str>);
impl Deref for ClaimedName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<Arc<str>> for ClaimedName {
    fn into(self) -> Arc<str> {
        self.0.into()
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UserConfig {
    pub max_reserved_names: u16,
    pub max_username_len: usize,
}

pub(super) fn stage() -> AdHoc {
    AdHoc::on_ignite("username manager", |r| async {
        let config = r
            .figment()
            .extract::<UserConfig>()
            .expect("No username config");
        let max_reserved_names = config.max_reserved_names;
        r.manage(config)
            .manage(UsernameManager::new(max_reserved_names))
    })
}
