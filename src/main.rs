#![feature(test)]
#![feature(convert_id)]
#![feature(proc_macro_hygiene)]
#![feature(try_from)]
#![feature(duration_as_u128)]
#![feature(libc)]

use std::env;
use std::sync::Arc;
use std::thread;
use std::sync::atomic;
use std::net::SocketAddr;

extern crate ascii;
extern crate getopts;
#[macro_use]
extern crate log;
extern crate tokio;
extern crate tokio_threadpool;
extern crate tokio_executor;
extern crate time;
extern crate simple_logger;
extern crate oath;
extern crate evmap;
extern crate test;
#[macro_use]
extern crate horrorshow;
extern crate random;
extern crate http;
extern crate httparse;
extern crate bytes;
extern crate thread_local;
extern crate cookie;
extern crate url;

use getopts::Options;
use log::LogLevel::{Debug, Warn};
use time::Duration;

mod auth_handler;
mod cookie_store;
mod http_server;
mod router;
mod system;
mod totp;

extern crate libc;

use cookie_store::CookieStore;

#[derive(Clone)]
pub struct ApplicationState {
    cookie_store: CookieStore,
    cookie_max_age: Duration,
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optopt("l", "port", "Listen address", "LISTEN-ADDR");
    opts.optflag("d", "debug", "Use loglevel Debug instead of Warn");
    opts.optflag("h", "help", "print this help menu");
    let matches = opts.parse(&args[1..]).unwrap_or_else(|f| panic!(f.to_string()));

    if matches.opt_present("h") {
        print_usage(&program, &opts);
        return;
    }

    simple_logger::init_with_level(if matches.opt_present("d") { Debug } else { Warn })
        .unwrap_or_else(|_| panic!("Failed to initialize logger"));


    let addr = matches.opt_str("l").unwrap_or_else(||"127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>()
        .unwrap_or_else(|_| panic!("Failed to parse LISTEN-ADDRESS"));


    // concurrent eventual consistent hashmap with <cookie-id, timeout>
    let state = ApplicationState { cookie_store: CookieStore::new(), cookie_max_age: Duration::days(1) };

    let server_shutdown_condvar = Arc::new(atomic::AtomicBool::new(false));

    let cookie_clean_thread_condvar = server_shutdown_condvar.clone();
    let cookie_clean_state = state.clone();
    let cookie_clean_thread = thread::spawn(move || {
        while !cookie_clean_thread_condvar.load(atomic::Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_secs(60));
            debug!("Clean cookie cache");
            cookie_clean_state.cookie_store.clean_outdated_cookies();
        }
    });

    let auth_handler = auth_handler::AuthHandler::make();
    http_server::serve(addr, state, auth_handler);

    server_shutdown_condvar.store(true, atomic::Ordering::Relaxed);
    debug!("Waiting for cleanup thread to shutdown");
    cookie_clean_thread.join().unwrap();
}
