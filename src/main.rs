#![deny(warnings)]
extern crate hyper;
extern crate pretty_env_logger;

use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::{Client, Server};
use std::net::SocketAddr;

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
}

#[derive(Debug)]
struct Configuration {
    paths: Vec<Path>,
}

impl Configuration {
    fn print(&self) {
        for p in &self.paths {
            println!("{:?}", p);
        }
    }
}

fn init_config() -> Configuration {
    // TODO: Read from file.
    Configuration {
        paths: vec![
            Path {
                path: "/error".to_string(),
                failure: Failure::Error,
                frequency: 0.5,
            },
            Path {
                path: "/delay".to_string(),
                failure: Failure::Delay,
                frequency: 0.25,
            },
            Path {
                path: "/timeout".to_string(),
                failure: Failure::Timeout,
                frequency: 0.4,
            },
        ],
    }
}

fn main() {
    pretty_env_logger::init();

    let in_addr = ([127, 0, 0, 1], 3001).into();
    let out_addr: SocketAddr = ([66, 39, 158, 129], 80).into();

    let config = init_config();
    config.print();

    let client_main = Client::new();

    // new_service is run for each connection, creating a 'service'
    // to handle requests for that specific connection.
    let new_service = move || {
        let client = client_main.clone();
        // This is the `Service` that will handle the connection.
        // `service_fn_ok` is a helper to convert a function that
        // returns a Response into a `Service`.
        service_fn(move |mut req| {
            let uri_string = format!(
                "http://{}{}",
                out_addr,
                req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("")
            );
            let uri = uri_string.parse().unwrap();
            *req.uri_mut() = uri;
            println!(" -> {}", req.uri());
            client.request(req)
        })
    };

    let server = Server::bind(&in_addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", in_addr);
    println!("Proxying on http://{}", out_addr);

    rt::run(server);
}
