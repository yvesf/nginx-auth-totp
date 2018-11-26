# nginx-auth-totp

A very simple authentication provider to be used with nginx's `auth_request`.
It uses TOTP (Time based One-Time Passwords) for verification. On success it stores
a cookie which is valid for one day.

### Compile

It's written in rust, compile it with `cargo build`.

### Run

```
Usage: nginx_auth_totp [options]

Options:
    -o, --port PORT     TCP Port to listen on
    -d, --debug         Use loglevel Debug instead of Warn
```

### Nginx configuration

Find example in `test/etc/nginx.conf`