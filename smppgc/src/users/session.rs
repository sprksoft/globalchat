use dashmap::{DashMap, Map};
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::RwLock;

use rocket::fairing::AdHoc;

use super::{role::Role, SesId, SmId};
use crate::db::models::User;

#[derive(Clone)]
pub struct Session {
    created_time: SystemTime,
    smid: SmId,
    role: Role,
    ses_id: SesId,

    in_chat: bool,
    ban_end_timestamp: SystemTime,
}

pub struct SessionMgr {
    sessions: Arc<DashMap<SesId, Session>>,
}
impl SessionMgr {
    pub async fn get_session(&self, ses_id: SesId) -> Option<Session> {
        self.clean_sessions().await;
        self.sessions.get(&ses_id).map(|s| s.clone())
    }

    async fn clean_sessions(&self) {
        let now = SystemTime::now();
        self.sessions.retain(|_, ses| {
            ses.in_chat
                || now
                    .duration_since(ses.created_time)
                    .map(|d| d.as_secs() < 10)
                    .unwrap_or(false)
        })
    }

    pub async fn drop_session(&self, ses_id: SesId) {
        self.sessions.remove(&ses_id);
    }

    pub async fn create_session(&self, user: User) -> SesId {
        let now = SystemTime::now();
        let ses_id = SesId::new();
        self.sessions.insert(
            ses_id.clone(),
            Session {
                role: Role::try_from(user.role).unwrap_or(Role::User),
                created_time: now,
                in_chat: false,
                ban_end_timestamp: user
                    .ban_release_timestamp
                    .map(|ts| SystemTime::UNIX_EPOCH + Duration::from_secs(ts as u64))
                    .unwrap_or(now),
                ses_id: ses_id.clone(),
                smid: SmId::from_string(user.smid),
            },
        );
        ses_id
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("sessions", |r| async {
        r.manage(SessionMgr {
            sessions: Arc::from(DashMap::new()),
        })
    })
}
