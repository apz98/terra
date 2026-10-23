//! Crash reports: a panic hook that writes what happened to a file.

use std::backtrace::Backtrace;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::panic::{self, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::thread;

use etcetera::app_strategy::{AppStrategy, AppStrategyArgs, choose_native_strategy};
use jiff::Timestamp;

/// Name of the crash report folder inside Terra's data directory.
pub const CRASH_DIR_NAME: &str = "crash-reports";

/// How many `-2`, `-3` … suffixes to try when reports land in the same second.
const MAX_NAME_ATTEMPTS: u32 = 100;

/// Where crash reports go by default: `<data dir>/crash-reports`.
///
/// The data directory is the platform's per-user data folder for Terra
/// (`$XDG_DATA_HOME/terra`, by default `~/.local/share/terra`, on Linux;
/// `%APPDATA%\Terra\data` on Windows). If the platform has none (no home directory),
/// `crash-reports` in the working directory is used instead. Never a shared folder such
/// as `/tmp`, where another user could create the directory first.
pub fn default_crash_dir() -> PathBuf {
    let terra = AppStrategyArgs {
        app_name: "Terra".to_owned(),
        ..AppStrategyArgs::default()
    };
    crash_dir(
        choose_native_strategy(terra)
            .ok()
            .map(|dirs| dirs.data_dir()),
    )
}

/// `<data_dir>/crash-reports`, or `<working dir>/crash-reports` without a data dir.
fn crash_dir(data_dir: Option<PathBuf>) -> PathBuf {
    // An unreadable working directory leaves a relative path, which still means it.
    data_dir
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join(CRASH_DIR_NAME)
}

/// Installs a panic hook that writes a crash report into `dir` (created if missing).
///
/// The report holds the time, `version`, OS, thread, panic message and location,
/// and a full backtrace (captured whatever `RUST_BACKTRACE` says). The hook then
/// prints the report's path (or why it couldn't be written) to stderr, also logs it
/// if logging is initialised, and runs the previously installed hook, so the usual
/// panic message still appears. The stderr line doesn't depend on the log filter,
/// so `TERRA_LOG=off` can't hide where the report went. The hook also runs before
/// the process aborts in release builds (`panic = "abort"`).
///
/// Call it once, from the binary's `main`, after [`init_logging`](super::init_logging).
/// Each call stacks another hook.
pub fn install_crash_handler(version: &'static str, dir: PathBuf) {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let written = CrashReport::capture(version, info).write_to(&dir);
        let logging = tracing::dispatcher::has_been_set();
        announce(&written, &dir, logging, &mut io::stderr());
        previous(info);
    }));
}

/// Tells the player where the crash report went: always a line on `out`, plus an
/// `error` event if `logging` (a global subscriber is installed). Write errors on `out` are ignored;
/// there's nowhere left to report them.
fn announce(written: &io::Result<PathBuf>, dir: &Path, logging: bool, out: &mut impl io::Write) {
    let _ = match written {
        Ok(path) => writeln!(out, "Terra crashed. Crash report: {}", path.display()),
        Err(err) => writeln!(
            out,
            "Terra crashed. Could not write a crash report to {}: {err}",
            dir.display()
        ),
    };
    if logging {
        match written {
            Ok(path) => tracing::error!(path = %path.display(), "crash report written"),
            Err(err) => {
                tracing::error!(dir = %dir.display(), "could not write crash report: {err}");
            }
        }
    }
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

    /// The XDG location, as documented (and as `directories` gave before 0.1.2).
    #[cfg(target_os = "linux")]
    #[test]
    fn default_dir_on_linux_is_xdg_data_home() {
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/share")
            });
        assert_eq!(
            default_crash_dir(),
            data_home.join("terra").join(CRASH_DIR_NAME)
        );
    }

    #[test]
    fn crash_dir_uses_the_data_dir() {
        let data = PathBuf::from("/home/someone/.local/share/terra");
        assert_eq!(crash_dir(Some(data.clone())), data.join(CRASH_DIR_NAME));
    }

    #[test]
    fn crash_dir_without_data_dir_is_the_working_dir_not_temp() {
        let dir = crash_dir(None);
        assert_eq!(dir, std::env::current_dir().unwrap().join(CRASH_DIR_NAME));
        assert!(!dir.starts_with(std::env::temp_dir()), "{}", dir.display());
    }

    #[test]
    fn announce_prints_the_path() {
        let mut out = Vec::new();
        let path = PathBuf::from("/data/crash-reports/crash-1.log");
        announce(&Ok(path), Path::new("/data/crash-reports"), false, &mut out);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "Terra crashed. Crash report: /data/crash-reports/crash-1.log\n"
        );
    }

    #[test]
    fn announce_prints_the_write_error() {
        let mut out = Vec::new();
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "read-only");
        announce(&Err(err), Path::new("/data/crash-reports"), false, &mut out);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "Terra crashed. Could not write a crash report to /data/crash-reports: read-only\n"
        );
    }

    #[test]
    fn announce_prints_even_when_logging_is_on() {
        use tracing_subscriber::prelude::*;

        // A subscriber that drops every event, like `TERRA_LOG=off`.
        let subscriber =
            tracing_subscriber::registry().with(tracing_subscriber::filter::LevelFilter::OFF);
        tracing::subscriber::with_default(subscriber, || {
            let mut out = Vec::new();
            announce(
                &Ok(PathBuf::from("crash-1.log")),
                Path::new("."),
                true,
                &mut out,
            );
            assert_eq!(
                String::from_utf8(out).unwrap(),
                "Terra crashed. Crash report: crash-1.log\n"
            );
        });
    }
}
