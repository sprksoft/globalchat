use rocket::{
    fairing::AdHoc,
    get,
    http::{
        uri::{Origin, Uri},
        Cookie,
    },
    response::Debug,
    response::{status::BadRequest, Redirect},
    routes,
    serde::Serialize,
    time::Duration,
    uri, Request, Responder,
};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use sqlx::query;

use crate::{
    db::{Db, DbResult},
    themes::Theme,
    users::{role::Role, AdminUser, SesId},
};

#[derive(Responder)]
enum PromoteResponse {
    Redirect(Redirect),
    Template(Template),
}

#[get("/promote?<key>")]
async fn promote(
    key: &str,
    theme: Theme<'_>,
    ses_id: SesId,
    mut db: Connection<Db>,
) -> DbResult<PromoteResponse> {
    let result = query!("SELECT claim_key($1,$2)", ses_id.inner(), key)
        .fetch_one(&mut **db)
        .await?;
    let status: String = result.claim_key.unwrap_or("invaliderror".to_string());

    Ok(if status == "ok" {
        PromoteResponse::Redirect(Redirect::to("/"))
    } else {
        PromoteResponse::Template(Template::render(
            "pages/promotekey",
            context! {
                theme_css: theme.css(),
                status:status,
            },
        ))
    })
}
#[allow(unused_variables)]
#[get("/promote?<key>", rank = 0)]
fn promote_nologin(key: &str, origin: &Origin<'_>) -> Redirect {
    Redirect::to(uri!(crate::pages::login(redirect = origin.to_string())))
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
struct TableUser {
    name: String,
    role: &'static str,
    id: i32,
    cantbedemoted: bool,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TableKey {
    used_by: String,
    key: String,
    new_role: &'static str,
}

#[get("/mods")]
async fn mods(
    theme: Theme<'_>,
    admin_user: AdminUser,
    mut db: Connection<Db>,
) -> DbResult<Template> {
    let users: Vec<TableUser> = query!("SELECT irl_name,role,id FROM users WHERE role > 0")
        .fetch_all(&mut **db)
        .await?
        .iter()
        .map(|u| {
            let role = Role::from_i32(u.role);
            TableUser {
                role: role.map(|r| r.to_str()).unwrap_or("unknown"),
                name: shorten_name(&u.irl_name),
                id: u.id,
                cantbedemoted: admin_user.0.role() <= role.unwrap_or(Role::User),
            }
        })
        .collect();

    let keys: Vec<TableKey> = query!("SELECT key,new_role,users.irl_name FROM promote_keys LEFT JOIN users ON users.id = promote_keys.used_by")
        .fetch_all(&mut **db)
        .await?
        .drain(..)
        .map(|k| TableKey {
            new_role: Role::from_i32(k.new_role)
                .map(|r| r.to_str())
                .unwrap_or("unknown"),
            used_by: k.irl_name.as_ref().map(|n|shorten_name(n))
                .unwrap_or("No one".to_string()),
            key: k.key,
        })
        .collect();
    Ok(Template::render(
        "pages/mods",
        context! {theme_css:theme.css(), users:users, keys:keys},
    ))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("promote", |r| async {
        r.mount("/", routes![promote, promote_nologin, mods])
    })
}
