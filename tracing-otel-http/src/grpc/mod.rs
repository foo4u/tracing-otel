use hyper::service::Service;
use hyper::{Body, HeaderMap};
use opentelemetry::propagation::Extractor;
use std::pin::Pin;
use std::task::{Context, Poll};
use tonic::body::BoxBody;
use tower::Layer;
use tracing::{info_span, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone, Default)]
pub struct TelemetryLayer;

impl<S> Layer<S> for TelemetryLayer {
    type Service = TelemetryMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        TelemetryMiddleware { inner: service }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryMiddleware<S> {
    inner: S,
}

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl<S> Service<hyper::Request<Body>> for TelemetryMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: hyper::Request<Body>) -> Self::Future {
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let span = create_server_span(req.headers().clone());

            match inner.call(req).instrument(span.clone()).await {
                Ok(response) => {
                    span.record("otel.status_code", "ok");
                    span.record(
                        "http.response.status_code",
                        tracing::field::debug(response.status()),
                    );
                    Ok(response)
                }
                Err(err) => {
                    span.record("otel.status_code", "error");
                    Err(err)
                }
            }
        })
    }
}

fn create_server_span(header_map: HeaderMap) -> Span {
    let span = info_span!(
        "tonic",
        otel.kind = "server",
        otel.status_code = tracing::field::Empty,
        http.response.status_code = tracing::field::Empty,
        foo = tracing::field::Empty,
    );
    let ctx = TonicPropagationContext::new(header_map);

    span.set_parent(ctx.extract());

    span
}

// Can't use the http one until Tonic upgrades to hyper 1.x
struct TonicPropagationContext(HeaderMap);

impl TonicPropagationContext {
    pub fn new(header_map: HeaderMap) -> Self {
        Self(header_map)
    }

    pub fn extract(&self) -> opentelemetry::Context {
        opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(self))
    }
}

impl Extractor for TonicPropagationContext {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
