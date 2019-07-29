//#![deny(warnings)]
extern crate hyper;
extern crate pretty_env_logger;

use futures::{future, Future};
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server, StatusCode};

use std::thread;
use std::time::Duration;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type ResponseFuture = Box<dyn Future<Item = Response<Body>, Error = GenericError> + Send>;

#[derive(Debug)]
enum Failure {
    Error,
    Delay,
    Timeout,
}

#[derive(Debug)]
struct Path {
    path: String,
    failure: Failure,
    frequency: f32,
    delay: u64,
}

#[derive(Debug)]
struct Configuration {
    paths: Vec<Path>,
    listen_address: String,
}

impl Configuration {
    fn print(&self) {
        for p in &self.paths {
            println!("{:?}", p);
        }
    }
}

fn init() -> Configuration {
    // TODO: Read from file.
    Configuration {
        listen_address: String::from("127.0.0.1:3001"),
        paths: vec![
            Path {
                path: "/error".to_string(),
                failure: Failure::Error,
                frequency: 0.5,
                delay: 300,
            },
            Path {
                path: "/delay".to_string(),
                failure: Failure::Delay,
                frequency: 0.25,
                delay: 800,
            },
            Path {
                path: "/timeout".to_string(),
                failure: Failure::Timeout,
                frequency: 0.4,
                delay: 300,
            },
        ],
    }
}

fn new_service(req: Request<Body>) -> ResponseFuture {
    let config = init();

    // Apply failure.
    for p in &config.paths {
        if p.path == req.uri().path() {
            match p.failure {
                Failure::Error => {
                    println!("{:?}", Failure::Error);
                    thread::sleep(Duration::from_millis(p.delay));
                }
                Failure::Delay => {
                    println!("{:?}", Failure::Delay);
                    let x: f32 = rand::random();
                    if x <= p.frequency {
                        thread::sleep(Duration::from_millis(p.delay));
                    }
                }
                Failure::Timeout => {
                    println!("{:?}", Failure::Timeout);
                    thread::sleep(Duration::from_millis(p.delay));
                }
            }
        }
    }

    let body = Body::from("Not Found");
    Box::new(future::ok(
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body)
            .unwrap(),
    ))
}

fn main() {
    pretty_env_logger::init();

    let config = init();
    config.print();

    let addr = config.listen_address.parse().unwrap();

    hyper::rt::run(future::lazy(move || {
        let new_service = move || service_fn(move |req| new_service(req));

        let server = Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        println!("Listening on http://{}", addr);

        server
    }));
}
