# ohkami-tracing-req

[![Rust](https://github.com/DoumanAsh/ohkami-tracing-req/actions/workflows/rust.yml/badge.svg)](https://github.com/DoumanAsh/ohkami-tracing-req/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/ohkami-tracing-req.svg)](https://crates.io/crates/ohkami-tracing-req)
[![Documentation](https://docs.rs/ohkami-tracing-req/badge.svg)](https://docs.rs/crate/ohkami-tracing-req/)

[ohkami](https://github.com/ohkami-rs/ohkami) middleware to provide reliable request tracing

## Usage

[TracingMiddleware] can be customized with [MiddlewareCustomization] custom struct

```rust
use std::net::IpAddr;

use ohkami_tracing_req::{TracingMiddleware, MiddlewareCustomization};

//Declare span creation function make_my_request_span()
ohkami_tracing_req::make_request_spanner!(make_my_request_span("my_request", tracing::Level::INFO));

//Customize middleware
#[derive(Copy, Clone)]
pub struct Customize;
impl MiddlewareCustomization for Customize {
    //I want to inspects proxy forwarding headers if any
    const INSPECT_HEADERS: &[&str] = &["Forwarded", "forwarded"];
    //I want to create request-id if it is not present
    const CREATE_REQ_ID: bool = true;
    fn extract_client_ip(&self, span: &tracing::Span, parts: &ohkami::Request) -> Option<IpAddr> {
       //You can define IP extraction logic here
       //For example using above mentioned headers to determine client's ip in case you're behind proxy
       None
    }

}
let fang = TracingMiddleware::new_with(make_my_request_span, Customize);
//you can pass your fang to ohkami now
```
