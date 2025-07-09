use std::sync::atomic::AtomicU16;

mod csp;
//mod ipcountry;
pub mod static_routing;

pub use csp::*;
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
