use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod constants;
pub mod traits;

pub fn get_timestamp() -> Duration {
    return SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
}
