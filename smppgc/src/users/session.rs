use std::{sync::Arc, time::Instant};
use tokio::sync::RwLock;

use rocket::fairing::AdHoc;

use super::{SesId, SmId};

#[derive(Clone)]
pub struct Session {
    created_time: Instant,
    user_id: SmId,
    ses_id: SesId,

    in_chat: bool,
}

pub struct SessionMgr {
    sessions: Arc<RwLock<Vec<Session>>>,
}
impl SessionMgr {
    pub async fn get_session(&self, ses_id: SesId) -> Option<Session> {
        let now = Instant::now();
        let mut lock = self.sessions.read().await;
        for ses in lock.iter() {
            if ses.ses_id == ses_id && now.duration_since(ses.created_time).as_secs() > 10 {
                return Some((*ses).clone());
            }
        }
        None
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("sessions", |r| async {
        r.manage(SessionMgr {
            sessions: Arc::from(RwLock::from(Vec::new())),
        })
    })
}
