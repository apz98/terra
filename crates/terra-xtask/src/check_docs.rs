//! `cargo xtask check-docs`: fails when the docs disagree with the code.
//!
//! Checks (`docs/WORKFLOW.md` lists when each doc must change):
//! - the crate tables in `ARCHITECTURE.md` and `STATUS.md` match every `Cargo.toml`,
//! - `CHANGELOG.md` has a heading for the current game version,
//! - the game version in `STATUS.md` is the `terra` crate version,
//! - at most one task in `TASKS.md` is `in progress`,
//! - every task ID mentioned outside code blocks exists in `TASKS.md` or the archive.
//!
//! Fenced code blocks are ignored by every check: they hold examples and templates.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use semver::Version;

use crate::workspace;

const ARCHITECTURE: &str = "docs/ARCHITECTURE.md";
const STATUS: &str = "docs/STATUS.md";
const CHANGELOG: &str = "CHANGELOG.md";
const TASKS: &str = "docs/TASKS.md";
const ARCHIVE_DIR: &str = "docs/tasks/archive/";

/// Each crate table is the first table with `Crate` and `Version` columns under this
/// heading (a prefix of the heading line), so another table elsewhere can't stand in.
const CRATE_TABLES: [(&str, &str); 2] = [(ARCHITECTURE, "## 2."), (STATUS, "## Crate versions")];

/// The Markdown files that are checked, keyed by path relative to the workspace root.
pub(crate) struct Docs(BTreeMap<String, String>);

impl Docs {
    /// Top-level `README.md`, `CLAUDE.md`, `CHANGELOG.md`, all of `docs/`, and the Claude
    /// commands in `.claude/commands/`. The rest of `.claude/` is skipped: it holds Claude
    /// Code worktrees (whole checkouts of other branches). Symlinks are not followed.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let mut files = BTreeMap::new();
        for file in ["README.md", "CLAUDE.md", CHANGELOG] {
            let path = root.join(file);
            if path.is_file() {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                files.insert(file.to_owned(), text);
            }
        }
        for dir in ["docs", ".claude/commands"] {
            collect_markdown(root, &root.join(dir), &mut files)?;
        }
        Ok(Self(files))
    }

    fn get(&self, path: &str) -> Option<&str> {
        self.0.get(path).map(String::as_str)
    }
}

fn collect_markdown(root: &Path, dir: &Path, files: &mut BTreeMap<String, String>) -> Result<()> {
    if !fs::symlink_metadata(dir).is_ok_and(|meta| meta.is_dir()) {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        // `DirEntry::file_type` doesn't follow symlinks, so a link can't loop us.
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_markdown(root, &path, files)?;
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            let text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            files.insert(relative, text);
        }
    }
    Ok(())
}

/// Every problem found, as `path[:line]: message`. Empty means the docs are in sync.
pub(crate) fn check(crates: &BTreeMap<String, Version>, docs: &Docs) -> Vec<String> {
    let mut problems = Vec::new();
    let missing = |path: &str| format!("{path}: file is missing");

    for (path, heading) in CRATE_TABLES {
        match docs.get(path) {
            Some(text) => problems.extend(check_crate_table(path, heading, text, crates)),
            None => problems.push(missing(path)),
        }
    }

    match crates.get("terra") {
        Some(terra) => {
            match docs.get(CHANGELOG) {
                Some(text) => problems.extend(check_changelog(text, terra)),
                None => problems.push(missing(CHANGELOG)),
            }
            if let Some(text) = docs.get(STATUS) {
                problems.extend(check_status_game_version(text, terra));
            }
        }
        None => problems.push("Cargo.toml: no `terra` crate, so no game version".to_owned()),
    }

    match docs.get(TASKS) {
        Some(text) => problems.extend(check_in_progress(text)),
        None => problems.push(missing(TASKS)),
    }
    problems.extend(check_task_ids(docs));
    problems
}

/// One row of a crate table.
struct CrateRow<'a> {
    line: usize,
    name: &'a str,
    version: &'a str,
}

/// The rows of the first table with `Crate` and `Version` columns in the section under
/// `heading`, which ends at the next heading of the same or a higher level.
fn crate_table<'a>(md: &'a str, heading: &str) -> Result<Vec<CrateRow<'a>>, String> {
    let lines = prose_lines(md);
    let level = heading_level(heading);
    let start = lines
        .iter()
        .position(|(_, line)| line.starts_with(heading))
        .ok_or_else(|| format!("no `{heading}` heading"))?
        + 1;
    let end = lines[start..]
        .iter()
        .position(|(_, line)| heading_level(line).is_some_and(|l| l <= level.unwrap_or(0)))
        .map_or(lines.len(), |offset| start + offset);
    let lines = &lines[start..end];

    let mut start = 0;
    while start < lines.len() {
        // A table is a run of table rows on consecutive source lines.
        let mut end = start;
        while end < lines.len()
            && is_table_row(lines[end].1)
            && (end == start || lines[end].0 == lines[end - 1].0 + 1)
        {
            end += 1;
        }
        let table = &lines[start..end];
        start = end.max(start + 1);
        let Some(((_, header), rows)) = table.split_first() else {
            continue;
        };
        let header = cells(header);
        let column = |name: &str| header.iter().position(|cell| *cell == name);
        let (Some(crate_col), Some(version_col)) = (column("Crate"), column("Version")) else {
            continue;
        };
        let rows = rows
            .iter()
            .filter(|(_, line)| !is_separator_row(line))
            .filter_map(|&(line, text)| {
                let cells = cells(text);
                Some(CrateRow {
                    line,
                    name: cells.get(crate_col)?.trim_matches('`'),
                    version: cells.get(version_col)?.trim_matches('`'),
                })
            })
            .collect();
        return Ok(rows);
    }
    Err(format!(
        "no table with `Crate` and `Version` columns under `{heading}`"
    ))
}

/// `Some(n)` if `line` is an ATX heading of level `n` (`## Title` → 2).
fn heading_level(line: &str) -> Option<usize> {
    let level = line.bytes().take_while(|&b| b == b'#').count();
    let rest = &line[level..];
    ((1..=6).contains(&level) && (rest.is_empty() || rest.starts_with([' ', '\t'])))
        .then_some(level)
}

fn is_table_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

fn is_separator_row(line: &str) -> bool {
    cells(line)
        .iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|c| matches!(c, '-' | ':')))
}

fn cells(line: &str) -> Vec<&str> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line.strip_suffix('|').unwrap_or(line);
    line.split('|').map(str::trim).collect()
}

fn check_crate_table(
    path: &str,
    heading: &str,
    md: &str,
    crates: &BTreeMap<String, Version>,
) -> Vec<String> {
    let rows = match crate_table(md, heading) {
        Ok(rows) => rows,
        Err(problem) => return vec![format!("{path}: {problem}")],
    };
    let mut problems = Vec::new();
    for (name, version) in crates {
        match rows.iter().find(|row| row.name == name) {
            None => problems.push(format!(
                "{path}: crate `{name}` is missing from the crate table"
            )),
            Some(row) if row.version != version.to_string() => problems.push(format!(
                "{path}:{}: `{name}` is listed as {} but its Cargo.toml says {version}",
                row.line, row.version
            )),
            Some(_) => {}
        }
    }
    for row in &rows {
        if !crates.contains_key(row.name) && Version::parse(row.version).is_ok() {
            problems.push(format!(
                "{path}:{}: `{}` has version {} but is not a workspace crate \
                 (use — for planned crates)",
                row.line, row.name, row.version
            ));
        }
    }
    problems
}

fn check_changelog(md: &str, terra: &Version) -> Option<String> {
    let heading = format!("## terra {terra}");
    let found = prose_lines(md).into_iter().any(|(_, line)| {
        line.strip_prefix(&heading)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    });
    (!found).then(|| format!("{CHANGELOG}: no `{heading}` heading for the current game version"))
}

fn check_status_game_version(md: &str, terra: &Version) -> Option<String> {
    let Some((line, text)) = prose_lines(md)
        .into_iter()
        .find(|(_, text)| is_table_row(text) && text.contains("**Game version**"))
    else {
        return Some(format!("{STATUS}: no **Game version** row"));
    };
    let stated = cells(text)
        .get(1)
        .and_then(|cell| cell.split_whitespace().next())
        .map_or("", |version| version.trim_matches('`'));
    (stated != terra.to_string()).then(|| {
        format!("{STATUS}:{line}: game version is {stated} but the `terra` crate is {terra}")
    })
}

fn check_in_progress(md: &str) -> Option<String> {
    let active: Vec<&str> = prose_lines(md)
        .into_iter()
        .map(|(_, line)| line)
        .filter(|line| line.starts_with('#') && line.contains("`in progress"))
        .filter_map(|line| task_ids(line).into_iter().next())
        .collect();
    (active.len() > 1).then(|| {
        format!(
            "{TASKS}: {} tasks are in progress ({}); finish one first",
            active.len(),
            active.join(", ")
        )
    })
}

fn check_task_ids(docs: &Docs) -> Vec<String> {
    let defined: BTreeSet<&str> = docs
        .0
        .iter()
        .filter(|(path, _)| *path == TASKS || path.starts_with(ARCHIVE_DIR))
        .flat_map(|(_, text)| defined_task_ids(text))
        .collect();
    let mut problems = Vec::new();
    for (path, text) in &docs.0 {
        for (line, text) in prose_lines(text) {
            for id in task_ids(text) {
                if !defined.contains(id) {
                    problems.push(format!(
                        "{path}:{line}: {id} is not a task in {TASKS} or {ARCHIVE_DIR}"
                    ));
                }
            }
        }
    }
    problems
}

/// Every `T-nnnn` token in `text`: exactly four digits, not part of a longer word.
fn task_ids(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut ids = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find("T-") {
        let start = from + offset;
        let end = start + 6;
        let word_start = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let digits = bytes
            .get(start + 2..end)
            .is_some_and(|d| d.iter().all(u8::is_ascii_digit));
        let word_end = bytes.get(end).is_none_or(|b| !b.is_ascii_alphanumeric());
        if word_start && digits && word_end {
            ids.push(&text[start..end]);
        }
        from = start + 2;
    }
    ids
}

/// Task IDs that a file defines: as a heading (`### T-0004 …`) or a sketch bullet
/// (`- **T-0101** …`).
fn defined_task_ids(md: &str) -> Vec<&str> {
    prose_lines(md)
        .into_iter()
        .filter_map(|(_, line)| {
            let rest = if line.starts_with('#') {
                line.trim_start_matches('#').trim_start()
            } else {
                line.strip_prefix("- **")?
            };
            task_ids(rest)
                .into_iter()
                .next()
                .filter(|id| rest.starts_with(id))
        })
        .collect()
}

/// `(line number, text)` of every line outside fenced code blocks. Examples and
/// templates live in code blocks and may name tasks that don't exist.
///
/// Follows `CommonMark`: a fence is 3+ backticks or tildes, and only a fence of the same
/// character, at least as long, with nothing after it, indented at most 3 columns more
/// than the opening fence, closes it. (`CommonMark` measures from the container; the
/// opening fence stands in for it.) Any opening indentation is accepted, because fences
/// inside nested list items are indented by 4+ spaces. (At the top level that would be
/// an indented code block, which is code either way.)
fn prose_lines(md: &str) -> Vec<(usize, &str)> {
    let mut open: Option<Fence> = None;
    let mut prose = Vec::new();
    for (i, line) in md.lines().enumerate() {
        match (open, fence(line)) {
            (None, Some(f)) => open = Some(f),
            (Some(o), Some(f))
                if f.ch == o.ch && f.len >= o.len && f.bare && f.indent <= o.indent + 3 =>
            {
                open = None;
            }
            (Some(_), _) => {}
            (None, None) => prose.push((i + 1, line)),
        }
    }
    prose
}

/// A code fence line.
#[derive(Clone, Copy)]
struct Fence {
    /// `` ` `` or `~`.
    ch: u8,
    /// How many of them.
    len: usize,
    /// Nothing but whitespace follows them (required to close a block).
    bare: bool,
    /// Columns of indentation, with tab stops every 4 columns.
    indent: usize,
}

/// The fence on `line`, if it is one.
fn fence(line: &str) -> Option<Fence> {
    let rest = line.trim_start_matches([' ', '\t']);
    let indent = line[..line.len() - rest.len()].bytes().fold(0, |col, c| {
        if c == b'\t' { col / 4 * 4 + 4 } else { col + 1 }
    });
    let ch = *rest
        .as_bytes()
        .first()
        .filter(|c| matches!(c, b'`' | b'~'))?;
    let len = rest.bytes().take_while(|&c| c == ch).count();
    let after = &rest[len..];
    // A backtick fence's info string can't contain backticks (that's inline code).
    if len < 3 || (ch == b'`' && after.contains('`')) {
        return None;
    }
    Some(Fence {
        ch,
        len,
        bare: after.trim().is_empty(),
        indent,
    })
}

/// Runs `cargo xtask check-docs`.
pub(crate) fn run(root: &Path) -> Result<()> {
    let crates = workspace::versions(root)?;
    let docs = Docs::load(root)?;
    let problems = check(&crates, &docs);
    if problems.is_empty() {
        println!(
            "check-docs: ok ({} crates, {} Markdown files)",
            crates.len(),
            docs.0.len()
        );
        return Ok(());
    }
    for problem in &problems {
        eprintln!("  {problem}");
    }
    bail!("check-docs found {} problem(s)", problems.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCH: &str = "\
# Architecture

## 2. Crates

| Crate | Kind | Version | Status |
|---|---|---|---|
| `terra` | bin | 0.1.0 | ok |
| `terra-core` | lib | 0.2.0 | ok |
| `terra-render` | lib | — | planned |
";

    const STATUS_MD: &str = "\
| | |
|---|---|
| **Game version** (`terra`) | 0.1.0 (first feature) |
| **Milestone** | M0 |

## Crate versions

| Crate | Version |
|---|---|
| `terra` | 0.1.0 |
| `terra-core` | 0.2.0 |
";

    const CHANGELOG_MD: &str = "# Changelog\n\n## [Unreleased]\n\n## terra 0.1.0 — 2026-10-22\n";

    const TASKS_MD: &str = "\
### T-0001 Docs — `done (0.0.0)`
### T-0002 Thing — `in progress`
### T-0003 Other — `todo`
- **T-0101** sketch
";

    fn crates() -> BTreeMap<String, Version> {
        [("terra", "0.1.0"), ("terra-core", "0.2.0")]
            .into_iter()
            .map(|(n, v)| (n.to_owned(), Version::parse(v).unwrap()))
            .collect()
    }

    fn docs(overrides: &[(&str, &str)]) -> Docs {
        let mut files: BTreeMap<String, String> = [
            (ARCHITECTURE, ARCH),
            (STATUS, STATUS_MD),
            (CHANGELOG, CHANGELOG_MD),
            (TASKS, TASKS_MD),
        ]
        .into_iter()
        .map(|(p, t)| (p.to_owned(), t.to_owned()))
        .collect();
        for (path, text) in overrides {
            files.insert((*path).to_owned(), (*text).to_owned());
        }
        Docs(files)
    }

    fn problems(overrides: &[(&str, &str)]) -> Vec<String> {
        check(&crates(), &docs(overrides))
    }

    #[test]
    fn consistent_docs_pass() {
        assert_eq!(problems(&[]), Vec::<String>::new());
    }

    #[test]
    fn crate_missing_from_architecture_table() {
        let arch = ARCH.replace("| `terra-core` | lib | 0.2.0 | ok |\n", "");
        assert_eq!(
            problems(&[(ARCHITECTURE, &arch)]),
            ["docs/ARCHITECTURE.md: crate `terra-core` is missing from the crate table"]
        );
    }

    #[test]
    fn crate_version_differs_from_table() {
        let arch = ARCH.replace("| lib | 0.2.0 |", "| lib | 0.1.9 |");
        assert_eq!(
            problems(&[(ARCHITECTURE, &arch)]),
            [
                "docs/ARCHITECTURE.md:8: `terra-core` is listed as 0.1.9 but its Cargo.toml says 0.2.0"
            ]
        );
        let status = STATUS_MD.replace("| `terra-core` | 0.2.0 |", "| `terra-core` | 0.1.0 |");
        assert_eq!(
            problems(&[(STATUS, &status)]),
            ["docs/STATUS.md:11: `terra-core` is listed as 0.1.0 but its Cargo.toml says 0.2.0"]
        );
    }

    #[test]
    fn versioned_row_without_a_crate() {
        let arch = ARCH.replace("| lib | — | planned |", "| lib | 0.1.0 | planned |");
        assert_eq!(
            problems(&[(ARCHITECTURE, &arch)]),
            [
                "docs/ARCHITECTURE.md:9: `terra-render` has version 0.1.0 but is not a workspace \
              crate (use — for planned crates)"
            ]
        );
    }

    #[test]
    fn missing_crate_table() {
        assert_eq!(
            problems(&[(ARCHITECTURE, "# Architecture\n")]),
            ["docs/ARCHITECTURE.md: no `## 2.` heading"]
        );
        assert_eq!(
            problems(&[(ARCHITECTURE, "## 2. Crates\n\nNone yet.\n")]),
            ["docs/ARCHITECTURE.md: no table with `Crate` and `Version` columns under `## 2.`"]
        );
    }

    #[test]
    fn crate_table_is_the_one_under_its_heading() {
        let other = "| Crate | Version |\n|---|---|\n| `terra` | 9.9.9 |\n";
        // Tables before the section, and in a later section, are not the crate table ...
        let arch = format!("# A\n\n## 1. Intro\n\n{other}\n{ARCH}\n## 3. More\n\n{other}");
        assert_eq!(problems(&[(ARCHITECTURE, &arch)]), Vec::<String>::new());
        // ... but a subsection belongs to the section.
        let arch = format!("# A\n\n## 2. Crates\n\n### 2.1 List\n\n{other}");
        assert_eq!(
            problems(&[(ARCHITECTURE, &arch)]),
            [
                "docs/ARCHITECTURE.md:9: `terra` is listed as 9.9.9 but its Cargo.toml says 0.1.0",
                "docs/ARCHITECTURE.md: crate `terra-core` is missing from the crate table",
            ]
        );
        // `## 20.` is a different section from `## 2.`.
        let arch = ARCH.replace("## 2. Crates", "## 20. Crates");
        assert_eq!(
            problems(&[(ARCHITECTURE, &arch)]),
            ["docs/ARCHITECTURE.md: no `## 2.` heading"]
        );
    }

    #[test]
    fn backticked_versions_are_accepted() {
        let arch = ARCH.replace("| lib | 0.2.0 |", "| lib | `0.2.0` |");
        let status = STATUS_MD
            .replace("| 0.1.0 (first feature) |", "| `0.1.0` (first feature) |")
            .replace("| `terra` | 0.1.0 |", "| `terra` | `0.1.0` |");
        assert_eq!(
            problems(&[(ARCHITECTURE, &arch), (STATUS, &status)]),
            Vec::<String>::new()
        );
        // The backticks don't hide a wrong version.
        let arch = ARCH.replace("| lib | 0.2.0 |", "| lib | `0.1.0` |");
        assert_eq!(
            problems(&[(ARCHITECTURE, &arch)]),
            [
                "docs/ARCHITECTURE.md:8: `terra-core` is listed as 0.1.0 but its Cargo.toml says 0.2.0"
            ]
        );
    }

    #[test]
    fn changelog_needs_current_version_heading() {
        let changelog = "## [Unreleased]\n## terra 0.0.0 — 2026-10-22\n## terra 0.1.01\n";
        assert_eq!(
            problems(&[(CHANGELOG, changelog)]),
            ["CHANGELOG.md: no `## terra 0.1.0` heading for the current game version"]
        );
        // A crate-only heading doesn't count either.
        let changelog = "## terra-core 0.1.0 — 2026-10-22\n";
        assert_eq!(problems(&[(CHANGELOG, changelog)]).len(), 1);
    }

    #[test]
    fn status_game_version_must_match_terra() {
        let status = STATUS_MD.replace("| 0.1.0 (first feature) |", "| 0.0.0 (baseline) |");
        assert_eq!(
            problems(&[(STATUS, &status)]),
            ["docs/STATUS.md:3: game version is 0.0.0 but the `terra` crate is 0.1.0"]
        );
        let status = STATUS_MD.replace("**Game version**", "Version");
        assert_eq!(
            problems(&[(STATUS, &status)]),
            ["docs/STATUS.md: no **Game version** row"]
        );
    }

    #[test]
    fn at_most_one_task_in_progress() {
        let tasks = TASKS_MD.replace("Other — `todo`", "Other — `in progress`");
        assert_eq!(
            problems(&[(TASKS, &tasks)]),
            ["docs/TASKS.md: 2 tasks are in progress (T-0002, T-0003); finish one first"]
        );
    }

    #[test]
    fn unknown_task_ids_are_reported_outside_code_blocks() {
        let notes = "See T-0002 and T-0101.\nT-0009 is gone.\n\
                     ```\nTask: T-0207 (example)\n```\nalso (T-0042).\n";
        assert_eq!(
            problems(&[("docs/NOTES.md", notes)]),
            [
                "docs/NOTES.md:2: T-0009 is not a task in docs/TASKS.md or docs/tasks/archive/",
                "docs/NOTES.md:6: T-0042 is not a task in docs/TASKS.md or docs/tasks/archive/",
            ]
        );
    }

    #[test]
    fn archived_tasks_count_as_defined() {
        let archive = "# M0\n\n### T-0009 Old task — `done (0.0.1)`\n";
        let notes = "Finished T-0009.\n";
        assert_eq!(
            problems(&[("docs/tasks/archive/M0.md", archive), ("README.md", notes)]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn fences_close_only_on_a_matching_fence() {
        let notes = "\
~~~
```
T-0901 inside a tilde fence
~~~
````md
```
T-0902 inside a longer fence
```
````
```rust
T-0903
``` not a closing fence
T-0904 still inside
```
T-0905 is prose
";
        assert_eq!(
            problems(&[("docs/NOTES.md", notes)]),
            ["docs/NOTES.md:15: T-0905 is not a task in docs/TASKS.md or docs/tasks/archive/"]
        );
    }

    #[test]
    fn fences_in_nested_list_items_are_fences() {
        let notes = "\
- Steps:
    1. Run this:
        ```sh
        echo T-0901
        ```
    2. Then T-0902.
\t```
\tT-0903 in a tab-indented fence
\t```
";
        assert_eq!(
            problems(&[("docs/NOTES.md", notes)]),
            ["docs/NOTES.md:6: T-0902 is not a task in docs/TASKS.md or docs/tasks/archive/"]
        );
    }

    #[test]
    fn a_closing_fence_indented_4_more_than_the_opening_one_is_code() {
        let notes = "\
```md
Example of a nested list in Markdown:
- Steps:
    ```sh
    T-0901 is code
    ```
T-0902 is still code
```
- Item
  ```
  T-0903 is code
     ```
  T-0904 is prose
\t```
\tT-0905 is code
\t\t```
\tT-0906 is still code
\t   ```
T-0907 is prose
";
        assert_eq!(
            problems(&[("docs/NOTES.md", notes)]),
            [
                "docs/NOTES.md:13: T-0904 is not a task in docs/TASKS.md or docs/tasks/archive/",
                "docs/NOTES.md:19: T-0907 is not a task in docs/TASKS.md or docs/tasks/archive/",
            ]
        );
    }

    #[test]
    fn fenced_templates_are_ignored_by_every_check() {
        let tasks =
            format!("{TASKS_MD}\n```md\n### T-0999 Template — `in progress`\n```\nSee T-0999.\n");
        // The template neither counts as a second in-progress task nor defines T-0999.
        assert_eq!(
            problems(&[(TASKS, &tasks)]),
            ["docs/TASKS.md:9: T-0999 is not a task in docs/TASKS.md or docs/tasks/archive/"]
        );
        // A fenced crate table is not the crate table.
        let arch = format!("```\n| Crate | Version |\n|---|---|\n| `terra` | 9.9.9 |\n```\n{ARCH}");
        assert_eq!(problems(&[(ARCHITECTURE, &arch)]), Vec::<String>::new());
    }

    #[test]
    fn load_skips_worktrees_and_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |path: &str, text: &str| {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        write("README.md", "readme");
        write("docs/STATUS.md", "status");
        write("docs/tasks/archive/M0.md", "archive");
        write(".claude/commands/next.md", "next");
        write(
            ".claude/worktrees/branch/docs/STATUS.md",
            "another checkout",
        );
        #[cfg(unix)]
        std::os::unix::fs::symlink(root, root.join("docs/loop")).unwrap();

        let docs = Docs::load(root).unwrap();
        let paths: Vec<&str> = docs.0.keys().map(String::as_str).collect();
        assert_eq!(
            paths,
            [
                ".claude/commands/next.md",
                "README.md",
                "docs/STATUS.md",
                "docs/tasks/archive/M0.md"
            ]
        );
    }

    #[test]
    fn task_id_tokens() {
        assert_eq!(
            task_ids("T-0001, (T-0102) T-01023 XT-0001 T-12 T-0003a T-0004."),
            ["T-0001", "T-0102", "T-0004"]
        );
    }
}
