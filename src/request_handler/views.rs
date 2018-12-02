use std::boxed::Box;
use horrorshow::{Render, RenderBox, Template};


fn render_base_template(title: &'static str, page_body: Box<RenderBox>) -> String {
    (html! {
        : horrorshow::helper::doctype::HTML;
        html {
            head {
                title: title;
                meta(name="viewport", content="width=device-width, initial-scale=1.5");
            }
            body {
                : page_body;
            }
        }
    }).into_string().unwrap()
}

pub(in super) fn info_debug<'a>(path_rest: &'a str, cookies: Vec<(String, String)>, wait_until: u64) -> String {
    let path = path_rest.to_string();
    render_base_template("Info (debug)", box_html! {
        h1(id = "heading") {
            : "Hello! This is <html />";
            : "And path rest is: ";
            : path;
            : "... ok :)";
        }
        h2: "Valid cookies are:";
        table(border="1") {
            thead {
                th: "Cookie value";
                th: "Valid until";
            }
            tbody {
                @ for (name, valid_until) in cookies {
                    tr {
                        td: name;
                        td: valid_until;
                    }
                }
            }
        }
        h2: "Request Slowdown";
        p {
            : "Until: ";
            : wait_until;
        }
    })
}

pub(in super) fn info<'a>(path_rest: &'a str) -> String {
    let path = path_rest.to_string();
    render_base_template("Info", box_html! {
        h1(id = "heading") {
            : "Hello! This is <html />";
            : "And path rest is: ";
            : path;
            : "... ok :)";
        }
    })
}

pub(in super) fn login_is_logged_in() -> String {
    render_base_template("Logged in", box_html! {
        h1(id = "heading") {
            : "Currently logged in"
        }
        a(href="logout") {
            : "Go to logout";
        }
    })
}

pub(in super) fn login_login_form<'a>(redirect: &'a str) -> String {
    let redirect = redirect.to_string();
    render_base_template("TOTP Login", box_html! {
        h1(id = "heading") {
            : "Login"
        }
        form(method="POST") {
            div {
                label(for="token") {
                        : "Enter TOTP token"
                }
            }
            div {
                input(name="token",id="token",type="number",autocomplete="off",required="");
                input(name="redirect", type="hidden", value=redirect);
            }
            div {
                input(name="send",type="submit",value="Submit");
            }
        }
    })
}

pub(in super) fn login_auth_success(redirect: &String) -> String {
    let redirect = redirect.clone();
    render_base_template("Login successful", box_html! {
        h1(id = "heading") {
            : "Login succesful"
        }
        a(href=&redirect) {
            : "redirecting to ";
        }
        span {
            : format!("{}", redirect)
        }
    })
}

pub(in super) fn login_auth_fail() -> String {
    render_base_template("Login failed", box_html! {
        h1(id = "heading") {
            : "Login failed"
        }
        a(href="login") {
            : "Try again... "
        }
    })
}

pub(in super) fn logout() -> String {
    render_base_template("Logout", box_html! {
        h1(id = "heading") {
            : "Logout applied"
        }
        a(href="login") {
            : "go to login again..."
        }
    })
}