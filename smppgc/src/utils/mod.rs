use std::sync::atomic::AtomicU16;

mod csp;
mod in_iframe;
//mod ipcountry;
pub mod static_routing;

pub use csp::*;
pub use in_iframe::*;
use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome},
};
//pub use ipcountry::*;

pub struct IdCounter {
    id_counter: AtomicU16,
}
impl IdCounter {
    pub fn new() -> Self {
        Self {
            id_counter: 1.into(),
        }
    }
    pub fn new_id(&self) -> u16 {
        self.id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

// A request guard to catch forwards.
pub enum CatchForward<T> {
    Success(T),
    Forward(Status),
}
impl<T> CatchForward<T> {
    pub fn map<T2>(self, f: impl FnOnce(T) -> T2) -> CatchForward<T2> {
        match self {
            Self::Success(s) => CatchForward::Success(f(s)),
            Self::Forward(s) => CatchForward::Forward(s),
        }
    }
    pub fn unwrap_or(self, def: T) -> T {
        match self {
            Self::Success(s) => s,
            Self::Forward(_) => def,
        }
    }
    pub fn is_success(&self) -> bool {
        match self {
            Self::Success(_) => true,
            _ => false,
        }
    }
}

#[async_trait]
impl<'r, E: std::fmt::Debug, T: FromRequest<'r, Error = E>> FromRequest<'r> for CatchForward<T> {
    type Error = E;
    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        match T::from_request(request).await {
            Outcome::Success(s) => Outcome::Success(CatchForward::Success(s)),
            Outcome::Forward(s) => Outcome::Success(CatchForward::Forward(s)),
            Outcome::Error(e) => Outcome::Error(e),
        }
    }
}
