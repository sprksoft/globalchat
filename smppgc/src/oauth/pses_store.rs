use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use log::*;
use rocket::fairing::AdHoc;
use uuid::Uuid;

use crate::users::SesId;

enum Status {
    Pending,
    Completed(SesId),
}
#[derive(Clone, Copy)]
pub enum LoginType {
    External,
    Internal,
}
impl LoginType {
    pub fn is_internal(self) -> bool {
        match self {
            Self::External => false,
            Self::Internal => true,
        }
    }
}
pub struct Completed {
    pub ses_id: SesId,
    pub redirect: String,
    pub login_type: LoginType,
}

struct PendingSession {
    created_at: Instant,
    redirect: String,
    login_type: LoginType,
    status: Status,
}

pub struct PendingSessionStore(Arc<DashMap<uuid::Uuid, PendingSession>>);
impl PendingSessionStore {
    pub fn new_pending(&self, mut redirect: String, login_type: LoginType) -> Uuid {
        if !validate_redirect_url(&redirect) {
            error!("Setting redirect url to /v1 because it was invalid");
            redirect = "/v1".to_string();
        }
        let id = Uuid::new_v4();
        self.0.insert(
            id.clone(),
            PendingSession {
                created_at: Instant::now(),
                status: Status::Pending,
                redirect,
                login_type,
            },
        );
        id
    }

    pub fn complete(&self, id: Uuid, ses_id: SesId) -> Option<LoginType> {
        match self.0.get_mut(&id) {
            Some(mut e) => {
                e.status = Status::Completed(ses_id.clone());
                Some(e.login_type)
            }
            None => None,
        }
    }

    pub fn get_completed(&self, id: Uuid) -> Option<Completed> {
        match self.0.remove(&id) {
            Some((
                _,
                PendingSession {
                    redirect,
                    login_type,
                    status: Status::Completed(ses_id),
                    ..
                },
            )) => Some(Completed {
                ses_id,
                redirect,
                login_type,
            }),
            _ => None,
        }
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
