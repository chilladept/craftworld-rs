# Level scripts, part 6: the Pod camera and the next Rookie natives (2026-10-08)

Built on part 5 (object members). Evidence: the declaring script type tables and how the
local listings (ignored `local-analysis/d133-script-listings-0133`) use each name and
result; Toolkit d6a2ef80 only for `makeLamsKeyID` (docs/u1-translation.md). Nothing derived
from bytecode is committed. Bindings are listed in docs/script-natives.md.

**Per-frame entry (general mechanism).** `LevelScripts::tick` runs, for a class listed in
`lbp_runtime::FRAME_ENTRIES`, that entry with argument registers built by the entry's
`args` function instead of `Update__` (`Runner::start_with` stages them after `this`).
PlanetsWorld has no `Update__`. Its per-frame entry is neither `UpdateState__` (private,
called only from the export below) nor `OnEnterPod__`/`OnExitPod__` (PodComputer calls
them on pod entry and exit). It is the export `UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb`,
which no script calls and which is the only caller of `UpdateState__`,
`UpdatePodCamera__Q11PlanetThingbb` and `UpdateInteractiveMusic__` (INFERRED: the engine
calls it each frame). Arguments, INFERRED from how the function uses them: +4 bool passed
on to `UpdatePodCamera` (false); +8 the Earth (it gets `EarthSpinOffset`); +12 the
OrbitingMoon (`MoonSpinOffset`); +16 "snap the view" (true on the first tick only; it
skips the transition and the camera smoothing); +17 "take the zoom from the pod
computer's state" (true).

**Engine objects.** `add_engine_objects(vm, host, budget)` makes, before create: 16 Gooey
objects for `CreateGooey__i` (natives cannot allocate VM objects mid-call, so a pool is
handed out), a Planets object with two Planet crafteroids (INFERRED: index 1 = the
level's Earth, 2 = its OrbitingMoon; PlanetsView reads crafteroids 1 and 2 and returns
early if either Thing is null), and the PodComputer, registered before its `.init__` runs
because that code already calls `GetPodComputer__`. Its `.ctor__` stops at the unbound
`Gooey.SetLastNavSelectionMode__b` (last call before its return; reported).

| Finding | Label |
|---|---|
| `Smooth*(pos, vel, target, max_acc, max_vel)`: PlanetThing passes its `MaxAcc` then `MaxVel` (defaults 0.01, 10); result = new velocity (2D/3D, caller adds it) or (new value, new velocity) (scalar `Smooth__fffff`, the caller reads x and y). `Smooth3D`'s step (part 40, EBOOT 0x5fed34 -> core 0x3905b8 -> 1-D step 0x38ef7c): the velocity along the target only, at most `max_vel`, a time-optimal move over a fixed step of 1 (speed up by `max_acc`, brake to stop on the target), zero once closer than the new speed. `Smooth2D` and the scalar keep the older rule (head for the target at most `max_vel`, velocity change at most `max_acc` per tick) | order: INFERRED from use (`Smooth3D`: FOUND, part 40); `Smooth3D` rule: FOUND (part 40); `Smooth2D`, scalar rule: INFERRED |
| `lerp(t, a, b)`, `SmoothStep(e0, e1, x)` | INFERRED, vector-library conventions |
| `Cam.*`, `SetWorldPosition__m`, `SetAnimPos`, `SetPlayerScoreMultiplierTimer` record state on `LevelWorld` (`camera`, `world_matrix`, `anim_pos`, `score_multiplier_timers`); the viewer can read `LevelWorld::camera` | recorded, nothing renders it yet |
| PC-run defaults: pod computer not in use, no crash zoom, no cinema mode, no avatars, no video, 16:9, player 0 leads, no music, interactive music audible, no attraction loop, interactive, no pending links | INFERRED defaults (`state_query`) |
| `IsLBP2GameProgressionFlagSet__i`: `LevelWorld::progression_flags` (empty = fresh profile; `level_scripts --flags=2,..`). On a fresh profile PlanetsWorld starts its intro cutscene (`SetState(1)` -> a Thread). Since part 22 the argument is a bit mask on one word (FOUND) and `--flags` takes masks | flag 2 meaning: INFERRED (part 22: the mask 2, set by the intro) |
| `MakeLamsKeyID`: Toolkit `makeLamsKeyID` as s32 | CONFIRMED_FROM_COMMUNITY_CODE |
| Resource handles: `LevelWorld::load_resource` returns the GUID (Gooey `.init__` loads a texture) | INFERRED stand-in |

Results (PC, 01.33 + d133 edat overlay, `--play=300`, unit = scripted Thing):

| Level | Create done / faults / no_function | Ticks | Faults |
|---|---|---|---|
| Pod planets, fresh profile (`d166-play-pod-planets-0133.json`) | 3 / 0 / 0 (D165: 2 / 1 / 0) | tick 1: PlanetsWorld stops | `Thread.StartImpl__` 1 (`Thread.Start__`, the intro cutscene thread) |
| Pod planets, `--flags=2` (`d166-play-pod-planets-flag2-0133.json`) | 3 / 0 / 0 | 300 x UpdatePlanetsView done, 0 faults | none |
| Pod (`d166-play-pod-0133.json`) | 0 / 1 / 1 (D165 same counts) | 0 faults | `Thing.FindThing__Q7StringA` 1 (Pod.OnCreate__, after `DebugPrint` wrote "19") |
| Rookie Test (`d166-play-rookie-0133.json`) | 189 / 23 / 9 (D165 same counts) | 0 faults | `LAMS.IsInLams__i` 20, `MakeLamsKeyID` on a null string 1 (Trigger_Speech_Bubble), `Game.SetPlayerScoreMultiplierCounter__Q18Game.EPlayerNumberi` 1 (World), `Gooey.SetStylesheetScalingFactor__f` 1 |

Camera after 300 ticks (`--flags=2`): from (0, -250, 1130, 1), to (0, 100, -1500, 1),
up (0, 1, 0, 0), floats (10, 100000, 0, 1); 300 `SetCameraFromToUp` calls; DOF and fog
(80000, 100000, 20000); motion blur cleared once (the snap tick). From and to are the
function's constant preset for "pod computer not in use"; they do not move with the
defaults above. Earth and OrbitingMoon have `Update__rb` (an export with a vector and a
bool) instead of `Update__`, so they still count as no_function each tick.

Tests: `lbp-script-vm` `part6_math_and_lams_keys`; `lbp-runtime`
`part6_frame_entry_gets_its_arguments_each_tick`,
`part6_camera_world_position_and_engine_state`, `part6_engine_objects_and_pod_computer`.
`script_tests` (`d166-script-tests-0133.json`): 47 functions done, 512 passed / 1 failed.

Not tested: the camera drawn in the viewer; any value against the game (the INFERRED
defaults and the Smooth rule are not checked in RPCS3).

Counts: +0 newly viewable, +0 newly Complete (no asset change).
