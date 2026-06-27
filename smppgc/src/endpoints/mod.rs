use rocket::{fairing::AdHoc, routes};

pub mod gcagent;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("endpoints", |r| async {
        r.mount(
            "/",
            routes![gcagent::chat_socket, gcagent::readonly_chat_socket],
        )
    })
}
