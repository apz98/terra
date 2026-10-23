//! Shared foundation for every Terra crate.
//!
//! `terra-core` is the bottom layer of the workspace: it depends on no other
//! `terra-*` crate. It will hold coordinates, ids, math re-exports, diagnostics
//! (logging, crash logs, profiling), config and common errors. See
//! `docs/ARCHITECTURE.md` §2 for the layering rules.
