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
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

mod config;
use crate::config::*;

mod metrics;
use crate::metrics::Metrics;

lazy_static! {
    static ref CONFIG: Configuration = config::init();
    static ref METRICS: Arc<Mutex<Metrics>> = Arc::new(Mutex::new(Metrics::new()));
}

async fn new_service(
    req: Request<Body>,
    config: &Configuration,
    metrics: Arc<Mutex<Metrics>>,
) -> Result<Response<Body>, Error> {
    metrics.lock().unwrap().requests.increment();

    // Find matching scenario and apply it.
    for scenario in &config.scenarios {
        let re = Regex::new(&scenario.path).unwrap();

        if re.is_match(req.uri().path()) {
            match scenario.failure_type {
                FailureType::Error => match inject_error(scenario, &metrics).await {
                    Some(x) => {
                        return Ok(x);
                    }
                    None => {
                        break;
                    }
                },
                FailureType::Delay => {
                    inject_delay(scenario, &metrics);
                    break;
                }
                FailureType::Timeout => match inject_timeout(scenario, &metrics) {
                    Some(x) => {
                        return Ok(x);
                    }
                    None => {
                        break;
                    }
                },
            }
        }
    }

    proxy(config, req).await
}

async fn proxy(config: &Configuration, req: Request<Body>) -> Result<Response<Body>, Error> {
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

    info!(
        "Proxying {}{} -> {}.",
        config.listener_address,
        parts.uri,
        proxy_req.uri()
    );

    let client = Client::new();

    client.request(proxy_req).await
}

fn inject_delay(scenario: &Scenario, metrics: &Arc<Mutex<Metrics>>) {
    let x: f32 = rand::random();

    if x <= scenario.frequency {
        metrics.lock().unwrap().delays.increment();
        thread::sleep(Duration::from_millis(scenario.delay));
    }
}

async fn inject_error(
    scenario: &Scenario,
    metrics: &Arc<Mutex<Metrics>>,
) -> Option<Response<Body>> {
    let x: f32 = rand::random();

    if x <= scenario.frequency {
        thread::sleep(Duration::from_millis(scenario.delay));
        metrics.lock().unwrap().errors.increment();

        return Some(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(""))
                .unwrap(),
        );
    };
    None
}

fn inject_timeout(scenario: &Scenario, metrics: &Arc<Mutex<Metrics>>) -> Option<Response<Body>> {
    let x: f32 = rand::random();

    if x <= scenario.frequency {
        thread::sleep(Duration::from_millis(scenario.delay));
        metrics.lock().unwrap().timeouts.increment();

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
        Ok::<_, Error>(service_fn(move |req| {
            new_service(req, &CONFIG, Arc::clone(&METRICS))
        }))
    });

    let server = Server::bind(&listening_addr).serve(make_service);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    info!("Listening on http://{}", listening_addr);
    info!("Proxying to http://{}", proxying_addr);
    info!("Started in {}ms.", now.elapsed().as_millis());
    info!("Ready to cause trouble.");

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");

    println!(
        "\nMetrics\n-----------------------\n{}",
        METRICS.lock().unwrap()
    );
}
