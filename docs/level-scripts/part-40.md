# Level scripts, part 40: the script camera's smoothing step (2026-10-14, D289)

Sixteenth PR of the "Pod picture" follow-up to stage 6, script VM only. In the game's
frame `open-t1` the camera is at its open eye while the tent still stands; the viewer's
camera only settled at tick 626 to 628 (parts 28 and 31), after the tent was gone. The
cause was the INFERRED step rule of `Math.Smooth3D__rrrff` (part 6). This part reads the
game's rule from the EBOOT and uses it.

### What was found

- FOUND (EBOOT 01.33): `native_Smooth3D__rrrff` (0x5fed34) hands the position, velocity
  and target to 0x5ef720, with the fourth argument as the acceleration and the fifth as
  the top speed. 0x5ef720 calls the core 0x3905b8 on copies and returns only the new
  velocity, with w cleared. The position the core works out is thrown away, so the
  script's own `cur_pos += vel` is the only place the camera moves.
- FOUND (core 0x3905b8): the distance to the target is taken. Under 1e-4 the velocity is
  zero. Otherwise the velocity is projected on the unit direction to the target and its
  sideways part dropped; a 1-D step runs on the distance with that speed; the result is
  the direction times the new speed, or zero when the distance is less than the new speed.
- FOUND (1-D step 0x38ef7c, also the scalar `Smooth__fffff`'s core): the speed is clamped
  to plus or minus the top speed. A time-optimal plan to the distance that ends at rest is
  worked out: speed up (or brake, when braking alone would already pass the target), cruise
  at the top speed if the plan needs it, then brake. The plan is advanced by one unit of
  time, the constant 1.0 in the TOC (0xd9fa64), not a frame time. If the whole plan fits in
  that step the result is the distance with speed zero.
- INFERRED, from the above: near the end the plan fits in one step and the new speed is
  zero, and since only the velocity comes back the camera stops a little short of its
  preset. For the Pod's open eye it stops about 2.45 units short.
- FOUND (RenderDoc dumps of frame 14790, `pod.json` and the room's `vs.json`): the game's
  resting eye is (0, -299.4804, 902.3901), not the preset (0, -300, 900). The rule above
  gives (0, -299.48, 902.39); the earlier rule gave the preset exactly.
- FOUND (the PlanetsWorld script listing, `UpdatePodCamera__Q11PlanetThingbb`): it calls `Smooth3D` with 10
  and 100 for both the eye and the focus, and no time term. MEASURED (part 28): the move
  starts on the same tick as the tent's dissolve (601).

### What the VM does

`lbp-script-vm` (`natives.rs`): `Math.Smooth3D__rrrff` uses the game's step
(`smooth3d` and `smooth_step_1d`). A zero or negative acceleration or top speed keeps the
clamped speed (not read in the EBOOT; no level script passes one). `Smooth2D__pppff` goes
to another core (0x38f2fc, not read) and the scalar `Smooth__fffff`'s wrapper (0x5ee844)
was not read for its argument order, so both keep the older INFERRED rule.

### Measured (PC, release build)

`level_scripts`, pod.bin with `--with-level` pod_planets.bin, `--play=700
--puter-in-use=600`, the d133 snapshot with the overlay
(`local-analysis/rscript/stage6/d289-camera-session.json`):

- The planets camera leaves (0, -250, 1130) on 601, as before, and its last move is on
  614: `camera_rests_at` on 615 at (0, -299.4803, 902.3904), and it stays there to 700.
  Before: rest on 626 at (0, -300, 900), after swinging 122 units past the eye on 612.
- The dissolve is unchanged: 0.05 on 601, 0.75 on 615, 0.95 on 619, 1.0 and the mesh
  swap to 145753 on 620. So from 614 to 619 the camera is at its open eye with the tent
  at 0.70 to 0.95, as in `open-t1`.
- The same rule in a Python model gives the eye within 5.3 units on 610 and no overshoot,
  and the focus from (0, 100, -1500) at rest at y 497.83 from its 17th call. The game's
  forward from its resting eye and focus is (0, 0.315, -0.949), as in the memory read of
  2026-10-09; the preset pair gives 0.316.

The parts 28 to 35 lines that say the camera rests at (0, -300, 900) on 626 or 628 were
true of the older rule. The viewer runs the same VM, so its Pod camera now settles on 614
too; its change-over stills were not taken again for this part.

### Left out

- `Smooth2D` and the scalar `Smooth` (see above).
- The Earth and Moon `PlanetThing.Update__rb` call the same native (0.01, 10); the runtime
  does not run it yet.
- Who calls `UpdatePlanetsView` and how often in the game (INFERRED once a tick at 30 Hz,
  as the viewer does; LBP2 renders at 30 fps, so the answer would not change the timing).

### Verification

`lbp-script-vm`, no game data: `d289_pod_eye_rests_short_of_the_preset_by_call_14` (the
Pod eye from (0, -250, 1130) to (0, -300, 900) with 10 and 100: never past the target,
the distance only falls, within 6 units by call 10, at rest from call 14 at
(0, -299.48, 902.39)), `d289_pod_focus_rests_short_of_the_preset` (at rest at y 497.83
from call 17), `d289_smooth3d_drops_sideways_velocity_and_brakes` (the sideways part is
dropped, the speed clamped, braking before the target, zero on it, a plan that fits in the
step lands). The part 6 test keeps its far-target case. fmt, the CI clippy line, the CI
test set and `cargo check` of both viewers pass.

Counts: +0 newly viewable / +0 newly Complete (script VM fidelity).
