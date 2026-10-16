# Handoff: level globals, part 4 (2026-10-16, PC)

Read first (max 5):
- `docs/level-globals.md` sections D332, D333 and "Research for the next PRs"
- `local-analysis/level-globals/fog-tint-research.md` sections "Q4. Water" and "Water (full
  read)" (the second is appended by part 4's background agent; check it is there)

## State

- PR 95 (fog tint, D332, branch `globals/fog-tint`): CI green, NOT merged. The PC's auto-mode
  classifier blocked `gh pr merge` ("Merge Without Review") and repo auto-merge is off; the
  user was asked in the thread to merge it or say "merge 95".
- PR 96 (fog, D333, branch `globals/fog`, worktree `E:\Projects\wt-globals2`, target dir
  `E:\Projects\target-globals`): stacked on 95. After 95 merges: rebase onto origin/main,
  push, mark ready, merge on green.
- Water: research running at handoff time; no code yet.

## Facts

- Toolchain: `. "E:\Projects\LBP2 Rust Rewrite\scripts\use-local-rust.ps1"` (the worktree has
  no local Rust of its own). CI does not run viewer tests; run them locally.
- Viewer check level: Rocket Funland
  (`--level-path gamedata/lbp2/th_cosmos/final_levels/comp_01_rocket_funland/comp_01_rocket_funland.bin
  --level-scripts --level-scripts-hold-at 600 --play-sackboy`): at about tick 291 the scripts set
  fog 0.3 and fog tint 0.875. Hidden-window capture script form: `local-analysis/rscript/stage6/scratch/d287-shot.ps1`;
  shots go to `local-analysis/level-globals/shots/` (local only).
- The light follower is `LightFollower<4>` over (lighting, darkness, fog tint, fog)
  (`SettingsInForce::light_and_fog`); `LevelRelight::light` takes all four.
- Open: the game's darkness scales Sun, Rim, Rim2 and Ambient (all lanes), not the exposure
  (0xac200). `at_lighting` scales the exposure (fitted to the Bravery Test live read). Settle
  this with a live read of the block before changing it.

## Next

1. Merge 95, then rebase and merge 96 (merge reply in the thread: PR link + one-line summary).
2. Water PR from the research note's smallest step; then the darkness question above.
