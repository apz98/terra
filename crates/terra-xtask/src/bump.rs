//! `cargo xtask bump`: per-crate `SemVer` bumps with the cascade from `docs/VERSIONING.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

use anyhow::{Context, Result, bail, ensure};
use semver::Version;

use crate::workspace::Workspace;

/// Internal tool crates: bumped when asked, but never bumped by a cascade and never
/// cascading to anything (`docs/VERSIONING.md`).
const NON_CASCADING: &[&str] = &["terra-xtask"];

/// How far a version moves. Ordered so that the highest level wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    Patch,
    Minor,
    Major,
}

impl Level {
    /// The next version at this level. A pre-release whose lower parts are already zero
    /// is released instead of skipped (`0.3.0-alpha.1` + patch or minor → `0.3.0`), as
    /// cargo-release does. Build metadata is dropped.
    pub(crate) fn apply(self, version: &Version) -> Version {
        let (major, minor, patch) = (version.major, version.minor, version.patch);
        let pre = !version.pre.is_empty();
        match self {
            Self::Major if pre && minor == 0 && patch == 0 => Version::new(major, 0, 0),
            Self::Minor if pre && patch == 0 => Version::new(major, minor, 0),
            Self::Patch if pre => Version::new(major, minor, patch),
            Self::Major => Version::new(major + 1, 0, 0),
            Self::Minor => Version::new(major, minor + 1, 0),
            Self::Patch => Version::new(major, minor, patch + 1),
        }
    }
}

impl FromStr for Level {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "major" => Ok(Self::Major),
            "minor" => Ok(Self::Minor),
            "patch" => Ok(Self::Patch),
            other => bail!("unknown bump level `{other}` (expected major, minor or patch)"),
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
        })
    }
}

/// One planned version change.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Bump {
    pub(crate) name: String,
    pub(crate) level: Level,
    pub(crate) from: Version,
    pub(crate) to: Version,
    /// `true` if asked for directly, `false` if it comes only from the cascade.
    pub(crate) requested: bool,
}

/// What `bump` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    requests: Vec<(String, Level)>,
    dry_run: bool,
    /// Allow a `0.x` crate to reach `1.0.0` (a manual decision, `docs/VERSIONING.md`).
    allow_1_0: bool,
}

/// Works out every bump implied by `requests`, dependencies first.
///
/// Each requested crate bumps at the highest level asked for it; every crate that ships
/// it (directly or transitively) bumps at the highest level among the crates it ships.
/// Fails if a `0.x` crate would become `1.0.0` and `allow_1_0` is not set.
pub(crate) fn plan(
    workspace: &Workspace,
    requests: &[(String, Level)],
    allow_1_0: bool,
) -> Result<Vec<Bump>> {
    let mut levels: BTreeMap<&str, Level> = BTreeMap::new();
    for (name, level) in requests {
        let Some(package) = workspace.get(name) else {
            bail!(
                "`{name}` is not a workspace crate (known: {})",
                workspace.names().join(", ")
            );
        };
        let entry = levels.entry(package.name.as_str()).or_insert(*level);
        *entry = (*entry).max(*level);
    }
    let requested: BTreeSet<&str> = levels.keys().copied().collect();

    let dependents = workspace.dependents();
    let mut queue: Vec<&str> = levels.keys().copied().collect();
    while let Some(name) = queue.pop() {
        if NON_CASCADING.contains(&name) {
            continue;
        }
        let level = levels[name];
        for &dependent in dependents.get(name).into_iter().flatten() {
            if NON_CASCADING.contains(&dependent) {
                continue;
            }
            if levels.get(dependent).is_none_or(|&current| current < level) {
                levels.insert(dependent, level);
                queue.push(dependent);
            }
        }
    }

    let order = workspace.topological_order()?;
    let bumps: Vec<Bump> = order
        .into_iter()
        .filter_map(|name| {
            let level = *levels.get(name)?;
            let from = workspace.get(name)?.version.clone();
            Some(Bump {
                name: name.to_owned(),
                level,
                to: level.apply(&from),
                from,
                requested: requested.contains(name),
            })
        })
        .collect();

    let to_1_0: Vec<String> = bumps
        .iter()
        .filter(|bump| bump.from.major == 0 && bump.to.major > 0)
        .map(|bump| format!("{} {} → {}", bump.name, bump.from, bump.to))
        .collect();
    ensure!(
        allow_1_0 || to_1_0.is_empty(),
        "this would release 1.0 ({}). Before 1.0, breaking changes bump the minor \
         (docs/VERSIONING.md); 1.0.0 is a manual decision, so pass --allow-1.0 if it's intended",
        to_1_0.join(", ")
    );
    Ok(bumps)
}

/// Applies `bumps` to the in-memory manifests: crate versions and internal requirements.
/// Nothing is written, so an error here leaves the files untouched.
pub(crate) fn apply(workspace: &mut Workspace, bumps: &[Bump]) -> Result<()> {
    for bump in bumps {
        workspace.set_version(&bump.name, &bump.to)?;
        workspace.set_requirements(&bump.name, &bump.to)?;
    }
    Ok(())
}

/// Writes the edited manifests and refreshes `Cargo.lock` with `refresh_lock`. If any of
/// it fails, the manifests and `Cargo.lock` are put back so a retry can't bump twice.
/// A `Cargo.lock` that exists but can't be read stops the bump before anything is written.
fn commit(
    root: &Path,
    workspace: &Workspace,
    refresh_lock: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let lock_path = root.join("Cargo.lock");
    let lock = match fs::read(&lock_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "reading {} (needed to roll back a failed bump); nothing changed",
                    lock_path.display()
                )
            });
        }
    };
    let Err(error) = workspace.save().and_then(|()| refresh_lock(root)) else {
        return Ok(());
    };
    // Restore the lock even if a manifest can't be restored, and report both.
    let manifests = workspace.restore();
    let lock = match &lock {
        Some(bytes) => fs::write(&lock_path, bytes),
        None if lock_path.exists() => fs::remove_file(&lock_path),
        None => Ok(()),
    }
    .with_context(|| format!("restoring {}", lock_path.display()));
    let failures: Vec<String> = [manifests, lock]
        .into_iter()
        .filter_map(Result::err)
        .map(|failure| format!("{failure:#}"))
        .collect();
    Err(if failures.is_empty() {
        error.context("bump failed and was rolled back; nothing changed")
    } else {
        error.context(format!(
            "bump failed and the rollback failed too ({}); check `git diff`",
            failures.join("; ")
        ))
    })
}

/// Brings `Cargo.lock` in line with the new member versions.
fn cargo_update(root: &Path) -> Result<()> {
    let status = Command::new(crate::cargo())
        .args(["update", "--workspace", "--quiet"])
        .current_dir(root)
        .status()
        .context("running `cargo update --workspace`")?;
    ensure!(status.success(), "`cargo update --workspace` failed");
    Ok(())
}

/// The `Bumps:` line for the commit message (`docs/WORKFLOW.md`).
pub(crate) fn summary(bumps: &[Bump]) -> String {
    let list: Vec<String> = bumps
        .iter()
        .map(|bump| format!("{} {} → {}", bump.name, bump.from, bump.to))
        .collect();
    format!("Bumps: {}", list.join(", "))
}

/// Parses `<crate> <level> [<crate> <level> ...] [--dry-run] [--allow-1.0]`.
fn parse_args(args: &[String]) -> Result<Options> {
    let mut options = Options::default();
    let mut positional = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--dry-run" => options.dry_run = true,
            "--allow-1.0" => options.allow_1_0 = true,
            flag if flag.starts_with("--") => bail!("unknown flag `{flag}`"),
            _ => positional.push(arg),
        }
    }
    ensure!(
        !positional.is_empty() && positional.len().is_multiple_of(2),
        "expected `<crate> <major|minor|patch>` pairs, got {positional:?}"
    );
    options.requests = positional
        .chunks(2)
        .map(|pair| Ok((pair[0].clone(), pair[1].parse()?)))
        .collect::<Result<_>>()?;
    Ok(options)
}

/// Runs `cargo xtask bump`.
pub(crate) fn run(root: &Path, args: &[String]) -> Result<()> {
    let options = parse_args(args)?;
    let mut workspace = Workspace::load(root)?;
    let bumps = plan(&workspace, &options.requests, options.allow_1_0)?;
    // Apply in memory even for a dry run, so unsupported manifests fail now.
    apply(&mut workspace, &bumps)?;
    for bump in &bumps {
        let why = if bump.requested { "" } else { ", cascade" };
        println!(
            "  {:<16} {} → {} ({}{why})",
            bump.name, bump.from, bump.to, bump.level
        );
    }
    println!("{}", summary(&bumps));
    if options.dry_run {
        println!("(dry run, nothing written)");
        return Ok(());
    }
    commit(root, &workspace, cargo_update)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::fixture::{Fixture, deps, read};

    /// The example workspace from `docs/VERSIONING.md`.
    fn example() -> Fixture {
        Fixture::new(
            "",
            &[
                ("terra-core", "0.2.0", ""),
                ("terra-render", "0.3.0", &deps(&["terra-core"])),
                (
                    "terra-client",
                    "0.5.2",
                    &deps(&["terra-render", "terra-core"]),
                ),
                ("terra-sim", "0.4.0", &deps(&["terra-core"])),
                ("terra", "0.8.1", &deps(&["terra-client", "terra-sim"])),
                ("terra-server", "0.2.3", &deps(&["terra-sim"])),
                ("terra-xtask", "0.1.0", &deps(&["terra-core"])),
            ],
        )
    }

    fn req(name: &str, level: Level) -> (String, Level) {
        (name.to_owned(), level)
    }

    fn load(fixture: &Fixture) -> Workspace {
        Workspace::load(fixture.root()).unwrap()
    }

    fn plan_summary(fixture: &Fixture, requests: &[(String, Level)]) -> String {
        summary(&plan(&load(fixture), requests, false).unwrap())
    }

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn levels_follow_semver() {
        assert_eq!(Level::Patch.apply(&v("0.8.1")), v("0.8.2"));
        assert_eq!(Level::Minor.apply(&v("0.8.1")), v("0.9.0"));
        assert_eq!(Level::Major.apply(&v("0.8.1")), v("1.0.0"));
        assert_eq!(Level::Patch.apply(&v("0.8.1+build.7")), v("0.8.2"));
        assert!(Level::Major > Level::Minor && Level::Minor > Level::Patch);
    }

    #[test]
    fn pre_releases_are_released_not_skipped() {
        assert_eq!(Level::Patch.apply(&v("0.3.0-alpha.1")), v("0.3.0"));
        assert_eq!(Level::Minor.apply(&v("0.3.0-alpha.1")), v("0.3.0"));
        assert_eq!(Level::Major.apply(&v("0.3.0-alpha.1")), v("1.0.0"));
        assert_eq!(Level::Minor.apply(&v("0.3.1-alpha.1")), v("0.4.0"));
        assert_eq!(Level::Major.apply(&v("1.0.0-rc.1")), v("1.0.0"));
    }

    #[test]
    fn versioning_doc_example_cascades_to_shipping_dependents_only() {
        assert_eq!(
            plan_summary(&example(), &[req("terra-render", Level::Minor)]),
            "Bumps: terra-render 0.3.0 → 0.4.0, terra-client 0.5.2 → 0.6.0, terra 0.8.1 → 0.9.0"
        );
    }

    #[test]
    fn highest_level_wins_and_each_crate_bumps_once() {
        let bumps = plan(
            &load(&example()),
            &[
                req("terra-core", Level::Patch),
                req("terra-render", Level::Minor),
            ],
            false,
        )
        .unwrap();
        assert_eq!(
            summary(&bumps),
            "Bumps: terra-core 0.2.0 → 0.2.1, terra-render 0.3.0 → 0.4.0, \
             terra-client 0.5.2 → 0.6.0, terra-sim 0.4.0 → 0.4.1, terra 0.8.1 → 0.9.0, \
             terra-server 0.2.3 → 0.2.4"
        );
        let requested: Vec<&str> = bumps
            .iter()
            .filter(|b| b.requested)
            .map(|b| b.name.as_str())
            .collect();
        assert_eq!(requested, ["terra-core", "terra-render"]);
    }

    #[test]
    fn repeated_request_takes_the_highest_level() {
        assert_eq!(
            plan_summary(
                &example(),
                &[
                    req("terra-server", Level::Patch),
                    req("terra-server", Level::Minor)
                ]
            ),
            "Bumps: terra-server 0.2.3 → 0.3.0"
        );
    }

    #[test]
    fn xtask_never_cascades() {
        let fixture = example();
        // terra-xtask depends on terra-core but is not bumped by it ...
        let bumps = plan(&load(&fixture), &[req("terra-core", Level::Minor)], false).unwrap();
        assert!(bumps.iter().all(|b| b.name != "terra-xtask"));
        // ... and bumping it directly touches nothing else.
        assert_eq!(
            plan_summary(&fixture, &[req("terra-xtask", Level::Minor)]),
            "Bumps: terra-xtask 0.1.0 → 0.2.0"
        );
    }

    #[test]
    fn going_to_1_0_needs_an_explicit_flag() {
        let fixture = Fixture::new(
            "",
            &[("lib", "1.4.0", ""), ("app", "0.9.0", &deps(&["lib"]))],
        );
        let ws = load(&fixture);
        // Directly ...
        let err = plan(&ws, &[req("app", Level::Major)], false).unwrap_err();
        assert!(err.to_string().contains("--allow-1.0"), "{err}");
        // ... and through the cascade.
        let err = plan(&ws, &[req("lib", Level::Major)], false).unwrap_err();
        assert!(err.to_string().contains("app 0.9.0 → 1.0.0"), "{err}");
        assert_eq!(
            summary(&plan(&ws, &[req("lib", Level::Major)], true).unwrap()),
            "Bumps: lib 1.4.0 → 2.0.0, app 0.9.0 → 1.0.0"
        );
        // Bumps that stay below 1.0 need no flag.
        assert!(plan(&ws, &[req("lib", Level::Minor)], false).is_ok());
    }

    #[test]
    fn dev_dependencies_do_not_cascade_but_requirements_update() {
        let fixture = Fixture::new(
            "",
            &[
                ("lib", "0.1.0", ""),
                (
                    "user",
                    "0.1.0",
                    "[dev-dependencies]\nlib = { path = \"../lib\", version = \"0.1.0\" }\n",
                ),
            ],
        );
        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        assert_eq!(summary(&bumps), "Bumps: lib 0.1.0 → 0.2.0");
        apply(&mut ws, &bumps).unwrap();
        assert!(
            ws.manifest_text(&fixture.manifest("user"))
                .contains("lib = { path = \"../lib\", version = \"0.2.0\" }")
        );
    }

    #[test]
    fn renamed_target_and_build_dependencies_cascade_and_update() {
        let fixture = Fixture::new(
            "",
            &[
                ("lib", "0.1.0", ""),
                (
                    "a",
                    "0.1.0",
                    "[dependencies]\ncore2 = { package = \"lib\", path = \"../lib\", version = \"=0.1.0\" }\n",
                ),
                (
                    "b",
                    "0.1.0",
                    "[target.'cfg(unix)'.dependencies]\nlib = { path = \"../lib\", version = \"~0.1\" }\n",
                ),
                (
                    "c",
                    "0.1.0",
                    "[build-dependencies.lib]\npath = \"../lib\"\nversion = \"0.1.0\"\n",
                ),
            ],
        );
        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        assert_eq!(
            summary(&bumps),
            "Bumps: lib 0.1.0 → 0.2.0, a 0.1.0 → 0.2.0, b 0.1.0 → 0.2.0, c 0.1.0 → 0.2.0"
        );
        apply(&mut ws, &bumps).unwrap();
        let text = |name| ws.manifest_text(&fixture.manifest(name));
        assert!(text("a").contains("version = \"=0.2.0\""), "{}", text("a"));
        assert!(text("b").contains("version = \"~0.2\""), "{}", text("b"));
        assert!(text("c").contains("version = \"0.2.0\""), "{}", text("c"));
    }

    #[test]
    fn root_workspace_dependencies_are_updated() {
        let fixture = Fixture::new(
            "\n[workspace.dependencies]\nlib = { path = \"crates/lib\", version = \"0.1.0\" }\n",
            &[
                ("lib", "0.1.0", ""),
                ("app", "0.1.0", "[dependencies]\nlib.workspace = true\n"),
            ],
        );
        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        assert_eq!(
            summary(&bumps),
            "Bumps: lib 0.1.0 → 0.2.0, app 0.1.0 → 0.2.0"
        );
        apply(&mut ws, &bumps).unwrap();
        commit(fixture.root(), &ws, cargo_update).unwrap();
        let root = read(&fixture.root().join("Cargo.toml"));
        assert!(
            root.contains("lib = { path = \"crates/lib\", version = \"0.2.0\" }"),
            "{root}"
        );
        assert!(read(&fixture.root().join("Cargo.lock")).contains("0.2.0"));
    }

    #[test]
    fn apply_preserves_formatting_and_comments_and_saves_only_changes() {
        let fixture = Fixture::new(
            "",
            &[
                ("lib", "0.1.0", "# lib has no deps\n"),
                (
                    "app",
                    "0.4.0",
                    "[dependencies]\n# internal\nlib = { path = \"../lib\", version = \"0.1.0\" } # pinned\n",
                ),
                ("other", "0.1.0", ""),
            ],
        );
        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        apply(&mut ws, &bumps).unwrap();
        assert_eq!(
            ws.changed_manifests(),
            [fixture.manifest("app"), fixture.manifest("lib")]
        );
        commit(fixture.root(), &ws, |_| Ok(())).unwrap();
        assert_eq!(
            read(&fixture.manifest("app")),
            "[package]\nname = \"app\"\nversion = \"0.5.0\"\nedition = \"2024\"\n\n\
             [dependencies]\n# internal\nlib = { path = \"../lib\", version = \"0.2.0\" } # pinned\n"
        );
    }

    #[test]
    fn failed_lock_refresh_rolls_everything_back() {
        let fixture = Fixture::new(
            "",
            &[("lib", "0.1.0", ""), ("app", "0.1.0", &deps(&["lib"]))],
        );
        let lock_path = fixture.root().join("Cargo.lock");
        fs::write(&lock_path, "# original lock\n").unwrap();
        let manifests = [fixture.manifest("lib"), fixture.manifest("app")];
        let before = manifests.clone().map(|p| read(&p));

        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        apply(&mut ws, &bumps).unwrap();
        let err = commit(fixture.root(), &ws, |root| {
            fs::write(root.join("Cargo.lock"), "# half-written\n")?;
            bail!("network is down")
        })
        .unwrap_err();

        assert!(format!("{err:#}").contains("rolled back"), "{err:#}");
        assert_eq!(manifests.map(|p| read(&p)), before);
        assert_eq!(read(&lock_path), "# original lock\n");
        // So running it again bumps from the original versions, not twice.
        assert_eq!(
            plan_summary(&fixture, &[req("lib", Level::Minor)]),
            "Bumps: lib 0.1.0 → 0.2.0, app 0.1.0 → 0.2.0"
        );
    }

    #[test]
    fn unreadable_lock_stops_the_bump_before_writing() {
        let fixture = Fixture::new(
            "",
            &[("lib", "0.1.0", ""), ("app", "0.1.0", &deps(&["lib"]))],
        );
        // A directory can't be read as a file, and the error isn't `NotFound`.
        fs::create_dir(fixture.root().join("Cargo.lock")).unwrap();
        let before = read(&fixture.manifest("lib"));

        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        apply(&mut ws, &bumps).unwrap();
        let err = commit(fixture.root(), &ws, |_| panic!("must not refresh")).unwrap_err();

        assert!(format!("{err:#}").contains("nothing changed"), "{err:#}");
        assert_eq!(read(&fixture.manifest("lib")), before);
    }

    #[test]
    fn lock_is_restored_even_if_a_manifest_restore_fails() {
        let fixture = Fixture::new(
            "",
            &[("lib", "0.1.0", ""), ("app", "0.1.0", &deps(&["lib"]))],
        );
        let lock_path = fixture.root().join("Cargo.lock");
        fs::write(&lock_path, "# original lock\n").unwrap();
        let lib_before = read(&fixture.manifest("lib"));

        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        apply(&mut ws, &bumps).unwrap();
        let err = commit(fixture.root(), &ws, |root| {
            fs::write(root.join("Cargo.lock"), "# half-written\n")?;
            // `app`'s manifest can't be written back once its directory is gone.
            fs::remove_dir_all(root.join("crates/app"))?;
            bail!("network is down")
        })
        .unwrap_err();

        let message = format!("{err:#}");
        assert!(message.contains("rollback failed too"), "{message}");
        assert!(message.contains("crates/app/Cargo.toml"), "{message}");
        // Every other file is still put back.
        assert_eq!(read(&lock_path), "# original lock\n");
        assert_eq!(read(&fixture.manifest("lib")), lib_before);
    }

    #[test]
    fn unsupported_requirement_fails_before_anything_is_written() {
        let fixture = Fixture::new(
            "",
            &[
                ("lib", "0.1.0", ""),
                (
                    "app",
                    "0.1.0",
                    "[dependencies]\nlib = { path = \"../lib\", version = \">=0.1, <0.3\" }\n",
                ),
            ],
        );
        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("lib", Level::Minor)], false).unwrap();
        let err = apply(&mut ws, &bumps).unwrap_err();
        assert!(format!("{err:#}").contains("edit it by hand"), "{err:#}");
    }

    #[test]
    fn unknown_crate_is_an_error() {
        let err = plan(&load(&example()), &[req("terra-nope", Level::Patch)], false).unwrap_err();
        assert!(err.to_string().contains("not a workspace crate"));
    }

    #[test]
    fn workspace_inherited_version_is_rejected() {
        let fixture = Fixture::new("\n[workspace.package]\nversion = \"0.1.0\"\n", &[]);
        fixture.add_crate("x", "0.0.0", "");
        let manifest = fixture.manifest("x");
        let text = read(&manifest).replace("version = \"0.0.0\"", "version.workspace = true");
        fs::write(&manifest, text).unwrap();
        let mut ws = load(&fixture);
        let bumps = plan(&ws, &[req("x", Level::Patch)], false).unwrap();
        let err = apply(&mut ws, &bumps).unwrap_err();
        assert!(err.to_string().contains("plain string"), "{err}");
    }

    #[test]
    fn args_are_pairs_with_optional_flags() {
        let args = |s: &str| s.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(
            parse_args(&args("terra-core minor --dry-run")).unwrap(),
            Options {
                requests: vec![req("terra-core", Level::Minor)],
                dry_run: true,
                allow_1_0: false,
            }
        );
        assert_eq!(
            parse_args(&args("a patch --allow-1.0 b major")).unwrap(),
            Options {
                requests: vec![req("a", Level::Patch), req("b", Level::Major)],
                dry_run: false,
                allow_1_0: true,
            }
        );
        assert!(parse_args(&args("terra-core")).is_err());
        assert!(parse_args(&args("")).is_err());
        assert!(parse_args(&args("terra-core huge")).is_err());
        assert!(parse_args(&args("terra-core minor --force")).is_err());
    }
}
