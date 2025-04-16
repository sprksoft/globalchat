use crate::LMetrics;
use rocket::{
    http::Method,
    outcome::IntoOutcome,
    response::Responder,
    route::{Handler, Outcome},
    Request, Route,
};

pub use rocket_prometheus;

#[rocket::async_trait]
impl Handler for LMetrics {
    async fn handle<'r>(&self, req: &'r Request<'_>, _: rocket::Data<'r>) -> Outcome<'r> {
        self.respond_to(req).or_error(())
    }
}
impl From<LMetrics> for Vec<Route> {
    fn from(other: LMetrics) -> Self {
        vec![Route::new(Method::Get, "/", other)]
    }
}

impl<'r> Responder<'r, 'static> for &LMetrics {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        self.encode_metrics()
            .map_err(|e| rocket::response::Debug(e))
            .respond_to(req)
    }
}
