# Level scripts, part 55: the anti-aliasing job and the depth of field on by default in the Pod (2026-10-15, D304)

Viewer only, no runtime change. Parts 49 and 53 added the game's depth of field (`--depth-of-field`)
and its anti-aliasing job (`--mlaa`) behind flags, off by default. Both run in every Pod frame the
game was captured in; this part turns them on by default in the Pod picture and leaves every other
level's picture as it was.

### What was found

- FOUND (parts 49 and 53, the replays of frames 5247 and 14790): the depth-of-field passes run in
  both Pod frames (in 5247 they change nothing, no gate reaches 7/8), and the EDGE MLAA job runs in
  both (the copy out, the SPU job, the copy back with alpha 0, part 51). The order is the depth of
  field, the glow's source, the job, the final pass; the viewer already runs them in that order.
- FOUND (part 50): the blank Moon level's frame has no copy to main memory (no job); no game frame
  of a level outside the Pod has been compared for the depth of field (part 49's "Left out"). So
  outside the Pod the job stays where it was (never: it runs only in the Pod picture) and the depth
  of field stays off by default.

### What changed

`lbp-bevy-viewer`, `main.rs`, `dof_post.rs`, `mlaa_post.rs`:

- In the Pod picture (a level with `--pod-camera`) on the level-lighting route, the depth of field
  and the anti-aliasing job (with the camera's MSAA off, part 53) are on by default.
  `--no-depth-of-field` and `--no-mlaa` turn them off; `--depth-of-field` and `--mlaa` are still
  accepted and change nothing there. The rules are `dof_post::dof_route` and
  `mlaa_post::mlaa_route`, which now take `Option<bool>` as `glow_post::final_pass_route` does.
- Every other level: unchanged. The depth of field is off unless `--depth-of-field` (on the
  level-lighting route, as before); the job never runs; `--mlaa` there, and either flag off the
  level-lighting route, still prints that it is ignored. The help text says so.

### Measured

d304 release build and main c717214 built from the same tree (`target-d304`), hidden window
(`scratch/d287-shot.ps1 -Prefix d304`, list in `scratch/d304-all.ps1`), stills in ignored
`local-analysis/rscript/stage6/shots/` (`d304-main-*` main's default, `d304-mainflags-*` main with
`--mlaa --depth-of-field`, `d304-on-*` the default, `d304-off-*` with `--no-mlaa
--no-depth-of-field`, `d304-flags-*` with the explicit flags, `d304-mlaaonly-*` / `d304-dofonly-*`
with one of the two turned off); edges by `scratch/cmp293/d302edge.py`, gradients by `d298grad.py`,
regions by `d294cmp.py`, as parts 49 and 53.

- MEASURED byte for byte: the default closed Pod at 600 and the change-over (in use at 600, held at
  614) equal main's `--mlaa --depth-of-field` and the explicit flags; `--no-mlaa
  --no-depth-of-field` equals main's default in both; Da Vinci's rookie test (0001e786) equals
  main's default with the new default, with `--no-mlaa --no-depth-of-field` and with
  `--depth-of-field` (the level passes no range). Two default runs of the closed Pod are equal.
- The closed Pod at 600: the depth of field alone changes nothing (its range (80,000, 100,000), as
  the game's 5247), so the default equals the job alone; 49,056 pixels differ from main's default
  (15,106 by more than 8 levels; part 53 gave 43,179 before D303's glow alpha). Staircase RMS, the
  game's `d156-rpcs3-pod-default` / main's default / now: the floor line on the left 0.136 / 0.349
  / 0.252; the right post's left outline 0.196 / 0.264 / 0.250; the left post's window edge 0.116 /
  0.179 / 0.151; the DualShock's shoulder 0.875 / 1.062 / 0.978. Viewport mean |d| against the game
  (warped s 0.875, dx +1, dy +1) 5.79 -> 5.78.
- The change-over at 614 against `open-t1` (warped s 0.877, dx +2): 532,951 pixels differ from
  main's default (77,351 by more than 8). Mean gradient magnitude, game / main's default / the
  depth of field alone / the job alone / now: the room behind the window 1.20 / 4.07 / 1.96 / 4.06
  / 1.93, room upper left 0.67 / 1.18 / 0.91 / 1.18 / 0.91, upper right 0.65 / 1.36 / 1.05 / 1.37
  / 1.05, upper centre 0.76 / 1.76 / 0.98 / 1.76 / 0.98; the DualShock 2.48 / 2.48 / 2.48 / 2.47 /
  2.47, the left post's card 7.89 / 7.14 / 7.14 / 6.66 / 6.66. Region means |d| (main's default ->
  now): DualShock body 0.1 -> 0.4, the post cards 2.9 / 2.4 -> 2.9 / 2.5, the wall below the window
  1.1 -> 1.1, the glass 0.3 / 3.6 -> 0.3 / 3.7, the room behind the window 3.0 -> 3.2. The halo
  over the line (median of the post columns, 1 / 2 / 3 / 5 / 9 px): game 47.4 / 14.7 / 11.2 / 5.0
  / -0.7, main's default 45.6 / 14.0 / 8.2 / 5.6 / 0.1, now 47.1 / 19.5 / 13.4 / 6.5 / 0.3.
  Viewport mean |d| 12.39 -> 11.72 (the depth of field alone 11.72, the job alone 12.37).
- How the two meet (MEASURED, 614): the job runs after the depth of field, so it filters the
  blurred room. On the sharp picture it changes 74,637 pixels, 28,210 of them in the depth of
  field's area (487,644 pixels), by 8.1 levels on average there (8,476 by more than 8); after the
  depth of field it changes 123,432, 76,420 in that area, by 2.5 on average (2,681 by more than 8).
  Outside that area its change is the same on 44,120 of the 47,012 pixels it touches. The combined
  still differs from the sum of the two changes on 85,132 pixels (7,395 by more than 8 levels),
  all of it in and beside the blurred room; the Pod, the DualShock and the glass take the job's
  change alone. INFERRED: the many small changes in the room are the job's floor of 10 levels met
  on the blur's half-size steps (the bilinear read-back of a 640 x 360 picture) in the dark room.

### Left out

- The depth of field by default in a level outside the Pod (no non-Pod game frame with it yet), and
  the stencil-marked composite, the offset buffer and the gate's code (part 49).
- The job outside the Pod: what keeps it off in a level's play (part 51, INFERRED there), and MSAA
  in the Pod's other views (part 53).
- The job's program beyond its edge test (part 53); whether the game's job sees the same small
  steps in the blurred room (its copy back is stale in the replays, part 51).

### Verification

`lbp-bevy-viewer`, no game data: `d304_dof_on_by_default_in_the_pod` (on by default in the Pod on
the level-lighting route, off by default in another level and on under `--depth-of-field` there,
off under `--no-depth-of-field`, never off the route) and `d304_mlaa_on_by_default_in_the_pod` (on
by default and under `--mlaa` in the Pod, off under `--no-mlaa`, never in another level or off the
route); `d302_route_and_extract` updated for the new argument. fmt, `cargo clippy -p
lbp-bevy-viewer --all-targets -- -D warnings` and `cargo check --workspace` pass; the viewer's tests
pass (141) but the one that fails on main (`sackboy_play::tests::input_script_steps`). Hidden
windows only; nothing in RPCS3 was started or captured.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
