//! Diagnostics: logging, crash reports and the hooks the profiler needs.
//!
//! Binaries call [`init_logging`] first thing in `main`, then
//! [`install_crash_handler`]. Libraries only emit `tracing` events and spans; they
//! never install anything global. See `docs/ARCHITECTURE.md` §8.

mod crash;
mod logging;

pub use crash::{CRASH_DIR_NAME, default_crash_dir, install_crash_handler};
pub use logging::{DEFAULT_LOG_FILTER, LOG_ENV_VAR, LoggingError, init_logging};
