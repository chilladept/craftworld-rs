# CLAUDE.md

Project rules live in AGENTS.md and are authoritative; they are imported here so
Claude Code loads them automatically. Do not duplicate or override them in this file.

@AGENTS.md

## Start of every session (context budget)

AGENTS.md says to start from STATUS.md, RESEARCH.md and the relevant docs. Do that
by section, never whole files: STATUS.md is ~200 KB (~50k tokens), RESEARCH.md,
MODLOG.md, docs/architecture.md, docs/formats.md and docs/roadmap.md are 30-50 KB each.

1. Read `STATUS-CURRENT.md` if it exists. Otherwise read only the last ~100 lines
   of `STATUS.md` (latest entry is at the end).
2. For RESEARCH.md, architecture.md, formats.md, roadmap.md: `grep -n '^## '` for the
   heading list, then read only the sections for the area you are touching.
   MODLOG.md is newest-first; read the first ~15 lines at most.
3. Read the one milestone doc under `docs/` for the area. Skip the others.
4. Continue from the "Next:" line unless the user asks for something else.

Do not open: `target/`, `*.zip`, `Cargo.lock`, `game-data/`, `references/**`
(grep a pinned file instead), large `local-analysis/*.json` (summarize with a
script; the snapshot is ~119 MB). View PNG captures only to answer a concrete
visual question (AGENTS.md already says this; images cost thousands of tokens).
Use an Explore subagent for broad searches and keep only its conclusion.

## Command output

Never let full logs into context. Iterate with the narrowest crate, run the
workspace once at the end, and cap output:

```powershell
cargo test -q -p <crate> 2>&1 | Select-Object -Last 30      # bash: | tail -n 30
cargo clippy -q -p <crate> --all-targets -- -D warnings 2>&1 | Select-Object -Last 40
cargo build -q -p lbp-bevy-viewer -p lbp-viewer 2>&1 | Select-Object -Last 20
```

Census, parity, oracle and scanner runs write JSON to files: report the summary
counts, don't print the JSON. Viewer QA logs: grep for placement/error lines.

## Handoff writing budget

Handoff notes are written every milestone and reread later, so keep them short.
The facts AGENTS.md requires stay; repetition goes.

- Full detail (rule evidence, pins, applicability, every command) goes once, in the
  milestone doc under `docs/`.
- STATUS.md entry: at most ~40 lines. Counts (newly viewable / newly Complete with
  baseline, unit, scope), one line per verification command with its result,
  failed experiments, not tested, Next. Link the milestone doc, don't restate it.
- MODLOG.md entry: at most 6 lines.
- Overwrite `STATUS-CURRENT.md` (at most ~60 lines) with the current state.

## Cloud (Linux) sessions

- Before every push run `scripts/precheck.sh` (`--fast` while iterating). It uses
  CI's pinned toolchain and crate list, adds the viewer `cargo check` CI skips on
  PRs, and prints only failing tails. The cloud default Rust is older than CI's, so
  plain `cargo clippy` passes locally and fails CI.
- New viewer flags: add one entry to `crates/lbp-bevy-viewer/src/help.rs`, never a
  long single-line string (it conflicted on every flag PR).
- Requests to the PC and its results go by pointer, not by copy: a result longer
  than ~10 lines is written once (PR comment or `results/` file, see
  docs/workflow.md §9) and every relay carries the link plus a 1-3 line verdict.

## Build and test (Windows, PowerShell)

The toolchain is project-local (Rust stable, x86_64-pc-windows-gnu, no MSVC).
Activate it once per shell: `. ./scripts/use-local-rust.ps1`, then
`cargo fmt --all --check`, the capped test/clippy/build commands above, and
`python -m unittest discover -s tests` (without `-v` unless a test fails).

Viewer: `target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid <guid>`
(`--qa-capture local-analysis/<new>.png` for a capture). Oracle checks use
`scripts/run-*-oracle.ps1` and the `lbp-catalog` / `lbp-preview` examples; the
latest exact commands are in the latest STATUS entry.

## Reminders

- Commit at each milestone (user asked for git); no push/publish unless asked.
- Output filenames under `local-analysis/` must be new; tools refuse to overwrite.
- Write docs as UTF-8 explicitly (a cp1252 default broke a write before).
- Never rewrite source/docs with PowerShell Get-Content/Set-Content/Out-File: Windows
  PowerShell 5.1 reads UTF-8 as cp1252 and double-encodes non-ASCII (broke D63 labels).
  Use the Edit tool or Python with explicit UTF-8 and preserved line endings.
- Native test windows go on the non-primary monitor; verify placement first.
- After meaningful work, append to STATUS.md and MODLOG.md within the budget above.
