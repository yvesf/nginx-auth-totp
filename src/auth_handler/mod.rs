#![allow(warnings)]

use std::cell::Cell;
use std::collections::HashMap;
use std::io;
use std::marker::Sync;
use std::str;
use std::str::FromStr;
use std::sync::{Arc, RwLock, Mutex, MutexGuard};
use std::cell::RefCell;

use time::{Tm, Duration};
use http::{Request, Response, StatusCode, Method};
use tokio::prelude::*;
use horrorshow;
use cookie::{Cookie,CookieBuilder};
use bytes::Bytes;

use router;
use cookie_store::CookieStore;
use cookie_store::to_cookie;
use http_server::HttpHandler;

mod urls;
mod handler_login;
mod handler_info;

pub(in auth_handler) struct HeaderExtract<'a> {
    totp_secrets: Vec<&'a str>,
    cookies: Vec<Cookie<'a>>,
}

pub struct HeaderMissing {
    name: &'static str,
}

static HTTP_HEADER_AUTHORIZATION: &'static str = r"Authorization";
static HTTP_HEADER_X_ORIGINAL_URL: &'static str = r"X-Original-Url";
static HTTP_HEADER_WWW_AUTHENTICATE: &'static str = r"WWW-Authenticate";
static HTTP_HEADER_X_TOTP_SECRET: &'static str = r"X-Totp-Secret";
static COOKIE_NAME: &'static str = r"totp_cookie";

#[derive(Clone)]
pub struct AuthHandler {
    routing_table: router::RoutingTable<urls::Route>,
}

pub(in auth_handler) fn make_response(code: StatusCode, body: String) -> Response<String> {
    Response::builder().status(code).body(body).unwrap()
}

pub(in auth_handler) fn error_handler_internal(body: String) -> Response<String> {
    Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(body).unwrap()
}

impl HttpHandler<super::ApplicationState> for AuthHandler {
    fn respond(&self, state: &super::ApplicationState, req: Request<Bytes>) -> Response<String> {
        match self.routing_table.match_path(req.uri().path()) {
            Ok((urls::Route::Info, rest)) => handler_info::respond(self, state, &req, rest),
            Ok((urls::Route::Login, rest)) => self.login(state, &req, rest),
            Ok((urls::Route::Logout, rest)) => self.logout(state, &req, rest),
            Ok((urls::Route::Check, rest)) => self.check(state, &req, rest),
            Err(error) => match error {
                router::NoMatchingRoute =>
                    make_response(StatusCode::NOT_FOUND, "Resource not found".to_string()),
            }
        }
    }
}

pub fn is_logged_in(cookies: &Vec<Cookie>, cookie_store: &CookieStore) -> bool {
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


impl AuthHandler {
    pub fn make() -> AuthHandler {
        AuthHandler { routing_table: urls::create_routing_table() }
    }

    fn login<'a>(&self, state: &super::ApplicationState, req: &Request<Bytes>, path_rest: &'a str,
    ) -> Response<String> {
        let header_infos = match Self::parse_header_infos(req) {
            Ok(infos) => infos,
            Err(message) => return error_handler_internal(message),
        };
        match *req.method() {
            Method::GET => handler_login::GET(&header_infos, state, path_rest),
            Method::POST => handler_login::POST(&header_infos, state, req),
            _ => error_handler_internal("Wrong method".to_string()),
        }
    }

    fn logout<'a>(&self, state: &super::ApplicationState, req: &Request<Bytes>, path_rest: &'a str,
    ) -> Response<String> {
        let header_infos = match Self::parse_header_infos(req) {
            Ok(infos) => infos,
            Err(message) => return error_handler_internal(message),
        };

        let body = format!("Rest: {}", path_rest);
        Response::builder().body(body.to_string()).unwrap()
    }


    fn check<'a>(&self, state: &super::ApplicationState, req: &Request<Bytes>, path_rest: &'a str) -> Response<String> {
        let header_infos = match Self::parse_header_infos(req) {
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
            let cookie = Cookie::parse(value).or(Err("Failed to parse cookie value"))?;
            cookies.push(cookie);
        }

        Ok(HeaderExtract { totp_secrets, cookies })
    }
}

#[cfg(test)]
mod test1 {
    //    use super::*;
    use test::Bencher;
    //    use horrorshow::prelude::*;
    use horrorshow::helper::doctype;

    #[bench]
    fn bench_1(_: &mut Bencher) {
        let _ = format!("{}", html! {
: doctype::HTML;
html {
head {
title: "Hello world!";
}
body {
// attributes
h1(id = "heading") {
// Insert escaped text
: "Hello! This is <html />"
}
}
}
});
    }
}
