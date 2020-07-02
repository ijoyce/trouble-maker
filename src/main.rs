#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

use http::header::HeaderValue;
use http::Uri;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Error, Request, Response, Server, StatusCode};
use regex::Regex;

use std::thread;
use std::time::{Duration, Instant};

mod config;
use crate::config::*;

lazy_static! {
    static ref CONFIG: Configuration = config::init();
}

async fn new_service(req: Request<Body>, config: &Configuration) -> Result<Response<Body>, Error> {
    // Find matching scenario and apply it.
    for scenario in &config.scenarios {
        let re = Regex::new(&scenario.path).unwrap();

        if re.is_match(req.uri().path()) {
            match scenario.failure_type {
                FailureType::Error => {
                    if let Some(x) = inject_error(scenario).await {
                        return Ok(x);
                    }
                }
                FailureType::Delay => {
                    inject_delay(scenario);
                }
                FailureType::Timeout => {
                    if let Some(x) = inject_timeout(scenario) {
                        return Ok(x);
                    }
                }
            }
        }
    }

    proxy(config, req).await
}

async fn proxy(config: &Configuration, req: Request<Body>) -> Result<Response<Body>, Error> {
    log_request(&req);

    let mut uri = format!("http://{}", config.proxy_address);

    let (parts, body) = req.into_parts();

    if let Some(x) = parts.uri.path_and_query() {
        uri.push_str(&x.to_string())
    }

    let mut proxy_req = Request::new(body);
    *proxy_req.method_mut() = parts.method;
    *proxy_req.version_mut() = parts.version;
    *proxy_req.headers_mut() = parts.headers;
    *proxy_req.uri_mut() = uri.parse::<Uri>().unwrap();
    proxy_req
        .headers_mut()
        .insert("x-trouble-maker-agent", HeaderValue::from_static("0.1"));

    log_request(&proxy_req);

    let client = Client::new();

    client.request(proxy_req).await
}

fn log_request(request: &Request<Body>) {
    info!(
        "> {:?} {:?} {:?}",
        request.method(),
        request.uri(),
        request.version()
    );

    let h = &request.headers();

    for key in h.keys() {
        info!("> > {:?}: {:?}", key, request.headers().get(key).unwrap());
    }
}

fn inject_delay(scenario: &Scenario) {
    let x: f32 = rand::random();
    if x <= scenario.frequency {
        thread::sleep(Duration::from_millis(scenario.delay));
    }
}

async fn inject_error(scenario: &Scenario) -> Option<Response<Body>> {
    let x: f32 = rand::random();
    if x <= scenario.frequency {
        thread::sleep(Duration::from_millis(scenario.delay));
        return Some(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(""))
                .unwrap(),
        );
    };
    None
}

fn inject_timeout(scenario: &Scenario) -> Option<Response<Body>> {
    let x: f32 = rand::random();
    if x <= scenario.frequency {
        thread::sleep(Duration::from_millis(scenario.delay));
        return Some(
            Response::builder()
                .status(StatusCode::GATEWAY_TIMEOUT)
                .body(Body::from(""))
                .unwrap(),
        );
    };
    None
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    CONFIG.print();

    let now = Instant::now();

    let listening_addr = &CONFIG.listener_address.parse().unwrap();
    let proxying_addr: String = CONFIG.proxy_address.parse().unwrap();

    let make_service = make_service_fn(move |_| async move {
        Ok::<_, Error>(service_fn(move |req| new_service(req, &CONFIG)))
    });

    let server = Server::bind(&listening_addr).serve(make_service);

    info!("Listening on http://{}", listening_addr);
    info!("Proxying to http://{}", proxying_addr);
    info!("Started in {}ms.", now.elapsed().as_millis());
    info!("Ready to cause trouble.");

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
