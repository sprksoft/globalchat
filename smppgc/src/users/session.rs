use dashmap::{DashMap, Map};
use std::{
    convert::Infallible,
    ops::Deref,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::RwLock;

use rocket::{
    async_trait,
    fairing::AdHoc,
    http::Status,
    outcome::{try_outcome, IntoOutcome},
    request::{FromRequest, Outcome},
    State,
};

use super::{role::Role, SesId, SmId, UserInfo2};
use crate::{db::models::User, oauth::SmUserInfo};

pub struct SessionLock<'a> {
    mgr: &'a SessionMgr,
    ses: Session,
    id: SesId,
}
impl<'a> SessionLock<'a> {
    pub fn ses(&self) -> &Session {
        &self.ses
    }
}
impl<'a> Drop for SessionLock<'a> {
    fn drop(&mut self) {
        let Some(mut s) = self.mgr.sessions.get_mut(&self.id) else {
            return;
        };
        s.chat_locked = false;
    }
}
impl<'a> Deref for SessionLock<'a> {
    type Target = Session;
    fn deref(&self) -> &Self::Target {
        &self.ses
    }
}

#[derive(Clone)]
pub struct Session {
    created_time: SystemTime,
    ses_id: SesId,
    pub user_info: Arc<UserInfo2>,
    chat_locked: bool,
}
impl Session {
    pub fn expired(&self, now: SystemTime) -> bool {
        now.duration_since(self.created_time)
            .map(|d| d.as_secs() > 604800)
            .unwrap_or(true)
    }
}

pub struct SessionMgr {
    sessions: Arc<DashMap<SesId, Session>>,
}
impl SessionMgr {
    pub async fn get_session(&self, ses_id: SesId) -> Option<Session> {
        let now = SystemTime::now();
        self.sessions
            .get(&ses_id)
            .filter(|s| !s.expired(now))
            .map(|s| s.clone())
    }

    async fn clean_sessions(&self) {
        let now = SystemTime::now();
        self.sessions
            .retain(|_, ses| ses.chat_locked || !ses.expired(now));
    }

    pub fn chat_lock_session<'a>(&'a self, ses_id: SesId) -> Option<SessionLock<'a>> {
        let mut ses = self.sessions.get_mut(&ses_id)?;
        if ses.chat_locked {
            return None;
        }
        ses.chat_locked = true;
        Some(SessionLock {
            ses: ses.clone(),
            mgr: self,
            id: ses_id,
        })
    }

    pub async fn create_session(&self, user: User) -> SesId {
        self.clean_sessions().await;
        let now = SystemTime::now();
        let ses_id = SesId::new();

        let user_info = UserInfo2 {
            smid: SmId::from_string(user.smid),
            irl_name: user.irl_name.into(),
            role: Role::try_from(user.role).unwrap_or(Role::User),
            ban_end_timestamp: user
                .ban_release_timestamp
                .map(|ts| SystemTime::UNIX_EPOCH + Duration::from_secs(ts as u64))
                .unwrap_or(now),
        };

        self.sessions.insert(
            ses_id.clone(),
            Session {
                created_time: now,
                ses_id: ses_id.clone(),
                chat_locked: false,
                user_info: Arc::from(user_info),
            },
        );
        ses_id
    }
}
#[async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = &'static str;
    async fn from_request(req: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let sessionmgr: &SessionMgr = try_outcome!(req
            .rocket()
            .state()
            .or_error((Status::InternalServerError, "No session manager found")));
        let ses_id = try_outcome!(req
            .guard::<SesId>()
            .await
            .map_error(|_| (Status::InternalServerError, "")));

        sessionmgr
            .get_session(ses_id)
            .await
            .or_forward(Status::Unauthorized)
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("sessions", |r| async {
        r.manage(SessionMgr {
            sessions: Arc::from(DashMap::new()),
        })
    })
}
