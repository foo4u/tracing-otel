use crate::http::http::HttpVersion;
use http::{HeaderMap, Request};
use opentelemetry::propagation::Extractor;
use tower_http::trace::MakeSpan;
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Creates a new span for an incoming HTTP request.
///
/// Trace span fields are compliant with the [OpenTelemetry HTTP] specification.
///
/// [OpenTelemetry HTTP]: https://opentelemetry.io/docs/specs/semconv/http/http-spans/
#[derive(Debug, Clone)]
pub struct MakeServerSpan {
    level: Level,
    component: String,
    include_headers: bool,
    propagate_context: bool,
}

/// Foober
impl MakeServerSpan {
    /// Create a new `DefaultMakeSpan`.
    pub fn new() -> Self {
        Self {
            level: Level::DEBUG,
            component: "tower.request".to_string(),
            include_headers: false,
            propagate_context: true,
        }
    }

    /// Set the [`Level`] used for the [`Span`].
    ///
    /// Defaults to [`Level::DEBUG`].
    ///
    /// [tracing span]: https://docs.rs/tracing/latest/tracing/#spans
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Sets the component name used for the [`Span`].
    ///
    /// Defaults to `tower.request`.
    ///
    /// [`Span`]: Span
    pub fn component(mut self, component: &str) -> Self {
        self.component = component.to_string();
        self
    }

    /// Include request headers on the [`Span`].
    ///
    /// By default, headers are not included.
    ///
    /// [`Span`]: Span
    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.include_headers = include_headers;
        self
    }

    /// Set the [`Span`] parent context from incoming headers.
    ///
    /// By default, the parent context is set from incoming headers.
    ///
    /// [`Span`]: Span
    pub fn propagate_context(mut self, propagate_context: bool) -> Self {
        self.propagate_context = propagate_context;
        self
    }
}

impl Default for MakeServerSpan {
    fn default() -> Self {
        Self::new()
    }
}

impl<B> MakeSpan<B> for MakeServerSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let (hostname, port) = if let Some(host) = request.uri().host() {
            let port = request.uri().port_u16().unwrap_or(80);
            (host, port)
        } else if let Some(host) = request.headers().get(http::header::HOST) {
            let host = host.to_str().unwrap_or("unknown:80");
            let hostname = host.split(':').nth(0).unwrap_or("unknown");
            let port = host.split(':').nth(1).unwrap_or("80").parse().unwrap_or(80);
            (hostname, port)
        } else {
            ("unknown", 0)
        };

        let http_version: HttpVersion = request.version().into();
        let binding = http::header::HeaderValue::from_static("");
        let user_agent = request
            .headers()
            .get(http::header::USER_AGENT)
            .unwrap_or(&binding);

        // FIXME: maybe populate headers if requested on request handler instead of here

        // This ugly macro is needed, unfortunately, because `tracing::span!`
        // required the level argument to be static. Meaning we can't just pass
        // `self.level`.
        macro_rules! make_span {
            ($level:expr) => {
                tracing::span!(
                    $level,
                    "request",
                    component = %self.component,
                    method = %request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                    headers = tracing::field::Empty,
                    otel.kind = "server",
                    otel.status_code = tracing::field::Empty,
                    http.host = hostname,
                    http.request.method = %request.method(), // OTEL required
                    http.route = %request.uri().path(),
                    http.path_group = tracing::field::Empty,
                    http.response.status_code = tracing::field::Empty,
                    network.protocol.name = http_version.protocol,
                    network.protocol.version = http_version.version,
                    network.transport = "tcp",
                    server.addresss = hostname,
                    server.port = port,
                    telemetry.sdk.language = "rust",
                    url.scheme = %request.uri().scheme_str().unwrap_or("httx"),
                    url.path = %request.uri().path(),
                    url.query = %request.uri().query().unwrap_or(""),
                    user_agent.original = tracing::field::debug(user_agent),
                )
            }
        }

        let span = match self.level {
            Level::ERROR => make_span!(Level::ERROR),
            Level::WARN => make_span!(Level::WARN),
            Level::INFO => make_span!(Level::INFO),
            Level::DEBUG => make_span!(Level::DEBUG),
            Level::TRACE => make_span!(Level::TRACE),
        };

        if self.propagate_context {
            let ctx = PropagationContext::new(request.headers().clone());
            span.set_parent(ctx.extract());
        }

        span
    }
}

struct PropagationContext(HeaderMap);

impl PropagationContext {
    pub fn new(header_map: HeaderMap) -> Self {
        Self(header_map)
    }

    pub fn extract(&self) -> opentelemetry::Context {
        opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(self))
    }
}

impl Extractor for PropagationContext {
    fn get(&self, key: &str) -> Option<&str> {
        tracing::warn!("Extracting {key}");
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
