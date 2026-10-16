# Level scripts, part 65: the open room's light against the game's (2026-10-15, D314)

Viewer, and a RenderDoc read of the open room's draws in capture 8980 (viewer tick 616). Part 64
left the viewer's open room sharper, brighter and with more contrast than the game's scene
before its post chain. This part finds why: the viewer turned the planets level's sun with the
world Thing's root, and the game does not.

### What was found

- FOUND (RenderDoc 1.46, 8980, `local-analysis/d314-mip/`, each draw's change to the main
  target read in its stored row order): three draws reach the box (scene x 47-207, y 442-559)
  before DoF 1. 2400 is the room's wall (921 vertices, 2014 strip indices) with a 256 x 256
  BC1_SRGB diffuse (r15693, at 2 x UV0), a 256 x 256 BC1_SRGB second colour layer (r15636, at
  UV1) and a 256 x 256 BC3 normal map (r15701, at 2 x UV0), plus a specular term (power 22).
  2416 is the same mesh again with the mural (256 x 512 BC3_SRGB, r15924, at UV1), alpha tested,
  no normal map and no specular: it is the last writer on 12,341 of the box's 18,720 pixels.
  2467 (another mesh, 1024 x 512 / 512 x 512 textures) covers 2,474.
- FOUND (the same draws): every sampler is LINEAR / LINEAR / LINEAR mip, anisotropy 1, LOD bias
  0, LOD 0 to 15, wrap; the programs sample with plain `texture()`. Each texture carries the
  disc's mips down to 4 x 4 (7 or 8 levels).
- MEASURED (post-VS positions and UVs, interpolated per pixel; the GL rule log2 of the larger
  texel step): the mural is magnified (LOD -0.74, mip 0 only) and so is r15636 (-1.6); the
  wall's diffuse and normal map sit at LOD 1.4 (mips 1 and 2).
- MEASURED (the disc's mips against the viewer's box-filtered chain, `adapter.rs` 426-448, built
  from the same mip 0): the disc's are softer, mean gradient x 1.50 (diffuse) and x 1.48
  (normal map) at mip 1, x 1.17 / x 1.24 at mip 2, the mural's x 1.17 to x 1.25. They decide
  only the 1,145 wall pixels the mural leaves, whose sharpness relative to their contrast is the
  game's (x 0.96).
- MEASURED (box, game against main): the extra detail is the same in every band (x 1.10 to
  x 1.20, highest at the coarsest), and relative to its contrast the box is the game's (x 1.02):
  part 64's "sharper" is contrast, not a sharper mip.
- FOUND (2416's program and constants): out = sqrt(4 x tex x L^2), then the fog, with
  L = ambient x (N.y 0.15 + 0.5) + sun x saturate(N.L) + saturate(rim colour x (N.V + 0.9)^2),
  N the normalised TC2. Ambient fc11 (0.1164, 0.4084, 0.5486) and sun fc12 (0.3641, 0.2007,
  0.0370) equal the viewer's (`raw_key` already holds the sun multiplier). The sun is a point:
  fc0 = (222886.6, 160217.0, 298809.0), the level's sun x 300,000, not turned. The fog share
  is (w - 7885) / 52649 here (w the clip w, fit exact), 0.37 on the mural.
- MEASURED (this model against the game's pixels, the texel taken from the viewer's unlit still):
  within 0.2 % in every channel. The viewer's unlit texel equals the game's (x 0.997).
- MEASURED (temporary debug builds, not kept): the viewer's normal on the mural is the game's
  (median (0.529, -0.224, 0.820) against (0.520, -0.223, 0.825)), its fog share 0.33. Its
  light was x 1.07 to x 1.09 the game's in every channel, all of it the sun term: N.L 0.93
  against 0.79. At 616 the planets root (under the world Thing) is turned 19 degrees, mostly
  about x, and the viewer turned the sun with it to (0.498, 0.120, 0.859). The game turns the
  geometry (its TC2 is the turned normal) but keeps the sun where the level puts it.
- INFERRED: the dolly's root (`planets_root`, no level scripts) is the same motion in the game,
  but no capture of it was compared, so it keeps turning the light.
- FOUND (2400's, 2416's and 2453 / 2469's programs): the sun is a point. The programs take
  `r = (TC4 - fc0) x fc.x` (0.000233) and use `-dot(N, r) / |r|`: the scale cancels, and no
  other use of the distance follows (no falloff; fc.y 94.5 is not read by these programs).
- FOUND: the 300,000 is the level settings' `sun_position_scale` (pod_planets.bin; the viewer's
  log prints it). INFERRED: fc0 is that point less the eye, as TC4 is the position less the eye:
  fc0's x is 13 short of the level's sun x 300,000 in both frames, while y / z move by 49.5 /
  227.7 between 5247 and 8980 (the eye's move).

### What the viewer does

`lbp-bevy-viewer`, `main.rs`: `apply_lighting` takes the planets layer's light turn from
`planets_light_turn`, which is no turn for a root that hangs under the world Thing
(`level_play::WorldThingRoot`) and the root's rotation otherwise. Under that root, on the
level-lighting route, the planets layer lights from the sun as a point (`planets_sun_point`):
`PreviewLight::sun_display` is `(sun - centre) x scale`, the level's sun in the display frame
(the root's world is that frame turned with the world Thing, so the point is not turned), and
the shader's `toward_light_at` takes `normalize(sun_point - position)` for the sun term, the
gloss and the glass's specular (`dual.rs`, uniform 146). Everywhere else the direction stays.
The shell layer is unchanged.

### Measured

Release build, hidden window, level lighting (`local-analysis/d314-mip/`, `vshot.ps1` /
`shot.ps1`, prefix d314; main is `target-d313`, 205c7c7). Open room box, scene before the post
chain (all post off) against 8980 before event 2706:

| | game | main | sun unturned (direction) | this build (point) |
| --- | --- | --- | --- | --- |
| mean colour | (151, 135, 110) | (158, 142, 114) | (148, 135, 112) | (149, 136, 113) |
| \|mean d\| | | 5.7 | 2.0 | 1.8 |
| luma 5 / 50 / 95 % ratio | 1 | 1.04 / 1.05 / 1.05 | 0.99 / 1.00 / 1.00 | 0.99 / 1.01 / 1.01 |
| contrast (sd) ratio | 1 | 1.10 | 1.02 | 1.04 |
| gradient | 1.008 | 1.136 | 1.056 | 1.068 |
| gradient over the mural, ratio | 1 | 1.26 | 1.18 | 1.19 |
| registration | | 0.966 | 0.983 | 0.983 |

- After the whole chain, presented pixels: the open room 0.482 game, 0.544 main, 0.500
  direction, 0.508 point (warped; mean (154, 137, 111) / (161, 143, 114) / (150, 136, 113) /
  (152, 137, 113)); the desk box 1.542 / 1.480 / 1.664 / 1.629.
- Left: red 1.5 % low and blue 2.3 % high on the mural (the fog share 0.33 against 0.37,
  INFERRED), and the mural's detail relative to its contrast x 1.12 with the same mip 0 on both
  sides (not resolved).
- Closed Pod at 600 (direction): 418,467 pixels change by more than 2 (mean |d| 1.64; the room
  and the planets x 0.955 / 0.973 / 0.994, the root is already turned there). Da Vinci
  (0001e786): byte-equal to main with the direction; with the point 126 pixels differ by at
  most 5 (mean 0.0004; no point is set there, so the shader's rebuilt code, INFERRED).
- FOUND (capture 5247, the Pod closed; room draws 2453 / 2469): the same sun, (222886.6,
  160167.5, 298581.3), unturned, and the same constants, while the room's normals are turned
  10.3 degrees from 8980's (the viewer's root: 9.5 degrees at 600, 19.3 at 616). Over the room's
  vertices N.L is 0.876 with the game's point sun, 0.846 with the unturned direction and 0.896
  on main (turned).
- MEASURED against 5247's final output (`closed.py`, |mean d|, main / direction / point): the
  room through the window 3.6 / 5.8 / 4.7, the globe 1.7 / 4.5 / 3.3, the left glass 1.4 / 3.9
  / 2.5, the right glass 0.4 / 4.7 / 0.4. The point brings the closed Pod back toward the game
  but not as close as main: main is nearer in three regions of four (equal in the fourth), so
  the closed Pod does not confirm the fix by itself.
- MEASURED (5247 term by term, `closedterms.py` / `closedL.py`; the room through the window is
  draw 2520, the program of 8980's 2467, and the globe 1337; the viewer at 600 with the point
  sun, all post off, read from a temporary debug build that wrote the pre-fog colour, the fog
  share, the normal and the direction to the sun in a 2 x 2 pattern): on the room the fog share
  is 0.556 / 0.549 (game / viewer), the normal (0.659, -0.092, 0.747) / (0.655, -0.090, 0.749),
  the direction to the sun (0.523, 0.331, 0.785) / (0.522, 0.333, 0.788), N.L 0.901 / 0.900
  (main's turned sun: 0.920). The light implied by the pre-fog colour and the viewer's unlit
  texel is the geometric model's x 1.005 in the game and x 1.000 in the viewer (medians). On the
  globe the normal differs by 5 degrees (n.y -0.215 / -0.153): the planet's own turn, a state
  difference, not the light.
- FOUND (replay artefact, part 64): a 5247 target read after other events carries the flat
  (164, 104, 58) tiles (2,420 of the room's pixels, 17,520 behind the left glass); each target
  above was read again in a fresh replay at its own event. The first room means of this part
  (pre-fog x 1.06 to 1.10) were those tiles.
- MEASURED (clean 5247 target after 2928, before the post chain, |mean d| main / point): the room
  through the window 0.02 / 0.79 (|d| 2.03 / 1.89); behind the left glass 7.4 / 8.3 and the right
  5.3 / 5.4 (registration 0.60 / 0.77, the shell's frame over them). After the post chain (clean
  final output): the room 3.8 / 4.9, the globe 1.9 / 3.5, the left glass 1.4 / 2.5, the right
  0.4 / 0.4.
- INFERRED: the point sun gives the game's terms; main's closer means at 600 come from its
  turned sun (N.L 2 % high) offsetting a 0.5 % shortfall elsewhere in the room's light (not in
  the fog, the normal, the sun or N.L), and the post chain leaves both builds 3 to 5 levels
  darker than the game on the room, which the final comparison carries. No further fix is
  supported by these terms.
- MEASURED (Da Vinci's 126 pixels): thin vertical runs at polygon edges (x 222-1043, y 111-415),
  at most 5. A debug build that paints every fragment with the point on gives none there, and a
  build without the point branch gives the same 126 pixels: the change of compiled shader (the
  new uniform and private variable), not the point path. Left as is.

### Verification

`lbp-bevy-viewer`, no game data:

- `d314_world_thing_root_keeps_the_sun_in_world_space`.
- `d314_open_room_sun_term_matches_the_capture`: the box's median normal against the kept sun
  gives the capture's N.L 0.79 (within 0.03); the turned sun gives above 0.9.
- `d314_point_sun_only_under_the_world_thing`, `d314_point_sun_direction_from_the_surface`
  (the box's median normal and position: 0.827 from the point, 0.808 from the direction) and
  `d314_sun_display_is_the_level_sun_in_display_units`.

fmt and `cargo clippy --all-targets -D warnings` pass. The viewer's tests pass (156), except
the one that fails on main (`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
