# Lessons — so mistakes happen once

Adrian clears Claude's context often (`/clear`). Anything learned in a session is lost
unless it is written here. This file is how Claude gets better across sessions.

**Read the Rules at the start of every session** (CLAUDE.md and `/next` require it).

## How this file works

- **When to add a lesson:** something went wrong or surprised us: a failed gate, red CI,
  a wrong assumption, a tool quirk, docs that drifted, a correction from Adrian, or
  time wasted on something that a note would have saved. Add it **in the same commit**
  as the task it came from (or a `docs:` commit if no task is open).
- **Log entry format:** date, task, *what happened*, *root cause*, *rule*. Keep it to a
  few lines. Be honest; a lesson nobody can act on is useless.
- **Promote:** when a log entry yields a rule worth following every time, add it to the
  Rules list below. If it belongs to a process, also put it into `WORKFLOW.md`,
  `CLAUDE.md` or `/next` so it's enforced where the work happens. If it can be checked
  by a tool (`cargo xtask check-docs`, clippy, CI), automate it. That's the strongest fix.
- **Prune:** delete a rule once it's automated or no longer applies, and say so in the
  log. Keep the Rules list short (≤ ~20) so it actually gets read.

## Rules (read every session)

1. **Format before checking.** Run `cargo fmt` after writing code, then `cargo xtask ci`.
   Don't hand-wrap lines and hope.
2. **Grep for stale references when finishing a task.** Search CLAUDE.md, `.claude/commands/`
   and `docs/` for the *exact* old text of anything renamed, moved or replaced, and for "once it exists"/"arrives in" notes about what you just built.
   Unknown task IDs are caught by `check-docs` automatically (since T-0004).
3. **The shell is zsh with aliases.** `ls` output may be swallowed: use `command ls -A`
   or the file tools. A word starting with `=` is a command lookup (`echo ====` fails),
   unmatched globs are errors (quote `--include='*.md'`), and `/usr/bin/time` and `bc`
   aren't installed: time with `date +%s%N` arithmetic. Separators between batched
   commands: `echo '-----'`, never `echo =====`.
4. **Verify, don't assume.** Look up crate versions on crates.io (`curl -A terra-dev`, no
   personal data in headers) and toolchain versions with `rustup check` at task time;
   read a new dependency's API in `~/.cargo/registry/src/*/<crate>-<version>/`, not from
   memory; check Ubuntu package names in the archive's `Packages.xz`; confirm a command's
   output before reporting it.
5. **Things outside the AC need a callout.** If you make a choice the task didn't ask
   for, say so in the report and the commit message, and say how to undo it.
6. **Look before you overwrite.** Before writing a file with `cat >`/`Write`, check that
   it doesn't exist (`git ls-files <path>`, Read). If it does, edit it instead.
7. **Tools enforce documented invariants.** If a doc says "never X" or "X is a manual
   decision" and a tool can do X, the tool refuses without an explicit flag.
8. **Wait on CI by commit SHA, not "latest run".** A run appears a few seconds after the
   push; `cargo xtask wait-ci` handles it (WORKFLOW step 9).
9. **Scripted edits stay dumb and checked.** Use the Edit tool for one-off replacements.
   In a Python/sed edit script, no clever expressions (conditionals, joins) inside
   `replace` arguments, and check the file's line count or `git diff --stat` afterwards.
10. **Look at the window, don't guess.** On the reference machine (Hyprland 0.56, Lua
    dispatch syntax) drive the game window with `hyprctl` and probe pixels with `grim`;
    the T-0101 log entry has the commands. Report what the pixels and logs showed.
    Mouse clicks can't be synthesized there (T-0102): ask Adrian, with a temporary debug
    log that shows what the game received.
11. **Follow every error branch to the next iteration.** For a loop that handles failure
    statuses (surface, network, I/O), read in the library source what each branch really
    receives and what the loop does next: does it block, spin, retry forever, or panic in
    a nested call?

## Log (newest first)

### 2026-10-23 — T-0102 Frame loop, timing and input
- **What happened:** I couldn't click into the window to test the cursor grab. Hyprland
  0.56 has no click dispatcher (`send_shortcut` with `key = "mouse:272"` returns `ok` and
  does nothing), and `wtype` only types keys. **Fix:** a temporary `debug` log of the mouse
  input, an AskUserQuestion with step-by-step instructions for Adrian, then the log to
  confirm what Adrian saw. **Rule:** 10 (extended).
- **Note:** the T-0101 note is confirmed: with vsync off the frame rate is uncapped (about
  6000 fps clearing the screen on the reference machine), so skipping `pre_present_notify`
  works. The 8192-sample frame-time ring buffer covers that.
- **Note:** clippy 1.99 flags `Box::new([0u32; 8192])` (`large_stack_arrays`: built on the
  stack first). Use `vec![0; N].into_boxed_slice()`.
- **Note:** `TERRA_LOG=terra_client=debug` alone switches every other target off. Write
  `info,terra_client=debug`.

### 2026-10-23 — T-0110 Fix `/code-review` findings on T-0101
- **Note:** Hyprland 0.56 hides a window with `hyprctl dispatch 'hl.dsp.window.move({
  workspace = "special:terra", silent = true, window = "address:0x…" })'` and brings it back
  with `workspace = "e+0"` (`move_to_workspace` doesn't exist). There's no minimize, and
  `xdotool` isn't installed, so a real 0×0 window can't be produced on the reference machine.
  Run with `env -u WAYLAND_DISPLAY` to test X11 (Xwayland).
- **Note:** under Xwayland, Mesa reports `Suboptimal` on the first frame and after every
  resize; one reconfigure clears it. That's the path the T-0110 once-per-size guard covers.
- **Note:** CPU measured from `/proc/<pid>/stat` (utime + stime, 100 ticks = 1 core-second):
  ~2.6 % visible or hidden with vsync on, Wayland and X11. With vsync **off** a hidden
  Wayland window keeps presenting at full speed (~45 % of a core): frames still succeed,
  so nothing waits. That's expected for an uncapped mode, but a cap while hidden or
  unfocused is worth considering with T-0102's frame timing.

### 2026-10-23 — `/code-review` of T-0101 (`811d5b2..ec594f2`)
- **What happened:** T-0101 handled every `CurrentSurfaceTexture` variant, but it never
  checked what the frame loop does after each one. A skipped frame requests the next one
  at once, so a minimized window spins. A lost device shows up as `Validation` (wgpu drops
  the `DeviceError::Lost` without telling us), not as `Lost`. **Cause:** read the enum's
  docs, not what wgpu-core does on each path, and didn't think about the next frame.
  **Rule:** 11.
- **What happened:** I summarized the review's findings for Adrian before checking them.
  Two were wrong ("the `Validation` arm is dead code"; the device-loss mechanism).
  **Rule:** none new. Rule 4 covers it: check review findings before stating them as facts,
  or label them unverified.

### 2026-10-23 — T-0101 Window and GPU surface
- **What happened:** wgpu 30's API differs from what I remembered: `get_current_texture`
  returns a `CurrentSurfaceTexture` enum, frames are presented with `queue.present(frame)`,
  the instance takes winit's display handle, and `util::initialize_adapter_from_env`
  panics (`expect`) when no adapter matches. Reading the registry source first avoided
  writing code against the old API. **Rule:** 4 (now names the registry source).
- **What happened:** I guessed the Ubuntu package for the Khronos validation layer as
  `vulkan-validation-layers` (Arch's name); in noble it's `vulkan-validationlayers`.
  The T-0007 `Packages.xz` check caught it. **Rule:** 4 (promoted from that log entry).
- **What happened:** `cargo deny check` failed on `ttf-parser` (unmaintained,
  RUSTSEC-2026-0192), pulled in by winit's default `wayland-csd-adwaita` feature (title-bar
  text). **Fix:** winit features without it, with `wayland-csd-adwaita-notitle`. That's what
  T-0108 was for: one finding, not a hundred-crate audit.
- **What happened:** `hyprctl dispatch <name> <args>` failed: Hyprland 0.56 takes Lua.
  Working commands: `hyprctl clients -j` (find the window by `title`, get `at`/`size`/
  `address`), `hyprctl dispatch 'hl.dsp.focus({ window = "address:0x…" })'`,
  `'hl.dsp.window.float({ action = "enable" })'`, `'hl.dsp.window.resize({ x = 900, y = 500 })'`
  (active window), `'hl.dsp.send_shortcut({ mods = "", key = "F9" })'`,
  `'hl.dsp.window.close()'`. Pixel probe: `grim -t ppm -g "X,Y 1x1" - | tail -c 3 | od
  -An -tu1` prints R G B. Screenshots: `grim -g "X,Y WxH" file.png`, then Read the PNG.
  **Rule:** 10.
- **Note:** winit sets no Wayland app id by default (`class` was empty in `hyprctl clients`);
  the client sets `terra`. Not in the task's AC (Rule 5): remove `with_app_id` to undo.
- **Note:** `pre_present_notify()` on Wayland aligns `RedrawRequested` with frame callbacks,
  which would cap vsync-off frame rates at the refresh rate; it's only called with vsync on.
  T-0102/T-0107 should confirm vsync-off fps is really uncapped once the counter exists.
- **Note:** clippy 1.99 suggests `as_chunks::<N>()` over `chunks_exact(N)` and
  `TextureViewDescriptor::default()` over `&Default::default()`.

### 2026-10-23 — T-0108 License and advisory checks (`cargo-deny`)
- **What happened:** again, the session's first command batched `cat docs/LESSONS.md` with
  `echo =====` and failed on Rule 3. The T-0109 fix ("read LESSONS on its own first") was
  only a log line, and `/next` still listed LESSONS as one read among others. **Fix:**
  `/next` step 1 now says to Read LESSONS alone, before any other command. A lesson that
  changes how a session starts belongs in `/next`, not just here.
- **What happened:** `cargo deny --manifest-path <scratch>/Cargo.toml init` ignored the
  manifest path and wrote the template `deny.toml` into the repo root. Rule 6 caught it
  (the file "existed" before I wrote mine). **Note:** `cargo deny init` writes to the
  current directory; `cd` into the scratch dir in the same command.
- **What happened:** the first `cargo deny check` rejected `option-ext` (MPL-2.0), pulled in
  by `directories` → `dirs-sys` since T-0006. Replaced `directories` with `etcetera` (same
  paths, MIT OR Apache-2.0) instead of adding an exception. **Rule:** none new; this is
  why T-0108 went ahead of wgpu/winit.
- **Note:** cargo-deny warns about allowed licenses no dependency uses yet;
  `unused-allowed-license = "allow"` silences it, since the allow list is policy.

### 2026-10-23 — T-0109 Fix `/code-review` findings on M0
- **What happened:** the session's first command was `cat docs/LESSONS.md && echo ===== …`,
  which failed on the exact zsh quirk Rule 3 lists, in the same command that loaded the
  rule. **Cause:** batching the LESSONS read with other reads, so the Rules weren't in
  context yet. **Rule:** 3 now names the safe separator. Read LESSONS on its own first.
- **What happened:** running the binary showed every invalid `TERRA_LOG` directive reported
  twice: `EnvFilter::parse_lossy` prints its own raw line to stderr. Tests couldn't see it.
  **Fix:** parse directives one by one with `Directive::from_str`. (`from_directives` is
  private, so the valid ones are re-joined and parsed strictly.)
- **Note:** `tracing::dispatcher::has_been_set()` only sees the *global* subscriber, not
  one set with `tracing::subscriber::with_default`. To test both branches of code that
  checks it, pass the flag in as a parameter.
- **Note:** GitHub's concurrency docs now have `queue: max` (up to 100 pending runs per
  group). Per-commit groups were the simpler fix here: `main` runs don't wait on each other.
- **Note:** clippy 1.99 rejects `assert!(v.is_empty())` without a message ("used `assert!`
  to check that a value is empty"). Use `assert_eq!(v, Vec::<T>::new())`, which also shows
  the contents when it fails.

### 2026-10-22 — `/code-review` of M0 (`2b2725b..bc4ab5c`)
- **What happened:** T-0010 claimed `cancel-in-progress: false` means `main` runs are never
  cancelled. GitHub keeps one pending run per concurrency group and cancels the older
  pending one, so a quick third push still cancels the second. **Cause:** reasoned about
  GitHub's semantics from the option's name instead of its documentation. **Rule:** 4
  covers it: for CI/platform behaviour, read the docs before writing a comment that
  promises something.
- **What happened:** T-0006 left "check `--features profile-tracy` by hand" as a LESSONS
  note; nothing gates it. **Cause:** a manual note where a gate was cheap. **Fix:** T-0109
  adds it to `cargo xtask ci`. A "check by hand" note is a missing gate.

### 2026-10-22 — T-0008 M0 wrap-up
- **What happened:** three shell commands failed on zsh quirks: `echo =====` (zsh expands
  `=word` to a command path), an unquoted `--include=*.md` (unmatched glob is an error),
  and timing with `/usr/bin/time` and `bc` (neither is installed). **Cause:** wrote bash
  habits for a zsh shell. **Rule:** 3 (widened from the `ls` alias to zsh quirks).
- **What happened:** two new task descriptions told future tasks to edit ADR 0010, which
  `docs/adr/README.md` says is never rewritten once accepted. Caught by the stale-reference
  grep before committing. **Rule:** none new; an ADR changes only through a superseding ADR.

### 2026-10-22 — T-0007 Linux build prerequisites
- **What happened:** checking 16 package names on packages.ubuntu.com took more than 3
  minutes and never finished. The Docker daemon wasn't running, so a container test
  wasn't possible either (starting it needs sudo). **Fix:** grep the archive's own index,
  `http://archive.ubuntu.com/ubuntu/dists/<release>/{main,universe}/binary-amd64/Packages.xz`
  (~16 MB, seconds). **Rule:** verify Ubuntu package names against `Packages.xz`, not
  the packages website.
- **Note:** winit and wgpu `dlopen` every system library (x11-dl, x11rb `dl-libxcb`,
  xkbcommon-dl, `wayland-dlopen`, ash `libvulkan.so`, EGL `dynamic`), so building needs
  only a C toolchain and pkg-config. Audio (`cpal`/ALSA) will be the first real build-time
  system dependency. Add it to `.github/ubuntu-packages.txt` and README then.
- **Note:** Rust 1.99 already links with `rust-lld` on x86-64 Linux
  (`cargo rustc -p terra -- --print link-args` shows `-fuse-ld=lld`), so `mold` would add
  little. Measure before adding it.

### 2026-10-22 — T-0006 Diagnostics foundation
- **What happened:** I passed Adrian's email in a `User-Agent` header when querying the
  crates.io API. Instructions say his email goes to no unrelated service unless he
  asks. **Cause:** copied crates.io's "include contact info" advice without thinking.
  **Rule:** use `-A terra-dev` (no personal data) for crates.io API calls.
- **What happened:** `tracing-subscriber` writes ANSI colour codes even when stderr is a
  pipe or file. Only running the binary showed it; the tests passed. **Fix:**
  `.with_ansi(io::stderr().is_terminal())`. **Rule:** WORKFLOW step 5 already says to run
  the binary for anything user-visible. This is why.
- **Note:** global state (subscriber, panic hook) is tested in its own `tests/*.rs` file,
  since each one runs as a separate process. The Tracy backend of `profiling` panics on the
  first scope unless `tracy_client::Client::start()` ran. (The manual
  `--features profile-tracy` check that was noted here is a `cargo xtask ci` gate since T-0109.)
- **Note:** clippy pedantic flags `name.ends_with(".log")` in tests
  (`case_sensitive_file_extension_comparisons`). Use `Path::extension()`.

### 2026-10-22 — T-0010 Fix `/code-review` findings on T-0009 and T-0005
- **What happened:** a Python edit script turned `wait_ci.rs` into two nested copies of
  itself. I had written `s.replace(a, X + Y if False else s.replace(a, Z))`. `if/else`
  binds looser than `+`, so the replacement text was the whole edited file. Rustc caught
  it (`//!` in the middle of a file) and I recovered the inner copy. **Cause:** a
  half-rewritten one-liner left in the script. **Rule:** 9.
- **Note:** clippy (1.99) wants `Duration::from_mins(45)` over `from_secs(45 * 60)`, and
  flags test helpers that always return `Ok` (`unnecessary_wraps`). Use `#[expect(…,
  reason = …)]` when the `Result` is the point.

### 2026-10-22 — T-0005 CI on GitHub Actions
- **What happened:** right after `git push`, `gh run list -L 1` returned no run, so
  `gh run watch` got an empty ID and failed. The WORKFLOW command I had just written had
  the same race. **Cause:** GitHub registers the run a few seconds after the push.
  **Rule:** 8 (WORKFLOW step 9 now polls by commit SHA).
- **What happened:** `/code-review` found CLAUDE.md rule 7 still saying `gh run list -L 1`
  after I changed WORKFLOW. My stale-reference grep looked for "no CI" wording, not the
  old command itself. **Cause:** I grepped for the topic, not the exact text I replaced.
  **Rule:** 2 (now says: grep for the exact old command/text in CLAUDE.md,
  `.claude/commands/` and `docs/`).
- **What happened:** the same review found `cancel-in-progress` cancelling `main` runs, and
  that the fix for the race was a pasted shell loop, not a tool. **Cause:** I copied a
  common PR-oriented workflow snippet without thinking about split commits on `main`.
  **Fix:** T-0010 (`cargo xtask wait-ci`).
- **Note:** first green run took 45 s with a cold cache. GitHub says `ubuntu-latest`
  moves to Ubuntu 26 from 2026-10-19; T-0007 should check package names there.

### 2026-10-22 — T-0009 Fix `/code-review` findings on T-0004
- **What happened:** a Python search-and-replace of `fixture.read(` missed three calls,
  because `cargo fmt` had already split them as `fixture\n.read(`. **Cause:** a textual
  edit after formatting. **Rule:** for mechanical renames across formatted Rust, match
  with a whitespace-tolerant regex (`fixture\s*\.read\(`), then run `cargo fmt` again.
- **Note:** tests now build throwaway workspaces with `tempfile` and run the real
  `cargo metadata`/`cargo update` (offline-safe: only path dependencies). That's about 15
  extra process spawns, and the whole xtask suite still runs in ~0.05 s.

### 2026-10-22 — T-0004 xtask `bump`, `check-docs`, `ci`
- **What happened:** I created `clippy.toml` with a heredoc and overwrote an existing
  one, dropping `allow-expect-in-tests` and `allow-dbg-in-tests`. `git status` showed it
  as `M` rather than `??`, and it was restored before committing. **Cause:** assumed the
  file didn't exist because T-0003's notes didn't mention it. **Rule:** 6.
- **Automated:** the 2026-10-22 T-0003 entry's task-ID drift is now checked by
  `check-docs` (IDs outside fenced code blocks must be defined in TASKS.md or the
  archive). Rule 2 was narrowed to match.
- **What happened:** the `/code-review` reminder went into STATUS in a follow-up commit
  rather than the task commit. **Cause:** I checked the review triggers after committing.
  **Rule:** check the WORKFLOW review triggers in the retro step, before committing.
- **What happened:** `/code-review` found 10 issues (T-0009). The worst, `bump major`
  silently making 0.x → 1.0.0, was a guard I had thought of and dropped as
  over-engineering. **Cause:** I treated a documented invariant ("1.0.0 is a manual
  decision") as something the user would remember. **Rule:** 7.
- **Note:** clippy pedantic rejects `format!` collected into a `String` (`format_collect`)
  and `push_str(&format!(…))` (`format_push_string`). Use `writeln!(s, …)` with
  `std::fmt::Write`.

### 2026-10-22 — T-0003 Workspace scaffold
- **What happened:** `cargo fmt --check` failed on a long `const` line I wrote by hand.
  **Cause:** ran the check before `cargo fmt`. **Rule:** 1.
- **What happened:** `docs/VERSIONING.md` said `xtask bump` is built in T-0003; it's T-0004.
  **Cause:** the baseline docs were written before the task list was final, and nobody
  cross-checked. **Rule:** 2. (Automated in T-0004: `check-docs` verifies task IDs.)
- **What happened:** a plain `ls -la` printed nothing. **Cause:** shell alias. **Rule:** 3.
- **What happened:** I set dev `opt-level = 1` for Terra's own code, which the task didn't
  specify. Fine, but it needed flagging. **Rule:** 5.
- **Note:** pinning an exact toolchain (`1.99.1`) makes rustup auto-download it on the
  first build in a fresh checkout or CI. That's expected, not an error.
