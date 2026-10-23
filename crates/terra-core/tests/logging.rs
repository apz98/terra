//! `init_logging` installs a process-global subscriber, so it gets its own test
//! binary (each file in `tests/` runs as a separate process).

use std::thread;

use terra_core::diagnostics::init_logging;

#[test]
fn init_logging_is_idempotent() {
    assert!(!tracing::dispatcher::has_been_set());
    init_logging().expect("first init installs the subscriber");
    assert!(tracing::dispatcher::has_been_set());
    init_logging().expect("second init is a no-op");

    let handles: Vec<_> = (0..8).map(|_| thread::spawn(init_logging)).collect();
    for handle in handles {
        handle
            .join()
            .expect("init thread panicked")
            .expect("concurrent init is a no-op");
    }
    tracing::info!("logging works after repeated init");
}
