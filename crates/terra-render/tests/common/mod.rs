//! Shared by the GPU tests.
//!
//! They need a GPU adapter (a software one such as lavapipe is fine). Without one they
//! skip with a message, unless `TERRA_REQUIRE_GPU=1` (set in CI), where they fail.

use terra_render::Gpu;

/// Set to `1` to make a missing GPU adapter a test failure instead of a skip.
const REQUIRE_GPU_VAR: &str = "TERRA_REQUIRE_GPU";

/// The headless GPU, or `None` (after saying why) if there is none and it isn't required.
pub(crate) fn gpu_or_skip() -> Option<Gpu> {
    match pollster::block_on(Gpu::headless()) {
        Ok(gpu) => Some(gpu),
        Err(error) if std::env::var(REQUIRE_GPU_VAR).is_ok_and(|v| v == "1") => {
            panic!("{REQUIRE_GPU_VAR}=1 but there is no GPU: {error}")
        }
        Err(error) => {
            eprintln!("skipping GPU test, no GPU ({error}); set {REQUIRE_GPU_VAR}=1 to fail");
            None
        }
    }
}
