use horrorshow::Template;

pub(in super) fn info_debug<'a>(path_rest: &'a str, cookies: Vec<(String, String)>) -> String {
    (html! {
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
            }
        }
    }).into_string().unwrap()
}

pub(in super) fn info<'a>(path_rest: &'a str) -> String {
    (html! {
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
    }).into_string().unwrap()
}

pub(in super) fn login_is_logged_in() -> String {
    (html! {
        : horrorshow::helper::doctype::HTML;
        html {
            head {
                title: "TOTP Login";
            }
            body {
                h1(id = "heading") {
                    : "Currently logged in"
                }
            }
        }
    }).into_string().unwrap()
}

pub(in super) fn login_login_form<'a>(redirect: &'a str) -> String {
    (html! {
        : horrorshow::helper::doctype::HTML;
        html {
            head {
                title: "TOTP Login";
            }
            body {
                h1(id = "heading") {
                    : "Login"
                }
                form(method="POST") {
                    label(for="token") {
                        : "Enter TOTP token"
                    }
                    input(name="token",id="token",type="text");
                    input(name="redirect", type="hidden", value=redirect);
                    input(name="send",type="submit",value="Submit");
                }
            }
        }
    }).into_string().unwrap()
}

pub(in super) fn login_auth_success(redirect: &String) -> String {
    (html! {
        : horrorshow::helper::doctype::HTML;
        html {
            head {
                title: "TOTP Successful";
                meta(http-equiv="refresh", content=format!("3; URL={}", redirect))
            }
            body {
                h1(id = "heading") {
                    : "Login succesful"
                }
                a(href=redirect) {
                    : "Try again... redirecting to ";
                }
                span {
                    : format!("{}", redirect)
                }
            }
        }
    }).into_string().unwrap()
}


pub(in super) fn login_auth_fail() -> String {
    (html! {
    : horrorshow::helper::doctype::HTML;
        html {
            head {
                title: "TOTP Login failed";
                meta(http-equiv="refresh", content="1")
            }
            body {
                h1(id = "heading") {
                    : "Login failed"
                }
                a(href="login") {
                    : "Try again... "
                }
            }
        }
    }).into_string().unwrap()
}