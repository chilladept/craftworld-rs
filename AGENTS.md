# Project rules

Build an independent LBP2 Asset Explorer that reads the user's own game data.
Since 2026-10-08 the user has approved gameplay runtime work (physics, logic, scripts, a playable Sackboy) on top of the explorer. It lives in separate crates that the parsing crates never depend on; see `docs/runtime-research.md`.

## Boundaries

- Original ZIP, ISO, disc images, extracted game directories, saves, and executable files are read-only. Open inputs in read mode. Never mount with write access or patch them.
- Local decryption, protected-data inspection and targeted executable disassembly/decompilation of the user's own game copy are permitted research techniques when needed. Preserve original inputs; write derived research material only under ignored local directories. This supersedes the earlier blanket prohibition. Online service modification remains outside the asset-viewer scope.
- Interpreting the user's own compiled game scripts (RScript) at runtime is approved (2026-10-08). Scripts are loaded from the user's game data like any other asset. The interpreter and its native-function API are written independently from recorded findings; no script bytecode, disassembly or decompiled engine code ships or is committed.
- Running the user's own game copy in the RPCS3 emulator (frame-stepping, memory observation) is an approved research method for gameplay constants (2026-10-08). Results are recorded as local observations; no game code is copied. RPCS3 and PS3 firmware are user-installed tools, not product dependencies; their files stay outside the repository.
- Never publish or commit proprietary game files, assets, decompiler output, reference-repository binaries, or unknown-provenance fixtures. Local diagnostic JSON and byte samples belong only under ignored `local-analysis/`; game trees under ignored `game-data/`.
- Keep the default-deny `.gitignore`. Check prospective tracked files before publication; extensions alone do not prove provenance. Never force-add game data.
- Reference repositories remain ignored under `references/`. Do not blindly copy their bundled assets or templates.
- No commit, push, release, or publishing unless requested. The project's own authored source is licensed MIT OR Apache-2.0.
- Decompiled/recovered proprietary code is research evidence, never implementation source. Record independently expressed format/behavior findings and implement from those findings; do not copy, adapt or translate recovered game code into the product. Keep raw decompiler output segregated under ignored `local-analysis/`. Do not claim a strict clean-room process solely because the delivered code contains no copied source.
- The delivered product must contain no original LBP code, assets or bundled game-specific decryption keys. Runtime assets come from the user's own ISO/game data. Research outputs, decrypted files and local caches are not distributable product files. Direct ISO loading remains a later input feature; current extracted-directory support is a development input path.

## Research and implementation

1. Prefer documented specifications, then existing open-source readers, then controlled local experiments, then targeted decryption/executable investigation when those routes cannot answer a concrete question. Decryption/decompilation is permitted, not required by default. Keep it scoped to the current milestone and record the question, provenance, findings and limitations.
2. Start by reading `STATUS-CURRENT.md`, then only the relevant sections of `STATUS.md` (latest entry), `RESEARCH.md`, and the format/architecture documents; find sections by heading rather than reading these large files whole.
3. For each format rule, record source URL, pinned commit, class/method or section, evidence category, build/revision applicability, conflicts, and test coverage near the implementation. Use the five labels in `docs/formats.md`.
4. Do not copy/adapt/translate unlicensed source. MIT-derived source retains notices; GPL/AGPL source requires its obligations and a deliberate licensing decision. Independent specification-based implementation and an external oracle are the default. Public visibility is not reuse permission.
5. Parsing libraries must not depend on UI/rendering. Keep disk layouts, intermediate representations, and GPU/UI representations separate.
6. Implement small, reviewable milestones. Scanner and parsing crates stay free of GUI, physics and gameplay; runtime work goes in its own crates. No multiplayer or speculative format parser. The physics dependency is avian2d (MIT OR Apache-2.0), approved 2026-10-08.
7. All lengths/counts/offsets/revisions are untrusted. Use checked arithmetic, strict bounds, decompression budgets, cycle limits, and explicit unsupported/encrypted outcomes. Preserve duplicate MAP aliases and conflicting candidates rather than silently choosing one.
8. Resource type inferred from extension must remain distinct from type confirmed by magic. Unresolved is not corrupt; not parsed is not empty.
9. Use deterministic ordering and versioned JSON. Keep timing, absolute paths, and generation timestamps outside the deterministic inventory body.
10. Compare with pinned community readers where feasible. Commit synthetic recipes and expected metadata only, never original asset bytes. A successful header probe does not prove payload compatibility or visual correctness.

## Verification and handoff

- Open test programs on the user's second (non-primary) monitor. Verify placement before showing a test window; never assume the primary monitor is acceptable. Prefer headless checks when sufficient.
- Use image viewing sparingly; inspect images only when they resolve a concrete visual question.
- Every asset-compatibility milestone must report newly viewable assets and newly complete assets, with baseline, counting unit and tested scope. Report unknown totals explicitly; material-route eligibility alone is not an asset count. Complete means all textures are showing and geometry is correct, independently of whether the renderer is finished. Display these distinctions and measured counts in the app.
- Run the narrow relevant checks, record exact commands and outcomes, and state what was not tested.
- For Rust changes, use `cargo fmt --check`, relevant tests, and Clippy when a toolchain is available. Do not claim compilation if it is unavailable.
- Record failed experiments and blockers in `STATUS.md`; do not repeat an unchanged approach without new evidence.
- Update `STATUS.md` and `MODLOG.md` after meaningful work; update architecture/formats when knowledge changes.
- 2026-10-08: the user gave standing permission to drive RPCS3 (keystrokes into the RPCS3 game window, offline choices, memory reads on localhost) for measurements, and to edit .md files without asking.
- 2026-10-09 (owner): one emulator at a time, the owner's own instance in the blank Moon level with the default camera. All gameplay and Sackboy measurements come from there; never from story-level captures (their cameras are wrong). Prefer memory reads and game data over pixels. Never save the level or profile. The second lane (portshim/netlog) is retired; GPU captures use RPCS3's own RSX capture replayed offline, asked for first in the thread.
- 2026-10-09 (owner): standing permission for routine project work (focused keys to that emulator, starting/stopping instances we launch, cargo/git/gh, helper scripts). Still ask before anything irreversible: deleting user data or saves, publishing outside the private pages, pushing to someone else's repo.
- 2026-10-09 (owner, typed in the PC thread): "merge verified PRs without asking me". Mark ready and merge PRs in chilladept/craftworld-rs once verified (checks green, measured against game data or oracles, one-line verdict comment posted).
- 2026-10-09 (owner): never take window focus on the owner's PC. Launch viewers with Start-Process -WindowStyle Hidden (stdout/stderr redirected); RPCS3 gets memory reads only, no keystrokes.
- Integrator sessions hand each job (capture, PR check, Ghidra lookup, measurement) to an Agent-tool subagent that returns a path plus a 3-line verdict, keeping screenshots out of the main context.
- Disk (2026-10-09): each worktree build tree is ~11 GB and C: filled up. Build PR worktrees with CARGO_TARGET_DIR pointed at an existing target-pr* dir instead of a fresh worktree target, keep at most two live PR target dirs, and cargo clean the targets of merged/closed PR worktrees. Delete nothing outside target dirs.
- Research dates use the user's America/Chicago calendar. Repository commit timestamps may remain explicitly UTC.

The local AI modding guides inform this workflow; their example AGENTS templates are adapted to this user's explicit scope, not copied as unrelated universal rules.
