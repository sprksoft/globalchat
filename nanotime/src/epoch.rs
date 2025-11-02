use lazy_static::lazy_static;
use std::time::{Duration, SystemTime};

pub trait Epoch {
    fn sys_time() -> SystemTime;
}

lazy_static! {
    static ref GC_EPOCH: SystemTime =
        SystemTime::UNIX_EPOCH + Duration::from_secs((2024 - 1970) * 31557600);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GCEpoch;
impl Epoch for GCEpoch {
    fn sys_time() -> SystemTime {
        *GC_EPOCH
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnixEpoch;
impl Epoch for UnixEpoch {
    fn sys_time() -> SystemTime {
        SystemTime::UNIX_EPOCH
    }
}
