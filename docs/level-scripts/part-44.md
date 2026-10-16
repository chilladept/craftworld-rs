# Level scripts, part 44: the 2-D and scalar smoothing steps (2026-10-14, D293)

Twentieth PR of the "Pod picture" follow-up to stage 6, script VM only. Part 40 put
`Math.Smooth3D__rrrff` on the game's rule and left its two siblings, `Smooth2D__pppff` and
the scalar `Smooth__fffff`, on the older INFERRED one. This part reads both from the EBOOT
and uses the game's rule for them too.

### What was found

- FOUND (EBOOT 01.33): `native_Smooth__fffff` (0x6019f8) passes its five floats in order to
  0x5ee844, which keeps the first two (position and velocity) in memory and calls the 1-D
  step 0x38ef7c (through the TOC stub 0x89abcc) with the third as the target, the fourth as
  the acceleration and the fifth as the top speed. The step moves the position and the
  velocity in place; the native returns (new position, new velocity, 0, 0). So the order is
  the one the VM already assumed, and the new position comes from the step itself, not from
  the position plus the new velocity.
- FOUND (1-D step 0x38ef7c, read again): it works on the position and the target, not on a
  distance from 0; where its plan fits in the step it sets the position to the target exactly
  and the speed to zero. The 3-D core calls it with the position 0 and the distance as the
  target, which is part 40's `smooth_step_1d`.
- FOUND: `native_Smooth2D__pppff` (0x60195c) passes the position, velocity and target
  vectors and the two floats (acceleration, then top speed) to 0x5ee7e4, which calls the core
  0x38f2fc (through the stub 0x8983dc) on copies and returns only the new velocity; the native
  writes x, y and z and clears w, as `Smooth3D`'s does.
- FOUND (core 0x38f2fc): the 2-D twin of 0x3905b8, the same code with two products where
  0x3905b8 has three. The distance and the projection of the velocity on the direction take
  x and y only; under the same 1e-4 (0xd9fa8c) the position snaps to the target and the
  velocity is zero; otherwise the 1-D step runs on the distance with the projected speed, and
  the velocity is the offset to the target over the distance times the new speed, or zero
  when the distance is less than the new speed. The sideways part of the velocity is dropped.
- FOUND (0x38ef7c): the step divides by the acceleration, so a zero acceleration gives values
  that are not finite; a top speed of zero or less was not read through.
- FOUND (the script listings): `Smooth__fffff` is called once, by PlanetsWorld's
  `UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb` on the extra planet zoom (the zoom, its
  velocity, the pod computer state's zoom, 10, 100; x back to the zoom, y to its velocity) on
  every frame that does not snap the view. `Smooth2D__pppff` is called once, by
  `PlanetThing.Update__rb` on (spin, pitch) with (spin speed, pitch speed), the result added
  to (spin, pitch). No other script calls either.

### What the VM does

`lbp-script-vm` (`natives.rs`):

- `smooth_nd` is the core for 2 or 3 components; `Smooth3D__rrrff` and `Smooth2D__pppff` both
  go through it (`smooth_vec`). For 3 components it is part 40's code, unchanged in result.
- `Smooth__fffff` runs `smooth_1d`: part 40's `smooth_step_1d` on the target less the
  position, landing exactly on the target where the plan fits, and returns (new position, new
  velocity, 0, 0).
- A zero or negative acceleration or top speed keeps the clamped speed, as before (not the
  game's non-finite values; no level script passes one).
- The older INFERRED rule (`smooth`, `clamp_len`) is gone.

### Measured (PC, release build)

`level_scripts`, pod.bin with `--with-level` pod_planets.bin, `--play=700
--puter-in-use=600`, the d133 snapshot with the overlay
(`local-analysis/rscript/stage6/d293-smooth-session.json`):

- The report is byte for byte the same as part 40's (`d289-camera-session.json`): the camera
  leaves (0, -250, 1130) on 601 and rests on 615 at (0, -299.4803, 902.3904); the dissolve and
  the mesh swap are unchanged.
- A probe build (not kept) logged the calls: `Smooth__fffff` 699 times in the 700 ticks
  (not on the tick that snaps the view), each with (0, 0, 0, 10, 100), so the zoom stays 0 under both rules.
  `Smooth2D__pppff` is not called in the session (the report's `planet_update_calls` is 0).

### Left out

- The game's non-finite results for a zero acceleration (above); a top speed of zero or less.
- `PlanetThing.Update__rb` in the Pod session (part 40), and so `Smooth2D` there; the zoom the
  pod computer's state gives.

### Verification

`lbp-script-vm`, no game data: `d293_scalar_smooth_takes_pos_vel_target_acc_top_speed`
(from rest with 0.5 and 2: (0.25, 0.5); swapped, 2 and 0.5: (0.4375, 0.5); relative to the
position and toward a target below it; the speed clamped first; a plan that fits lands on the
target at rest) and `d293_smooth2d_is_the_2d_twin_of_smooth3d` (toward (30, 40) at speed 2
along (0.6, 0.8); the sideways velocity and z take no part; at rest within the step; zero
under 1e-4; the Pod eye's (y, z) path in 2-D equals the 3-D one bit for bit, resting short of
the preset). The part 6 test's scalar case now lands at rest ((2, 0), was (2, 1)). The d289
tests are unchanged. fmt, `cargo clippy -p lbp-script-vm --all-targets -- -D warnings`, the
VM's tests and `cargo check --workspace` pass.

Counts: +0 newly viewable / +0 newly Complete (script VM fidelity).
