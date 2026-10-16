# Level globals: gravity, colour correction, fog, water

Follows `docs/rscript-plan.md` stage 5 PR 3 (D267, `docs/level-scripts.md` part 18): the
scripts' global settings are kept and blended (`WorldState::settings_in_force`), lighting and
darkness light the level again, and fog, fog tint, colour correction, gravity and water were
left unapplied. This file records what the game does with each of them and what the viewer
applies, one PR per setting. Evidence: the 01.33 executable (Ghidra bridge, local only) and
RPCS3 memory reads (no writes, window minimized and never activated).

## D330: gravity

### What the game does (FOUND, 01.33)

- `Game.SetGlobalSettingsGravityTo(value, blend)` (0x5fbfd4 -> 0x5edc14 -> 0xcef3c) keeps the
  PWorld's start, target and blend factor (PWorld+0x5ac, +0x5b0, +0x5bc) as part 14 found, and
  then writes `setting x -2.3 - 0.4` into the y lane of the physics world's gravity vector
  (the other lanes kept), with the setting as now blended.
- `Game.SetGlobalSettingsBlendFactor` (0xd623c) writes the same lane again for the gravity type
  each time a script steps the blend, so the world's gravity follows the blend tick by tick.
- `Game.GetWorldGravityStrength` (0x5fd4c8 -> 0x5eb284) returns that vector's length.
- A gravity-ratio getter (0xce6f0, called from three places) divides the same expression by
  -2.7 and keeps it at least 1.0. So the normal setting 1.0 gives 2.7 per tick², the value
  `lbp_sim::sackboy::GRAVITY_TICK` was measured at (2.698, RPCS3 falls).

### What changed

- `lbp-sim`: `gravity_per_tick(setting)` = 2.3 x setting + 0.4, `NORMAL_GRAVITY_PER_TICK`
  (2.7) and the resource `WorldGravity { setting }` (1.0 by default) with `scale()` = the
  world's gravity over the normal one. `SimPlugin` sets avian's gravity to `GRAVITY x scale`
  when the resource changes; the Sackboy controller falls by `GRAVITY_TICK x scale`
  (`Sackboy::step_with_gravity`; `step` keeps `GRAVITY_TICK`).
- Viewer (`level_play.rs`, `follow_gravity`): with `--level-scripts` and a physics world
  (`--play-sackboy`, the scripted Pod's Sackboy) the gravity setting in force is handed to
  `WorldGravity` after each tick, one line per change. The scripts window's global-settings
  line lists gravity as applied.
- At the normal setting nothing changes (scale exactly 1.0).

### Not modelled

- The jump profile at another gravity (Sackboy's take-off speeds are a measured table; the
  game's rule for them under other gravity was not read), and what the gravity-ratio getter's
  three callers do with it.
- Material gravity multipliers (as before).

### Verification

`cargo test -p lbp-sim` (new `d330_fall_follows_the_gravity_setting`), fmt and clippy. Over
378 levels part 18 counted the gravity setting unlike the file in 3 / 11 / 15 levels (plain /
trigger visit / pulsed); none of them was played with Sackboy here.

## D331: colour correction

### What the game does (FOUND, 01.33)

- The setting is a position over eight preset records read from
  `gamedata/data/colcorrectpresets.raw` (GUID 31250; the Pod's `colcorrectpresetspod.raw`, GUID
  127928, holds the same records in 01.33): two u32 words, then 8 x 28 big-endian floats
  (loader 0x5e0814, parser 0x5df7e8; equal to the RAM tables read from RPCS3). Record floor(v)
  is blended with the next by the fraction, 7 from 7 up (0x316a7c). The world builds a record
  for the blend's start and its target and blends the two by the lighting group's factor
  (0xda43c, 0xd978c).
- Record: two transforms (tint rgb + weight w; saturation, hue in radians, contrast,
  brightness), a picked colour (rgb, gain w), the vignette's top and bottom colours.
- 0xe6e04 makes each transform a 4-row matrix for `(c, 1) x M`: hue rotation about the grey
  axis, saturation with weights (0.2, 0.7, 0.1), `contrast x (c + brightness - 0.5) + 0.5`,
  tint `c x (w + (1 - w) t) + w t` (checked against an emulation of the function's
  instructions on three preset transforms).
- The final pass (PS 14857, RPCS3 raw ucode): `c = scene x 1.2998 + glow`,
  `g = lerp(c x M0, c x M1, saturate(pick.w x |c - pick.rgb|))`, then
  `g x lerp(top + v x (bottom - top), 1, smoothstep(0, 1, saturate(1.2 - r)))`, then the mix
  back by the scene's alpha. Record 0 is identity with a 0.2 grey vignette (part 45's frame).

### What changed

`colour_grade.rs` (matrices, record blend, presets file, the uniform and its WGSL); the final
entries of `glow_post.rs` call `graded(..)`; `level_play.rs` reads the presets with the level
and hands the record in force to the pass every tick (`follow_colour_correction`).

### Not modelled

- MEASURED, not FOUND: the top colour is at the top of the frame. Over 20 game frames of
  maingrapple the top band's channels stand to the bottom band's as 1.20 / 1.41 / 1.10
  (r / g / b); the record predicts green highest, then red, then blue with top at the top, and
  the reverse order otherwise. The flag at 0xe31e34 (saturation x 0.8; a 0/1/2 render state,
  possibly cinematic) is taken as clear.
- The pass's uv offset by a neighbour difference of a single-channel buffer (`u3`, gated at
  0.025), new in this read; part 45 did not have it.

### Verification

`cargo test -p lbp-bevy-viewer` (d331 tests; the d294/d300/d322 shader tests updated), clippy.
Viewer: Cosmos `main_03_maingrapple` (setting 5.77, the only story level not at 0) logs the
record and draws graded.

Game (RPCS3, 2026-10-16, "Where In The World Is Avalon Centrifuge?", patches off): FOUND the
slot mapping. The live copies of PS 14857 in guest memory hold M0 = matrix(Matrix[0]) in fc19,
fc17, fc23, fc25, M1 = matrix(Matrix[1]) in fc20, fc14, fc21, fc22, ColPick in fc16 and fc24,
VignetteTop in fc2 and fc12, VignetteBottom in fc8 and (0, 0, 0, 1) in fc28: the viewer's
record at 5.77 within 1e-3 (`d331_record_matches_the_live_constants`). The level's start in the
game does not look like the viewer's at the same place (a scrap shell over an orange planet,
against the viewer's toy shop); that is the level's scene, not this grade.

## D332: fog tint

### What the game does (FOUND, 01.33)

- The light-block function (0xac6c0) blends the stored presets at the target lighting (P3m)
  and takes the block's fog colour (offset 0x10; the block's field order, from the field
  visitor 0xb3f0b0: SunColor, FogColor, RimColor, RimColor2, AmbientColor, SunPosition, then
  the floats). Darkness scales its rgb by 1 - darkness, alpha kept (lane mask 0xc3bf60).
- Fog tint T > 0 then replaces it from a ring: entry 0 is that colour, entries 1..N the
  palette `FogColours_0..14` (N = 15) of `gamedata/data/game_constants.con` (GUID 52107, a
  text file; visitor 0xaa0198), or `PodFogColours_0..6` (N = 7) when the level is the Pod's
  (`pod_planets.bin`, GUID 102363; 0xd0e80). `x = 8 T`, `i = trunc(x) mod (N + 1)`, the next
  entry `j = (i + 1) mod (N + 1)`, `c = P[i] + (P[j] - P[i]) x frac(x)`, all four lanes. T is
  clamped to 0..2 (0..1 in the Pod): T = k/8 is palette colour k - 1, T = 2 is the level's own
  colour again.
- The shown colour then eases to the new one over the lighting group's transition
  (0xac31c..0xac330); it is the fog colour constant and the frame's clear colour.

### What changed

`LevelSettings::tinted_fog` / `with_fog_tint` (`lbp-level`), applied after `at_lighting`;
`fog_tint.rs` reads the palettes with the level; the level's light at load and on every
relight (`LevelRelight::light`, the follower now follows lighting, darkness and fog tint).

### Not modelled

- The ease of the shown colour over the transition: the viewer relights in steps
  (`RELIGHT_STEP`) at the values in force, which already follow the blend.
- Darkness on the other colours: 0xac200 scales SunColor, RimColor, RimColor2 and
  AmbientColor by 1 - darkness on all four lanes and does not scale the exposure, while
  `at_lighting` scales the exposure (P3m, fitted to the Bravery Test live read). Not changed
  here; to settle with a live read of the block.

### Verification

`cargo test -p lbp-level` (`d332_fog_tint_runs_over_the_level_colour_then_the_palette`),
`-p lbp-bevy-viewer` (`d332_palettes_read_in_order_and_stop_at_a_gap`), clippy.

## Research for the next PRs (not applied yet)

- Fog (FOUND): with F the eased fog setting in -1..1, `FogNear += F (0 - FogNear)` and
  `FogFar += F (1000 - FogFar)` after the preset blend (0xac200, for every preset count), then
  the script bias (`Cam.SetDOFAndFog` third value) is added to both (0x34fc48); no fog when
  far <= near; the vertex fog factor is `(view depth - near) / (far - near)` (0x317bac).
- Water (partly read): the tint picks from a 9-colour list at 8 x tint, murkiness x 0.7 goes
  in the same vector's w (shader parameter 0x4da2db17); water level and waves not read.
