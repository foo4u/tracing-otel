mod http;
pub mod server_make_span;
pub mod server_on_response;

/// [`Layer`] that adds Open Telemetry compliant HTTP [tracing] to a [`Service`].
///
/// See the [module docs](crate::trace) for more details.
///
/// [`Layer`]: tower_layer::Layer
/// [tracing]: https://crates.io/crates/tracing
/// [`Service`]: tower_service::Service
pub type HttpTraceLayer = tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    server_make_span::MakeServerSpan,
    tower_http::trace::DefaultOnRequest,
    server_on_response::ServerOnResponse,
>;
