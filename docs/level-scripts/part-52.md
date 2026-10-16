# Level scripts, part 52: the pod computer's step against the planets view (2026-10-14, D301)

Script VM only (`lbp-runtime`). Part 50 measured the game's planets zoom about two ticks
ahead of the VM's at the change-over (`open-t1`, the dissolve's line at about tick 614.2,
looks like the VM's tick 616: transition about 0.75 against 0.65). Part 46 left one of the
ticks to the unread order of the pod computer's step (0x40663c) against the planets
object's function that ends in `UpdatePlanetsView` (0x43ac68). That order is read here and
taken over; the other tick is not found.

### What was found (01.33 executable)

- FOUND: the main loop (0x19348) runs, per simulation step, the network pump (0x28baa4,
  0x28cb50) and the main level's world update (0x1a077c -> 0xeacd4); after that loop it
  calls the frame function (0x18558) once. The frame function calls slot 0x14 of the pod
  object the game holds (0x3f9284: the pod computer, vtable 0xd73d40, while its mode is 1;
  a small object of vtable 0xd73f38 in mode 2), then the rest of the frame; later slot
  0x18, and the render (0x31e820) slots 0x1c and 0x28 (`UpdateVideo__`). 0x18558 is the
  only caller of slot 0x14.
- FOUND: the pod computer's slot 0x14 (0x40663c) calls `PerFrameUpdate__b` and then eleven
  functions of the object, the first of them 0x4064bc. 0x4064bc calls the planets object's
  slot 0x1c (0x43d900), which calls 0x4381d8. 0x4381d8 runs the planets level's world
  update (0x895adc -> 0xebe08 -> 0xeacd4), then for each planet in its list the vector
  function and slot 0x28 (`PlanetThing.Update__rb`, part 25), then its own slot 0x58
  (0x43ac68, `UpdatePlanetsView`). The planets object's slot 0x1c has one other caller,
  slot 0x14 of the mode-2 object (0x3fd018); 0x43ac68 has no other.
- So in one frame: the main level's update (INFERRED the shell, with the Pod's change-over), then
  `PerFrameUpdate__b` (the main menu's push and `SetView`, part 46), then the planets
  level's update, the planets, and `UpdatePlanetsView`, which steps the new blend at once:
  `ViewTransition` is 0.05 on the frame of the push.
- FOUND: 0x4064bc hands the planets object 0 as its last argument, which 0x4381d8 passes to
  0x43ac68 as the first bool of `UpdatePlanetsView` (+4): false, as the VM had it. The
  last bool (+17) is the first byte of a record 0x3fce40 fills (1 when the pod computer
  or a global object has a pair of ids set); it only picks the state's extra zoom, 0 in
  the main menu. "In use" is byte +0x408 of the game object (`GetPodComputerInUse`,
  0x502f1c); its setter 0x18ed30 is called by the handler of network message 0x30
  (0x3fa868, registered by 0x401d40 through 0x28a804), by the level start (0x1ab508) and
  on leaving (0x1a7580). The pod computer's step reads the same byte.

### What the runtime does

`LevelScripts::tick_level_in_session`: the pod computer's step
(`pod_computer_frame`, a no-op outside the planets level) first; then the Things' part of
the tick without the planets object's entries (`FRAME_ENTRIES`: `UpdatePlanetsView` and the
planets' `Update__rb`); the trigger, switch and global-settings passes; then the planets'
entries and the world script's `UpdatePlanetsView` (in that order, as before, and with the
tick count of this tick, so "snap" is still the first tick's); then `update_video` and the
camera. `LevelScripts::tick` alone keeps its one pass over every entry. The entries still
run in a level without a pod computer object (INFERRED, as before).

### Measured (PC, release build)

`level_scripts`, pod.bin with `--with-level` pod_planets.bin, `--play=700
--puter-in-use=600`, the d133 snapshot with the overlay
(`local-analysis/rscript/stage6/d301-session.json`, against part 46's
`d295-planets-session.json`):

| Tick | Change-over (pod.bin) | planets_view transition, D295 | now | SmoothStep now |
|---|---|---|---|---|
| 601 | 0.05 | 0.0 (`SetView`) | 0.05 | 0.00725 |
| 614 | 0.70 | 0.65 | 0.70 | 0.784 |
| 616 | 0.80 | 0.75 | 0.80 | 0.896 |
| 620 | 1.0 (mesh swap) | 0.95 | 1.0 | 1.0 |
| 621 | 1.0 | 1.0 | 1.0 | 1.0 |

- Different, and why: the 21 `planets_view` events of 601 to 621 are now 20, on 601 to 620,
  each one tick earlier (0.05 on 601, 1.0 on 620): the blend's first step is in the tick of
  the push. The zoom and the change-over now move together, tick for tick.
- Unchanged against D295: every other path event (the flags 3 and the camera at rest on
  452, "in use" on 601, the change-over 0.05 to 1.0 on 601 to 620, the mesh swap on 620, the
  camera leaving its rest on 601 and resting at (0, -299.4803, 902.3904) on 615), the
  faults (0), the Things stopped (0), the step's 700 calls, the unbound natives and every
  count of both levels.
- Still open: at the dissolve's line (614.2) the VM's transition is now about 0.71, the
  game's about 0.75: one tick.

### The second tick (not found)

- Not "in use" starting earlier for the pod computer than for the shell: both read the same
  byte, and in the frame the shell's update comes first, so a byte set before the step loop
  is seen by both in the same frame, and one set between the shell's update and the pod
  computer's step would put the zoom one tick ahead. Where the network layer runs message
  0x30's handler was not found (the dispatcher of the table that 0x28a804 fills was looked
  for and not found; 0x28b304 calls only message 0x79's), so this stays the open candidate.
- Not the extra planet zoom (0 in the main menu's state, part 44; the +17 bool only picks
  it), not `GetTransitionSpeed` (a constant 0.05 for 0 -> 1 on the same focus planet,
  listing), not SmoothStep's argument (the transition after the add, listing).
- The main loop's step loop can run more than one simulation step for one frame while the
  pod computer's step runs once a frame; at a steady 30 frames a second it is one each.

### Left out

- The dispatch of message 0x30 within the frame (the candidate above); the eleven other
  functions 0x40663c calls after `PerFrameUpdate__b`; the mode-2 pod object.
- The +17 bool of `UpdatePlanetsView` (the VM passes true; the zoom it reaches is 0 here).

### Verification

`lbp-runtime`, no game data: `d301_the_pod_computer_step_comes_first_and_the_planets_view_last`
(per tick: the step's false, the Lamp's update, the planet, `UpdatePlanetsView` with "snap"
on the first tick only, the video call; `tick` alone keeps planet, world, Lamp) and
`d301_a_stopped_pod_computer_step_leaves_the_planets_running`. The d295 step test now
expects the step before the Things. fmt, `cargo clippy -p lbp-runtime -p lbp-script-vm
--all-targets -- -D warnings`, both crates' tests and `cargo check --workspace` pass.

Counts: +0 newly viewable / +0 newly Complete (script VM fidelity).
