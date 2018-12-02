#![feature(test,integer_atomics,duration_as_u128)]
use std::sync::Arc;
use std::thread;
use std::sync::atomic;
use std::net::SocketAddr;

#[macro_use]
extern crate log;
extern crate tokio;
extern crate tokio_threadpool;
extern crate tokio_executor;
extern crate tokio_signal;
extern crate futures;
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
extern crate structopt;

use structopt::StructOpt;
use log::LogLevel::{Debug, Warn};
use time::Duration;
use futures::{Future, Stream};
use tokio_threadpool::Builder;
use tokio_executor::enter;

mod request_handler;
mod cookie_store;
mod http_server;
mod router;
mod system;
mod totp;

use cookie_store::CookieStore;

#[derive(Clone)]
pub struct ApplicationState {
    cookie_store: CookieStore,
    cookie_max_age: Duration,
    debug: bool,
    request_slowdown: Arc<atomic::AtomicU64>,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "nginx-auth-totp")]
struct Opt {
    #[structopt(short = "l", long = "port", default_value = "127.0.0.1:8080")]
    addr: SocketAddr,
    #[structopt(short = "d", long = "debug")]
    debug: bool,
}

fn main() {
    let opt = Opt::from_args();
    simple_logger::init_with_level(if opt.debug { Debug } else { Warn })
        .unwrap_or_else(|_| panic!("Failed to initialize logger"));
    debug!("If you read this message then we're running debug (-d) mode.");
    debug!("Debug mode is not safe for public accesible instances");

    let state = ApplicationState {
        cookie_store: CookieStore::new(),
        cookie_max_age: Duration::days(1),
        debug: opt.debug,
        request_slowdown: Arc::new(atomic::AtomicU64::new(0)),
    };

    let server_shutdown_condvar = Arc::new(atomic::AtomicBool::new(false));

    let cookie_clean_thread = {
        let server_shutdown_condvar = server_shutdown_condvar.clone();
        let state = state.clone();
        thread::spawn(move || {
            thread::park_timeout(std::time::Duration::from_secs(10));
            while !server_shutdown_condvar.load(atomic::Ordering::Relaxed) {
                info!("Clean cookie cache");
                state.cookie_store.clean_outdated_cookies();
                thread::park_timeout(std::time::Duration::from_secs(60));
            }
        })
    };

    let request_handler = request_handler::RequestHandler::make();
    let runtime = Builder::new()
        .name_prefix("httpd-")
        .after_start(|| {
            debug!("Start new worker: {}", thread::current().name().unwrap_or("-"));
            system::initialize_rng_from_time();
        })
        .build();

    let program = http_server::serve(opt.addr, state, request_handler);
    runtime.spawn(program);

    let ctrl_c_block = tokio_signal::ctrl_c()
        .flatten_stream().take(1).for_each(|()| {
        info!("ctrl-c received");
        Ok(())
    });

    enter().expect("nested tokio::run")
        .block_on(ctrl_c_block)
        .unwrap();
    runtime.shutdown();

    info!("Waiting for cookie cleanup thread to stop");
    server_shutdown_condvar.store(true, atomic::Ordering::Relaxed);
    cookie_clean_thread.thread().unpark();
    cookie_clean_thread.join().unwrap();
}
