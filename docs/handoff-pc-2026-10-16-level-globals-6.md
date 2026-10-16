# Handoff: level globals, part 6 (2026-10-16, PC)

Read first (max 5):
- `docs/level-globals.md` sections D334 and D335
- `crates/lbp-level/src/parts.rs` `LevelSettings::at_lighting`

## State

- PR 97 (D334 water) is merged, with a shader fix: blended draws (`AlphaMode::Blend`) now
  write the material's base alpha (`dual.rs`, `ALPHA_MODE_BLEND`); before, they drew opaque,
  which made every Whale Tale capture flat pink. Rocket Funland capture unchanged.
- PR 98 (D335, branch `globals/darkness`, worktree `E:\Projects\wt-globals2`) is ready:
  darkness dims sun, ambient, rim, rim2 (all lanes) and fog RGB by 1 - D, not the exposure
  (01.33 0xac200 LAB_000ac50c, 0xac6c0). Merge on green CI if not merged yet.
- All five level globals (gravity, colour correction, fog tint, fog, water) plus darkness
  now have a PR each. The lane's list in `docs/level-globals.md` is done.

## Facts

- Whale Tale default play spawn (checkpoint z + forward offset, z 200) has no floor: Sackboy
  falls ~28000 units. `--play-sackboy-at -13602,30400,-50` stays at the start checkpoint
  (Y unchanged, may be stuck). Good capture: `local-analysis/level-globals/shots/d334d-final-spawn.png`.
- Water alpha 0.35 is a PLACEHOLDER; the body's front face is at level z 622 so it tints
  near objects below the surface too. Pink at tint 0.3 not checked against the game.
- Bravery Test live sun RGB is 0.96 of the predicted value (all three lanes); both darkness
  rules predict the same there. Unexplained.
- Sticker decals also use `AlphaMode::Blend`; they now blend by their material base alpha.

## Next

1. Merge PR 98 if still open (merge reply: PR link + one-line summary).
2. Candidates: the Whale Tale spawn rule; the sun RGB 0.96 residual (live read);
   the water's underwater pass (murkiness, shader parameter 0x4da2db17).
