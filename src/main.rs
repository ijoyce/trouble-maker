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
    sync::Mutex,
    time::{Duration, Instant},
};

mod config;
use crate::config::*;

mod metrics;
use crate::metrics::Metrics;

type Delay = u64;
type Fault = (Option<Response<Body>>, Option<Delay>);

lazy_static! {
    static ref CONFIG: Configuration = config::init();
    static ref METRICS: Mutex<Metrics> = Mutex::new(Metrics::new());
}

async fn new_service(
    request: Request<Body>,
    config: &Configuration,
    metrics: &Mutex<Metrics>,
) -> Result<Response<Body>, Error> {
    let path = request.uri().path();

    // Serve up metrics.
    if path == config.metrics_path {
        return load_metrics(metrics);
    }

    // Inc metrics.
    metrics.lock().unwrap().requests.increment();
    metrics.lock().unwrap().concurrent_requests.increment();

    // 503 if we're over the max_concurrent_requests.
    if metrics.lock().unwrap().concurrent_requests.value >= config.max_concurrent_requests {
        info!("Shedding {}{}.", config.listener_address, path);
        metrics.lock().unwrap().concurrent_requests.decrement();
        metrics.lock().unwrap().shed_requests.increment();
        return load_overloaded(config);
    }

    // Find matching scenario and apply it.
    for scenario in &config.scenarios {
        let re = Regex::new(&scenario.path).unwrap();

        if re.is_match(path) {
            let fault = match scenario.failure_type {
                FailureType::Error => determine_error(scenario, metrics),
                FailureType::Delay => determine_delay(scenario, metrics),
                FailureType::Timeout => determine_timeout(scenario, metrics),
            };

            match fault {
                (None, Some(delay)) => {
                    info!("Causing a {}ms delay for {}.", delay, path);
                    thread::sleep(Duration::from_millis(delay));
                    break;
                }
                (Some(response), None) => {
                    info!("Return an HTTP {} for {}.", response.status(), path);
                    return Ok::<Response<Body>, Error>(response);
                }
                (Some(response), Some(delay)) => {
                    info!(
                        "Causing a {}ms delay and an HTTP {} for {}.",
                        delay,
                        response.status(),
                        path
                    );
                    thread::sleep(Duration::from_millis(delay));
                    return Ok::<Response<Body>, Error>(response);
                }
                (None, None) => {
                    break;
                }
            };
        }
    }

    // No matching scenario, proxy.
    proxy(config, metrics, request).await
}

async fn proxy(
    config: &Configuration,
    metrics: &Mutex<Metrics>,
    req: Request<Body>,
) -> Result<Response<Body>, Error> {
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
    let result = client.request(proxy_req).await;

    metrics.lock().unwrap().concurrent_requests.decrement();

    result
}

fn determine_delay(scenario: &Scenario, metrics: &Mutex<Metrics>) -> Fault {
    let x: f32 = rand::random();

    if x <= scenario.frequency {
        metrics.lock().unwrap().delays.increment();
        return (None, Some(scenario.delay));
    }

    (None, None)
}

fn determine_error(scenario: &Scenario, metrics: &Mutex<Metrics>) -> Fault {
    let x: f32 = rand::random();

    if x <= scenario.frequency {
        metrics.lock().unwrap().errors.increment();

        let response = Some(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(""))
                .unwrap(),
        );

        return (response, Some(scenario.delay));
    };

    (None, None)
}

fn determine_timeout(scenario: &Scenario, metrics: &Mutex<Metrics>) -> Fault {
    let x: f32 = rand::random();

    if x <= scenario.frequency {
        metrics.lock().unwrap().timeouts.increment();

        let response = Some(
            Response::builder()
                .status(StatusCode::GATEWAY_TIMEOUT)
                .body(Body::from(""))
                .unwrap(),
        );

        return (response, Some(scenario.delay));
    };

    (None, None)
}

fn load_metrics(metrics: &Mutex<Metrics>) -> Result<Response<Body>, Error> {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(metrics.lock().unwrap().to_json()))
        .unwrap();

    Ok::<Response<Body>, Error>(response)
}

fn load_overloaded(config: &Configuration) -> Result<Response<Body>, Error> {
    let response = Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .body(Body::from(format!(
            "The maximum number of concurrent requests({}) has been exceeded.",
            config.max_concurrent_requests
        )))
        .unwrap();

    Ok::<Response<Body>, Error>(response)
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    CONFIG.print();

    let now = Instant::now();

    let listening_addr = &CONFIG.listener_address.parse().unwrap();
    let proxying_addr: String = CONFIG.proxy_address.parse().unwrap();

    let make_service = make_service_fn(move |_| async move {
        Ok::<_, Error>(service_fn(move |req| new_service(req, &CONFIG, &METRICS)))
    });

    let server = Server::bind(&listening_addr).serve(make_service);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    info!("Listening on http://{}", listening_addr);
    info!("Proxying to http://{}", proxying_addr);
    info!(
        "Metrics available at http://{}{}",
        listening_addr, CONFIG.metrics_path
    );
    info!(
        "Maximum concurrent requests: {}",
        CONFIG.max_concurrent_requests
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determine_timeout_with_100_percent_frequency_returns_timeout() {
        let metrics = Metrics::new();
        let scenario = Scenario {
            path: String::from("/test"),
            failure_type: FailureType::Timeout,
            frequency: 1.0,
            delay: 500,
        };

        let fault = determine_timeout(&scenario, &Mutex::new(metrics));

        assert_eq!(fault.0.unwrap().status(), StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(fault.1.unwrap(), 500);
    }

    #[test]
    fn determine_timeout_with_0_percent_frequency_returns_no_timeout() {
        let metrics = Metrics::new();
        let scenario = Scenario {
            path: String::from("/test"),
            failure_type: FailureType::Timeout,
            frequency: 0.0,
            delay: 500,
        };

        let fault = determine_timeout(&scenario, &Mutex::new(metrics));

        assert_eq!(fault.0.is_none(), true);
        assert_eq!(fault.1.is_none(), true);
    }

    #[test]
    fn determine_error_with_100_percent_frequency_returns_error() {
        let metrics = Metrics::new();
        let scenario = Scenario {
            path: String::from("/test"),
            failure_type: FailureType::Error,
            frequency: 1.0,
            delay: 500,
        };

        let fault = determine_error(&scenario, &Mutex::new(metrics));

        assert_eq!(fault.0.unwrap().status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(fault.1.unwrap(), 500);
    }

    #[test]
    fn determine_error_with_0_percent_frequency_returns_no_error() {
        let metrics = Metrics::new();
        let scenario = Scenario {
            path: String::from("/test"),
            failure_type: FailureType::Error,
            frequency: 0.0,
            delay: 500,
        };

        let fault = determine_error(&scenario, &Mutex::new(metrics));

        assert_eq!(fault.0.is_none(), true);
        assert_eq!(fault.1.is_none(), true);
    }

    #[test]
    fn determine_delay_with_100_percent_frequency_returns_delay() {
        let metrics = Metrics::new();
        let scenario = Scenario {
            path: String::from("/test"),
            failure_type: FailureType::Delay,
            frequency: 1.0,
            delay: 500,
        };

        let fault = determine_delay(&scenario, &Mutex::new(metrics));

        assert_eq!(fault.0.is_none(), true);
        assert_eq!(fault.1.unwrap(), 500);
    }

    #[test]
    fn determine_delay_with_0_percent_frequency_returns_no_delay() {
        let metrics = Metrics::new();
        let scenario = Scenario {
            path: String::from("/test"),
            failure_type: FailureType::Delay,
            frequency: 0.0,
            delay: 500,
        };

        let fault = determine_error(&scenario, &Mutex::new(metrics));

        assert_eq!(fault.0.is_none(), true);
        assert_eq!(fault.1.is_none(), true);
    }
}
