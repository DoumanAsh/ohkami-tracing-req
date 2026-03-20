//!Request tracing middleware for [ohkami](https://github.com/ohkami-rs/ohkami)
//!
//!## Usage
//!
//![TracingMiddleware] can be customized with [MiddlewareCustomization] custom struct
//!
//!```rust
//!use std::net::IpAddr;
//!
//!use ohkami_tracing_req::{TracingMiddleware, MiddlewareCustomization};
//!
//!//Declare span creation function make_my_request_span()
//!ohkami_tracing_req::make_request_spanner!(make_my_request_span("my_request", tracing::Level::INFO));
//!
//!//Customize middleware
//!#[derive(Copy, Clone)]
//!pub struct Customize;
//!impl MiddlewareCustomization for Customize {
//!    //I want to inspects proxy forwarding headers if any
//!    const INSPECT_HEADERS: &[&str] = &["Forwarded", "forwarded"];
//!    //I want to create request-id if it is not present
//!    const CREATE_REQ_ID: bool = true;
//!    fn extract_client_ip(&self, span: &tracing::Span, parts: &ohkami::Request) -> Option<IpAddr> {
//!       //You can define IP extraction logic here
//!       //For example using above mentioned headers to determine client's ip in case you're behind proxy
//!       None
//!    }
//!
//!}
//!let fang = TracingMiddleware::new_with(make_my_request_span, Customize);
//!//you can pass your fang to ohkami now
//!```

#![warn(missing_docs)]
#![allow(clippy::style)]

use core::task;
use core::pin::Pin;
use core::net::IpAddr;
use core::future::Future;

//Re-export of [tracing](https://docs.rs/tracing/)
pub use tracing;

mod request_id;
pub use request_id::RequestId;
mod headers;
pub use headers::{REQUEST_ID_LOW, REQUEST_ID};

///Alias to function signature required to create span
pub type MakeSpan = fn() -> tracing::Span;

#[macro_export]
///Declares `fn` function compatible with `MakeSpan` using provided parameters
///
///## Span fields
///
///Following fields are declared when span is created:
///- `http.request.method`
///- `url.path`
///- `url.query`
///- `http.request_id` - Inherited from request 'X-Request-Id' or random uuid
///- `user_agent.original` - Only populated if user agent header is present
///- `http.headers` - Optional. Populated if more than 1 header specified via layer [config](struct.HttpRequestLayer.html#method.with_inspect_headers)
///- `network.protocol.name` - Either `http` or `grpc` depending on `content-type`
///- `network.protocol.version` - Set to HTTP version in case of plain `http` protocol.
///- `client.address` - Optionally added if IP extractor is specified via layer [config](struct.HttpRequestLayer.html#method.with_extract_client_ip)
///- `http.response.status_code` - Semantics of this code depends on `protocol`
///- `error.type` - Populated with `core::any::type_name` value of error type used by the service.
///- `error.message` - Populated with `Display` content of the error, returned by underlying service, after processing request.
///
///Loosely follows <https://opentelemetry.io/docs/specs/semconv/http/http-spans/#http-server>
///
///## Additional fields
///
///Additional fields can be declared by passing extra arguments after `level` in the same way as you would pass it to `tracing::span!` macro
///
///Note that you need to use `tracing::field::Empty` if you want to add value later
///
///## Usage
///
///```
///use ohkami_tracing_req::make_request_spanner;
///
///make_request_spanner!(make_my_request_span("my_request", tracing::Level::INFO));
/////Customize span with extra fields. You can use tracing::field::Empty if you want to omit value
///make_request_spanner!(make_my_service_request_span("my_request", tracing::Level::INFO, service_name = "<your name>"));
///
///let span = make_my_request_span();
///span.record("url.path", "I can override span field");
///
///```
macro_rules! make_request_spanner {
    ($fn:ident($name:literal, $level:expr)) => {
        $crate::make_request_spanner!($fn($name, $level,));
    };
    ($fn:ident($name:literal, $level:expr, $($fields:tt)*)) => {
        #[track_caller]
        pub fn $fn() -> $crate::tracing::Span {
            use $crate::tracing::field;

            $crate::tracing::span!(
                $level,
                $name,
                //Defaults
                span.kind = "server",
                //Assigned on creation of span
                http.request.method = field::Empty,
                url.path = field::Empty,
                url.query = field::Empty,
                http.request_id = field::Empty,
                user_agent.original = field::Empty,
                http.headers = field::Empty,
                network.protocol.name = "http",
                network.protocol.version = field::Empty,
                //Optional
                client.address = field::Empty,
                //Assigned after request is complete
                http.response.status_code = field::Empty,
                error.type = field::Empty,
                error.message = field::Empty,
                $(
                    $fields
                )*
            )
        }
    };
}

///[TracingMiddleware] customization interface
pub trait MiddlewareCustomization: Clone + Send + Sync + 'static {
    ///Specifies list of headers you want to inspect via `http.headers` attribute
    const INSPECT_HEADERS: &'static [&'static str] = &[];
    ///Specifies whether to create request id in case client provided none
    const CREATE_REQ_ID: bool = false;

    #[allow(unused)]
    #[inline(always)]
    ///Defines way to extract `IpAddr` from `parts`
    ///
    ///Defaults to always return `None`, in which case, middleware will attempt to use `Request::ip` unless it is unspecified IP
    fn extract_client_ip(&self, span: &tracing::Span, parts: &ohkami::Request) -> Option<IpAddr> {
        None
    }

    #[allow(unused)]
    #[inline(always)]
    ///Callback to be called on incoming request
    ///
    ///Defaults to be noop
    fn on_request(&self, span: &tracing::Span, request: &ohkami::Request) {
    }

    #[allow(unused)]
    #[inline(always)]
    ///Callback to be called when response is returned
    ///
    ///Defaults to be noop
    fn on_response(&self, span: &tracing::Span, response: &mut ohkami::Response) {
    }
}

impl<I: MiddlewareCustomization> MiddlewareCustomization for std::sync::Arc<I> {
    const INSPECT_HEADERS: &'static [&'static str] = I::INSPECT_HEADERS;
    const CREATE_REQ_ID: bool = I::CREATE_REQ_ID;

    #[inline(always)]
    fn on_request(&self, span: &tracing::Span, request: &ohkami::Request) {
        I::on_request(self, span, request)
    }

    #[inline(always)]
    fn on_response(&self, span: &tracing::Span, response: &mut ohkami::Response) {
        I::on_response(self, span, response)
    }

    #[inline(always)]
    fn extract_client_ip(&self, span: &tracing::Span, req: &ohkami::Request) -> Option<IpAddr> {
        I::extract_client_ip(self, span, req)
    }
}

#[derive(Copy, Clone)]
///Default customization that does nothing
pub struct NoCustomization;
impl MiddlewareCustomization for NoCustomization {
    const INSPECT_HEADERS: &'static [&'static str] = &[];
}

#[derive(Copy, Clone)]
struct Context<C> {
    span_maker: MakeSpan,
    customization: C,
}

#[derive(Copy, Clone)]
///Tracing middleware implementing [FangAction](https://docs.rs/ohkami/latest/ohkami/fang/trait.Fanc.html)
pub struct TracingMiddleware<C> {
    ctx: Context<C>
}

impl TracingMiddleware<NoCustomization> {
    #[inline(always)]
    ///Creates new instance
    pub const fn new(span_maker: MakeSpan) -> Self {
        Self::new_with(span_maker, NoCustomization)
    }
}

impl<C: MiddlewareCustomization> TracingMiddleware<C> {
    #[inline(always)]
    ///Creates new instance
    pub const fn new_with(span_maker: MakeSpan, customization: C) -> Self {
        Self {
            ctx: Context {
                span_maker,
                customization
            }
        }
    }
}

impl<C: MiddlewareCustomization, F: ohkami::FangProc> ohkami::Fang<F> for TracingMiddleware<C> {
    type Proc = TracingMiddlewareProc<F, C>;
    #[inline(always)]
    fn chain(&self, inner: F) -> Self::Proc {
        TracingMiddlewareProc {
            inner,
            ctx: self.ctx.clone(),
        }
    }
}

///Tracing middleware implementing [FangAction](https://docs.rs/ohkami/latest/ohkami/fang/trait.FancProc.html)
pub struct TracingMiddlewareProc<F, C> {
    inner: F,
    ctx: Context<C>,
}

impl<F: ohkami::FangProc, C: MiddlewareCustomization> ohkami::FangProc for TracingMiddlewareProc<F, C> {
    #[inline]
    fn bite<'b>(&'b self, req: &'b mut ohkami::Request) -> impl Future<Output = ohkami::Response> {
        let span = (self.ctx.span_maker)();
        let _entered = span.enter();

        let req_id = req.headers.get(REQUEST_ID).map(|value| (REQUEST_ID, value)).or_else(|| req.headers.get(REQUEST_ID_LOW).map(|value| (REQUEST_ID_LOW, value)));
        let req_id = if let Some((header_key, req_id)) = req_id {
            let req_id = RequestId::from_str(req_id);
            req.context.set(req_id.clone());
            span.record("http.request_id", req_id.as_str());
            Some((header_key, req_id))
        } else if C::CREATE_REQ_ID {
            let req_id = RequestId::from_uuid(uuid::Uuid::new_v4());
            req.context.set(req_id.clone());
            span.record("http.request_id", req_id.as_str());
            Some((REQUEST_ID_LOW, req_id))
        } else {
            None
        };

        if let Some(user_agent) = req.headers.user_agent() {
            span.record("user_agent.original", user_agent);
        }

        span.record("http.request.method", req.method.as_str());
        span.record("url.path", req.path.str().as_ref());
        //Need to make PR to provide is_empty() to determine whether query has any value
        span.record("url.query", tracing::field::debug(&req.query));
        //ohkami doesn't suport anything more than HTTP1 so for now assume it is 1.1
        span.record("network.protocol.version", 1.1);
        if !C::INSPECT_HEADERS.is_empty() {
            span.record("http.headers", tracing::field::debug(headers::InspectHeaders {
                header_list: C::INSPECT_HEADERS,
                headers: &req.headers
            }));
        }

        if let Some(client_ip) = self.ctx.customization.extract_client_ip(&span, &req) {
            span.record("client.address", tracing::field::display(client_ip));
        } else if !req.ip.is_unspecified() {
            span.record("client.address", tracing::field::display(req.ip));
        }

        self.ctx.customization.on_request(&span, &req);

        drop(_entered);

        ResponseFuture {
            inner: tracing::instrument::Instrument::instrument(self.inner.bite(req), span),
            customization: self.ctx.customization.clone(),
            req_id,
        }
    }
}

///Response future
pub struct ResponseFuture<F, C> {
    inner: tracing::instrument::Instrumented<F>,
    customization: C,
    req_id: Option<(&'static str, RequestId)>,
}

impl<F: Future<Output = ohkami::Response>, C: MiddlewareCustomization> Future for ResponseFuture<F, C> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, ctx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match Future::poll(unsafe { Pin::new_unchecked(&mut this.inner) }, ctx) {
            task::Poll::Ready(mut response) => {
                let span = this.inner.span();

                let _entered = span.enter();

                span.record("http.response.status_code", response.status.code());
                this.customization.on_response(span, &mut response);

                if let Some((key, value)) = &this.req_id {
                    //Make PR to introduce method to insert string directly
                    if response.headers.get(key).is_none() {
                        response.headers.set().x(key, ohkami::header::append(value.as_str().to_owned()));
                    }
                }

                task::Poll::Ready(response)
            },
            std::task::Poll::Pending => task::Poll::Pending,
        }
    }
}
