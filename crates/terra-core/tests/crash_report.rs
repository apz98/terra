//! `install_crash_handler` replaces the process-global panic hook, so it gets its
//! own test binary (each file in `tests/` runs as a separate process).

use std::fs;
use std::thread;

use terra_core::diagnostics::{init_logging, install_crash_handler};

#[test]
fn forced_panic_writes_crash_report() {
    init_logging().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("crash-reports");
    install_crash_handler("9.8.7", dir.clone());

    let result = thread::Builder::new()
        .name("doomed".to_owned())
        .spawn(|| panic!("forced test panic"))
        .unwrap()
        .join();
    assert!(result.is_err(), "the thread should have panicked");

    let reports: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!(reports[0].extension().unwrap(), "log");
    let name = reports[0].file_stem().unwrap().to_str().unwrap();
    assert!(name.starts_with("crash-"), "{name}");

    let text = fs::read_to_string(&reports[0]).unwrap();
    assert!(text.starts_with("Terra crash report\n"), "{text}");
    assert!(text.contains("version:  9.8.7"), "{text}");
    assert!(text.contains("thread:   doomed"), "{text}");
    assert!(text.contains("message:  forced test panic"), "{text}");
    assert!(text.contains("crash_report.rs"), "location missing: {text}");
    assert!(text.contains("\nbacktrace:\n"), "{text}");
    assert!(
        text.contains("forced_panic_writes_crash_report"),
        "no frames: {text}"
    );
}
