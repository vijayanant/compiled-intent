use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Initializes the telemetry stack.
/// Per ADR 002: All logs MUST be routed to `stderr` so they do not 
/// corrupt the `stdout` JSON-RPC stream used by the MCP protocol.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false)
                .with_thread_ids(false)
                .compact(),
        )
        .init();
}
