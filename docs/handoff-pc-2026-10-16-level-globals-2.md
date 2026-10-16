# Handoff: level globals, part 2 (2026-10-16, PC)

Read first (max 5):
- `docs/level-globals.md` sections "D331: colour correction" and "Research for the next PRs"
- `crates/lbp-bevy-viewer/src/colour_grade.rs` (the whole D331 model)
- `local-analysis/level-globals/final-pass-shader.md` (PS 14857 decoded: slots, uv offset)

## State

- D331 colour correction: branch `globals/D331-colour-correction`, worktree
  `E:\Projects\wt-globals2`, target dir `E:\Projects\target-globals` (release build current).
  PR open, NOT merged: it waits for the game comparison below.
- Nothing running; RPCS3 not launched in this part.

## Facts

- Presets: `gamedata/data/colcorrectpresets.raw` in DATA.FARC (904 bytes: u32 revision, u32 0,
  8 x 28 BE f32); read at level load by `colour_grade::load_presets`.
- Only story level with a non-zero setting: Cosmos `main_03_maingrapple` (GUID 0001f527,
  5.77; its `_b` variant 00020bff 5.76). Viewer shot:
  `local-analysis/level-globals/shots/d331-grapple-on.png` (local only).
- Viewer capture command: `local-analysis/rscript/stage6/scratch/d300-all.ps1` form, with
  `--level-guid 0001f527 --level-scripts --play-sackboy`, output under local-analysis.
- Notes from this part (local only): `e6e04-colour-matrix.md`, `cc-table-source.md`,
  `final-pass-shader.md` in `local-analysis/level-globals/`.
- Shader slot mapping and "top = top of frame" are INFERRED; matrix math and presets FOUND.

## Next

1. Game frame of maingrapple's start in RPCS3 (owner save; patches OFF; minimized), compare
   with the viewer shot, then merge D331 (rebase on main first).
2. Darkness / fog tint colour (block offset 0x10 of 0xac6c0), then fog, then water.
