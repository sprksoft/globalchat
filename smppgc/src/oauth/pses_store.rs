use std::{
    fmt::Display,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use log::*;
use rocket::{fairing::AdHoc, form::FromFormField};
use thiserror::Error;
use uuid::Uuid;

use crate::models::SesId;

enum Status {
    Pending,
    Completed(SesId),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingSessionType {
    Delayed,
    Immediate,
}
impl PendingSessionType {
    pub fn str(self) -> &'static str {
        match self {
            Self::Delayed => "delayed",
            Self::Immediate => "immediate",
        }
    }
}
impl Display for PendingSessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.str())
    }
}

#[derive(Debug, Error)]
pub enum PendingSessionTypeParseError {
    #[error("Invalid pending session type")]
    InvalidPSesType,
}
impl FromStr for PendingSessionType {
    type Err = PendingSessionTypeParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "immediate" => Ok(Self::Immediate),
            "delayed" => Ok(Self::Delayed),
            _ => Err(PendingSessionTypeParseError::InvalidPSesType),
        }
    }
}
impl<'v> FromFormField<'v> for PendingSessionType {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        let date = Self::from_str(field.value)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(date)
    }
}

pub struct Completed {
    pub ses_id: SesId,
    pub redirect: String,
    pub ses_type: PendingSessionType,
}

struct PendingSession {
    created_at: Instant,
    redirect: String,
    ses_type: PendingSessionType,
    status: Status,
}

pub struct PendingSessionStore(Arc<DashMap<uuid::Uuid, PendingSession>>);
impl PendingSessionStore {
    pub fn new_pending(&self, redirect: String, ses_type: PendingSessionType) -> Uuid {
        let ses = self.new_pending_and_get(redirect, ses_type);
        let id = ses.id.clone();
        self.0.insert(id, ses.session);
        ses.id
    }

    /// Creates a new pending session and returns.
    pub fn new_pending_and_get<'a>(
        &'a self,
        mut redirect: String,
        login_type: PendingSessionType,
    ) -> PendingSessionGuard<'a> {
        if !validate_redirect_url(&redirect) {
            error!("Setting redirect url to /v1 because it was invalid");
            redirect = "/v1".to_string();
        }
        let id = Uuid::new_v4();
        PendingSessionGuard {
            session: PendingSession {
                created_at: Instant::now(),
                status: Status::Pending,
                redirect,
                ses_type: login_type,
            },
            id,
            store: self,
        }
    }

    pub fn session<'a>(&'a self, id: Uuid) -> Option<PendingSessionGuard<'a>> {
        let (id, session) = self.0.remove(&id)?;

        Some(PendingSessionGuard {
            session,
            id,
            store: self,
        })
    }

    pub fn consume_delayed_session(&self, id: Uuid) -> Option<Completed> {
        match self.0.remove(&id) {
            Some((
                _,
                PendingSession {
                    redirect,
                    ses_type: login_type,
                    status: Status::Completed(ses_id),
                    ..
                },
            )) => Some(Completed {
                ses_id,
                redirect,
                ses_type: login_type,
            }),
            _ => None,
        }
    }
}

pub enum CompletionResult {
    Delayed,
    Immediate(Completed),
}

pub struct PendingSessionGuard<'a> {
    id: Uuid,
    session: PendingSession,
    store: &'a PendingSessionStore,
}
impl<'a> PendingSessionGuard<'a> {
    pub fn complete(self, ses_id: SesId) -> CompletionResult {
        let mut ses = self.session;
        match ses.ses_type {
            PendingSessionType::Delayed => {
                ses.status = Status::Completed(ses_id);
                self.store.0.insert(self.id, ses);
                CompletionResult::Delayed
            }
            PendingSessionType::Immediate => CompletionResult::Immediate(Completed {
                ses_id,
                redirect: ses.redirect,
                ses_type: ses.ses_type,
            }),
        }
    }
    pub fn abort(self) {
        drop(self)
    }
}

fn validate_redirect_url(url: &str) -> bool {
    if !url.starts_with("/") {
        error!("Invalid redirect url (no starting slash): {}", url);
        return false;
    }
    if url.contains("://") || url.contains("javascript:") {
        error!("Invalid redirect url (contains :// | javascript:): {}", url);
        return false;
    }
    for char in url.chars() {
        if !(char.is_alphanumeric() || ['/', '=', '_', '-', '?', '&'].contains(&char)) {
            error!("Invalid redirect url: '{}' invalid char: {}", url, char);
            return false;
        }
    }
    true
}

fn launch_cleaner(store: Arc<DashMap<uuid::Uuid, PendingSession>>) {
    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let now = Instant::now();
            store.retain(|_, entry| now.duration_since(entry.created_at).as_secs() < 60 * 10);
        }
    });
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Pending Session Store", |r| async {
        let store = Arc::new(DashMap::new());
        launch_cleaner(store.clone());
        r.manage(PendingSessionStore(store))
    })
}
