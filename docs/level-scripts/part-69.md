# Level scripts, part 69: the Pod planets' turn against the game's captures (2026-10-15, D318)

Part 25 runs the planets' `PlanetThing.Update__rb` with a placeholder point, (0, 0, 1), which asks
for no turn (spin 0, pitch 0), because the game's point (the selected entry's place, else a vector
the planet object keeps) was not read. Here the Earth's and the Moon's orientation is read from
lane B's six GPU captures in the planets level's own frame. Research only, no code change: the
game's planets do not turn either.

### What was found

- MEASURED (RenderDoc 1.46 dumps of `local-analysis/d316-zoom/`): RPCS3's vertex programs leave
  clip y negated. Through the inverse of `vc[0..3]` alone the drawn vertices of both planets fit
  the model's only with a mirror (determinant < 0); with clip y negated back they fit a rotation
  and a scale (median residual 0.4 to 2.4 level units on an Earth of radius about 1640).
  Vertices are paired by order: a draw's post-VS vertices are its index range in order (Earth
  primitives 0 and 2, vertices 6 to 10935 and 15219 to 17656; Moon primitives 1 and 2, vertices
  5081 to 8103; positions placed per bone from the level file, part 68).
- MEASURED: taken back into the planets level through the runtime's World Thing matrix at the
  same tick (`LevelWorld::world_thing_moved`, shell and planets session, in use at 600), both
  planets sit at their class places, (-1250, 0, 0) and (2000, 0, 0) within 11 units, with axis
  lengths 0.960 to 0.962 (Earth, stored 0.9610) and 2.110 to 2.114 (Moon, 2.1121), and turned by
  at most 0.025 degrees, at every capture:

| capture | tick | Earth spin, pitch (rad) | Moon spin, pitch (rad) | point that gives it |
|---|---|---|---|---|
| 6082 | 600 (closed, before in use) | -0.0001, -0.0003 | 0.0000, -0.0002 | (0, 0, 1) |
| 7295 | 606 | -0.0001, -0.0003 | 0.0001, -0.0002 | (0, 0, 1) |
| 7927 | 609 | -0.0001, -0.0004 | 0.0001, -0.0004 | (0, 0, 1) |
| 8348 | 611 | -0.0001, 0.0000 | 0.0000, 0.0000 | (0, 0, 1) |
| 8980 | 616 | 0.0000, 0.0000 | 0.0000, 0.0000 | (0, 0, 1) |
| 9601 | 619 ("Story" selected) | 0.0000, 0.0000 | 0.0000, 0.0000 | (0, 0, 1) |

  Spin is the turn about y and pitch about x of the fitted rotation; at pitch 0 the script's spin
  and its spin offset are one turn about y, so the table gives their sum (the runtime's
  `SpinOffset` is 0 on every one of these ticks). The point is the direction in the planet's
  frame that the script turns to +z: any point (0, 0, z > 0).
- So the game's point gives no turn before "in use" (the planet's own vector, INFERRED: along
  +z, its length not measured) and the planets do not turn from 600 to 619: no `Smooth2D` easing
  toward the "Story" entry within these ticks (whether that entry's place lies on +z, or the
  entry is not yet on the planet at 619, is not established). The viewer has spin 0, pitch 0 and
  spin offset 0 on every tick: the game's turn.
- MEASURED: the World Thing matrix the runtime computes is the game's at all six captures (the
  planets' places and turn above are read through it). Part 67's extra turn of the planets world
  about the eye (2 x the camera's pitch change - 0.2 s) was read without the y flip: INFERRED,
  the mirror's (a reflection through the view's x-z plane turns a rotation's sense). There is no
  extra turn to find.
- MEASURED (land centroid inside the limb, `r > b + 10` within 0.85 of the fitted ellipse, in
  radii from the centre): game scene targets at 600 / 611 / 616 (0.220, -0.238) / (0.181, -0.249)
  / (0.128, -0.246); the viewer's d320-fix stills (0.221, -0.231) / (0.173, -0.247) / (0.123,
  -0.254). The Earth shows the same face within 0.01 radius (about 0.5 degree). `rt20_8348_up`
  and `rt20_9601_up` are final frames with the menu over the Earth (their 0.220 at 611 is the
  menu's colour), not usable for this.

### What the runtime does

Unchanged: `PLACEHOLDER_PLANET_POINT` (0, 0, 1) gives the measured turn on these ticks.
`set_planet_point` stays for the entries' places.

### Verification

| tick | game land centroid | viewer (d320-fix) | Earth turn in the level frame (game) | viewer |
|---|---|---|---|---|
| 600 | (0.220, -0.238) | (0.221, -0.231) | 0.017 deg | 0 |
| 611 | (0.181, -0.249) | (0.173, -0.247) | 0.007 deg | 0 |
| 616 | (0.128, -0.246) | (0.123, -0.254) | 0.001 deg | 0 |
| 619 / 625 | menu over the Earth | - | 0.002 deg | 0 |

RenderDoc dumps reused (no new replay); the planets' model vertices from pod_planets.bin through a
local probe (not committed); the runtime's World Thing matrix and the planets' members from a local
session probe. No RPCS3 run; no new viewer stills (no code change; main's d320-fix stills used).
Scripts and fits stay in ignored `local-analysis/d321-planet-turn/`.

Counts: +0 newly viewable / +0 newly Complete (research).
