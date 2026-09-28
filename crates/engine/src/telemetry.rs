use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Resolves the default tracing filter expression based on verbosity and config.
fn resolve_filter_directive(verbose: bool, config_level: Option<&str>) -> &'static str {
    if verbose {
        "warn,compiled_intent=debug,intent=debug"
    } else {
        match config_level {
            Some("debug") => "warn,compiled_intent=debug,intent=debug",
            Some("info") => "warn,compiled_intent=info,intent=info",
            Some("error") => "error",
            _ => "warn",
        }
    }
}

/// Initializes the telemetry stack.
/// Per ADR 005: All logs MUST be routed to `stderr` so they do not
/// corrupt the `stdout` JSON-RPC stream used by the MCP protocol.
pub fn init_tracing(verbose: bool, config_level: Option<&str>) {
    let directive = resolve_filter_directive(verbose, config_level);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(directive));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false)
                .with_thread_ids(false)
                .compact(),
        )
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_filter_directive() {
        assert_eq!(
            resolve_filter_directive(true, None),
            "warn,compiled_intent=debug,intent=debug"
        );
        assert_eq!(
            resolve_filter_directive(false, Some("debug")),
            "warn,compiled_intent=debug,intent=debug"
        );
        assert_eq!(
            resolve_filter_directive(false, Some("info")),
            "warn,compiled_intent=info,intent=info"
        );
        assert_eq!(resolve_filter_directive(false, Some("error")), "error");
        assert_eq!(resolve_filter_directive(false, None), "warn");
    }
}
