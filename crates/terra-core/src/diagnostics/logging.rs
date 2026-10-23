//! The global `tracing` subscriber.

use std::io::{self, IsTerminal};
use std::sync::OnceLock;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::{FromEnvError, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Environment variable holding the log filter, in `tracing` `EnvFilter` syntax
/// (e.g. `debug`, or `info,terra_render=trace`).
pub const LOG_ENV_VAR: &str = "TERRA_LOG";

/// Filter used when [`LOG_ENV_VAR`] is unset or invalid.
pub const DEFAULT_LOG_FILTER: LevelFilter = LevelFilter::INFO;

/// Why [`init_logging`] could not install Terra's subscriber.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LoggingError {
    /// Something other than [`init_logging`] already set the global subscriber
    /// (or the `log` crate's global logger).
    #[error("a global logger is already installed: {0}")]
    AlreadySet(String),
}

/// Outcome of the first [`init_logging`] call, returned again by later calls.
static INIT: OnceLock<Result<(), LoggingError>> = OnceLock::new();

/// Installs the global `tracing` subscriber, writing to stderr.
///
/// The filter comes from [`LOG_ENV_VAR`] (default [`DEFAULT_LOG_FILTER`]; an
/// invalid value falls back to the default and logs a warning). Debug builds use
/// the multi-line `pretty` format, release builds a compact one; colours only
/// when stderr is a terminal. Events from crates using the `log` facade (wgpu,
/// winit) are forwarded too.
///
/// Idempotent and thread-safe: only the first call installs anything, and every
/// call returns that first call's result.
pub fn init_logging() -> Result<(), LoggingError> {
    INIT.get_or_init(install).clone()
}

fn install() -> Result<(), LoggingError> {
    let (filter, filter_error) = build_filter(std::env::var(LOG_ENV_VAR).ok().as_deref());
    let output = fmt::layer()
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal());
    let registry = tracing_subscriber::registry().with(filter);
    let result = if cfg!(debug_assertions) {
        registry.with(output.pretty()).try_init()
    } else {
        registry.with(output.compact()).try_init()
    };
    result.map_err(|err| LoggingError::AlreadySet(err.to_string()))?;

    if let Some(err) = filter_error {
        tracing::warn!(
            "ignoring invalid {LOG_ENV_VAR}: {err}; using `{DEFAULT_LOG_FILTER}` instead"
        );
    }
    Ok(())
}

/// Parses a [`LOG_ENV_VAR`] value. `None` or an empty value gives the default
/// filter; an invalid value gives the default filter plus the parse error.
fn build_filter(value: Option<&str>) -> (EnvFilter, Option<FromEnvError>) {
    let builder = || EnvFilter::builder().with_default_directive(DEFAULT_LOG_FILTER.into());
    match value {
        None => (builder().parse_lossy(""), None),
        Some(value) => match builder().parse(value) {
            Ok(filter) => (filter, None),
            Err(err) => (builder().parse_lossy(""), Some(err.into())),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_filter_uses_default() {
        let (filter, err) = build_filter(None);
        assert_eq!(filter.to_string(), "info");
        assert!(err.is_none());
    }

    #[test]
    fn empty_filter_uses_default() {
        let (filter, err) = build_filter(Some(""));
        assert_eq!(filter.to_string(), "info");
        assert!(err.is_none());
    }

    #[test]
    fn valid_filter_is_used() {
        let (filter, err) = build_filter(Some("warn,terra_core=trace"));
        assert!(err.is_none());
        let text = filter.to_string();
        assert!(text.contains("terra_core=trace"), "{text}");
        assert!(text.contains("warn"), "{text}");
    }

    #[test]
    fn invalid_filter_falls_back_with_error() {
        let (filter, err) = build_filter(Some("terra_core=loud"));
        assert_eq!(filter.to_string(), "info");
        assert!(err.is_some());
    }
}
