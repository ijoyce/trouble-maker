#![deny(warnings)]
extern crate hyper;
extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use futures::{future, Future};
use http::Uri;
use hyper::service::service_fn;
use hyper::{Body, Client, Request, Response, Server, StatusCode};
use regex::Regex;
use serde::{Deserialize, Serialize};

use std::thread;
use std::time::{Duration, Instant};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type ResponseFuture = Box<dyn Future<Item = Response<Body>, Error = GenericError> + Send>;

#[derive(Clone, Debug, Serialize, Deserialize)]
enum FailureType {
    Error,
    Delay,
    Timeout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Failure {
    path: String,
    failure_type: FailureType,
    frequency: f32,
    delay: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Configuration {
    failures: Vec<Failure>,
    listener_address: String,
    proxy_address: String,
}

impl Configuration {
    fn print(&self) {
        for f in &self.failures {
            info!("{:?}", f);
        }
    }
}

fn init() -> Configuration {
    let mut config = config::Config::default();
    config.merge(config::File::with_name("Configuration")).unwrap();
    let config = config.try_into::<Configuration>().unwrap();

    info!("Settings: {:?}", config);

    config
}

fn new_service(req: Request<Body>, config: &Configuration) -> ResponseFuture {
    // Apply failure.
    for failure in &config.failures {
        let re = Regex::new(&failure.path).unwrap();

        if re.is_match(req.uri().path()) {
            match failure.failure_type {
                FailureType::Error => {
                    if let Some(x) = inject_error(failure) {
                        return x;
                    }
                }
                FailureType::Delay => {
                    inject_delay(failure);
                }
                FailureType::Timeout => {
                    if let Some(x) = inject_timeout(failure) {
                        return x;
                    }
                }
            }
        }
    }

    proxy(config, req)
}

fn proxy(config: &Configuration, req: Request<Body>) -> ResponseFuture {
    log_request(&req);

    let mut uri = format!("http://{}", config.proxy_address);

    let (parts, body) = req.into_parts();

    match parts.uri.path_and_query() {
        Some(x) => uri.push_str(&x.to_string()),
        None => (),
    }

    let mut proxy_req = Request::new(body);
    *proxy_req.method_mut() = parts.method;
    *proxy_req.version_mut() = parts.version;
    *proxy_req.headers_mut() = parts.headers;
    *proxy_req.uri_mut() = uri.parse::<Uri>().unwrap();

    log_request(&proxy_req);

    let client = Client::new();

    Box::new(client.request(proxy_req).from_err().map(|web_res| web_res))
}

fn log_request(request: &Request<Body>) {
    info!("> {:?} {:?} {:?}", request.method(), request.uri(), request.version());

    let h = &request.headers();

    for key in h.keys() {
        info!("> {:?}: {:?}", key, request.headers().get(key).unwrap());
    }
}

fn inject_delay(failure: &Failure) {
    let x: f32 = rand::random();
    if x <= failure.frequency {
        thread::sleep(Duration::from_millis(failure.delay));
    }
}

fn inject_error(failure: &Failure) -> Option<ResponseFuture> {
    let x: f32 = rand::random();
    if x <= failure.frequency {
        thread::sleep(Duration::from_millis(failure.delay));
        return Some(Box::new(future::ok(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(""))
                .unwrap(),
        )));
    };
    None
}

fn inject_timeout(failure: &Failure) -> Option<ResponseFuture> {
    let x: f32 = rand::random();
    if x <= failure.frequency {
        thread::sleep(Duration::from_millis(failure.delay));
        return Some(Box::new(future::ok(
            Response::builder()
                .status(StatusCode::GATEWAY_TIMEOUT)
                .body(Body::from(""))
                .unwrap(),
        )));
    };
    None
}

fn main() {
    pretty_env_logger::init();

    hyper::rt::run(future::lazy(move || {
        let now = Instant::now();

        let config = init();
        config.print();

        let listening_addr = config.listener_address.parse().unwrap();
        let proxying_addr: String = config.proxy_address.parse().unwrap();

        let new_service = move || {
            let config = config.clone();
            service_fn(move |req| new_service(req, &config))
        };

        let server = Server::bind(&listening_addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        info!("Listening on http://{}", listening_addr);
        info!("Proxying to http://{}", proxying_addr);
        info!("Started in {}ms.", now.elapsed().as_millis());

        server
    }));
}
