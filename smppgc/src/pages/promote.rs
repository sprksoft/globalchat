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

fn shorten_name(name: &str) -> String {
    let mut iter = name.split_whitespace();
    let mut name = iter.next().unwrap_or("").to_string();
    name.push(' ');

    let last_name: String = iter
        .flat_map(|part| part.chars().next().map(|p| [p, '.']))
        .flatten()
        .collect();
    name.push_str(&last_name);
    name
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct User {
    name: String,
    role: &'static str,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Key {
    used_by: String,
    key: String,
    new_role: &'static str,
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
            role: Role::from_i32(u.role)
                .map(|r| r.to_str())
                .unwrap_or("unknown"),
            name: shorten_name(&u.irl_name),
        })
        .collect();

    let keys: Vec<Key> = query!("SELECT key,new_role,users.irl_name FROM promote_keys LEFT JOIN users ON users.id = promote_keys.used_by")
        .fetch_all(&mut **db)
        .await?
        .drain(..)
        .map(|k| Key {
            new_role: Role::from_i32(k.new_role)
                .map(|r| r.to_str())
                .unwrap_or("unknown"),
            used_by: k.irl_name.map(|n|shorten_name(n))
                .unwrap_or("No one".to_string()),
            key: k.key,
        })
        .collect();
    Ok(Template::render(
        "pages/mods",
        context! {theme_css:theme.css(), users:users, keys:keys},
    ))
}
