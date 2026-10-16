# Level scripts, part 46: the pod computer's per-frame step (2026-10-14, D295)

Script VM only (`lbp-runtime`, four natives in `lbp-script-vm`). In the game, "in use" makes
the planets menu zoom in: the view of the planets world eases from far out to the main
menu's view. Here it never moved, because the runtime called the planets' entries,
`UpdatePlanetsView` and the pod computer's `UpdateVideo__`, but never the pod computer's
own per-frame step, the one that opens the main menu. This part adds that call.

### What was found

- FOUND (EBOOT 01.33, 0x40663c, slot 0x14 of the pod computer object's vtable 0xd73d40):
  the object's per-frame function returns early unless its script holder is valid
  (0x89479c) and a test of the object (0x3fcaa8) passes; then, when a word of the object is
  not 0, it calls the script (0x8947dc -> 0x503a48 -> the wrapper 0x659c58,
  `PodComputer.PerFrameUpdate__b`) with a byte of the object and clears that byte. The call
  is not tied to "in use": every frame.
- FOUND (script listings): `PerFrameUpdate__b`, while `GetPodComputerInUse()`, not
  `IsSendingSetUserMessage()`, the state stack empty and the build not a Move demo, E3 or E3
  crossplay demo, makes a `PodStateMainMenu`, runs its constructor and pushes it. The push
  calls the state's `OnGainFocus`, which hands `GetPlanetsViewType()` to
  `PodComputer.SetPlanetsView`: 1 for the main menu (the base state answers 0). That fills a
  view with `PlanetsView.SetPlanetsView__i(1)` and calls `PlanetsWorld.SetView` on
  `GetWorld()` cast to PlanetsWorld.
- FOUND: `SetView`, for a type unlike the current one, sets `ViewType`, sets
  `ViewTransition` to 0 and copies the current view as the old one and the new view as the
  wanted one. `UpdatePlanetsView` then adds `GetTransitionSpeed(old, wanted)` to the
  transition each frame (0.05 for 0 -> 1 on the same focus planet: 20 frames, capped at 1)
  and blends the two views with `SmoothStep(0, 1, transition)`. View 0 has the offset
  (0, 0, -15000), view 1 (-700, 400, -5000): the zoom in.
- FOUND: the main menu's `WantVO__` names the voice-over "pod_select_controller" (the
  "_alt" one when `HasPlayedLBP1`).
- INFERRED: the bool argument is false (what sets the byte was not read); the two
  conditions of the call hold every frame. The step casts the world to PlanetsWorld, and a
  probe that also called it in the shell level faulted there (its world script is the Pod's:
  `PlanetsWorld.SetView` on a null Thing), so the game's one pod computer belongs to the
  planets level: the call is made only in a level whose world Thing's script is a
  PlanetsWorld.

### What the runtime does

`lbp-runtime` (`lib.rs`):

- `POD_COMPUTER_PER_FRAME` = `PerFrameUpdate__b`. `LevelScripts::pod_computer_frame` calls
  it on the level's PodComputer object with `this` and `false` (+4), once a tick, only when
  the world Thing's script class is `PlanetsWorld`. A fault or a yield is reported once in
  `StepReport::thread_faults` and the call is not made again
  (`LevelScripts::pod_computer_frame_stopped`), as for `update_video`.
  `StepReport::pod_computer_frames` counts the calls that ran to their end.
- Order in `tick_level_in_session`: the Things' entries (with `UpdatePlanetsView`), the
  trigger, switch and global-settings passes, then this step, then `update_video`. The
  place of 0x40663c in the game's frame against the planets object's function (0x43ac68,
  which ends in `UpdatePlanetsView`) was not read; with this order the blend's first step
  is on the tick after the push. The other order would move the whole blend one tick
  earlier.

`lbp-script-vm` (`natives.rs`), bound as state queries, all false in a level world
(INFERRED: the retail game, no user message being sent; their code was not read):
`Game.IsMoveDemoBuild__`, `System.IsE3__`, `System.IsE3CrossplayDemo__`,
`PodComputer.IsSendingSetUserMessage__`. The table has 261 natives (was 257).

`level_scripts` (session report): each level's `pod` block now has the session's planet
entry calls, the "arrived" answers and the video calls (they were always 0: the sums were
built without the step reports), plus `pod_computer_per_frame_calls`; each level lists
`first_thread_faults`; the path has a `planets_view` event (the world script's `ViewType`
and `ViewTransition`) whenever either changes. The single-level report has
`pod.pod_computer_per_frame_calls`.

### Measured (PC, release build)

`level_scripts`, pod.bin with `--with-level` pod_planets.bin, `--play=700
--puter-in-use=600`, the d133 snapshot with the overlay
(`local-analysis/rscript/stage6/d295-planets-session.json`, against part 44's
`d293-smooth-session.json`):

| Tick | planets_view (type, transition) | SmoothStep of it |
|---|---|---|
| 1 | 0, 1.0 | 1.0 (rest) |
| 601 | 1, 0.0 (`SetView`, the tick "in use" starts) | 0.0 |
| 602 | 1, 0.05 | 0.00725 |
| 614 | 1, 0.65 | 0.71825 |
| 621 | 1, 1.0 | 1.0 |

- 614 is part-way through the zoom (`open-t1` is at about 614.2): the blend is 0.718 of the
  way from view 0 to view 1.
- The planets level's step runs 700 times (also before "in use": it asks its Gooey and
  callout natives every frame) with no fault; the shell level's step is not called.
- Unchanged against D293: every path event that report had (the flags 3 on 452, the camera
  resting at (0, -250, 1130) on 452, leaving it on 601 and resting at (0, -299.4803,
  902.3904) on 615, the change-over 0.05 to 1.0 on 601 to 620, the mesh swap on 620), the
  faults (0), the Things stopped (0) and the shell level's unbound natives (none).
- Different, and why: 22 new `planets_view` path events (above). `voice_over_requests` 1 ->
  2 (both levels report the session's one list): the main menu's "pod_select_controller" on
  tick 601. The planets level's unbound natives 0 -> 28 kinds, 5,783 calls (Gooey, Callout,
  Network, StreamManager and pod computer UI natives of the step, answered by the defaults;
  the most called `Gooey.NeedsUpdate__` 1,400, `Gooey.SetColoursFromPlayer` 700,
  `Planets.CheckOverlayNeedsUpdate__` 700, `PodComputer.GetPodComputerUserPlayerNumber__`
  700). The report fix shows what was always run: `planet_update_calls` 1,400 (all
  answering "arrived", the placeholder point), `pod_computer_update_video_calls` 700 in
  each level.

### Left out

- The frame order of the step against `UpdatePlanetsView` (a one-tick shift of the blend);
  0x3fcaa8, the word of the object and what sets the byte (the call's conditions and its
  argument).
- The main menu itself (its Gooey, callouts and voice-over); the unbound natives above
  answer with defaults. Under `--strict` the first of them faults and stops the step.
- The view in the viewer's Pod picture.

### Verification

`lbp-runtime`, no game data: `d295_the_pod_computer_steps_once_a_tick_in_the_planets_level_only`
(once a tick with false, after the Things and before `UpdateVideo__`, in and out of "in
use", in a session tick too; no call where the world script is not PlanetsWorld, the video
call as before), `d295_a_pod_computer_step_that_faults_is_not_made_again` (reported once,
the video call goes on) and `d295_the_build_and_message_queries_are_false`. The VM's table
count asserts are 261. fmt, `cargo clippy -p lbp-runtime -p lbp-script-vm --all-targets --
-D warnings`, both crates' tests and `cargo check --workspace` pass.

Counts: +0 newly viewable / +0 newly Complete (script VM fidelity).
