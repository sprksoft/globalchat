use profanity::{Flags, RuleFlags, Token};
use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar},
    response::{Debug, Redirect},
    routes,
    serde::Serialize,
    time::{Duration, OffsetDateTime},
    Responder, State,
};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use sqlx::query;

use crate::{
    db::{Db, DbResult},
    themes::Theme,
    users::{role::Role, AdminSession, Session},
};

#[derive(Responder)]
enum PromoteResponse {
    Redirect(Redirect),
    Error(Template),
}

#[get("/promote?<key>")]
pub async fn promote(
    key: &str,
    theme: Theme<'_>,
    session: Session,
    mut db: Connection<Db>,
) -> DbResult<PromoteResponse> {
    let user_id = session.user_info.id;
    let role = query!(
        "UPDATE promote_keys SET used_by=$2 WHERE key=$1 AND used_by IS NULL RETURNING new_role",
        key,
        user_id.to_i32(),
    )
    .fetch_optional(&mut **db)
    .await?
    .map(|r| r.new_role);

    Ok(match role {
        Some(key) => {
            query!(
                "UPDATE users SET role=$1 WHERE id=$2",
                role,
                user_id.to_i32()
            )
            .execute(&mut **db)
            .await?;
            PromoteResponse::Redirect(Redirect::to("/"))
        }
        None => PromoteResponse::Error(Template::render(
            "pages/error_page",
            context! {
                theme_css: theme.css(),
                title: "Key doesn't exist",
                error: "The key doesn't exist or has already been used. (Ask on discord for a new key)",
                internal: ""
            },
        )),
    })
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct User {
    name: String,
    role: &'static str,
}

fn shorten_name(name: &str) -> String {
    let mut iter = name.split_whitespace();
    let mut new_name = String::new();
    if let Some(first_name) = iter.next() {
        new_name.push_str(first_name);
    }
    new_name.push(' ');
    let mut first = true;
    for part in iter {
        let Some(part) = part.chars().next() else {
            continue;
        };
        if !first {
            new_name.push('.');
        }
        new_name.push(part);
        first = false;
    }
    new_name
}

#[get("/mods")]
pub async fn mods(
    theme: Theme<'_>,
    _adminses: AdminSession,
    mut db: Connection<Db>,
) -> DbResult<Template> {
    let users: Vec<User> = query!("SELECT irl_name,role FROM users WHERE role > 0")
        .fetch_all(&mut **db)
        .await?
        .iter()
        .map(|u| User {
            role: Role::from_i32(u.role).unwrap_or(Role::User).to_str(),
            name: shorten_name(&u.irl_name),
        })
        .collect();
    Ok(Template::render(
        "pages/mods",
        context! {theme_css:theme.css(), users:users},
    ))
}
