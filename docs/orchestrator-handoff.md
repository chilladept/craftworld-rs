# Orchestrator handoff (local PC session), 2026-10-10

Read CLAUDE.md (standing rules, loaded automatically), then this file, then
STATUS-CURRENT.md. The Claude Project is now only a phone status remote.

## How to work as orchestrator
- You plan, verify and merge; subagents do the jobs. One job per Agent-tool subagent
  (capture, PR check, Ghidra lookup, measurement), each returning a file path plus a
  3-line verdict. Parallel jobs get their own git worktree (E:\Projects\lbp2-wt-<name>)
  and a `CARGO_TARGET_DIR` from an existing `target-pr*` dir (disk rule in CLAUDE.md).
- Savvy Progress: run `/agents-info` to see the mod's progress view, and report
  subagent state through its `progress`/`step` tools so the owner sees it locally.
- Read before work: AGENTS.md (the owner's local, uncommitted edit holds the dated
  standing-rule lines; CLAUDE.md repeats them), the newest `docs/handoff-pc-*.md`.
- Implementers run as local `claude -p` workers on plan usage, branch
  `claude/<slug>` or `cloud/D<NN>-<slug>`, PR to main. Verify (fmt, test, clippy,
  census, gameplay-distance shots), post one verdict comment, merge on CI green.
- Never launch RPCS3 or viewer windows with focus; memory reads and headless checks first.

## State (STATUS-CURRENT.md is stale, dated 2026-10-08; trust git log)
- Main = 079f820 (#121). Merged since the last handoff: #117 baked AO, #118 cube
  reflection, #119 level selector tab with loading overlay, #120 `.cpv` crossplay
  levels marked unsupported, #121 CI fontconfig/Bevy libs (viewers job on main only),
  #102 level lighting blended from presets by PWorld lighting and darkness.
- Level lighting: live Moon = stored sun/amb x0.4725, rim x0.63 (preset 1, plan 0001a2fe);
  Pod = preset 0 (plan 0001d054). Bravery Test 0001e795 and tutorial_23 preset reads were
  an open RPCS3 job (see handoff-pc-2026-10-10a.md); #103 level fog verification may
  have no PR comment yet, redo it if so.

## Open items
- Cube reflection +X/-X face order is unconfirmed against a game capture.
- Shape bevels (D122) are inferred; parked until a RenderDoc capture of a bevelled
  shape draw exists (replay `vc[10].x = 10000` defect breaks positions).
- Level selector tooltip nit (cosmetic, open since the level selector landed).
- Leftovers from the idle threads: stale worktrees `lbp2-wt-pr99..pr112`,
  `lbp2-wt-base101/112`, target dirs `lbp2-wt-target101..112` (10-20 GB each); delete
  targets of merged PRs when space matters. `.claude/worktrees/agent-*` are old
  cloud-batch worktrees.
- RPCS3 measurements, X1/S9 (docs/rpcs3-measurement.md), need the owner at the controls.
- Local AGENTS.md edit and `docs/handoff-pc-2026-10-09*.md`/`10a` are untracked in
  E:\Projects\LBP2 Rust Rewrite; commit them when the owner agrees.

## Key paths and links
- Canonical repo: E:\Projects\LBP2 Rust Rewrite (main); remote
  github.com/chilladept/craftworld-rs.
- Launcher: `E:\Projects\LBP2 Rust Rewrite\Launch Asset Explorer (Levels).cmd` runs the
  build from worktree E:\Projects\lbp2-explorer-test (target
  E:\Projects\lbp2-explorer-test-target), snapshot
  `local-analysis/d68-inventory-0133-verified.json`, game dirs
  `game-data/a1-disc;game-data/a1-update-0133`. Rebuild there after main moves.
- Latest handoff: docs/handoff-pc-2026-10-10a.md (a copy is in the project files).
  Shading evidence: local-analysis/shading/ (lightscan.py, *-shading-*.md).
- Oracles: lbp2/oracles/memory/moon-sackboy.json; Toolkit pin under `references/`.
- Screenshot page https://claude.ai/artifact/UfbA32sCfkeRtnUDCJiy9u (newest capture per
  view); archive https://claude.ai/artifact/E1SfmBud7Eb7p3bWoFio7u. Read both before
  republishing; the Artifact tool wants the live version viewed first. Source:
  local-analysis/pc-helpers/screenshots-artifact.html.
- Pod capture command: docs/p2k one with d133-inventory-0133-edat.json +
  d133-edat-overlay, `--pod-game-camera --pod-open --lbp-lighting --window-16-9`.

## Coordinating with the Project
- The Project is a phone remote for status checks and will not start threads unless
  the owner asks, so it and this session never work the same task. If the owner starts
  a Project thread, take its branch as that thread's; do not touch it.
- To post status to the Project, the owner can ask; there is no automatic channel.
  Leave the result in a PR comment or the newest handoff, which the Project reads.
- Project memory (/mnt/project-files/lbp2/...) is not visible here. Everything that
  matters is in CLAUDE.md; add new standing rules there.
