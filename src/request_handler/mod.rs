#![allow(warnings)]

use std::cell::Cell;
use std::collections::HashMap;
use std::io;
use std::marker::Sync;
use std::str;
use std::str::FromStr;
use std::sync::{Arc, RwLock, Mutex, MutexGuard};
use std::cell::RefCell;


use time;
use http::{Request, Response, StatusCode, Method};
use tokio::prelude::*;
use horrorshow;
use cookie::{Cookie, CookieBuilder};
use bytes::Bytes;

use router;
use cookie_store::CookieStore;
use cookie_store::to_cookie;
use http_server::HttpHandler;

mod handler_login;
mod views;

#[derive(Clone, Copy)]
enum Route {
    Login,
    Logout,
    Info,
    Check,
}

fn create_routing_table() -> router::RoutingTable<Route> {
    let mut r = router::RoutingTable::new();
    r.insert("/info", Route::Info);
    r.insert("/login", Route::Login);
    r.insert("/logout", Route::Logout);
    r.insert("/check", Route::Check);
    r
}

struct HeaderExtract<'a> {
    totp_secrets: Vec<&'a str>,
    cookies: Vec<Cookie<'a>>,
}

static HTTP_HEADER_X_TOTP_SECRET: &'static str = r"X-Totp-Secret";
static COOKIE_NAME: &'static str = r"totp_cookie";

#[derive(Clone)]
pub struct RequestHandler {
    routing_table: router::RoutingTable<Route>,
}

pub(in request_handler) fn make_response(code: StatusCode, body: String) -> Response<String> {
    Response::builder().status(code).body(body).unwrap()
}

pub(in request_handler) fn error_handler_internal(body: String) -> Response<String> {
    Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(body).unwrap()
}

impl HttpHandler<super::ApplicationState> for RequestHandler {
    fn respond(&self, state: &super::ApplicationState, req: Request<Bytes>) -> Response<String> {
        match self.routing_table.match_path(req.uri().path()) {
            Ok((Route::Info, rest)) => info(self, state, &req, rest),
            Ok((Route::Login, rest)) => login(state, &req, rest),
            Ok((Route::Logout, rest)) => logout(state, &req, rest),
            Ok((Route::Check, rest)) => check(state, &req, rest),
            Err(error) => match error {
                router::NoMatchingRoute =>
                    make_response(StatusCode::NOT_FOUND, "Resource not found".to_string()),
            }
        }
    }
}

impl RequestHandler {
    pub fn make() -> RequestHandler {
        RequestHandler { routing_table: create_routing_table() }
    }
}

pub(in request_handler) fn is_logged_in(cookies: &Vec<Cookie>, cookie_store: &CookieStore) -> bool {
    for cookie in cookies {
        if cookie.name() == COOKIE_NAME {
            let cookie_value = to_cookie(cookie.value());
            if cookie_value.is_some() && cookie_store.is_cookie_authenticated(&cookie_value.unwrap()) {
                return true;
            }
        }
    }
    false
}

fn info<'a>(request_handler: &RequestHandler, state: &super::ApplicationState,
            req: &Request<Bytes>, path_rest: &'a str) -> Response<String> {
    let ftime = |ts| -> String {
        let ts = time::Timespec::new(ts, 0);
        let tm = time::at_utc(ts);
        time::strftime("%c", &tm).unwrap_or("</>".to_string())
    };
    let view = if state.debug {
        let valid_cookies: Vec<(String, String)> = state.cookie_store.reader
            .map_into(|k, v|
                (k.to_string(), ftime(v[0] as i64)));
        views::info_debug(path_rest, valid_cookies)
    } else {
        views::info(path_rest)
    };
    Response::builder().body(view).unwrap()
}

fn login<'a>(state: &super::ApplicationState, req: &Request<Bytes>, path_rest: &'a str,
) -> Response<String> {
    let header_infos = match parse_header_infos(req) {
        Ok(infos) => infos,
        Err(message) => return error_handler_internal(message),
    };
    match *req.method() {
        Method::GET => handler_login::GET(&header_infos, state, path_rest),
        Method::POST => handler_login::POST(&header_infos, state, req),
        _ => error_handler_internal("Wrong method".to_string()),
    }
}

fn logout<'a>(state: &super::ApplicationState, req: &Request<Bytes>, path_rest: &'a str,
) -> Response<String> {
    let header_infos = match parse_header_infos(req) {
        Ok(infos) => infos,
        Err(message) => return error_handler_internal(message),
    };

    let body = format!("Rest: {}", path_rest);
    Response::builder().body(body.to_string()).unwrap()
}


fn check<'a>(state: &super::ApplicationState, req: &Request<Bytes>, path_rest: &'a str) -> Response<String> {
    let header_infos = match parse_header_infos(req) {
        Ok(infos) => infos,
        Err(message) => return error_handler_internal(message),
    };
    if is_logged_in(&header_infos.cookies, &state.cookie_store) {
        make_response(StatusCode::OK, "".to_string())
    } else {
        make_response(StatusCode::UNAUTHORIZED, "Cookie expired".to_string())
    }
}


fn parse_header_infos(req: &Request<Bytes>) -> Result<HeaderExtract, String> {
    let mut totp_secrets = Vec::new();
    for header_value in req.headers().get_all(HTTP_HEADER_X_TOTP_SECRET) {
        let value = header_value.to_str().or(Err("Failed to read totp-secret header value"))?;
        totp_secrets.push(value);
    }

    let mut cookies = Vec::new();
    for header_value in req.headers().get_all(::http::header::COOKIE) {
        let value = header_value.to_str().or(Err("Failed to read cookie value"))?;
        for cookie_part in value.split("; ") {
            let cookie = Cookie::parse(cookie_part).or(Err("Failed to parse cookie value"))?;
            cookies.push(cookie);
        }
    }

    Ok(HeaderExtract { totp_secrets, cookies })
}
