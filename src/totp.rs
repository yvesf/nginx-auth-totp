use oath::totp_custom_time;
use oath::HashType;
use std::time::{UNIX_EPOCH, SystemTime};

pub fn verify(secret: &str, token: &str) -> Result<bool, &'static str> {
    let time_step = 30;
    let totp = |time| {
        totp_custom_time(secret, 6, 0, time_step, time, &HashType::SHA512)
            .map(|t| {
                debug!("Generated OTP for probing {} for key {}", t, secret);
                t
            })
            .map(|t| format!("{:06}", t) == *token)
    };
    let current_time: u64 = SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("Earlier than 1970-01-01 00:00:00 UTC").as_secs();
    if current_time % time_step <= 5 && totp(current_time - 30)? {
        return Ok(true);
    }

    if current_time % time_step >= 25 && totp(current_time + 30)? {
        return Ok(true);
    }

    if totp(current_time)? {
        return Ok(true);
    }

    Ok(false)
}