# trouble-maker
A layer 7 fault injection proxy server.

![CI](https://github.com/ijoyce/trouble-maker/workflows/CI/badge.svg)

### Supported Protocols
* HTTP/1.1
* More in progress

### Building
> $ cargo build --release

### Running
> RUST_LOG=info cargo run --release

### Failure Types
* Error
* Delay
* Timeout

### Configuration
##### A basic example
This will proxy requests from 127.0.0.1:3001 to 127.0.0.1:8080 while 50% of the time adding a delay for 300ms and returning an HTTP 500 response.
```TOML
listener_address = "127.0.0.1:3001"
proxy_address = "127.0.0.1:8080"
metrics_path = "/metrics"
max_concurrent_requests = 5000
scenarios = [
    { path = "/.*", failure_type = "Error", frequency = 0.5, delay = 300 },
]
```

An example configuration file can be found [here](https://github.com/ijoyce/trouble-maker/blob/master/Configuration.toml).

### Resources and Inspiration
* LinkedOut: A Request-Level Failure Injection Framework https://engineering.linkedin.com/blog/2018/05/linkedout--a-request-level-failure-injection-framework
* Awesome Chaos Engineering https://github.com/dastergon/awesome-chaos-engineering
* Toxic Proxy: https://github.com/shopify/toxiproxy
