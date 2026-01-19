use std::{str::FromStr, sync::Arc};

use rocket::{fairing::AdHoc, get, routes, State};
use rocket_dyn_templates::{context, Template};

use crate::{
    themes::Theme,
    users::AdminUser,
    wf::{Filter, WFTag},
};

#[get("/words?<min_count>&<max_len>&<tags>")]
fn word_stats(
    _ses: AdminUser,
    filter: &State<Arc<Filter>>,
    min_count: Option<usize>,
    max_len: Option<usize>,
    tags: Option<&str>,
    theme: Theme,
) -> Template {
    let max_len = max_len.unwrap_or(30);

    let tags: &[WFTag] = match tags {
        None => &[WFTag::Unknown, WFTag::Bad],
        Some(t) => &t
            .split(',')
            .flat_map(|t| WFTag::from_str(t).ok())
            .collect::<Vec<WFTag>>(),
    };

    let mut stats = filter.calc_stats(min_count.unwrap_or(2), tags);
    stats.truncate(max_len);

    Template::render(
        "pages/word_stats",
        context! { stats: stats, theme_css: theme.css() },
    )
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("stats", async |r| r.mount("/stats", routes![word_stats]))
}
