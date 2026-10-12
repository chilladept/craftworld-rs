# D241: landing dip against the game's vertices (2026-10-12)

Owner (2026-10-11): "sackboy still seems extra bouncy like the model is stretching vertically a bit".

## Measured (RPCS3 vertex capture `local-analysis/d234-anim-1to1/jump.bin`, Moon, standing jump)
- Vertical arc: game centroid rise per tick 12.8, 31, 53, 78, 104, 129, 152, 172, 189, 203 ... apex +234;
  ours (body y + pose) 12, 31, 55, 80, 105, 130, 152, 171, 188, 202 ... +232. Matches; no change.
- Landing, lowest vertex relative to rest, one value per tick from the tick the feet reach the floor: +0.4,
  -29.0, -16.2, 0.0, +1.4, +2.6, +4.6, +0.7, -3.2, -3.1, -2.0, -1.3 ... (rest 0.0; idle holds -0.0).
  Corrected in D247: this list used to start -26.7, -29.0; the -26.7 came from a half-written vertex
  buffer read, and the clean per-tick series has the feet at the floor on that tick. He is below rest for
  two ticks, not three.
- Ours before: dip table -26.2, -35, -35, -31.8, -25.3, -15.6, -8.6, -4.3, -2.5 ... (15 ticks) plus the jump
  clip tail's feet 4.5-6.0 below the root: lowest point about -41, 12 below the game, and 10+ ticks under rest.
- Idle: the game's top vertex moves 5.3 (201 -> 206.4) with feet fixed; ours moves 23, but that is the 408
  extra vertices of mesh 0000043f (2920 vs the game's 2512); the ICP fits of the stand clip stay 1.3-1.6.

## Change
`LANDING_DIP` (lbp-sim) = game lowest point series minus the pose's 5.5: -21.2, -23.5, -10.7, 0. Visual only.
Superseded by D247 below.

## D247: landing on the touchdown tick, two-tick dip (2026-10-12)
MEASURED: d234 `jump.bin` (standing 6-tick jump, one vertex buffer per tick) against a release viewer run of
the same input, tick-aligned on the take-off; k = ticks from the tick the fall ends on the floor. Height =
lowest to highest vertex; lowest = lowest vertex relative to rest.

| k | game height | ours before | ours after | game lowest | ours before | ours after |
|---|---|---|---|---|---|---|
| -2 | 199.7 | 199.9 | 199.9 | 62.5 | 61.7 | 61.7 |
| -1 | 201.8 | 201.9 | 201.9 | 28.8 | 28.1 | 28.1 |
| 0 | 201.6 | 201.9 | 201.9 | 0.4 | 1.0 | 1.0 |
| +1 | 206.0 | 201.9 | 206.3 | -29.0 | -20.2 | -29.0 |
| +2 | 202.9 | 206.3 | 202.6 | -16.2 | -26.6 | -16.1 |
| +3 | 194.9 | 202.6 | 202.6 | 0.0 | -10.9 | -1.0 |

- Timing. The game's mesh is on landing tail frame 15 one tick after the feet reach the floor; ours held
  frame 14 for one more tick. Cause FOUND in our own code: `SackboyBody::grounded` was the floor probe taken
  before the tick's move, so the tick the fall was stopped by the floor still read as airborne. It is now
  the contact after the move (a fall stopped by a floor counts on that tick). The controller step still
  gets the probe, so jumps, lane changes, the dip counter and walking off a ledge are timed as before.
- Dip. Re-derived after the timing fix as the game's lowest point minus our pose's lowest point on tail
  frames 15 and 16 (-3.1, -0.3 against the standing pose): `LANDING_DIP` = -25.9, -15.9, 0.
- Flight (take-off to the last airborne tick, 27 ticks) is unchanged by both: height max 2.3 / rms 1.2,
  lowest point max 1.1.
- Not fixed: at k +3..+6 the game's mesh is 194.9, 194.9, 195.3, 196.5 tall, ours 202.6, 201.0, 199.5, 199.3
  (was 202.6, 202.6, 201.0, 199.5): a leg squash of about 8 units that our tail frames do not have
  (unattributed). After a landing our body rests 1.6 above the spawn height (floor cast skin), and the
  table above is relative to that resting height.
- Fitted on one standing jump; a ledge fall or a running jump has no capture.

## D248 (2026-10-12): the crouch spring is not the post-landing squash

Question: is the leg squash at k +3..+6 the game's `stretch_crouch` spring fed by the landing?
Answer: no (MEASURED). Nothing drawn changed; `LANDING_DIP` stays -25.9, -15.9, 0.

- Spring, FOUND (EBOOT 0x2011d4, every tick from the dispatcher 0x20e28c after jump, lane and grab; input
  0x6c56c = max over contacts of min(depth / 350, 1), 0 in the wall-cling state phys+0x41). Per tick, all
  clamps to [-1, 1]: x = clamp(sqrt(E), 0.01, 0.99); h = x^2.3219, 0 if below 0.01; a = s858 + h;
  b = s854 + a; v = s850 + b; s854 = b; pose += stretch_crouch frame (0.5v + 0.5)n minus frame 0.5n at
  weight 1 if |v| > 0.01; then s850 = 0.75v, s858 = -0.25v. A one-tick kick gives v/h = 1, 1.5, 1.5, 1.125,
  0.56, 0, -0.42, -0.63, -0.63, -0.47, -0.24, 0. In `lbp-sim` as `CrouchSpring`, with
  `SackboyAnim::set_contact_depth` as its only input; nothing calls that yet, so it stays at rest.
- Method: release viewer, standing 6-tick jump, the spring value v forced to a constant from touchdown
  (-0.3, 0, 0.2, 0.4, 0.6) to get height, pelvis height and spine length as functions of v per tick; then
  the recurrence with a kick of depth k x 31.84 (our fall speed on the last airborne tick) starting on
  k +0, +1, +2 or +3, for one or two ticks, k fitted to the game's height and pelvis height on k 0..+14.
- What the game needs on top of our pose (game minus ours without the spring), k +0..+8:
  height 0, 0, 0, -7.6, -5.8, -4.7, -3.3, -1.7, -0.9; pelvis above the lowest vertex 0, 0, 0, -7.6, -6.0,
  -4.4, -3.0, -1.6, -0.5; head-to-pelvis length within 0.4 on every tick.
- Best fits (rms / max over both series; without the spring 2.95 / 7.6):

  | Kick starts | Ticks | Depth | k | rms | max |
  |---|---|---|---|---|---|
  | k +0 | 1 | 33.2 | 1.04 | 2.80 | 6.3 |
  | k +1 | 2 | 31.0 | 0.97 | 1.95 | 5.2 |
  | k +2 | 1 | 56.4 | 1.77 | 1.69 | 3.9 |
  | k +2 | 2 | 31.0 | 0.97 | 1.60 | 4.6 |
  | k +3 | 1 | 50.2 | 1.58 | 1.80 | 5.7 |

  None is within 1 unit. Three reasons, each enough: (1) shape in time: the game's squash is full on its
  first tick and decays monotonically, the spring rises for two ticks; (2) shape in the body: 0.2 of
  stretch_crouch lowers the pelvis 4.3 and the height 7.7 because it also bends the torso, so every fit
  shortens head-to-pelvis by 3 to 4 units where the game shows none; (3) the rebound: see below.
- Rebound: the spring would make him taller than rest 6 to 10 ticks after the kick. Against one viewer run
  the game is 0.9..1.1 lower on k +8..+12, against another 0.2..1.0 higher: the stand clip's breathing
  phase at the landing differs between viewer runs (below) and is worth +-1 there, so a rebound of that
  size can be neither shown nor excluded. A rebound matching a 7-unit squash (about 3 units) is not there.
- What it is instead (INFERRED from the same capture): pelvis height above the floor line (lowest vertex
  plus pelvis-above-lowest), k +0..+12: 43.9, 18.8, 31.1, 39.3, 40.8, 42.5, 45.5, 43.7, 42.1, 43.3, 45.1,
  46.2, 47.1 (rest about 48). The body keeps rising for ten ticks after the two under-floor ticks, while
  from k +3 the lowest vertex is back on the floor (0.0, 1.4, 2.6, 4.6, 0.7, -3.2, -3.1, -2.0, -1.3). So
  the legs are shortened by exactly the body's remaining dip: feet held on the floor under a body still
  below rest. The EBOOT has a foot placement pass that fits (0x210dd8, after the layer dispatcher: foot
  target = leg anchor minus direction x min(contact distance, rest length), only while the body moved 15
  or less this tick; 29 on k +1). Not implemented: it needs a leg solver in the pose blender and the body
  dip series continued past two ticks (-7.6, -6.0, -4.4, -3.0, -1.6, -0.5 after the current table).
- Table, game against ours (unchanged by this pass), k = ticks from touchdown:

  | k | height game | ours | pelvis game | ours | lowest game | ours |
  |---|---|---|---|---|---|---|
  | -1 | 201.8 | 201.9 | 43.7 | 43.7 | 28.8 | 28.1 |
  | 0 | 201.6 | 201.9 | 43.6 | 43.7 | 0.4 | 1.0 |
  | +1 | 206.0 | 206.3 | 47.8 | 48.0 | -29.0 | -29.0 |
  | +2 | 202.9 | 202.7 | 47.3 | 47.1 | -16.2 | -16.2 |
  | +3 | 194.9 | 202.9 | 39.3 | 47.0 | 0.0 | -1.0 |
  | +4 | 194.9 | 201.3 | 39.4 | 45.7 | 1.4 | -1.3 |
  | +5 | 195.3 | 200.0 | 39.8 | 44.3 | 2.6 | -1.3 |
  | +6 | 196.5 | 199.7 | 40.9 | 43.9 | 4.6 | -1.1 |
  | +7 | 199.1 | 200.5 | 43.0 | 44.6 | 0.7 | -1.0 |
  | +8 | 201.3 | 201.6 | 45.3 | 45.7 | -3.2 | -0.9 |
  | +9 | 202.6 | 202.4 | 46.5 | 46.5 | -3.1 | -0.7 |
  | +10 | 203.2 | 202.8 | 47.1 | 46.9 | -2.0 | -0.6 |
  | +11 | 203.7 | 203.0 | 47.5 | 47.0 | -1.3 | -0.5 |
  | +12 | 204.3 | 203.3 | 47.9 | 47.2 | -0.8 | -0.5 |
  | +13 | 204.8 | 203.8 | 48.4 | 47.5 | -0.6 | -0.4 |
  | +14 | 205.3 | 204.3 | 48.7 | 47.8 | -0.4 | -0.3 |

  Flight (27 ticks): height max 2.3, lowest max 1.1, as before. Idle height 200.8..206.1, as before.
- Measuring: the viewer's vertex dump now prints a `sackboy pose` line per dumped pose, so a pose is tied
  to the `sackboy tick` line above it. Counting poses was wrong whenever two ticks fell in one rendered
  frame (two of five runs here lost a pose between take-off and landing). The animation picker starts when
  the figure has loaded (around script tick 86, varying), so the stand clip's phase at a given script tick
  differs between runs by a few frames: up to +-1 in height once the landing tail has faded.

## Open
- The physics body position dips 35 (d230); the drawn figure no longer follows it. Whether the game draws
  the root at the body position and cancels the clip's root offset is not resolved.
- Pose fit of the landing ticks against the clip frames is 3.5-3.8 (stop/turn were 3-5): landing tail poses
  still need tick-aligned captures. Take-off: game goes straight to a stretched pose (h 214-219); ours plays
  a 6-tick crouch first in the hold-6 test; not changed (needs aligned capture).
