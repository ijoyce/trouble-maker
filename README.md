# trouble-maker
A layer 7 fault injection proxy server.

![CI](https://github.com/ijoyce/trouble-maker/workflows/CI/badge.svg)

### Building
> $ cargo build --release

### Running
> $ cargo run --release

### Logs
To run with TRACE logging enabled:
> RUST_LOG=trace cargo run --release

### Resources
* LinkedOut: A Request-Level Failure Injection Framework https://engineering.linkedin.com/blog/2018/05/linkedout--a-request-level-failure-injection-framework
* Awesome Chaos Engineering https://github.com/dastergon/awesome-chaos-engineering
* Toxic Proxy: https://github.com/shopify/toxiproxy
