use std::time;

pub fn initialize_rng_from_time() {
    let r = random::default();
    let now = time::SystemTime::now();
    let nano_secs = now.duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    r.seed([(nano_secs >> 64) as u64, nano_secs as u64]);
}
