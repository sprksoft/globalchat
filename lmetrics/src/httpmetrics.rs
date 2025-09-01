use rocket::{fairing::AdHoc, http::StatusClass};

use crate::metrics;

metrics! {
pub counter http_errors_total("Amount of total http errors",
    [status_code]);
pub counter http_req_total("Amount of total http requests");
}

pub fn http_errors_metrics() -> AdHoc {
    AdHoc::on_response("response metrics", |_, res| {
        Box::pin(async move {
            let class = res.status().class();
            http_req_total::inc();
            if class == StatusClass::ClientError || class == StatusClass::ServerError {
                http_errors_total::inc(&res.status().code.to_string());
            }
        })
    })
}
