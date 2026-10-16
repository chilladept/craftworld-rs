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

## Research for the next PRs (not applied yet)

- Colour correction: the setting is a position over eight records the engine keeps (0x316a7c:
  index = floor, the next one blended by the fraction, clamped to 0..7; two tables of eight,
  picked by a flag). Each record holds a tint, saturation, hue, contrast and brightness pair,
  and two vignette colours (record 0: identity grading, vignette 0.2 grey, the final pass's
  0.2 floor of part 45). The final pass (0x323354) turns the blended record into the two
  colour-matrix constants part 45 saw as identity (0xe6e04). Read from RPCS3 memory at boot.
- Darkness and fog tint: the light-block function (0xac6c0) blends the stored presets at the
  lighting value (the P3m rule), then scales one colour of the blended block by 1 - darkness
  (masked lanes) and, for fog tint above 0, replaces it with a blend between that colour and a
  palette of 15 (or 7) colours at fog tint x a constant. Which field that colour is (block
  offset 0x10) is not settled yet.
- Fog: stored with the others on the settings Thing (0xacc7c); its use in the renderer was not
  read yet.
