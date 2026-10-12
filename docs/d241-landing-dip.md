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

## Open
- The physics body position dips 35 (d230); the drawn figure no longer follows it. Whether the game draws
  the root at the body position and cancels the clip's root offset is not resolved.
- Pose fit of the landing ticks against the clip frames is 3.5-3.8 (stop/turn were 3-5): landing tail poses
  still need tick-aligned captures. Take-off: game goes straight to a stretched pose (h 214-219); ours plays
  a 6-tick crouch first in the hold-6 test; not changed (needs aligned capture).
