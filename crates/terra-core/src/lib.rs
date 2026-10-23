//! Shared foundation for every Terra crate.
//!
//! `terra-core` is the bottom layer of the workspace: it depends on no other
//! `terra-*` crate. It holds [`diagnostics`] (logging, crash reports) and will
//! hold coordinates, ids, math re-exports, config and common errors. See
//! `docs/ARCHITECTURE.md` §2 for the layering rules.

pub mod diagnostics;
