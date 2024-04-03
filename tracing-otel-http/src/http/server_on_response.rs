use http::Response;
use std::fmt;
use std::fmt::Debug;
use std::time::Duration;
use tower_http::trace::OnResponse;
use tower_http::LatencyUnit;
use tracing::{Level, Span};

enum OpenTelemetryStatusCode {
    Ok,
    Error,
}

impl Debug for OpenTelemetryStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenTelemetryStatusCode::Ok => write!(f, "OK"),
            OpenTelemetryStatusCode::Error => write!(f, "ERROR"),
        }
    }
}

impl<B> From<&Response<B>> for OpenTelemetryStatusCode {
    fn from(response: &Response<B>) -> Self {
        if response.status().is_server_error() {
            OpenTelemetryStatusCode::Error
        } else {
            OpenTelemetryStatusCode::Ok
        }
    }
}

struct Latency {
    unit: LatencyUnit,
    duration: Duration,
}

impl fmt::Display for Latency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unit {
            LatencyUnit::Seconds => write!(f, "{} s", self.duration.as_secs_f64()),
            LatencyUnit::Millis => write!(f, "{} ms", self.duration.as_millis()),
            LatencyUnit::Micros => write!(f, "{} μs", self.duration.as_micros()),
            LatencyUnit::Nanos => write!(f, "{} ns", self.duration.as_nanos()),
            _ => write!(f, "{} ms", self.duration.as_millis()),
        }
    }
}

/// Custom [`OnResponse`] implementation used by [`Trace`].
///
/// [`Trace`]: super::Trace
#[derive(Clone, Debug)]
pub struct ServerOnResponse {
    level: Level,
    latency_unit: LatencyUnit,
    include_headers: bool,
}

impl Default for ServerOnResponse {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            latency_unit: LatencyUnit::Millis,
            include_headers: false,
        }
    }
}

impl ServerOnResponse {
    /// Create a new `ServerOnResponse`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the [`Level`] used for [tracing events].
    ///
    /// Please note that while this will set the level for the tracing events
    /// themselves, it might cause them to lack expected information, like
    /// request method or path. You can address this using
    /// [`DefaultMakeSpan::level`].
    ///
    /// Defaults to [`Level::DEBUG`].
    ///
    /// [tracing events]: https://docs.rs/tracing/latest/tracing/#events
    /// [`ServerMakeSpan::level`]: crate::trace::DefaultMakeSpan::level
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Set the [`LatencyUnit`] latencies will be reported in.
    ///
    /// Defaults to [`LatencyUnit::Millis`].
    pub fn latency_unit(mut self, latency_unit: LatencyUnit) -> Self {
        self.latency_unit = latency_unit;
        self
    }

    /// Include response headers on the [`Event`].
    ///
    /// By default, headers are not included.
    ///
    /// [`Event`]: tracing::Event
    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.include_headers = include_headers;
        self
    }
}

impl<B> OnResponse<B> for ServerOnResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        let latency = Latency {
            unit: self.latency_unit,
            duration: latency,
        };
        let response_headers = self
            .include_headers
            .then(|| tracing::field::debug(response.headers()));

        span.record(
            "otel.status_code",
            tracing::field::debug(OpenTelemetryStatusCode::from(response)),
        );
        span.record("status", status(response));
        span.record("http.status_code", &response.status().as_u16());

        tracing::event!(
            Level::INFO,
            %latency,
            response_headers,
            "finished processing request"
        );
    }
}

fn status<B>(res: &Response<B>) -> Option<i32> {
    Some(res.status().as_u16().into())
}
