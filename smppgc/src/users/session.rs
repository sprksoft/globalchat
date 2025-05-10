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

use super::{role::Role, SesId, SmId, UserInfo};
use crate::{db::models::User, oauth::SmUserInfo};

#[derive(Clone)]
pub struct Session {
    created_time: SystemTime,
    ses_id: SesId,
    pub user_info: Arc<UserInfo>,
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
        self.sessions.retain(|_, ses| !ses.expired(now));
    }

    pub async fn create_session(&self, user: User) -> SesId {
        self.clean_sessions().await;
        let now = SystemTime::now();
        let ses_id = SesId::new();

        let user_info = UserInfo {
            id: super::UserId(user.id),
            irl_name: user.irl_name.into(),
            role: Role::try_from(user.role).unwrap_or(Role::User),
            ban_end_timestamp: SystemTime::UNIX_EPOCH
                + Duration::from_secs(user.ban_end_timestamp as u64),
        };

        self.sessions.insert(
            ses_id.clone(),
            Session {
                created_time: now,
                ses_id: ses_id.clone(),
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
