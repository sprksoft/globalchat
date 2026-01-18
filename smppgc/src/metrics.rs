use lmetrics::LMetrics;
use log::*;
use rocket::{fairing::AdHoc, get, routes, State};
use rocket_db_pools::Connection;
use sqlx::query;

use crate::db::Db;

lmetrics::metrics! {
    gauge total_db_users("Total amount of users in the db");
    gauge total_active_bans("Total amount of active bans");
}

#[get("/metrics")]
async fn metrics(mut db: Connection<Db>, metrics: &State<LMetrics>) -> &LMetrics {
    let (user_count, ban_count) = match query!("SELECT (SELECT COUNT(*) FROM users) AS user_count, (SELECT COUNT(*) FROM bans WHERE expiration_time-EXTRACT(epoch from now()) > 0) AS ban_count")
        .fetch_one(&mut **db)
        .await
    {
        Err(e) => {
            error!("Failed to call db while serving /metrics: {}", e);
            (-1, -1)
        }
        Ok(c) => (c.user_count.unwrap_or(-1) as i64, c.ban_count.unwrap_or(-1) as i64),
    };

    total_db_users::set(user_count);
    total_active_bans::set(ban_count);
    &metrics
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("metrics", async |r| {
        let lmetrics = LMetrics::new(&[
            &total_db_users::METRIC,
            &total_active_bans::METRIC,
            &crate::oauth::total_started_oauth_flows::METRIC,
            &crate::oauth::total_failed_oauth_flows::METRIC,
            &crate::oauth::total_logins::METRIC,
            &crate::static_routing::static_req_total::METRIC,
            &crate::chat::joined_total::METRIC,
            &crate::chat::left_total::METRIC,
            &crate::chat::ro_joined_total::METRIC,
            &crate::chat::ro_left_total::METRIC,
            &crate::chat::history_events_lost_total::METRIC,
            &crate::chat::agent::messages_total::METRIC,
            &crate::chat::agent::messages_blocked::METRIC,
            &lmetrics::http_errors_total::METRIC,
            &lmetrics::http_req_total::METRIC,
        ]);

        r.manage(lmetrics)
            .mount("/", routes![metrics])
            .attach(lmetrics::http_errors_metrics())
    })
}
