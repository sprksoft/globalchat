use log::*;
use rocket::fairing::AdHoc;
use rocket::response;
use rocket_db_pools::Database;

#[derive(Database)]
#[database("sqlx")]
pub struct Db(sqlx::PgPool);

pub type DbResult<T, E = response::Debug<sqlx::Error>> = std::result::Result<T, E>;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("db", |r| async {
        r.attach(Db::init())
            .attach(AdHoc::try_on_ignite("db migrations", |r| async {
                match Db::fetch(&r) {
                    Some(db) => match sqlx::migrate!("./migrations").run(&**db).await {
                        Ok(_) => Ok(r),
                        Err(e) => {
                            error!("Failed to migrate database: {}", e);
                            Err(r)
                        }
                    },
                    None => {
                        error!("Can't migrate db. No db found");
                        Err(r)
                    }
                }
            }))
    })
}
