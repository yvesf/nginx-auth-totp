use std::io;

use tokio::prelude::*;

use http::{Request, Response, StatusCode};
use bytes::Bytes;

use ::ApplicationState;
use super::AuthHandler;

pub(in super) fn respond<'a>(auth_handler: &AuthHandler, state: &ApplicationState, req: &Request<Bytes>,
                             path_rest: &'a str) -> Response<String> {
    let body = html! {
            : horrorshow::helper::doctype::HTML;
            html {
                head {
                    title: "Hello world!";
                }
                body {
                    h1(id = "heading") {
                        : "Hello! This is <html />";
                        : "And path rest is: ";
                        : path_rest;
                        : "... ok :)";
                    }
                }
            }
        };
    Response::builder().body(body.to_string()).unwrap()
}