#![deny(warnings)]
extern crate hyper;
extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use futures::{future, Future};
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};

use std::thread;
use std::time::Duration;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type ResponseFuture = Box<dyn Future<Item = Response<Body>, Error = GenericError> + Send>;

#[derive(Debug, Serialize, Deserialize)]
enum FailureType {
    Error,
    Delay,
    Timeout,
}

#[derive(Debug, Serialize, Deserialize)]
struct Failure {
    path: String,
    failure_type: FailureType,
    frequency: f32,
    delay: u64,
}

#[derive(Debug, Serialize, Deserialize)]
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
    // TODO: Read from file.
    Configuration {
        listener_address: String::from("127.0.0.1:3001"),
        proxy_address: String::from("127.0.0.1:3002"),
        failures: vec![
            Failure {
                path: "/error".to_string(),
                failure_type: FailureType::Error,
                frequency: 0.5,
                delay: 300,
            },
            Failure {
                path: "/delay".to_string(),
                failure_type: FailureType::Delay,
                frequency: 0.25,
                delay: 800,
            },
            Failure {
                path: "/timeout".to_string(),
                failure_type: FailureType::Timeout,
                frequency: 0.4,
                delay: 300,
            },
        ],
    }
}

fn new_service(req: Request<Body>) -> ResponseFuture {
    let config = init();

    // Apply failure.
    for failure in &config.failures {
        if failure.path == req.uri().path() {
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

    proxy(req)
}

fn proxy(req: Request<Body>) -> ResponseFuture {
    let body = Body::from(req.uri().to_string());
    Box::new(future::ok(
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body)
            .unwrap(),
    ))
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

    let config = init();
    config.print();

    let listening_addr = config.listener_address.parse().unwrap();
    let proxying_addr: String = config.proxy_address.parse().unwrap();

    hyper::rt::run(future::lazy(move || {
        let new_service = move || service_fn(move |req| new_service(req));

        let server = Server::bind(&listening_addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        info!("Listening on http://{}", listening_addr);
        info!("Proxying to http://{}", proxying_addr);

        server
    }));
}
