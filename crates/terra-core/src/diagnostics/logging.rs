//! The global `tracing` subscriber.

use std::io::{self, IsTerminal};
use std::sync::OnceLock;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::{Directive, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Environment variable holding the log filter, in `tracing` `EnvFilter` syntax
/// (e.g. `debug`, or `info,terra_render=trace`).
pub const LOG_ENV_VAR: &str = "TERRA_LOG";

/// Filter used when [`LOG_ENV_VAR`] is unset, empty, or has no valid directive.
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
/// The filter comes from [`LOG_ENV_VAR`] (default [`DEFAULT_LOG_FILTER`]).
/// Invalid directives are dropped with a warning and the valid ones still apply;
/// if none is valid, the default is used. Debug builds use
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
    let (filter, invalid) = build_filter(std::env::var(LOG_ENV_VAR).ok().as_deref());
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

    if !invalid.is_empty() {
        tracing::warn!(
            "ignoring invalid directives in {LOG_ENV_VAR}: {}. The valid ones still apply \
             (`{DEFAULT_LOG_FILTER}` if none is left).",
            invalid.join("; ")
        );
    }
    Ok(())
}

/// Parses a [`LOG_ENV_VAR`] value: comma-separated directives, as `EnvFilter` splits
/// them. The valid ones form the filter (the default filter if there are none); each
/// invalid one is dropped and described in the returned list, for the warning.
///
/// Directives are parsed one by one rather than with `EnvFilter::parse_lossy`, which
/// prints its own unformatted line to stderr for every invalid directive.
fn build_filter(value: Option<&str>) -> (EnvFilter, Vec<String>) {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    let directives = value.unwrap_or_default().split(',').map(str::trim);
    for directive in directives.filter(|d| !d.is_empty()) {
        match directive.parse::<Directive>() {
            Ok(_) => valid.push(directive),
            Err(err) => invalid.push(format!("`{directive}` ({err})")),
        }
    }
    let builder = || EnvFilter::builder().with_default_directive(DEFAULT_LOG_FILTER.into());
    // Every directive left parsed on its own, so the strict parse succeeds; the lossy
    // fallback (just the default, as nothing is left to drop) only covers the impossible.
    let filter = builder()
        .parse(valid.join(","))
        .unwrap_or_else(|_| builder().parse_lossy(""));
    (filter, invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_filter_uses_default() {
        let (filter, invalid) = build_filter(None);
        assert_eq!(filter.to_string(), "info");
        assert_eq!(invalid, Vec::<String>::new());
    }

    #[test]
    fn empty_filter_uses_default() {
        let (filter, invalid) = build_filter(Some(""));
        assert_eq!(filter.to_string(), "info");
        assert_eq!(invalid, Vec::<String>::new());
    }

    #[test]
    fn valid_filter_is_used() {
        let (filter, invalid) = build_filter(Some("warn,terra_core=trace"));
        assert!(invalid.is_empty(), "{invalid:?}");
        let text = filter.to_string();
        assert!(text.contains("terra_core=trace"), "{text}");
        assert!(text.contains("warn"), "{text}");
    }

    #[test]
    fn invalid_filter_falls_back_with_error() {
        let (filter, invalid) = build_filter(Some("terra_core=loud"));
        assert_eq!(filter.to_string(), "info");
        assert_eq!(invalid.len(), 1);
        assert!(invalid[0].starts_with("`terra_core=loud` ("), "{invalid:?}");
    }

    #[test]
    fn invalid_directives_are_dropped_and_valid_ones_kept() {
        let (filter, invalid) = build_filter(Some("terra_core=loud, warn,terra_render=trace,=x"));
        assert_eq!(invalid.len(), 2, "{invalid:?}");
        assert!(invalid[1].starts_with("`=x` ("), "{invalid:?}");
        let text = filter.to_string();
        assert!(text.contains("terra_render=trace"), "{text}");
        assert!(text.contains("warn"), "{text}");
        assert!(!text.contains("terra_core"), "{text}");
    }
}
