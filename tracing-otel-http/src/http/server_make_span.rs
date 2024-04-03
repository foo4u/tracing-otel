use http::Request;
use tower_http::trace::MakeSpan;
use tracing::{Level, Span};

#[derive(Debug, Clone)]
pub struct MakeServerSpan {
    level: Level,
    component: String,
    include_headers: bool,
}

impl MakeServerSpan {
    /// Create a new `DefaultMakeSpan`.
    pub fn new() -> Self {
        Self {
            level: Level::DEBUG,
            component: "tower.request".to_string(),
            include_headers: false,
        }
    }

    /// Set the [`Level`] used for the [tracing span].
    ///
    /// Defaults to [`Level::DEBUG`].
    ///
    /// [tracing span]: https://docs.rs/tracing/latest/tracing/#spans
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    pub fn component(mut self, component: &str) -> Self {
        self.component = component.to_string();
        self
    }

    /// Include request headers on the [`Span`].
    ///
    /// By default, headers are not included.
    ///
    /// [`Span`]: tracing::Span
    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.include_headers = include_headers;
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
                    http.method = %request.method(),
                    http.request.method = %request.method(), // OTEL required
                    http.route = %request.uri().path(),
                    http.path_group = tracing::field::Empty,
                    http.status_code = tracing::field::Empty,
                    server.addresss = hostname,
                    server.port = port,
                )
            }
        }

        match self.level {
            Level::ERROR => make_span!(Level::ERROR),
            Level::WARN => make_span!(Level::WARN),
            Level::INFO => make_span!(Level::INFO),
            Level::DEBUG => make_span!(Level::DEBUG),
            Level::TRACE => make_span!(Level::TRACE),
        }
    }
}
