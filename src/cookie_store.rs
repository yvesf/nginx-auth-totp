use evmap;
use std::sync::{Arc, Mutex, MutexGuard};
use evmap::{WriteHandle, ReadHandle};
use std::time;
use std::str;
use std::hash;

use random;
use random::Source;

// cookie is a 64-byte printable-characters-only array
pub struct CookieKey([u8; 64]);

impl PartialEq for CookieKey {
    fn eq(&self, other: &CookieKey) -> bool {
        self.0[..] == other.0[..]
    }
}

impl Eq for CookieKey {}

impl hash::Hash for CookieKey {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        for i in self.0.iter() {
            state.write_u8(*i)
        }
    }
}

impl Clone for CookieKey {
    fn clone(&self) -> CookieKey {
        CookieKey(self.0)
    }
}

impl ToString for CookieKey {
    fn to_string(&self) -> String {
        unsafe { str::from_utf8_unchecked(&self.0).to_string() }
    }
}

pub struct CookieStore {
    pub reader: ReadHandle<CookieKey, u64>,
    pub writer: Arc<Mutex<WriteHandle<CookieKey, u64>>>,
}

pub fn to_cookie(data: &str) -> Option<CookieKey> {
    if data.len() == 64 {
        let mut cookie_key: [u8; 64] = [0; 64];
        cookie_key.copy_from_slice(data.as_bytes());
        Some(CookieKey(cookie_key))
    } else {
        None
    }
}

static HEXTABLE: [u8; 16] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
    b'A', b'B', b'C', b'D', b'E', b'F'];


impl Clone for CookieStore {
    fn clone(&self) -> Self {
        CookieStore {
            reader: self.reader.clone(),
            writer: self.writer.clone(),
        }
    }
}

impl CookieStore {
    pub fn new() -> CookieStore {
        let (r, w) = evmap::new::<CookieKey, u64>();
        CookieStore {
            reader: r,
            writer: Arc::new(Mutex::new(w)),
        }
    }

    pub fn create_authenticated_cookie(&self) -> CookieKey {
        let mut r = random::default();
        let mut key = [0; 64];
        for it in key.iter_mut() {
            let random: u8 = r.read();
            let value = HEXTABLE[(random & 0x0f) as usize];
            *it = value;
        }

        let timeout = time::SystemTime::now() + time::Duration::from_secs(60 * 60 * 24); // 1 day
        let timeout = timeout.duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_secs();
        {
            let mut writer = self.write_handle();
            writer.insert(CookieKey(key), timeout);
            warn!("Insert: {}", CookieKey(key).to_string());
            writer.refresh();
        }
        CookieKey(key)
    }


    fn write_handle(&self) -> MutexGuard<WriteHandle<CookieKey, u64>> {
        self.writer.lock().unwrap()
    }

    fn now_unix_epoch() -> u64 {
        time::SystemTime::now()
            .duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_secs()
    }

    /// true -> cookie is valid until time
    /// false -> cookie is outdated
    pub fn is_cookie_authenticated(&self, key: &CookieKey) -> bool {
        let reader = &self.reader;
        let value = reader.get_and(key, |v| v[0]);

        warn!("Reading {} -> {:?}", key.to_string(), value);
        if value.is_none() {
            false
        } else if value.unwrap() < Self::now_unix_epoch() {
            // outdated, remove from map
            let mut writer = self.write_handle();
            writer.empty(key.clone());
            // but no refresh - it's not urgent
            false
        } else {
            true
        }
    }

    pub fn clean_outdated_cookies(&self) {
//        unimplemented!()
    }
}

