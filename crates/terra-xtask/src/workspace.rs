//! The workspace as Cargo sees it (`cargo metadata`), plus comment-preserving manifest
//! edits (`toml_edit`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use cargo_metadata::{DependencyKind, MetadataCommand};
use semver::{Op, Version, VersionReq};
use toml_edit::{DocumentMut, Item, TableLike, Value};

/// Dependency tables whose requirements `bump` keeps in sync (top level and `[target.*]`).
const DEP_TABLES: &[&str] = &["dependencies", "build-dependencies", "dev-dependencies"];

/// A dependency on another workspace member.
pub(crate) struct Dep {
    pub(crate) name: String,
    /// `false` for `[dev-dependencies]`: they don't end up in the shipped artifact.
    pub(crate) ships: bool,
}

/// One workspace member.
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) version: Version,
    pub(crate) manifest_path: PathBuf,
    pub(crate) deps: Vec<Dep>,
}

impl Package {
    /// Members this package ships (depends on through non-dev dependencies).
    fn shipped_deps(&self) -> impl Iterator<Item = &str> {
        self.deps
            .iter()
            .filter(|dep| dep.ships)
            .map(|dep| dep.name.as_str())
    }
}

/// A manifest as read from disk and as edited so far.
struct Manifest {
    original: String,
    doc: DocumentMut,
}

impl Manifest {
    fn read(path: &Path) -> Result<Self> {
        let original =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let doc = original
            .parse()
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(Self { original, doc })
    }

    fn text(&self) -> String {
        self.doc.to_string()
    }

    fn is_changed(&self) -> bool {
        self.text() != self.original
    }
}

/// All workspace members (sorted by name) and the manifests `bump` may edit.
pub(crate) struct Workspace {
    pub(crate) packages: Vec<Package>,
    root_manifest: PathBuf,
    manifests: BTreeMap<PathBuf, Manifest>,
}

impl Workspace {
    /// Asks Cargo for the members of the workspace at `root` (so `exclude`, globs and
    /// inherited fields behave exactly as in a build) and reads their manifests.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let metadata = MetadataCommand::new()
            .cargo_path(crate::cargo())
            .manifest_path(root.join("Cargo.toml"))
            .no_deps()
            .exec()
            .context("running `cargo metadata`")?;
        let members = metadata.workspace_packages();
        let names: BTreeSet<&str> = members.iter().map(|p| p.name.as_ref()).collect();

        let mut packages = Vec::new();
        let mut manifests = BTreeMap::new();
        for member in &members {
            let deps = member
                .dependencies
                .iter()
                .filter(|dep| dep.path.is_some() && names.contains(dep.name.as_str()))
                .map(|dep| Dep {
                    name: dep.name.clone(),
                    ships: dep.kind != DependencyKind::Development,
                })
                .collect();
            let manifest_path = member.manifest_path.clone().into_std_path_buf();
            manifests.insert(manifest_path.clone(), Manifest::read(&manifest_path)?);
            packages.push(Package {
                name: member.name.as_ref().to_owned(),
                version: member.version.clone(),
                manifest_path,
                deps,
            });
        }
        packages.sort_by(|a, b| a.name.cmp(&b.name));

        let root_manifest = metadata
            .workspace_root
            .join("Cargo.toml")
            .into_std_path_buf();
        if !manifests.contains_key(&root_manifest) {
            manifests.insert(root_manifest.clone(), Manifest::read(&root_manifest)?);
        }
        Ok(Self {
            packages,
            root_manifest,
            manifests,
        })
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|package| package.name == name)
    }

    pub(crate) fn names(&self) -> Vec<&str> {
        self.packages.iter().map(|p| p.name.as_str()).collect()
    }

    /// `(name, version)` of every member.
    pub(crate) fn versions(&self) -> BTreeMap<String, Version> {
        self.packages
            .iter()
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect()
    }

    /// For each member, the members that ship it.
    pub(crate) fn dependents(&self) -> BTreeMap<&str, Vec<&str>> {
        let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for package in &self.packages {
            for dep in package.shipped_deps() {
                dependents.entry(dep).or_default().push(&package.name);
            }
        }
        dependents
    }

    /// Member names, each after every member it ships (ties broken by name).
    pub(crate) fn topological_order(&self) -> Result<Vec<&str>> {
        let mut remaining: BTreeMap<&str, BTreeSet<&str>> = self
            .packages
            .iter()
            .map(|p| (p.name.as_str(), p.shipped_deps().collect()))
            .collect();
        let mut order = Vec::with_capacity(remaining.len());
        while !remaining.is_empty() {
            let Some(next) = remaining
                .iter()
                .find(|(_, deps)| deps.is_empty())
                .map(|(name, _)| *name)
            else {
                let names: Vec<&str> = remaining.keys().copied().collect();
                bail!("dependency cycle among: {}", names.join(", "));
            };
            remaining.remove(next);
            for deps in remaining.values_mut() {
                deps.remove(next);
            }
            order.push(next);
        }
        Ok(order)
    }

    /// Sets `name`'s `package.version`. It must be a literal (`docs/VERSIONING.md`).
    pub(crate) fn set_version(&mut self, name: &str, version: &Version) -> Result<()> {
        let package = self
            .packages
            .iter_mut()
            .find(|package| package.name == name)
            .with_context(|| format!("`{name}` is not a workspace crate"))?;
        let path = &package.manifest_path;
        let item = self
            .manifests
            .get_mut(path)
            .and_then(|manifest| manifest.doc.get_mut("package"))
            .and_then(|table| table.get_mut("version"))
            .filter(|item| item.is_str())
            .with_context(|| {
                format!(
                    "{}: package.version must be a plain string \
                     (versions live in each crate's own Cargo.toml, see docs/VERSIONING.md)",
                    path.display()
                )
            })?;
        set_string(item, &version.to_string());
        package.version = version.clone();
        Ok(())
    }

    /// Points every version requirement on `dep` at `version`: in every member's
    /// dependency tables and in the root `[workspace.dependencies]`.
    pub(crate) fn set_requirements(&mut self, dep: &str, version: &Version) -> Result<()> {
        for (path, manifest) in &mut self.manifests {
            for table in dep_tables_mut(&mut manifest.doc) {
                rewrite_entries(table, dep, version, path)?;
            }
            if *path == self.root_manifest
                && let Some(table) = manifest
                    .doc
                    .get_mut("workspace")
                    .and_then(|workspace| workspace.get_mut("dependencies"))
                    .and_then(Item::as_table_like_mut)
            {
                rewrite_entries(table, dep, version, path)?;
            }
        }
        Ok(())
    }

    /// Manifests whose edited text differs from what was read.
    #[cfg(test)]
    pub(crate) fn changed_manifests(&self) -> Vec<&Path> {
        self.manifests
            .iter()
            .filter(|(_, manifest)| manifest.is_changed())
            .map(|(path, _)| path.as_path())
            .collect()
    }

    /// Writes the changed manifests (only those, so untouched crates don't rebuild).
    pub(crate) fn save(&self) -> Result<()> {
        for (path, manifest) in &self.manifests {
            if manifest.is_changed() {
                fs::write(path, manifest.text())
                    .with_context(|| format!("writing {}", path.display()))?;
            }
        }
        Ok(())
    }

    /// Writes back the text originally read for every changed manifest.
    pub(crate) fn restore(&self) -> Result<()> {
        for (path, manifest) in &self.manifests {
            if manifest.is_changed() {
                fs::write(path, &manifest.original)
                    .with_context(|| format!("restoring {}", path.display()))?;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn manifest_text(&self, path: &Path) -> String {
        self.manifests[path].text()
    }
}

/// Rewrites the requirement of every entry in `table` that refers to crate `dep`.
fn rewrite_entries(
    table: &mut dyn TableLike,
    dep: &str,
    version: &Version,
    path: &Path,
) -> Result<()> {
    for (key, item) in table.iter_mut() {
        if dependency_name(key.get(), item) != dep {
            continue;
        }
        let requirement = if item.is_str() {
            Some(item)
        } else {
            item.as_table_like_mut()
                .and_then(|entry| entry.get_mut("version"))
        };
        let Some(requirement) = requirement else {
            continue;
        };
        let new = match requirement.as_str() {
            Some(old) => rewrite_requirement(old, version)
                .with_context(|| format!("{}: dependency `{}`", path.display(), key.get()))?,
            None => continue,
        };
        set_string(requirement, &new);
    }
    Ok(())
}

/// Points requirement `old` at `version`, keeping its operator and precision:
/// `0.1.0` → `0.2.0`, `=0.1.0` → `=0.2.0`, `~0.1` → `~0.2`, `^0` → `^0`.
pub(crate) fn rewrite_requirement(old: &str, version: &Version) -> Result<String> {
    let unsupported = || {
        format!(
            "can't rewrite requirement `{old}` to {version}: only a single `x.y.z`, `^`, `~` \
             or `=` requirement is supported, so edit it by hand"
        )
    };
    let parsed = VersionReq::parse(old).with_context(unsupported)?;
    let [comparator] = parsed.comparators.as_slice() else {
        bail!(unsupported());
    };
    ensure!(
        matches!(comparator.op, Op::Exact | Op::Tilde | Op::Caret),
        unsupported()
    );

    let trimmed = old.trim();
    let operator = &trimmed[..trimmed.find(|c: char| c.is_ascii_digit()).unwrap_or(0)];
    // A shortened requirement (`0.3`) never matches a pre-release, so spell those out.
    let full = !version.pre.is_empty();
    let mut new = format!("{operator}{}", version.major);
    if comparator.minor.is_some() || full {
        write!(new, ".{}", version.minor)?;
    }
    if comparator.patch.is_some() || full {
        write!(new, ".{}", version.patch)?;
        if !version.pre.is_empty() {
            write!(new, "-{}", version.pre)?;
        }
    }
    ensure!(
        VersionReq::parse(&new).is_ok_and(|req| req.matches(version)),
        unsupported()
    );
    Ok(new)
}

/// The crate a dependency entry refers to: its `package` key if renamed, else the key.
fn dependency_name<'a>(key: &'a str, item: &'a Item) -> &'a str {
    item.as_table_like()
        .and_then(|entry| entry.get("package"))
        .and_then(Item::as_str)
        .unwrap_or(key)
}

/// Replaces a string value, keeping the whitespace and comments around it.
fn set_string(item: &mut Item, new: &str) {
    if let Some(value) = item.as_value_mut() {
        let decor = value.decor().clone();
        *value = Value::from(new);
        *value.decor_mut() = decor;
    }
}

/// The dependency tables at the top level and under every `[target.<cfg>]`.
fn dep_tables_mut(doc: &mut DocumentMut) -> Vec<&mut dyn TableLike> {
    let mut tables = Vec::new();
    let mut targets = None;
    for (key, item) in doc.as_table_mut().iter_mut() {
        if DEP_TABLES.contains(&key.get()) {
            tables.extend(item.as_table_like_mut());
        } else if key.get() == "target" {
            targets = item.as_table_like_mut();
        }
    }
    for (_, target) in targets.into_iter().flat_map(|targets| targets.iter_mut()) {
        let Some(target) = target.as_table_like_mut() else {
            continue;
        };
        for (key, item) in target.iter_mut() {
            if DEP_TABLES.contains(&key.get()) {
                tables.extend(item.as_table_like_mut());
            }
        }
    }
    tables
}

/// Throwaway workspaces on disk, so tests exercise the real `cargo metadata`.
#[cfg(test)]
pub(crate) mod fixture {
    use std::fmt::Write as _;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    /// A workspace with members under `crates/`.
    pub(crate) struct Fixture {
        pub(crate) dir: TempDir,
    }

    impl Fixture {
        /// `root_extra` is appended to the root `[workspace]` table. Each crate is
        /// `(name, version, rest of its manifest after [package])`.
        pub(crate) fn new(root_extra: &str, crates: &[(&str, &str, &str)]) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root =
                format!("[workspace]\nresolver = \"3\"\nmembers = [\"crates/*\"]\n{root_extra}\n");
            fs::write(dir.path().join("Cargo.toml"), root).unwrap();
            let fixture = Self { dir };
            for (name, version, rest) in crates {
                fixture.add_crate(name, version, rest);
            }
            fixture
        }

        pub(crate) fn add_crate(&self, name: &str, version: &str, rest: &str) {
            let dir = self.root().join("crates").join(name);
            fs::create_dir_all(dir.join("src")).unwrap();
            fs::write(dir.join("src/lib.rs"), "").unwrap();
            let manifest = format!(
                "[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2024\"\n\n{rest}"
            );
            fs::write(dir.join("Cargo.toml"), manifest).unwrap();
        }

        pub(crate) fn root(&self) -> &Path {
            self.dir.path()
        }

        pub(crate) fn manifest(&self, name: &str) -> PathBuf {
            self.root().join("crates").join(name).join("Cargo.toml")
        }
    }

    pub(crate) fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    /// `[dependencies]` with a path dependency on each of `deps`.
    pub(crate) fn deps(deps: &[&str]) -> String {
        let mut table = String::from("[dependencies]\n");
        for dep in deps {
            writeln!(table, "{dep} = {{ path = \"../{dep}\" }}").unwrap();
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{Fixture, deps, read};
    use super::*;

    #[test]
    fn requirements_keep_operator_and_precision() {
        let v = |s| Version::parse(s).unwrap();
        let cases = [
            ("0.1.0", "0.2.0", "0.2.0"),
            ("=0.1.0", "0.2.0", "=0.2.0"),
            ("~0.1", "0.2.0", "~0.2"),
            ("^0.1", "0.1.1", "^0.1"),
            ("0", "0.3.0", "0"),
            ("= 0.1.0", "0.1.1", "= 0.1.1"),
            ("0.3", "0.3.0-alpha.1", "0.3.0-alpha.1"),
        ];
        for (old, version, new) in cases {
            assert_eq!(rewrite_requirement(old, &v(version)).unwrap(), new, "{old}");
        }
        for old in [">=0.1, <0.3", "*", "<0.2", "0.1.*"] {
            assert!(rewrite_requirement(old, &v("0.2.0")).is_err(), "{old}");
        }
    }

    #[test]
    fn cargo_decides_membership_including_exclude() {
        let fixture = Fixture::new(
            "exclude = [\"crates/scratch\"]",
            &[("a", "0.1.0", ""), ("scratch", "0.1.0", "")],
        );
        let ws = Workspace::load(fixture.root()).unwrap();
        assert_eq!(ws.names(), ["a"]);
    }

    #[test]
    fn graph_resolves_renames_targets_build_and_dev_deps() {
        let fixture = Fixture::new(
            "",
            &[
                ("lib", "0.1.0", ""),
                (
                    "a",
                    "0.1.0",
                    "[dependencies]\ncore2 = { package = \"lib\", path = \"../lib\" }\n",
                ),
                (
                    "b",
                    "0.1.0",
                    "[target.'cfg(unix)'.dependencies]\nlib = { path = \"../lib\" }\n",
                ),
                (
                    "c",
                    "0.1.0",
                    "[build-dependencies.lib]\npath = \"../lib\"\n",
                ),
                (
                    "d",
                    "0.1.0",
                    "[dev-dependencies]\nlib = { path = \"../lib\" }\n",
                ),
            ],
        );
        let ws = Workspace::load(fixture.root()).unwrap();
        assert_eq!(ws.dependents()["lib"], ["a", "b", "c"]);
        // `d` only dev-depends on `lib`, so it is free to come first.
        assert_eq!(ws.topological_order().unwrap(), ["d", "lib", "a", "b", "c"]);
    }

    #[test]
    fn only_changed_manifests_are_saved_and_restore_undoes_them() {
        let fixture = Fixture::new("", &[("a", "0.1.0", ""), ("b", "0.1.0", &deps(&["a"]))]);
        let mut ws = Workspace::load(fixture.root()).unwrap();
        ws.set_version("a", &Version::new(0, 2, 0)).unwrap();
        assert_eq!(ws.changed_manifests(), [fixture.manifest("a")]);

        let before = read(&fixture.manifest("a"));
        ws.save().unwrap();
        assert!(read(&fixture.manifest("a")).contains("version = \"0.2.0\""));
        ws.restore().unwrap();
        assert_eq!(read(&fixture.manifest("a")), before);
    }
}
