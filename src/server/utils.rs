// utils.rs

use chrono::Utc;
use rand::Rng;

pub fn generate_id() -> String {
    let timestamp = Utc::now().timestamp_millis();
    let mut rng = rand::thread_rng();
    let random_part: u16 = rng.gen_range(0..10000);
    format!("{}-{:04}", timestamp, random_part)
}
