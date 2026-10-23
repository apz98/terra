//! Crash reports: a panic hook that writes what happened to a file.

use std::backtrace::Backtrace;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::panic::{self, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::thread;

use jiff::Timestamp;

/// Name of the crash report folder inside Terra's data directory.
pub const CRASH_DIR_NAME: &str = "crash-reports";

/// How many `-2`, `-3` … suffixes to try when reports land in the same second.
const MAX_NAME_ATTEMPTS: u32 = 100;

/// Where crash reports go by default: `<data dir>/crash-reports`.
///
/// The data directory is the platform's per-user data folder for Terra
/// (`~/.local/share/terra` on Linux, `%APPDATA%\Terra\data` on Windows). If the
/// platform has none (no home directory), the system temp folder is used instead.
pub fn default_crash_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "Terra")
        .map_or_else(
            || std::env::temp_dir().join("terra"),
            |dirs| dirs.data_dir().to_path_buf(),
        )
        .join(CRASH_DIR_NAME)
}

/// Installs a panic hook that writes a crash report into `dir` (created if missing).
///
/// The report holds the time, `version`, OS, thread, panic message and location,
/// and a full backtrace (captured whatever `RUST_BACKTRACE` says). The hook then
/// logs the report's path (to stderr if logging isn't initialised) and runs the
/// previously installed hook, so the usual panic message still appears. It also
/// runs before the process aborts in release builds (`panic = "abort"`).
///
/// Call it once, from the binary's `main`, after [`init_logging`](super::init_logging).
/// Each call stacks another hook.
pub fn install_crash_handler(version: &'static str, dir: PathBuf) {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let report = CrashReport::capture(version, info);
        let logging = tracing::dispatcher::has_been_set();
        match report.write_to(&dir) {
            Ok(path) if logging => {
                tracing::error!(path = %path.display(), "crash report written");
            }
            Ok(path) => eprintln!("Terra crashed. Crash report: {}", path.display()),
            Err(err) if logging => {
                tracing::error!(dir = %dir.display(), "could not write crash report: {err}");
            }
            Err(err) => eprintln!(
                "Terra crashed. Could not write a crash report to {}: {err}",
                dir.display()
            ),
        }
        previous(info);
    }));
}

/// Everything a crash report records.
#[derive(Debug)]
struct CrashReport {
    time: Timestamp,
    version: &'static str,
    thread: String,
    message: String,
    location: String,
    backtrace: String,
}

impl CrashReport {
    fn capture(version: &'static str, info: &PanicHookInfo<'_>) -> Self {
        let current = thread::current();
        Self {
            time: Timestamp::now(),
            version,
            thread: current.name().unwrap_or("<unnamed>").to_owned(),
            message: info
                .payload_as_str()
                .unwrap_or("<non-string panic payload>")
                .to_owned(),
            location: info
                .location()
                .map_or_else(|| "<unknown>".to_owned(), ToString::to_string),
            backtrace: Backtrace::force_capture().to_string(),
        }
    }

    fn render(&self) -> String {
        let mut text = String::new();
        // Writing to a `String` cannot fail.
        let _ = writeln!(text, "Terra crash report");
        let _ = writeln!(text);
        let _ = writeln!(text, "time:     {:.3}", self.time);
        let _ = writeln!(text, "version:  {}", self.version);
        let _ = writeln!(
            text,
            "os:       {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        let _ = writeln!(text, "thread:   {}", self.thread);
        let _ = writeln!(text, "location: {}", self.location);
        let _ = writeln!(text, "message:  {}", self.message);
        let _ = writeln!(text);
        let _ = writeln!(text, "backtrace:");
        let _ = writeln!(text, "{}", self.backtrace);
        text
    }

    /// Writes the report to a new file in `dir` and returns its path. Never
    /// overwrites: a name already taken gets a `-2`, `-3` … suffix.
    fn write_to(&self, dir: &Path) -> io::Result<PathBuf> {
        fs::create_dir_all(dir)?;
        // No `:` in file names, for Windows.
        let stem = format!("crash-{}", self.time.strftime("%Y-%m-%dT%H-%M-%SZ"));
        let text = self.render();
        for attempt in 1..=MAX_NAME_ATTEMPTS {
            let name = if attempt == 1 {
                format!("{stem}.log")
            } else {
                format!("{stem}-{attempt}.log")
            };
            let path = dir.join(name);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    file.write_all(text.as_bytes())?;
                    return Ok(path);
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
                Err(err) => return Err(err),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{MAX_NAME_ATTEMPTS} crash reports named {stem}* already exist"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> CrashReport {
        CrashReport {
            time: "2026-10-22T12:30:05.123456Z".parse().unwrap(),
            version: "1.2.3",
            thread: "main".to_owned(),
            message: "boom".to_owned(),
            location: "src/main.rs:10:5".to_owned(),
            backtrace: "   0: terra::main".to_owned(),
        }
    }

    #[test]
    fn render_lists_every_field() {
        let text = report().render();
        assert!(
            text.contains("time:     2026-10-22T12:30:05.123Z"),
            "{text}"
        );
        assert!(text.contains("version:  1.2.3"), "{text}");
        assert!(text.contains(&format!("os:       {}", std::env::consts::OS)));
        assert!(text.contains("thread:   main"), "{text}");
        assert!(text.contains("location: src/main.rs:10:5"), "{text}");
        assert!(text.contains("message:  boom"), "{text}");
        assert!(text.contains("backtrace:\n   0: terra::main"), "{text}");
    }

    #[test]
    fn write_creates_dir_and_names_file_by_time() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested").join(CRASH_DIR_NAME);
        let path = report().write_to(&dir).unwrap();
        assert_eq!(path, dir.join("crash-2026-10-22T12-30-05Z.log"));
        assert_eq!(fs::read_to_string(&path).unwrap(), report().render());
    }

    #[test]
    fn write_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let first = report().write_to(tmp.path()).unwrap();
        let second = report().write_to(tmp.path()).unwrap();
        let third = report().write_to(tmp.path()).unwrap();
        assert_eq!(second, tmp.path().join("crash-2026-10-22T12-30-05Z-2.log"));
        assert_eq!(third, tmp.path().join("crash-2026-10-22T12-30-05Z-3.log"));
        assert!(first.exists());
    }

    #[test]
    fn write_into_a_file_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("not-a-dir");
        fs::write(&file, "").unwrap();
        assert!(report().write_to(&file).is_err());
    }

    #[test]
    fn default_dir_ends_in_crash_reports() {
        assert!(default_crash_dir().ends_with(CRASH_DIR_NAME));
    }
}
