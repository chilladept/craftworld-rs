# Level scripts, part 1 (runtime plan item Rt)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Builds on the
script VM (`docs/script-vm.md`; the game's own test script passes 512 of 513 checks at
D145) and the native bindings (`docs/script-natives.md`). No script bytecode, listing,
level bytes or engine code was used or is committed.

## What it adds

- **`lbp-level`**: the PScript part now keeps its data (`PartData::script`): the script
  resource and, when the instance has a layout, one `ScriptField` per serialized layout
  field (name, machine type, value). Values: bool, char, s32, f32, v4, m44, Thing
  reference (Thing index), object reference (kind, object number and, on its first
  occurrence, the contents; part 5), and `NotStored` for a DIVERGENT field the instance does not
  serialize. The read order is unchanged (the module doc's SCRIPT rule). A PScript shared
  by back-reference is not repeated.
- **New crate `lbp-runtime`** (runtime only; the parsing crates do not depend on it):
  - `LevelWorld` implements the VM `World` from a read level: Thing handles are Thing
    indices; parent, first child and next sibling (children in Thing-list order); the
    world Thing (the one with PWorld); UID; world position = translation column of the
    POS matrix; `has_part` maps `PSpriteLight` to the reader's `SPRITE_LIGHT`; script
    members (`GET/SET_SP_MEMBER` on a Thing) start from the stored PScript fields;
    `WRITE` lines are kept.
  - `instances()`: every Thing with a PScript resource GUID, with its class when a
    script with that GUID is loaded.
  - `Runner`: starts a named function on an instance with the Thing as `this` in
    register 0, keeps scripts that yield, and resumes them once per `tick()`.
- **Example `level_scripts`** (PC): scripted Things per level, class resolution by GUID,
  stored field names against the class layout, function names over the classes in use,
  and optionally a run of named entry functions on one level.

## Evidence

CONFIRMED_FROM_COMMUNITY_CODE: the PScript / InstanceLayout / ScriptObjectUID layout
(Toolkit d6a2ef80, already recorded in the `parts.rs` module doc).

INFERRED (each is a place the PC run or RPCS3 can disagree):

| Rule | Choice | How it gets settled |
|---|---|---|
| Script members | A Thing's script fields are its `SP` members, by field name | `level_scripts`: stored names vs class field names (`fields_missing_in_class`) |
| `this` | The Thing, in register 0 | Faults when the first entry runs |
| Thing handle | Thing index in the level's list | Internal choice; never shown to scripts except through natives |
| Children order | Thing-list order | Scripts that walk children (`GetFirstChild`/`GetNextSibling`) |
| World position | POS world matrix translation, w as stored | RPCS3 comparison |
| Object references | Strings become string values; arrays and instances become VM heap objects (part 5); resources and audio handles are null and counted as `unconverted_values` | The PC count says whether they matter |

Entry names (settled, PC D152 `level_scripts --all`, unit: distinct class, 49 classes in
use over 378 levels): `OnCreate__` 24 classes, `Update__` 11. The other frequent names are
constructors (`.ctor__`, `.init__`), UI (`UpdateUI__`, `Render__`), switch reactions and
enter/leave handlers, which the matching engine events will call later.

## Verification

Cloud (Linux): `cargo fmt --all --check` clean;
`cargo clippy -q -p lbp-runtime -p lbp-script-vm -p lbp-level --all-targets -- -D warnings`
clean; `cargo test -q -p lbp-runtime -p lbp-level -p lbp-script-vm -p lbp-ui-script` all
pass. New tests:
- `lbp-level` `script_part_keeps_field_names_and_values`: an inline layout of four fields
  (bool, f32, a divergent s32, a wide-string object), with and without compressed
  integers.
- `lbp-runtime`: the Thing tree, UID, position, parts and members of a three-Thing level;
  a synthetic `Lamp.Tick` that increments a member, yields, and finishes on the next
  tick; missing function and missing class outcomes.

CI now also tests `lbp-runtime`.

Not tested: any real level or game script (PC only).

## PC checks

1. `cargo run -q -p lbp-runtime --example level_scripts -- <snapshot> <game dirs incl. the
   d133 edat overlay> --all local-analysis/<new>.json`. Report the summary line, the top
   `instances_by_class`, `unresolved_by_guid`, and the top 30 `function_names`.
2. Then, on one level with scripted Things (the empty test level, or the first story
   level), run with `--entry=<name>` for the likely create and update names from step 1,
   for example `--entry=OnCreate --entry=OnUpdate --ticks=30`. Report the printed run
   lines, the first faults and the top unbound natives.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 2: Rookie Test create/update faults (2026-10-08)

Bound the world members and natives behind the most common faults of the D150 Rookie Test
run (`main_01_rookie_test`, `--entry=OnCreate__ --entry=Update__ --ticks=30`). Evidence is
the type tables of the user's scripts (field, property and function declarations with
machine types) and the local listings under ignored `local-analysis/` for how a result is
used; nothing from them is committed. Toolkit d6a2ef80 has none of these names.

| Item | Finding (label) | Binding |
|---|---|---|
| `Trigger_Speech_Bubble.IsTweaking` | A field of superclass `Tweakable` (bool, modifiers 0x1040), never in the stored layout (INFERRED: not serialized) | General: `LevelWorld::add_class_defaults` gives each scripted Thing every class and superclass field at its type default (`Vm::field_defaults`, shared with `instantiate`); stored values win |
| `Thing.SpriteLight` | `GET_SP_NATIVE_MEMBER` of type raw_ptr, then `SET_RP_MEMBER` FarDist (f32) and Color (v4) on it: the PSpriteLight part | Native member whose name is a reader part name (`thing::PARTS`) on any Thing script class: a part handle (`Raw(index + 1)`), `Raw(0)` when the Thing lacks the part; `ref_ptr` members of a handle keep what scripts write (reading an unwritten one is an error; part data is not read yet) |
| `Game.GetPlayOrEditMode__` | Returns s32 (`Game.EPlayOrEditMode`); `World.Update` takes the play-only branch on `== 0`, `World.RenderPreviewIcon` the edit branch on `== 1` (INFERRED) | `World::play_or_edit_mode`; LevelWorld: 0 (play) |
| `Game.GetNumPlayers__` | Already bound; LevelWorld lacked it | LevelWorld: 1 local player |
| `Audio.GetDialogueList__` | Returns s32 with builtin type 9, stored straight into `DialogueListFile` (same type) | `World::dialogue_list`; LevelWorld: 0 (no list loaded, INFERRED) |
| `Score_Board.GetShapeEditorColour__Q5Thing` | Declared native on `Thing` (returns v4); the VM named the referencing subclass | General VM fix: a native reached through a subclass reference goes to the host under the declaring class (`Vm::native_owner`). Bound `Thing.GetShapeEditorColour__Q5Thing` / `SetShapeEditorColour__Q5Thingr` on the PShape colour (packed 0xAARRGGBB, INFERRED byte order) |
| `PartList.HasPRenderMesh__Q5Thing` | Next fault after the above (21 Things) | Same part check as the other `HasP*` |

`level_scripts` now also writes `faults_by_kind` per entry (fault text with the Thing
number removed) and applies the class defaults before a run.

Results (PC, 01.33 with the d133 edat overlay; output
`local-analysis/d154-level-scripts-rookie-0133.json`), counting unit = scripted Thing:

| Entry | D150 done / fault / no_function | Now |
|---|---|---|
| OnCreate__ | 170 / 42 / 9 | 189 / 23 / 9 |
| Update__ | 0 / 22 / 199 | 21 / 1 / 199 |

Remaining faults (`faults_by_kind`): OnCreate `Native.SetAnimSpeed__Q5Thingf` (12,
Trigger_Speech_Bubble), `PartList.HasPCameraTweak__Q5Thing` (9), `Game.GetPlayerNumber__i`
(1, World), `Thing.SetRenderMeshEditorColourTint__Q5Thingr` (1, Score_Board); Update
`PWorld.GameMode` read through `World.World` before any script wrote it (1; needs PWorld
part data). The game's own test script still passes 512 of 513 checks
(`script_tests`, `local-analysis/d154-script-tests-0133.json`).

Tests: `lbp-runtime` `class_defaults_inherited_natives_and_parts` (superclass field
default under a stored value, part handle and its `ref_ptr` member, null part, non-part
name, players and mode, and an inherited native reported under its declaring class).

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 3: per-tick schedule (2026-10-08)

`lbp_runtime::LevelScripts` runs a level's scripts: `create` calls `OnCreate__` on every
scripted Thing in Thing order; each `tick` resumes scripts suspended by `System.Yield` in
the order they yielded, then calls `Update__` on every scripted Thing that was not
suspended when the tick began, in Thing order. A Thing whose script faults is not run
again. `TICK_HZ` = 30 (X1). `level_scripts ... LEVEL-PATH NEW-JSON --play=N` runs create
and N ticks and reports totals (done, yielded, resumed, no function, faults), ticks with
faults, Things stopped by a fault, scripts still suspended, faults by kind, unbound
natives and `WRITE` lines.

INFERRED (each settles against RPCS3 or script behaviour):

| Rule | Choice | How it gets settled |
|---|---|---|
| Create order | Thing order, all before the first tick | Scripts that read another Thing's members in `OnCreate__` |
| Suspended `Update__` | Resumed next tick instead of a new `Update__` | Scripts that yield inside `Update__` (a loop that yields every tick behaves the same either way) |
| Update order | Thing order | RPCS3: two scripts that write the same member |
| After a fault | The Thing's script stops | Our choice; the game does not fault |

Cloud test: `schedule_creates_then_updates_and_resumes_suspended_scripts` (a class with
`Update__` that increments a member and yields: create reports no function, tick 1 runs and
yields, tick 2 only resumes, tick 3 runs again).

PC check: `cargo run -q -p lbp-runtime --example level_scripts -- <snapshot> "<game dirs>"
<Rookie Test path> local-analysis/<new>.json --play=300` (10 s). Report the printed play
line, `faults_by_kind` and the top unbound natives.

## Part 4: the next four natives (2026-10-08)

Bound the four natives behind the Rookie Test create faults left after part 2. Evidence:
the declaring script type tables and how the local listings (ignored `local-analysis/`)
use each result; Toolkit d6a2ef80 has none of these names. Nothing from them is committed.

| Native | Finding (label) | Binding |
|---|---|---|
| `Native.SetAnimSpeed__Q5Thingf` | Static, (Thing, f32) -> void; scripts pass their own Thing and 0 (stop), 1, or a tweak value x 4 | `World::set_anim_speed`; LevelWorld records `ThingState::anim_speed` (INFERRED: a rate multiplier; nothing animates by it yet) |
| `PartList.HasPCameraTweak__Q5Thing` | Static, Thing -> bool, like the other `HasP*` | Part check on `CAMERA_TWEAK` |
| `Game.GetPlayerNumber__i` | Static, s32 -> `Game.EPlayerNumber` (s32); every caller loops the index over `0..GetNumPlayers()` | `World::player_number`; LevelWorld: index -> same value (INFERRED, the tables carry no enum values), outside the range an error |
| `Thing.SetRenderMeshEditorColourTint__Q5Thingr` | Static, (Thing, v4) -> void; called right after `SetShapeEditorColour` with a colour | `World::set_render_mesh_tint`; LevelWorld records `ThingState::render_mesh_tint` (any Thing; nothing draws it yet) |

Results (PC, 01.33 + d133 edat overlay, `--play=300`, unit = scripted Thing):

| Level | Create done / faults / no_function | Ticks | First faults (`faults_by_kind`) |
|---|---|---|---|
| Rookie Test (`d165-play-rookie-0133.json`) | 189 / 23 / 9 (D159 same counts) | 0 faults; all 198 running Things have no `Update__` | `Native.SetAnimPos__Q5Thingf` 12, `LAMS.MakeLamsKeyID__Q6String` 9 (both Trigger_Speech_Bubble), `Game.SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi` 1 (World), `Gooey.CreateGooey__i` 1 |
| Pod planets (`d165-play-pod-planets-0133.json`) | 2 / 1 / 0 | 0 faults | `PodComputer.GetPodComputer__` 1 (PlanetsWorld via `PlanetsView.SetPlanetsView__i`) |
| Pod (`d165-play-pod-0133.json`) | 0 / 1 / 1 | 0 faults | `Native.DebugPrint__i` 1 (Pod.OnCreate__) |

The counts did not move on Rookie Test: each Thing now faults later in the same function.
The 21 Things with an `Update__` (Trigger_Speech_Bubble) are among the 23 stopped in create.

What PlanetsWorld and OrbitingMoon need next (names and counts of call sites, from the
listings). OrbitingMoon has only `GetPosition__` and calls no natives; PlanetsWorld drives
the camera and has no `Update__` (its per-frame entry is unknown: `UpdateState__`,
`OnEnterPod__` and `OnExitPod__` are candidates). PlanetsWorld calls 34 unbound natives
(44 sites): `Audio.SetInteractiveMusicStereoPairVolume__giff` 6, `PodComputer.GetPodComputer__` 3,
`Math.lerp__frr` 3, `Math.Smooth3D__rrrff` 2, `Cam.SetCameraFromToUp__rrrffff`,
`Cam.SetDOFAndFog__fff`, `Cam.ClearMotionBlur__`, `Thing.SetWorldPosition__m`, `Math.Smooth__fffff`,
`Math.SmoothStep__fff`, `PodComputer.GetPodComputerInUse__/GetCrashZoom__/GetLoadingBadgeMat__`,
`HUD.GetCinemaModeCount__`, `Video.IsVideoPlaying__`, `Input.IsInAttractionLoop__i`,
`Profile.IsLBP2GameProgressionFlagSet__i`, `Game.GetPreviousLevelSlotID__/IsPreviousLevelValid__/
GetLocalLeadersPlayerNumber__/IsNonInteractive__/GetNumPendingLinks__/GetNumAvatars__/
GetPoppetPos|Mode|SubModeFromTemporaryIndex__i`, `PlanetsWorld.CanStartEndOfLevelSequence__`,
a `GetType__`, and the other `Audio` music natives (1 each). PlanetsView (its helper class)
adds `System.Is4to3__` 3, `Planets.GetCrafteroid__i` 2, `GetThing__` 2,
`PodComputer.GetPlanets__` 1, and an unnamed native member (field 12, raw_ptr) read 11 times
before `GET_RP_MEMBER WorldPosition` (INFERRED: the PPos part). World members: object
members of script instances (`GET_OBJ_MEMBER` 124 x `Offset`, `Yaw`, `FocusPlanetType`, ...)
need object-reference values (Next).

Tests: `lbp-runtime` `part4_anim_speed_tint_camera_tweak_and_player_numbers`.
`script_tests` (`d165-script-tests-0133.json`): 47 functions done, 512 passed / 1 failed.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 5: object-reference members (2026-10-09)

The Pod planets run showed PlanetsView reading about 124 object-reference members the
VM did not model. Stored script objects now reach the VM:

- **`lbp-level`**: `ScriptValue::Object { kind, id, body }`. `id` numbers the level's
  script objects in first-occurrence order (the per-class sequence of the shared
  reference table), so a back-reference carries the `id` of its first occurrence; only
  the first occurrence has a `body` (`ScriptObject`): bool / s32 / f32 / v4 arrays, Thing
  arrays (Thing indices), object arrays (nested values), a resource (type and
  descriptor), a nested instance (`ScriptPart`), text (STRINGW and STRINGA), audio handle.
  The read order is the module doc's ScriptObjectUID rule, unchanged.
- **`lbp-script-vm`**: `Vm::new_array`, `Vm::array_mut`, `Vm::fields_mut` for hosts that
  load stored objects.
- **`lbp-runtime`**: `LevelWorld::from_things` collects every object body by number
  (members, array elements, instance fields). `LevelWorld::bind_objects(&mut vm)`
  allocates one heap object per number (so shared references stay one object, cycles
  included), fills object arrays and instance fields, then points members at them. An
  instance gets the class with its script GUID, class defaults first, stored fields on
  top. Strings stay string values. Resources, audio handles and instances of classes the
  VM has not loaded stay null and are counted in `unconverted`. Returns the number of
  heap objects built; `level_scripts` reports it as `objects_built`.

| Rule | Evidence | Applicability |
| --- | --- | --- |
| Object numbers follow the shared tag table (a back-reference is the same object) | INFERRED from the ScriptObjectUID layout (module doc of `parts.rs`) | Instances in levels read by `lbp-level` |
| An instance object starts from class defaults, stored fields override | INFERRED, the same rule as a scripted Thing's members (part 2) | Classes the VM has loaded |
| Resource objects are null | Not modelled: the VM has no resource value yet | Counted, so the PC count shows whether it matters |

Tests: `lbp-level` `script_objects_keep_contents_and_shared_identity` (string, f32
array, object array with a back-reference, a null and a nested instance);
`lbp-runtime` `stored_objects_become_shared_heap_objects`.

Not tested: real levels (cloud has no game data). PC check: `level_scripts` on
pod_planets (00018fdb) with `--play`, reporting `unconverted_values`, `objects_built`
and the PlanetsView faults.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 6: the Pod camera and the next Rookie natives (2026-10-08)

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
| `Smooth*(pos, vel, target, max_acc, max_vel)`: PlanetThing passes its `MaxAcc` then `MaxVel` (defaults 0.01, 10); result = new velocity (2D/3D, caller adds it) or (new value, new velocity) (scalar `Smooth__fffff`, the caller reads x and y). The step rule (head for the target at most `max_vel`, velocity change at most `max_acc` per tick) | order: INFERRED from use; rule: INFERRED |
| `lerp(t, a, b)`, `SmoothStep(e0, e1, x)` | INFERRED, vector-library conventions |
| `Cam.*`, `SetWorldPosition__m`, `SetAnimPos`, `SetPlayerScoreMultiplierTimer` record state on `LevelWorld` (`camera`, `world_matrix`, `anim_pos`, `score_multiplier_timers`); the viewer can read `LevelWorld::camera` | recorded, nothing renders it yet |
| PC-run defaults: pod computer not in use, no crash zoom, no cinema mode, no avatars, no video, 16:9, player 0 leads, no music, interactive music audible, no attraction loop, interactive, no pending links | INFERRED defaults (`state_query`) |
| `IsLBP2GameProgressionFlagSet__i`: `LevelWorld::progression_flags` (empty = fresh profile; `level_scripts --flags=2,..`). On a fresh profile PlanetsWorld starts its intro cutscene (`SetState(1)` -> a Thread) | flag 2 meaning: INFERRED |
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

## Part 7b: pod computer in use (2026-10-09)

`LevelWorld::pod_computer_in_use` backs `PodComputer.GetPodComputerInUse__` (default
false, INFERRED: nobody at the computer when the Pod loads). `level_scripts --play=N
--puter-in-use=TICK` makes it true from that tick (0 = from `OnCreate__`) and reports
`camera_before_in_use` next to the final `camera`, so one run gives both presets for the
RPCS3 comparison. `SetCameraFromToUp` floats (10, 100000, 0, 1): INFERRED near 10 and far
100000 (the PC reading); the last two are UNKNOWN. Test: `lbp-runtime`
`part6_camera_world_position_and_engine_state` (flag on and off). Counts: +0 newly
viewable, +0 newly Complete (no asset change).

## Part 8: script threads (2026-10-09)

Fresh-profile Pod planets stopped at `Thread.StartImpl__`: PlanetsWorld.SetState starts
the intro cutscene as a thread (PodIntroductionThread : PodThread : Thread). PC findings
(behaviour only): Thread is abstract with a private `Running` bool; `Start__` asserts
`!Running`, sets it and calls the native `StartImpl__(this)`; `ThreadMain__` asserts
`Running`, makes three virtual calls on `this` (most likely OnStart, Run, OnFinish) and
clears `Running`; `.get.Finished__` = `!Running`. No stop, sleep or is-running native:
waiting is a script loop over `System.Yield__` (PodThread `Wait__i`,
`WaitUntilStartPressed__i`, `PlayAudioAndWait__`). `Thread.Start__` is called from 15
classes (PodComputer 9, Race_End 3, Score_Board 3, PodState menus, PlanetsWorld 2, ...).

Implementation: the VM records the object passed to `Thread.StartImpl__` (null faults the
caller). `LevelScripts` runs `ThreadMain__` on each new thread at the end of the step
that started it (create or tick) until it yields, and resumes suspended threads once per
tick after the Things' suspended scripts and before the updates. Threads that start
threads run in the same step (at most `MAX_THREAD_STARTS`, 256). A faulted thread is
dropped and reported. `StepReport` gains `threads_started`, `threads_done` and
`thread_faults`; `level_scripts --play` reports them and `threads_running_at_end`.

| Rule | Label |
| --- | --- |
| A started thread runs `ThreadMain__` with the thread object as `this` | INFERRED from the listings (PC) |
| It first runs at the end of the starting step, then once per tick | INFERRED; the Pod intro timing in RPCS3 can check it |
| Threads resume after suspended Thing scripts, before updates | INFERRED |

Test: `lbp-runtime` `script_thread_runs_until_yield_then_resumes_each_tick` (a synthetic
Thread and a Thing script that starts it: writes 1 at create, 2 on the next tick; a
null thread faults the starting script). `cargo +1.99.0 clippy -p lbp-script-vm -p
lbp-runtime --all-targets -- -D warnings` clean; tests pass.

PC check: `level_scripts` on Pod planets with `--play=300` and no flags (fresh profile).
Expect `threads_started` 1 and the intro thread's next blockers in `thread_faults` (video
and voice-over natives: `StartVideo__g`, `IsPlayingVideo__`, `RequestVoiceOver__Q7StringAbb`,
`IsVoiceOverPlaying__`). Then compare with RPCS3: what the fresh-profile Pod shows first
and for how many ticks.

## Part 7: level scripts in the viewer (2026-10-09)

`lbp-bevy-viewer --level-guid HEX|--level-path PATH --level-scripts
[--level-scripts-flags N,..]` (module `level_play.rs`) runs the level's scripts inside the
viewer, at the game tick: `Time<Fixed>` at `lbp_runtime::TICK_HZ` (30), one
`LevelScripts` step per `FixedUpdate`. The first step is `OnCreate__`; every later one
is a tick (resume, then `Update__` or the class's frame entry). The setup is the same as
`level_scripts --play`: load every script with its GUID, read the level,
`bind_objects`, `add_class_defaults`, the progression flags, `add_engine_objects`. A
"Level scripts" window shows the tick, create and tick counts, ticks with faults, the
first fault lines, the most-called unbound natives, the last camera set through
`Cam.SetCameraFromToUp` and the last `WRITE` lines. `--qa-hide-ui` hides it.

Nothing the scripts change is drawn yet (camera, positions, tints): this part is the
clock and the counts. Built and linted in the cloud (`cargo +1.99.0 clippy -p
lbp-bevy-viewer --all-targets -- -D warnings`, viewer tests pass); not run, as the cloud
has no game data or display.

PC check: the viewer on Pod planets (00018fdb) with `--level-scripts
--level-scripts-flags 2` on the second monitor. After a few seconds the window should
show ticks rising at 30 per second, 0 faults and the camera from (0, -250, 1130) to
(0, 100, -1500), as `level_scripts --play` reported in part 6. Then Rookie Test, where
the counts should match part 6 (create 189 / 23 / 9).

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 9: run past unbound natives, counted at run time (2026-10-12, D258)

Stage 1 of `docs/rscript-plan.md`.

- `Bindings::default_unbound` (constructor `Bindings::defaulting`): a native without a
  binding returns nothing, which the VM already turns into the default of the call's return
  machine type (false, 0, null, zero vector). The call is counted in `Bindings::unbound`
  and never as bound. Off by default, so the VM and runtime tests still see a missing
  binding as a fault. On in the viewer (`--level-scripts`, whose window now shows bound and
  defaulted calls) and in `level_scripts --play`; `--strict` turns it off there.
- `level_scripts ... --all NEW-JSON --play=TICKS [--strict]` plays every readable level on
  its own VM (no heap object or static outlives its level) and writes `play_all`: native
  calls bound and unbound, every unbound native with its calls and levels, and the faults
  that remain by kind. A bound native whose `World` lacks the feature still faults.
- 19 natives bound (116 in the table). Evidence for each: declared types from the script
  tables, and the rule read in the 01.33 executable through the Ghidra bridge (FOUND)
  unless marked.
  - `String`: `Length`, `IsNullOrEmpty`, `Find`, `SubStr__Q6Stringii`, `ToUpper`,
    `StringFromStringA`. A wide string: lengths and indices count UTF-16 units. Null has
    length 0, is "null or empty", is never found in (-1) and gives an empty substring.
    `SubStr(s, start, count)` is empty when `start` is at or past the end and stops at the
    end. Negative `start` or `count` is an error here (not observed in a run). `ToUpper`
    maps unit by unit (INFERRED to be the C library's wide upper-case).
  - `StringA`: `Length`, `IsNullOrEmpty`, `Find`, `SubStr`, on bytes. INFERRED to follow
    the `String` forms; their code was not read.
  - `Math`: `identity__`, `ceil__f`, `exp__f`, `pow__ff` (C library), `sign__f` (1 from
    zero up, -1 below, never 0), `lerp__fff` (t, a, b: a + (b - a) t), `Delt__fff` (from,
    to, step: move towards `to` by at most `step`, landing on it), `BiasCosine__f`
    (0.5 - 0.5 cos(pi x)).
  - `Resource.LoadTextureByFilename__gb`: the `__g` loader with a flag passed to the
    load. The flag's meaning is not settled; nothing is loaded either way.
  - `LAMS.MakeLamsKeyID__Q6String` now takes a null string as the empty string (FOUND).
- Not bound, of the plan's 10 Math natives: `DeterministicRandomBetween__ff` (FOUND: a
  30-bit linear congruential state kept by the world, top 14 bits scaled to the range; the
  state's start value is not known yet) and `ScreenToWorldSpace__pf` (needs the camera).

PC run, 378 levels read (20 not read), 60 ticks each, 13,547 scripted Things. Unit: native
call; scripted Thing (a Thing stops at its first fault).

| | Things stopped by a fault | Native calls bound | Unbound calls (distinct natives) |
|---|---|---|---|
| Before (main, unbound natives fault) | 4,940 | 26,186 | 4,169 (19) |
| After, `--strict` | 4,940 | 26,390 | 4,108 (18) |
| After (defaulted) | 1,722 | 51,030 | 76,269 (64) |

Before, the engine-owned objects also faulted in all 378 levels (the Gooey pool's
`.init__`); after, in none. Thread faults 1 before, 0 after. The static census of
`docs/rscript-plan.md` drops from 272 to 253 unbound natives reachable from create/update;
`LoadTextureByFilename__gb` was never called in these runs (it is reachable, not reached).

Most-called unbound natives after (calls / levels): `Native.IsEnemyAI__Q5Thing`,
`Native.SetWalkInput__Q5Thingp`, `Thing.IsDead__`, `Thing.IsInLimbo__` 11,529 / 26 each;
`Thing.GetBodyRoot__` 7,076 / 27; `Gooey.PerFrameUpdate__Q18Game.EPlayerNumberi` 6,222 /
101; `LAMS.IsInLams__i` 4,041 / 161; `HUD.PreloadGlyphs__Q11Fonts.EFontQ6String` 2,938 /
161; `EnemyWard.EnemyWardNotify__Q5Thing` 2,013 / 4; `Thing.UpdateScale__` 1,469 / 161;
`GameMessagingManager.Messaging_ShouldFilterMessageStrings__` 1,184 / 142;
`Thing.SetRenderMeshEditorColour__Q5Thingr` 1,180 / 135.

Faults that remain after (Things / levels):

| Fault | Things / levels |
|---|---|
| `Thing.IsSwitchTriggered__i`: no switch inputs (ScoreTweak 1,106, TweakSpinningObject 34, TriggerAirStone 3) | 1,143 / 56 at most |
| `PWorld.GameMode` member not provided (World script) | 220 / 219 |
| `Thing.SetLocalPosition__m`: no local transforms (TriggerCollectHandle 102, TriggerCollectGun 97) | 199 / 34 at most |
| Null object in `TriggerLevelLink.Init__` (after a defaulted native returned null) | 113 / 103 |
| Native member `Pos` of Race_Start (24) and Race_End (22) | 46 / 41 at most |
| Null Thing in `Pod.SetPodTransition__b` | 1 / 1 |

Rookie Test alone, 300 ticks: create 189 done, 22 yielded, 1 fault (the World script, on
`PWorld.GameMode`), 9 without the function; 221 instances, 1 stopped (part 6: create
189 / 23 / 9).

A defaulted native is a guess at "nothing there": a script that runs on may take a path the
game would not. The counts say which natives to bind next, not that those scripts are
right.

Verification: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings` clean, `cargo test` on the non-viewer crates passes (new tests
`stage1_strings_and_math`, `unbound_natives_fault_or_default`). Not run in the viewer.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 10: trigger events (2026-10-12, D259)

Stage 2 of `docs/rscript-plan.md`: a level's PTrigger parts call `Enter__Q5Thing`,
`Inside__Q5Thing` and `Leave__Q5Thing` on its scripts when a player walks in.

- `lbp-level` keeps PTrigger (`PartData::trigger`): type, the stored in-list, radius
  multiplier, all-layers flag, hysteresis multiplier, enabled flag, z offset.
- `LevelWorld::add_player` / `move_player`: a player's Sackboy as a Thing of the level
  (BODY, POS, YELLOWHEAD, CREATURE) with a position and the z span of his body.
  `PYellowHead.PlayerNumber` is his index (INFERRED s32).
- `LevelWorld::update_triggers`, one tick of every trigger. FOUND in the 01.33 executable
  through the Ghidra bridge (trigger pass 0xe3bc0, per trigger 0x1364f0, test 0x135fe0,
  add 0x135a18, remove 0x1346fc):
  - The player's position is taken into the trigger Thing's own space (inverse world
    matrix), so the Thing's scale sizes the trigger. The radius is the radius multiplier,
    times the hysteresis multiplier while the in-list is not empty.
  - Type 0: x^2 + y^2 < r^2. Type 1: |x| < r and |y| < r. Type 4: x^2 + y^2 + z^2 < r^2.
    Any other type holds nothing. A trigger that is not enabled holds nothing.
  - Without the all-layers flag a Sackboy counts only when the trigger's z plus its z
    offset lies within the z span of his body.
  - Newly inside: appended to the in-list, Enter. No longer inside: removed, Leave. Then
    Inside for every Thing in the list.
  - Ordinary triggers test the players' Sackboys only.
- `LevelScripts::trigger_events` calls the scripts; `tick_level` is `tick` followed by it.
  FOUND: Enter and Leave go to the script of the trigger's Thing, or to its parent's when
  that Thing has none; Inside goes to the parent's script. The Thing that entered is the
  argument (register +4).
- Bound (118 in the table): `Native.GetNumInThings__Q5Thing` (FOUND 0x648770: the in-list
  count, 0 for a null Thing or no PTrigger) and `PartList.HasPYellowHead__Q5Thing`
  (INFERRED from its name). `thing.YellowHead` now resolves to the YELLOWHEAD part.
- Viewer: with `--level-scripts` and `--play-sackboy` together, the play-mode Sackboy is
  the player; the Level scripts window shows the events raised and run. His z span is a
  PLACEHOLDER (lane z +-100).
- `level_scripts`: `triggers` (census), and `--visit-triggers`: after the `--play` ticks a
  player stands on every trigger's Thing in turn, two ticks on it and one far away.

Not modelled, read in the same code: seven engine object kinds whose triggers test other
Things than players; four object kinds that alone get Inside (here any parent script with
the function gets it; the three classes that define it are the collect power-ups);
object kinds the engine handles without a script (19,328 triggers have no script on their
Thing or its parent); Sackboy states that never count (creature state 5, a YELLOWHEAD
flag); the world-bounds cull. INFERRED: the player's point is his Thing position (the game
asks his creature part); triggers run in Thing order, after the tick's updates.

PC run, 378 levels read, d133 snapshot with the edat overlay. Unit: Thing with a PTrigger
part; trigger event; scripted Thing.

| PTrigger census | Things |
|---|---|
| All | 23,990 |
| Type 0 (circle) / type 1 (box) | 23,715 / 275 |
| All layers | 5,397 |
| Not enabled | 5 |
| Stored in-list not empty | 2 |
| No POS | 2,235 |
| Script on its own Thing / on the parent / none | 2,388 / 2,274 / 19,328 |

Script classes behind the 4,662 scripted triggers: Trigger_Speech_Bubble 1,402 (parent),
TriggerGlobalSettings 1,180, TriggerMusic 981, Score_Board 205 (parent),
TriggerCollectGrapple 163, TriggerLevelLink 113, TriggerCollectGun 109,
TriggerCollectHandle 102, TriggerCollectCape 80, TriggerWormHole 79,
TriggerCollectGauntlets 75, 10 more below 50 each.

| 60 ticks per level | No player | Then a visit to each of 21,755 triggers |
|---|---|---|
| Enter / Inside / Leave raised | 0 / 120 / 0 | 39,212 / 101,918 / 39,178 |
| Enter / Inside / Leave run by a script | 0 / 60 / 0 | 11,430 / 895 / 7,357 |
| Events with no script to call | 0 | 117,648 |
| Events for a Thing already stopped | 0 | 1,410 |
| Things stopped by a fault (of 13,547) | 1,722 | 1,722 |
| Native calls bound / unbound (distinct unbound) | 51,030 / 76,329 (65) | 215,292 / 778,473 (94) |

The events add no fault. They add 29 unbound natives; the most called (calls / levels):
`Thing.GetInput__Q5Thingi` 57,400 / 166, `BaseWorld.IsScoreBoard__Q5Thing` 46,386 / 108,
`Thing.GetOutput__Q5Thingii` 8,249 / 150, `Game.IsOnSequencer__Q5Thing` 7,333 / 130,
`Audio.StopSound__Q11AudioHandle` 5,226 / 88, `Game.Get/SetGlobalSettingUID` 2,549 / 96
each, `GameMessagingManager.Messaging_NewGameEvent__iiQ5Thingb` 1,845 / 93,
`Native.GetInThing__Q5Thingi` 1,764 / 89, `Audio.IsSafeToTriggerMusic__i` 1,446 / 133.
The unbound call total grows mostly because the visit adds about 65,000 level ticks of
the enemy scripts' per-tick natives.

The visit stands on a trigger's Thing with his z span around the trigger, so it measures
what the scripts do once an event fires, not where a real player would walk. A defaulted
native still makes a script take a path the game might not.

Verification: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test` on the non-viewer crates (new:
`stage2_trigger_holds_the_player_by_shape_hysteresis_and_layer`,
`stage2_trigger_events_call_the_parent_script`), `cargo check -p lbp-bevy-viewer`. Not run
in the viewer window.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 11: switch inputs (2026-10-13, D260)

Stage 3a of `docs/rscript-plan.md`: a script can ask what is wired to its Thing. The logic
network is not evaluated yet, and `SwitchPerformReaction__b` is still not called.

- `lbp-level` keeps PSwitch (`PartData::switch`): inverted, type, behavior, the
  manualActivation signal and every output with its signal (activation f32, ternary s32,
  player s32) and its targets (Thing, port). The bytes read are the same as before.
- `LevelWorld::wire_inputs` builds each Thing's input table (`ThingState::inputs`).
  FOUND in the 01.33 executable through the Ghidra bridge: a Thing has a table with one
  entry per input port, each empty or pointing at the SwitchOutput wired to it; the
  output holds its signal and a pointer to its PSwitch. INFERRED: the table is what the
  outputs' stored targets say. A target (Thing, port) puts its output on that port; the
  table is as long as the highest wired port + 1; when two outputs target one port the
  first in Thing order is kept and the other counted.
- Natives (123 in the table). FOUND, native and the reader it wraps:
  - `Thing.IsSwitchTriggered__i` (0x64228c, 0x7541e0): true when an output is wired to
    the port. It does not look at the signal.
  - `Thing.GetSwitchActivationBinary__i` (0x642228, 0x754308): wired and ternary not 0.
  - `Thing.GetSwitchActivationAnalogue__i` (0x6421c4, 0x7542a4): the activation; 0.0 when
    nothing is wired (the constant the reader loads, read at 0xdb26b8).
  - `Thing.GetSwitchActivationTernary__i` (0x642160, 0x75437c), new: the ternary as a
    float (declared f32); the same 0.0 otherwise.
  - `Thing.GetSwitchActivationPlayer__i` (0x6420fc, 0x754ccc): the signal's player when
    it is not -1.
  - `Thing.GetInput__Q5Thingi` (0x643f2c, 0x754244), new: the Thing of the PSwitch whose
    output is wired to the port; null for a null Thing, a port out of range or nothing
    wired.
  - `Thing.GetOutput__Q5Thingii` (0x643ea0, 0x754af8), new: (thing, output, target), the
    Thing that target of that output of the Thing's own PSwitch points at; null for a
    null Thing, no PSwitch or a target index past the output's targets.
  - `Native.GetInThing__Q5Thingi` (0x6486ec, 0x6d558c), new: entry i of the PTrigger's
    in-list; null for a null Thing or no PTrigger.
  - `BaseWorld.IsScoreBoard__Q5Thing` (0x64bae0, 0x7569ec, 0x622658, 0x164454), new: the
    Thing has a PScript whose script is the scoreboard script (GUID 11599, the constant
    the test loads its script by) or has it on its superclass chain.
- The old `SwitchInput` (triggered, binary, analogue, player, an INFERRED split) is now
  the wired signal, and `World::switch_input` returns it or nothing.

Not modelled, read in the same code: with player -1 or nothing wired,
`GetSwitchActivationPlayer` falls back to the player who owns the Thing (here -1); the
Binary, Analogue, Ternary and Player forms read as "nothing wired" while a flag byte of the
Thing (+0x7f, meaning unknown) is set; `GetOutput` and `GetInThing` do not check the
output index or the in-list index (here out of range is null).

The limit of this part: signals are the ones the level file stores. No sensor, gate or
timer runs, so a signal never changes, and a script that waits for its input to turn on
waits for ever. `IsSwitchTriggered` is exact all the same, because it only asks whether
something is wired.

PC run, 378 levels read, d133 snapshot with the edat overlay. Unit: Thing with a PSwitch
part; switch output; output target; input port; scripted Thing.

| PSwitch census | Count |
|---|---|
| Things with a PSwitch | 141,572 |
| Inverted | 9,588 |
| Outputs / null outputs | 144,608 / 0 |
| Stored ternary 0 / 1 / -1 | 136,253 / 8,279 / 76 |
| Outputs with activation not 0 / with a player | 9,486 / 0 |
| Targets / with a null Thing | 177,800 / 0 |
| Things with an input table | 136,337 |
| Wired ports / port collisions | 177,800 / 0 |
| Most ports on one Thing | 75 |
| Scripted Things with a wired port (of 13,547) | 3,732 |

Every target lands on its own port: no collision in 177,800 targets, which supports the
INFERRED table build. By type, 38 values; the largest: 36 (microchip) and 37 28,278 each,
4 16,953, 16 15,513, 15 7,090, 23 5,794, 14 5,727, 3 5,627, 28 4,599, 21 4,089. Script
classes of the 3,732: ScoreTweak 1,102, Trigger_Speech_Bubble 822, TriggerGlobalSettings
530, Launcher 333, TriggerMusic 329, SmokeMachine 227, 19 more below 80 each.
`IsScoreBoard` holds for 102 scripted Things, the 102 Score_Board instances.

| 60 ticks per level | D259 | D260 | D259, trigger visit | D260, trigger visit |
|---|---|---|---|---|
| Things stopped by a fault (of 13,547) | 1,722 | 582 | 1,722 | 582 |
| Fault kinds | 11 | 9 | 11 | 9 |
| Native calls bound | 51,030 | 186,679 | 215,292 | 1,069,101 |
| Unbound calls (distinct natives) | 76,329 (65) | 76,569 (66) | 778,473 (94) | 645,956 (91) |

Trigger events raised and run are the same as in part 10. The 1,140 Things that stopped on
`IsSwitchTriggered` (ScoreTweak 1,106, TweakSpinningObject 34) now run their update every
tick, which is most of the new bound calls.

Faults that remain (Things / levels), the same with and without the visit:

| Fault | Things / levels |
|---|---|
| `PWorld.GameMode` member not provided (World script) | 220 / 219 |
| `Thing.SetLocalPosition__m`: no local transforms (TriggerCollectHandle 102, TriggerCollectGun 97) | 199 / 34 at most |
| Null object in `TriggerLevelLink.Init__` (after a defaulted native returned null) | 113 / 103 |
| Native member `Pos` of Race_Start (24) and Race_End (22) | 46 / 41 at most |
| `Thing.IsInactive__`: no activity state (TriggerAirStone, now past its switch test) | 3 / 1 |
| Null Thing in `Pod.SetPodTransition__b` | 1 / 1 |

Unbound natives. Without the visit the list is the one of the D259 run plus
`Game.IsOnSequencer__Q5Thing` 240 / 3. With the visit, `Thing.GetInput__Q5Thingi`,
`BaseWorld.IsScoreBoard__Q5Thing`, `Thing.GetOutput__Q5Thingii` and
`Native.GetInThing__Q5Thingi` are gone (113,799 calls in part 10) and
`Game.IsLocalPlayerID__Q18Game.EPlayerNumber` is new (1,086 / 62). Most called now (calls /
levels): `Native.IsEnemyAI__Q5Thing`, `Native.SetWalkInput__Q5Thingp`, `Thing.IsDead__`,
`Thing.IsInLimbo__` 113,850 / 26 each; `Thing.GetBodyRoot__` 65,258 / 27;
`Gooey.PerFrameUpdate__Q18Game.EPlayerNumberi` 53,301 / 101;
`EnemyWard.EnemyWardNotify__Q5Thing` 31,101 / 4; `Game.IsOnSequencer__Q5Thing` 4,459 / 79;
`LAMS.IsInLams__i` 4,157 / 161; `Audio.StopSound__Q11AudioHandle` 3,246 / 62;
`HUD.PreloadGlyphs__Q11Fonts.EFontQ6String` 2,938 / 161; `Game.IsVcrPaused__` 2,549 / 96;
`Game.Get/SetGlobalSettingUID` 2,091 / 73 each. Several counts fell against part 10
(`LAMS.IsInLams__i` by 5,081): with real inputs and outputs the scripts take other paths.

Verification: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test` on the non-viewer crates (new:
`stage3_input_tables_are_built_from_the_switch_outputs`,
`stage3_switch_input_natives_follow_the_wired_output`,
`stage3_outputs_in_things_and_score_boards`), `cargo check -p lbp-bevy-viewer`. Not run in
the viewer window; no switch compared against the game in RPCS3.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Next

- Stage 3b of `docs/rscript-plan.md`: evaluate the logic network each tick (sensors,
  gates, timers write the outputs' signals) and call `SwitchPerformReaction__b`. Until
  then the signals of part 11 never change.
- With it: the owner-player fallback of `GetSwitchActivationPlayer`, the Thing flag at
  +0x7f, `Thing.IsInactive__` on the level world, `Game.IsOnSequencer__Q5Thing`.
- The z span of Sackboy's body for the layer rule (a memory read in RPCS3).
- Drive the viewer camera from `LevelWorld::camera` (part 7 runs the scripts and shows
  the camera values; the level-unit to display-unit mapping needs a PC capture).
- Script threads (`Thread.Start__` -> `Thread.StartImpl__`) for the Pod intro cutscene.
- PlanetThing `Update__rb` as a frame entry (its arguments are still unknown).
- From part 9, still open: `PWorld.GameMode`, local transforms, `Pos` of the race gates.
- Rookie Test: `LAMS.IsInLams__i` (needs the translation table), then
  `SetPlayerScoreMultiplierCounter`, Gooey members; Pod: `Thing.FindThing__Q7StringA`.
- Resource object values, if the PC count shows scripts reading them.
