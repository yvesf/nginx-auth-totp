use std::ops::Add;
use std::io;
use std::borrow::Cow;
use std::time;
use std::thread;
use std::sync::atomic;

use tokio::prelude::*;

use http::{Request, Response, StatusCode, Method};
use http::header::{SET_COOKIE, COOKIE};
use url::form_urlencoded;

use ::ApplicationState;
use ::totp;
use super::*;

pub(in super) fn GET<'a>(header_infos: &HeaderExtract, state: &ApplicationState, path_rest: &'a str)
                         -> Response<String> {
    if is_logged_in(&header_infos.cookies, &state.cookie_store) {
        Response::builder().set_defaults().body(views::login_is_logged_in()).unwrap()
    } else {
        Response::builder().set_defaults().body(views::login_login_form(path_rest)).unwrap()
    }
}

fn test_secrets(secrets: &Vec<&str>, token: &String) -> bool {
    secrets.iter()
        .any(|secret| {
            match totp::verify(secret, token) {
                Ok(true) => true,
                Ok(false) => false,
                Err(e) => {
                    error!("Error from totp::verify: {}", e);
                    false
                }
            }
        })
}

pub(in super) fn POST<'a>(header_infos: &HeaderExtract, state: &ApplicationState, req: &Request<Bytes>)
                          -> Response<String> {
let wait_until = state.request_slowdown.load(atomic::Ordering::Acquire);
    let now = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs();

    let slept = if wait_until > now {
        let time = wait_until - now;
        warn!("Sleep {}s", time);
        thread::sleep(time::Duration::from_secs(time));
        time
    } else {
        0
    };

    let mut token = None;
    let mut redirect = None;
    for (key, val) in form_urlencoded::parse(req.body()) {
        if key == "token" {
            token = Some(val.into_owned())
        } else if key == "redirect" {
            redirect = Some(val.into_owned())
        }
    }
    if token.is_none() {
        return error_handler_internal("missing argument 'token'".to_string());
    }
    let redirect = redirect.unwrap_or(Default::default());

    if header_infos.totp_secrets.is_empty() {
        return error_handler_internal("no secrets configured".to_string());
    }

    if test_secrets(&header_infos.totp_secrets, &token.unwrap()) {
        let cookie_value = state.cookie_store.create_authenticated_cookie();
        let cookie = CookieBuilder::new(COOKIE_NAME, cookie_value.to_string())
            .http_only(true)
            .path("/")
            .max_age(state.cookie_max_age)
            .finish();
        warn!("Authenticated user with cookie {}", cookie);
        Response::builder()
            .set_defaults()
            .header(SET_COOKIE, cookie.to_string())
            .body(views::login_auth_success(&redirect)).unwrap()
    } else {
        let current_wait = state.request_slowdown.load(atomic::Ordering::Acquire);
        let wait_until = time::SystemTime::now()
            .add(time::Duration::from_secs(8))
            .duration_since(time::UNIX_EPOCH).unwrap()
            .as_secs();
        // if this request was already delayed then we double-delay
        let wait_until = wait_until.max(current_wait + 8 + slept);
        state.request_slowdown.store(wait_until, atomic::Ordering::Release);

        Response::builder()
            .set_defaults()
            .body(views::login_auth_fail()).unwrap()
    }
}