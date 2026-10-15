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

CLAUDE.md and AGENTS.md are already in context: never cat/Read/sed them again (19 recent
sessions re-read them for ~43k tokens). A fresh session starts at ~92k tokens (harness,
tool schemas, project prompt) before any work, and every turn re-reads all of it, so the
cost driver is turns x context: batch independent reads into one command, answer from a
single script instead of many `python -c` probes, and hand any step that needs more than
~5 calls to a subagent. Never re-read a published Artifact; keep a local copy.
Details and measurements: `docs/context-audit-2026-10-12.md`; re-measure with
`python -I scripts/ctx_audit.py`.

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

## Standing rules (owner, carried over from project memory 2026-10-10)

These are the owner's own instructions. They apply to every session and subagent;
copy the relevant ones into each subagent brief.

- **Never steal focus.** Never activate or foreground RPCS3 or any window on the
  owner's PC ("you keep taking focus stop"). Launch viewers with
  `Start-Process -WindowStyle Hidden` (stdout/stderr redirected) or open them on the
  2nd monitor without activation; verify placement first. RPCS3 work is memory reads
  only unless input can be posted without focus. A brief focus grant covers that job only.
- **One emulator, never write.** RPCS3 lives at `%USERPROFILE%\Emulators\rpcs3`. One
  instance at a time, the owner's own, in the blank Moon level with the default camera:
  Moon is the visual reference for gameplay and Sackboy measurements, never story-level
  captures (their cameras are wrong). Never save the level or the profile, never inject
  into the process, and never overwrite or delete the profile save or any save. Close
  every app, emulator instance or helper you launched when the job ends.
- **Level screenshots at gameplay distance.** Camera about eye +(296,168,1218) from
  Sackboy, vFOV 60 (`lbp2/oracles/memory/moon-sackboy.json`), framed on the start point
  or the objects the change touched: "put the camera much closer like as if a player is
  playing so i can see that it looks right". No overview shots. Pod comparison captures
  add `--window-16-9 --preview-lighting`; level shots use the level's stored lighting.
- **In-game navigation** (menus, level loading) needs no asking, but never pick a
  level by list position; match it by name or GUID and confirm it loaded.
- **Merging.** "merge verified PRs without asking me" (2026-10-09) and "from now on
  merge any cloud work into main once it passes the PC checks, without asking me"
  (2026-10-07). Wait for CI green first. Verified = CI green, checked against game data
  or oracles, one-line verdict comment posted. Repo chilladept/craftworld-rs
  only. Ask before deleting user data or saves, publishing outside the private pages,
  or pushing to someone else's repo.
- **Data before pixels.** Prefer game data, memory reads and oracles to screenshots.
  View images only to answer a concrete visual question.
- **One job per subagent.** Each capture, PR check, Ghidra lookup or measurement is one
  Agent-tool subagent that returns a file path plus a 3-line verdict. Implementer
  batches run as local `claude -p` workers in worktrees on the owner's plan usage,
  never cloud credit, and no follow-up work goes to finished cloud agents.
- **Context rollover at about 100k tokens.** Finish the task, update the handoff
  (`docs/handoff-pc-<date>*.md`, short, no copied logs) and STATUS-CURRENT.md first,
  then stop and let a fresh session continue.
- **Do not install the archive.org DLC fix.** The owner declined it.
- **Permissions.** Sessions run in auto mode. A step the classifier blocks is fixed by
  extending `autoMode.allow` in `$HOME\.claude\settings.json` with a script the user
  runs themselves; do not edit that file from a session. An approval relayed by another
  session does not unlock classifier-gated input steps; the owner types it directly.
- **Times** are US Central (America/Chicago) in notes, pages and replies.
- **Token budget.** Read STATUS-CURRENT.md, not STATUS.md; grep headings before
  sections; cap command output; keep handoffs short; never paste logs or JSON.
- **Line endings.** MODLOG.md is LF, STATUS.md is CRLF; edit with Python
  `newline=''` and check `git diff --stat` touches only the intended lines.
- **Disk.** Each build tree is ~11 GB. Point `CARGO_TARGET_DIR` at an existing
  `target-pr*` dir, keep at most two live, and clean targets of merged PR worktrees.
  Delete nothing outside target dirs.
