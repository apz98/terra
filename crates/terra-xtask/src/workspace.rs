//! Workspace discovery: member manifests, their versions and internal dependencies.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use semver::Version;
use toml_edit::{DocumentMut, Item, TableLike, Value};

/// Dependency tables that end up in the shipped artifact, so a bump in them cascades.
pub(crate) const SHIPPING_DEP_TABLES: &[&str] = &["dependencies", "build-dependencies"];

/// Every dependency table. All of them get their version requirements kept in sync.
pub(crate) const ALL_DEP_TABLES: &[&str] =
    &["dependencies", "build-dependencies", "dev-dependencies"];

/// One workspace member, with its manifest kept editable (formatting and comments survive).
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) version: Version,
    pub(crate) manifest_path: PathBuf,
    manifest: DocumentMut,
}

impl Package {
    /// Parses a member manifest. `package.version` must be a literal, per `docs/VERSIONING.md`.
    pub(crate) fn parse(manifest_path: PathBuf, text: &str) -> Result<Self> {
        let at = manifest_path.display();
        let manifest: DocumentMut = text.parse().with_context(|| format!("parsing {at}"))?;
        let package = manifest
            .get("package")
            .and_then(Item::as_table_like)
            .with_context(|| format!("{at} has no [package] table"))?;
        let name = package
            .get("name")
            .and_then(Item::as_str)
            .with_context(|| format!("{at} has no package.name"))?
            .to_owned();
        let Some(version) = package.get("version").and_then(Item::as_str) else {
            bail!(
                "{at}: package.version must be a plain string \
                 (versions live in each crate's own Cargo.toml, see docs/VERSIONING.md)"
            );
        };
        let version = Version::parse(version).with_context(|| format!("{at}: package.version"))?;
        Ok(Self {
            name,
            version,
            manifest_path,
            manifest,
        })
    }

    /// The manifest as it would be written to disk.
    pub(crate) fn manifest_text(&self) -> String {
        self.manifest.to_string()
    }

    /// Names of the crates this package depends on through `tables` (renames resolved).
    fn dependency_names(&self, tables: &[&str]) -> BTreeSet<&str> {
        dep_tables(&self.manifest, tables)
            .into_iter()
            .flat_map(|table| table.iter().map(|(key, item)| dependency_name(key, item)))
            .collect()
    }

    pub(crate) fn set_version(&mut self, version: &Version) {
        if let Some(item) = self
            .manifest
            .get_mut("package")
            .and_then(|package| package.get_mut("version"))
        {
            set_string(item, &version.to_string());
        }
        self.version = version.clone();
    }

    /// Points every version requirement on `dep`, in any dependency table, at `version`.
    pub(crate) fn set_dependency_requirement(&mut self, dep: &str, version: &Version) {
        let requirement = version.to_string();
        for_each_dep_table_mut(&mut self.manifest, ALL_DEP_TABLES, &mut |table| {
            for (key, item) in table.iter_mut() {
                if dependency_name(key.get(), item) != dep {
                    continue;
                }
                if item.is_str() {
                    set_string(item, &requirement);
                } else if let Some(entry) = item.as_table_like_mut()
                    && let Some(version) = entry.get_mut("version")
                {
                    set_string(version, &requirement);
                }
            }
        });
    }
}

/// All workspace members, sorted by name.
pub(crate) struct Workspace {
    pub(crate) packages: Vec<Package>,
}

impl Workspace {
    /// Reads `<root>/Cargo.toml` and every member manifest it lists.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let root_manifest = root.join("Cargo.toml");
        let text = fs::read_to_string(&root_manifest)
            .with_context(|| format!("reading {}", root_manifest.display()))?;
        let doc: DocumentMut = text
            .parse()
            .with_context(|| format!("parsing {}", root_manifest.display()))?;
        let members = doc
            .get("workspace")
            .and_then(|workspace| workspace.get("members"))
            .and_then(Item::as_array)
            .context("root Cargo.toml has no workspace.members array")?;

        let mut packages = Vec::new();
        for member in members {
            let pattern = member
                .as_str()
                .context("workspace.members entries must be strings")?;
            for dir in expand_member(root, pattern)? {
                let path = dir.join("Cargo.toml");
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                packages.push(Package::parse(path, &text)?);
            }
        }
        Self::new(packages)
    }

    pub(crate) fn new(mut packages: Vec<Package>) -> Result<Self> {
        packages.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(pair) = packages
            .windows(2)
            .find(|pair| pair[0].name == pair[1].name)
        {
            bail!("two workspace members are named `{}`", pair[0].name);
        }
        Ok(Self { packages })
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

    /// Workspace members that `package` depends on through `tables`.
    fn internal_deps(&self, package: &Package, tables: &[&str]) -> Vec<&Package> {
        package
            .dependency_names(tables)
            .into_iter()
            .filter_map(|name| self.get(name))
            .collect()
    }

    /// For each member, the members that depend on it through `tables`.
    pub(crate) fn dependents(&self, tables: &[&str]) -> BTreeMap<&str, Vec<&str>> {
        let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for package in &self.packages {
            for dep in self.internal_deps(package, tables) {
                dependents
                    .entry(dep.name.as_str())
                    .or_default()
                    .push(package.name.as_str());
            }
        }
        dependents
    }

    /// Member names, each after all of its dependencies (ties broken by name).
    pub(crate) fn topological_order(&self, tables: &[&str]) -> Result<Vec<&str>> {
        let mut remaining: BTreeMap<&str, BTreeSet<&str>> = self
            .packages
            .iter()
            .map(|package| {
                let deps = self.internal_deps(package, tables);
                let deps = deps.into_iter().map(|dep| dep.name.as_str()).collect();
                (package.name.as_str(), deps)
            })
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

    /// Writes every member manifest back to disk.
    pub(crate) fn save(&self) -> Result<()> {
        for package in &self.packages {
            fs::write(&package.manifest_path, package.manifest_text())
                .with_context(|| format!("writing {}", package.manifest_path.display()))?;
        }
        Ok(())
    }
}

/// Expands one `workspace.members` entry. Supports plain paths and `dir/*`.
fn expand_member(root: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let is_glob = |s: &str| s.contains(['*', '?', '[']);
    let Some(parent) = pattern.strip_suffix("/*") else {
        if is_glob(pattern) {
            bail!("unsupported workspace.members pattern `{pattern}` (use a path or `dir/*`)");
        }
        return Ok(vec![root.join(pattern)]);
    };
    if is_glob(parent) {
        bail!("unsupported workspace.members pattern `{pattern}` (use a path or `dir/*`)");
    }
    let dir = root.join(parent);
    let mut dirs = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.join("Cargo.toml").is_file() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
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

/// The `kinds` tables at the top level and under every `[target.<cfg>]`.
fn dep_tables<'a>(doc: &'a DocumentMut, kinds: &[&str]) -> Vec<&'a dyn TableLike> {
    let mut parents: Vec<&dyn TableLike> = vec![doc.as_table()];
    if let Some(targets) = doc.get("target").and_then(Item::as_table_like) {
        parents.extend(targets.iter().filter_map(|(_, t)| t.as_table_like()));
    }
    parents
        .into_iter()
        .flat_map(|parent| {
            kinds
                .iter()
                .filter_map(|kind| parent.get(kind).and_then(Item::as_table_like))
        })
        .collect()
}

/// Mutable counterpart of [`dep_tables`].
fn for_each_dep_table_mut(
    doc: &mut DocumentMut,
    kinds: &[&str],
    f: &mut dyn FnMut(&mut dyn TableLike),
) {
    fn visit(parent: &mut dyn TableLike, kinds: &[&str], f: &mut dyn FnMut(&mut dyn TableLike)) {
        for kind in kinds {
            if let Some(table) = parent.get_mut(kind).and_then(Item::as_table_like_mut) {
                f(table);
            }
        }
    }
    visit(doc.as_table_mut(), kinds, f);
    if let Some(targets) = doc.get_mut("target").and_then(Item::as_table_like_mut) {
        for (_, target) in targets.iter_mut() {
            if let Some(target) = target.as_table_like_mut() {
                visit(target, kinds, f);
            }
        }
    }
}
