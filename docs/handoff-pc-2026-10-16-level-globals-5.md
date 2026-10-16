# Handoff: level globals, part 5 (2026-10-16, PC)

Read first (max 5):
- `docs/level-globals.md` section D334
- `crates/lbp-bevy-viewer/src/water.rs` and `spawn_water` in `main.rs`, `follow_water` in
  `level_play.rs`
- `local-analysis/level-globals/fog-tint-research.md` section "Water (full read)"

## State

- PR 95 (D332) and PR 96 (D333) are merged.
- Water PR (D334, branch `globals/water`, worktree `E:\Projects\wt-globals2`, target dir
  `E:\Projects\target-globals`) is a draft: unit tests and clippy pass, viewer look NOT
  verified yet.
- Viewer check so far: the load line works (Whale Tale logs `level 0.698 -> surface Y 29792.5,
  tint 0.300; 9 colours`, Rocket Funland logs `none`). Hidden-window captures
  (`Start-Process -WindowStyle Hidden`, hold-at 120/600) gave a flat fog colour or a wall,
  with and without the water body, so they show nothing about the water. A capture agent was
  finding the working capture form (the D333 run put the window on the 2nd monitor without
  activation) and writing `local-analysis/level-globals/shots/d334b-*-a35.png` / `-a0.png`.
  Its result did not reach this thread; check those files exist.

## Facts

- Story levels with stored water (PWorld water_level > 0.0001): Whale Tale
  (`th_eve_assylum/final_levels/side_01_whale_tale`, L 0.698, tint 0.3), garage outro
  (`th_henry_garage/final_levels/cutscene_outro_theme4`, L 0.469, tint 1, waves 1.67), eve
  intro cutscene (L 0.076), Mind Control (`side_02_mind_control`, L 0.069, tint 0).
- The body is a unit cuboid scaled over the level's bounds (backdrop excluded) from bounds
  bottom to the surface, `ViewerMaterial` unlit + `AlphaMode::Blend`, alpha 0.35
  (PLACEHOLDER), pushed into `v.scene` so `clear` removes it. Palette from texture GUID
  0x19bf3 via `decode_texture`.
- Toolchain: `. "E:\Projects\LBP2 Rust Rewrite\scripts\use-local-rust.ps1"`.

## Capture result (arrived after the handoff)

- Working form: run the exe directly (no Start-Process; `--qa-capture` already shows the
  window unactivated on the 2nd monitor), hold-at 600. Whale Tale needs
  `--play-sackboy-at "-13602,30320,0"` (the default spawn has no floor; Sackboy falls to
  Y about 1726 and the frame goes flat pink). `--play-sackboy-input "-:700"` logs his position.
- Shots `d334b-whale_tale-a35/-a0.png`: good gameplay frame, but **the water body is not drawn**
  (identical at alpha 0, 0.35, 1). The surface Y 29792 is below the floor at 30074, so the body
  should show below the platform. Suspects: `thing_view`'s follow hiding `v.scene` entities that
  map to no Thing, or the body's Z span/transform. (The exe had a temporary `TMP_WATER_ALPHA`
  env knob, removed from the source before commit.)

## Next

1. Verify the water body in a working capture (Whale Tale near the spawn); fix if the body
   hides the level or sits wrong (check the cuboid's Z span against the camera).
2. Mark the water PR ready, merge on green CI (merge reply: PR link + one-line summary).
3. Then the darkness question (part 4 handoff "Open").
