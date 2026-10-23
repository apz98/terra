//! `cargo xtask bump`: per-crate `SemVer` bumps with the cascade from `docs/VERSIONING.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

use anyhow::{Context, Result, bail, ensure};
use semver::Version;

use crate::workspace::{SHIPPING_DEP_TABLES, Workspace};

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
    /// The next version at this level. Pre-release and build metadata are dropped.
    pub(crate) fn apply(self, version: &Version) -> Version {
        match self {
            Self::Major => Version::new(version.major + 1, 0, 0),
            Self::Minor => Version::new(version.major, version.minor + 1, 0),
            Self::Patch => Version::new(version.major, version.minor, version.patch + 1),
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

/// Works out every bump implied by `requests`, dependencies first.
///
/// Each requested crate bumps at the highest level asked for it; every crate that ships
/// it (directly or transitively) bumps at the highest level among the crates it ships.
pub(crate) fn plan(workspace: &Workspace, requests: &[(String, Level)]) -> Result<Vec<Bump>> {
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

    let dependents = workspace.dependents(SHIPPING_DEP_TABLES);
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

    let order = workspace.topological_order(SHIPPING_DEP_TABLES)?;
    Ok(order
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
        .collect())
}

/// Applies `bumps` to the in-memory manifests: crate versions and internal requirements.
pub(crate) fn apply(workspace: &mut Workspace, bumps: &[Bump]) {
    for bump in bumps {
        for package in &mut workspace.packages {
            if package.name == bump.name {
                package.set_version(&bump.to);
            }
            package.set_dependency_requirement(&bump.name, &bump.to);
        }
    }
}

/// The `Bumps:` line for the commit message (`docs/WORKFLOW.md`).
pub(crate) fn summary(bumps: &[Bump]) -> String {
    let list: Vec<String> = bumps
        .iter()
        .map(|bump| format!("{} {} → {}", bump.name, bump.from, bump.to))
        .collect();
    format!("Bumps: {}", list.join(", "))
}

/// Parses `<crate> <level> [<crate> <level> ...] [--dry-run]`.
fn parse_args(args: &[String]) -> Result<(Vec<(String, Level)>, bool)> {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let rest: Vec<&String> = args.iter().filter(|arg| *arg != "--dry-run").collect();
    if let Some(flag) = rest.iter().find(|arg| arg.starts_with("--")) {
        bail!("unknown flag `{flag}`");
    }
    ensure!(
        !rest.is_empty() && rest.len().is_multiple_of(2),
        "expected `<crate> <major|minor|patch>` pairs, got {rest:?}"
    );
    let requests = rest
        .chunks(2)
        .map(|pair| Ok((pair[0].clone(), pair[1].parse()?)))
        .collect::<Result<_>>()?;
    Ok((requests, dry_run))
}

/// Runs `cargo xtask bump`.
pub(crate) fn run(root: &Path, args: &[String]) -> Result<()> {
    let (requests, dry_run) = parse_args(args)?;
    let mut workspace = Workspace::load(root)?;
    let bumps = plan(&workspace, &requests)?;
    for bump in &bumps {
        let why = if bump.requested { "" } else { ", cascade" };
        println!(
            "  {:<16} {} → {} ({}{why})",
            bump.name, bump.from, bump.to, bump.level
        );
    }
    println!("{}", summary(&bumps));
    if dry_run {
        println!("(dry run, nothing written)");
        return Ok(());
    }

    apply(&mut workspace, &bumps);
    workspace.save()?;
    // Keep Cargo.lock in step with the manifests so the commit is self-consistent.
    let status = Command::new(crate::cargo())
        .args(["update", "--workspace", "--quiet"])
        .current_dir(root)
        .status()
        .context("running `cargo update --workspace`")?;
    ensure!(status.success(), "`cargo update --workspace` failed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::path::PathBuf;

    use super::*;
    use crate::workspace::Package;

    fn manifest(name: &str, version: &str, deps: &[&str]) -> String {
        let mut table = String::new();
        for dep in deps {
            writeln!(table, "{dep} = {{ path = \"../{dep}\" }}").unwrap();
        }
        format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n\n[dependencies]\n{table}")
    }

    fn workspace(manifests: &[String]) -> Workspace {
        let packages = manifests
            .iter()
            .enumerate()
            .map(|(i, text)| Package::parse(PathBuf::from(format!("{i}/Cargo.toml")), text))
            .collect::<Result<_>>()
            .unwrap();
        Workspace::new(packages).unwrap()
    }

    /// The example workspace from `docs/VERSIONING.md`.
    fn example() -> Workspace {
        workspace(&[
            manifest("terra-core", "0.2.0", &[]),
            manifest("terra-render", "0.3.0", &["terra-core"]),
            manifest("terra-client", "0.5.2", &["terra-render", "terra-core"]),
            manifest("terra-sim", "0.4.0", &["terra-core"]),
            manifest("terra", "0.8.1", &["terra-client", "terra-sim"]),
            manifest("terra-server", "0.2.3", &["terra-sim"]),
            manifest("terra-xtask", "0.1.0", &["terra-core"]),
        ])
    }

    fn req(name: &str, level: Level) -> (String, Level) {
        (name.to_owned(), level)
    }

    fn plan_summary(workspace: &Workspace, requests: &[(String, Level)]) -> String {
        summary(&plan(workspace, requests).unwrap())
    }

    #[test]
    fn levels_follow_semver() {
        let v = Version::parse("0.8.1").unwrap();
        assert_eq!(Level::Patch.apply(&v), Version::new(0, 8, 2));
        assert_eq!(Level::Minor.apply(&v), Version::new(0, 9, 0));
        assert_eq!(Level::Major.apply(&v), Version::new(1, 0, 0));
        let pre = Version::parse("0.3.0-alpha.1+build").unwrap();
        assert_eq!(Level::Patch.apply(&pre), Version::new(0, 3, 1));
        assert!(Level::Major > Level::Minor && Level::Minor > Level::Patch);
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
            &example(),
            &[
                req("terra-core", Level::Patch),
                req("terra-render", Level::Minor),
            ],
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
        // terra-xtask depends on terra-core but is not bumped by it ...
        let bumps = plan(&example(), &[req("terra-core", Level::Minor)]).unwrap();
        assert!(bumps.iter().all(|b| b.name != "terra-xtask"));
        // ... and bumping it directly touches nothing else.
        assert_eq!(
            plan_summary(&example(), &[req("terra-xtask", Level::Minor)]),
            "Bumps: terra-xtask 0.1.0 → 0.2.0"
        );
    }

    #[test]
    fn dev_dependencies_do_not_cascade_but_requirements_update() {
        let user = "[package]\nname = \"user\"\nversion = \"0.1.0\"\n\n\
                    [dev-dependencies]\nlib = { path = \"../lib\", version = \"0.1.0\" }\n";
        let mut ws = workspace(&[manifest("lib", "0.1.0", &[]), user.to_owned()]);
        let bumps = plan(&ws, &[req("lib", Level::Minor)]).unwrap();
        assert_eq!(summary(&bumps), "Bumps: lib 0.1.0 → 0.2.0");
        apply(&mut ws, &bumps);
        assert!(
            ws.get("user")
                .unwrap()
                .manifest_text()
                .contains("lib = { path = \"../lib\", version = \"0.2.0\" }")
        );
    }

    #[test]
    fn renamed_target_and_build_dependencies_cascade() {
        let renamed = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n\
                       [dependencies]\ncore2 = { package = \"lib\", path = \"../lib\" }\n";
        let target = "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n\
                      [target.'cfg(unix)'.dependencies]\nlib = { path = \"../lib\" }\n";
        let build = "[package]\nname = \"c\"\nversion = \"0.1.0\"\n\n\
                     [build-dependencies.lib]\npath = \"../lib\"\nversion = \"0.1.0\"\n";
        let ws = workspace(&[
            manifest("lib", "0.1.0", &[]),
            renamed.to_owned(),
            target.to_owned(),
            build.to_owned(),
        ]);
        assert_eq!(
            plan_summary(&ws, &[req("lib", Level::Patch)]),
            "Bumps: lib 0.1.0 → 0.1.1, a 0.1.0 → 0.1.1, b 0.1.0 → 0.1.1, c 0.1.0 → 0.1.1"
        );
    }

    #[test]
    fn apply_preserves_formatting_and_comments() {
        let lib = "[package]\nname = \"lib\"\nversion = \"0.1.0\"   # the version\n";
        let app = "[package]\nname = \"app\"\nversion = \"0.4.0\"\n\n\
                   [dependencies]\n# internal\nlib = { path = \"../lib\", version = \"0.1.0\" }\n\
                   [dependencies.renamed]\npackage = \"lib\"\npath = \"../lib\"\nversion = \"0.1.0\"\n";
        let mut ws = workspace(&[lib.to_owned(), app.to_owned()]);
        let bumps = plan(&ws, &[req("lib", Level::Minor)]).unwrap();
        apply(&mut ws, &bumps);
        assert_eq!(
            ws.get("lib").unwrap().manifest_text(),
            "[package]\nname = \"lib\"\nversion = \"0.2.0\"   # the version\n"
        );
        assert_eq!(
            ws.get("app").unwrap().manifest_text(),
            "[package]\nname = \"app\"\nversion = \"0.5.0\"\n\n\
             [dependencies]\n# internal\nlib = { path = \"../lib\", version = \"0.2.0\" }\n\
             [dependencies.renamed]\npackage = \"lib\"\npath = \"../lib\"\nversion = \"0.2.0\"\n"
        );
        assert_eq!(ws.get("app").unwrap().version, Version::new(0, 5, 0));
    }

    #[test]
    fn unknown_crate_is_an_error() {
        let err = plan(&example(), &[req("terra-nope", Level::Patch)]).unwrap_err();
        assert!(err.to_string().contains("not a workspace crate"));
    }

    #[test]
    fn workspace_inherited_version_is_rejected() {
        let text = "[package]\nname = \"x\"\nversion.workspace = true\n";
        assert!(Package::parse(PathBuf::from("x/Cargo.toml"), text).is_err());
    }

    #[test]
    fn args_are_pairs_with_optional_dry_run() {
        let args = |s: &str| s.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
        let (requests, dry_run) = parse_args(&args("terra-core minor --dry-run")).unwrap();
        assert_eq!(requests, [req("terra-core", Level::Minor)]);
        assert!(dry_run);
        let (requests, dry_run) = parse_args(&args("a patch b major")).unwrap();
        assert_eq!(requests, [req("a", Level::Patch), req("b", Level::Major)]);
        assert!(!dry_run);
        assert!(parse_args(&args("terra-core")).is_err());
        assert!(parse_args(&args("")).is_err());
        assert!(parse_args(&args("terra-core huge")).is_err());
        assert!(parse_args(&args("terra-core minor --force")).is_err());
    }
}
