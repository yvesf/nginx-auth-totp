use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::boxed::Box;
use bytes::Bytes;

use tokio;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::codec::{Encoder, Decoder};
use tokio_threadpool::Builder;
use tokio_executor::enter;
use bytes::BytesMut;
use http::header::HeaderValue;
use http::{Request, Response};
use thread_local::ThreadLocal;

use system;

pub trait HttpHandler<T> {
    fn respond(&self, state: &T, req: Request<Bytes>) -> Response<String>;
}

pub fn serve<
    T: 'static + Send + Clone + HttpHandler<X>,
    X: Send + Clone + 'static
>(addr: SocketAddr, state: X, handler: T) {
    let listener = TcpListener::bind(&addr).expect("failed to bind");
    info!("Listening on: {}", addr);
    let tl_handler: Arc<ThreadLocal<T>> = Arc::new(ThreadLocal::new());
    let tl_state: Arc<ThreadLocal<X>>=  Arc::new(ThreadLocal::new());

    let program =
        listener.incoming()
            .map_err(|e| error!("failed to accept socket; error = {:?}", e))
            .for_each(move |socket| {
                let peer_addr = match socket.peer_addr() {
                    Ok(addr) => format!("{}", addr),
                    Err(_) => "<error>".to_string(),
                };

                let (tx, rx) =
                    HttpFrame.framed(socket).split();

                let tl_handler = tl_handler.clone();
                let handler = handler.clone();

                let tl_state = tl_state.clone();
                let state = state.clone();

                let rx_task = rx.and_then(move |req| {
                    let state = state.clone();
                    let state = tl_state.get_or(||{
                        debug!("Clone state");
                        Box::new(state.clone())
                    });
                    let handler = tl_handler.get_or(|| {
                        debug!("Clone handler");
                        Box::new(handler.clone())
                    });
                    info!("{:?} {} {} {:?}", peer_addr, req.method(), req.uri(), req.version());
                    let response = handler.respond(&state, req);
                    Box::new(future::ok(response))
                });
                let tx_task = tx.send_all(rx_task)
                    .then(|res| {
                        if let Err(e) = res {
                            error!("failed to process connection; error = {:?}", e);
                        }
                        Ok(())
                    });

                // Spawn the task that handles the connection.
                tokio::spawn(tx_task);
                Ok(())
            });


    let mut builder = Builder::new();
    let runtime = builder
        .name_prefix("httpd-")
        .after_start(|| {
            debug!("Start new worker");
            system::initialize_rng_from_time();
        })
        .build();
    runtime.spawn(program);
    enter().expect("nested tokio::run")
        .block_on(runtime.shutdown_on_idle())
        .unwrap();
}

///
/// The following code is mostly copied from:
/// https://github.com/tokio-rs/tokio/blob/master/examples/tinyhttp.rs
///-------------------------------------------------------------------------------------------------
struct HttpFrame;

/// Implementation of encoding an HTTP response into a `BytesMut`, basically
/// just writing out an HTTP/1.1 response.
impl Encoder for HttpFrame {
    type Item = Response<String>;
    type Error = io::Error;

    fn encode(&mut self, item: Response<String>, dst: &mut BytesMut) -> io::Result<()> {
        use std::fmt::Write;

        write!(BytesWrite(dst), "\
            HTTP/1.1 {}\r\n\
            Server: nginx-auth-totp\r\n\
            Content-Length: {}\r\n\
            Date: {}\r\n\
        ", item.status(), item.body().len(), date::now()).unwrap();

        for (k, v) in item.headers() {
            dst.extend_from_slice(k.as_str().as_bytes());
            dst.extend_from_slice(b": ");
            dst.extend_from_slice(v.as_bytes());
            dst.extend_from_slice(b"\r\n");
        }

        dst.extend_from_slice(b"\r\n");
        dst.extend_from_slice(item.body().as_bytes());

        return Ok(());

        // Right now `write!` on `Vec<u8>` goes through io::Write and is not
        // super speedy, so inline a less-crufty implementation here which
        // doesn't go through io::Error.
        struct BytesWrite<'a>(&'a mut BytesMut);

        impl<'a> fmt::Write for BytesWrite<'a> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.0.extend_from_slice(s.as_bytes());
                Ok(())
            }

            fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
                fmt::write(self, args)
            }
        }
    }
}

/// Implementation of decoding an HTTP request from the bytes we've read so far.
/// This leverages the `httparse` crate to do the actual parsing and then we use
/// that information to construct an instance of a `http::Request` object,
/// trying to avoid allocations where possible.
impl Decoder for HttpFrame {
    type Item = Request<Bytes>;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<Request<Bytes>>> {
        // TODO: we should grow this headers array if parsing fails and asks
        //       for more headers
        let mut headers = [None; 16];
        let (method, path, version, amt) = {
            let mut parsed_headers = [httparse::EMPTY_HEADER; 16];
            let mut r = httparse::Request::new(&mut parsed_headers);
            let status = r.parse(src).map_err(|e| {
                let msg = format!("failed to parse http request: {:?}", e);
                io::Error::new(io::ErrorKind::Other, msg)
            })?;

            let amt = match status {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => return Ok(None),
            };

            let toslice = |a: &[u8]| {
                let start = a.as_ptr() as usize - src.as_ptr() as usize;
                assert!(start < src.len());
                (start, start + a.len())
            };

            for (i, header) in r.headers.iter().enumerate() {
                let k = toslice(header.name.as_bytes());
                let v = toslice(header.value);
                headers[i] = Some((k, v));
            }

            (toslice(r.method.unwrap().as_bytes()),
             toslice(r.path.unwrap().as_bytes()),
             r.version.unwrap(),
             amt)
        };
        if version != 1 && version != 0 { // TODO
            error!("Version: {}", version);
            return Err(io::Error::new(io::ErrorKind::Other, "only HTTP/1.1 accepted"));
        }
        let data = src.split_to(amt).freeze();
        let mut req_builder = Request::builder();
        req_builder.method(&data[method.0..method.1]);
        req_builder.uri(data.slice(path.0, path.1));
        req_builder.version(http::Version::HTTP_11);
        for header in headers.iter() {
            let (k, v) = match *header {
                Some((ref k, ref v)) => (k, v),
                None => break,
            };
            let value = unsafe {
                HeaderValue::from_shared_unchecked(data.slice(v.0, v.1))
            };
            req_builder.header(&data[k.0..k.1], value);
        }


        let request_body = src.split_off(0).freeze();
        let req = req_builder.body(request_body).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, e)
        })?;
        Ok(Some(req))
    }
}

mod date {
    use std::cell::RefCell;
    use std::fmt::{self, Write};
    use std::str;

    use time::{self, Duration};

    pub struct Now(());

    /// Returns a struct, which when formatted, renders an appropriate `Date`
    /// header value.
    pub fn now() -> Now {
        Now(())
    }

    // Gee Alex, doesn't this seem like premature optimization. Well you see
    // there Billy, you're absolutely correct! If your server is *bottlenecked*
    // on rendering the `Date` header, well then boy do I have news for you, you
    // don't need this optimization.
    //
    // In all seriousness, though, a simple "hello world" benchmark which just
    // sends back literally "hello world" with standard headers actually is
    // bottlenecked on rendering a date into a byte buffer. Since it was at the
    // top of a profile, and this was done for some competitive benchmarks, this
    // module was written.
    //
    // Just to be clear, though, I was not intending on doing this because it
    // really does seem kinda absurd, but it was done by someone else [1], so I
    // blame them!  :)
    //
    // [1]: https://github.com/rapidoid/rapidoid/blob/f1c55c0555007e986b5d069fe1086e6d09933f7b/rapidoid-commons/src/main/java/org/rapidoid/commons/Dates.java#L48-L66

    struct LastRenderedNow {
        bytes: [u8; 128],
        amt: usize,
        next_update: time::Timespec,
    }

    thread_local!(static LAST: RefCell<LastRenderedNow> = RefCell::new(LastRenderedNow {
        bytes: [0; 128],
        amt: 0,
        next_update: time::Timespec::new(0, 0),
    }));

    impl fmt::Display for Now {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            LAST.with(|cache| {
                let mut cache = cache.borrow_mut();
                let now = time::get_time();
                if now >= cache.next_update {
                    cache.update(now);
                }
                f.write_str(cache.buffer())
            })
        }
    }

    impl LastRenderedNow {
        fn buffer(&self) -> &str {
            str::from_utf8(&self.bytes[..self.amt]).unwrap()
        }

        fn update(&mut self, now: time::Timespec) {
            self.amt = 0;
            write!(LocalBuffer(self), "{}", time::at(now).rfc822()).unwrap();
            self.next_update = now + Duration::seconds(1);
            self.next_update.nsec = 0;
        }
    }

    struct LocalBuffer<'a>(&'a mut LastRenderedNow);

    impl<'a> fmt::Write for LocalBuffer<'a> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let start = self.0.amt;
            let end = start + s.len();
            self.0.bytes[start..end].copy_from_slice(s.as_bytes());
            self.0.amt += s.len();
            Ok(())
        }
    }
}
