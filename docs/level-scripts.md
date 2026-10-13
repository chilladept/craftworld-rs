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

## Part 12: the switch network and switch reactions (2026-10-13, D261)

Stage 3b of `docs/rscript-plan.md`. Part 11 let scripts read switch signals, but the
signals stayed as the level file stores them. Now the network is evaluated each tick and
the scripts are told when an output wired to them turns on or off.

### What the game does (FOUND, 01.33 executable)

- The world update (0xeacd4) runs one switch pass (0xdd228): the switches are sorted by
  their wiring, a source before its targets, and updated once each in that order
  (0x1244a4). A wire that goes backwards in the order reads last tick's value.
- Each output is computed by 0x11bd58 through a jump table on the switch type (0x11be00,
  a second table 0x11bfd4 for an inverted switch). A signal is {activation, ternary,
  player}; an unwired port reads {0.0, 0, -1}.
- Per output: `on` = new ternary not 0 and (ternary changed or forced). The targets are
  visited when on/off changed, the player changed, or `on`. So a target sees a rising
  edge as true, a falling edge as false, and a sign flip while on as true again.
- The visit (0x11ddf0) acts by the target Thing's object kind. Toggle, counter, selector
  and timer change their state here, on the input edge. For 16 script GUIDs it calls
  `SwitchPerformReaction__b(on)` on the target Thing's own script; for one GUID
  `SwitchPerformReaction__bf(on, x)` with x = 1.0 when the output's ternary is not 0.
- The object kind (0x3ed2c) looks at other parts first and at the switch type before the
  script: a scripted Thing that also carries one of those parts or a switch of most types
  gets no reaction call.
- A microchip is off when its port 0 is wired and reads 0, or when the board it sits on
  is off (0x409dc). Everything on an off board outputs {0, 0, -1}; latches, counts and
  timers keep their state.

### What the runtime does

`crates/lbp-runtime/src/switches.rs`. `LevelScripts::tick_level` runs the script tick, the
trigger events, then `LevelWorld::update_switches`.

| Type | Name (INFERRED from the string order) | Output |
|---|---|---|
| 36 | microchip | output i = the signal on the board Thing's port i |
| 37 | circuit board | output i = the signal on the owner's port i + k |
| 23, 24, 40 | NOT, pass, node | input 0; NOT inverts through the inverted flag |
| 14 | AND | mode 0: smallest magnitude with the sign product; mode 1: product |
| 21 | OR | mode 0: largest magnitude; mode 1: sum, ternary clamped |
| 22 | XOR | the one active input; two or more give 0 |
| 28 | battery | the stored manual value |
| 20 | direction | magnitude of input 1 minus magnitude of input 0 |
| 27 | sign split | output 0 the negative part, output 1 the positive part |
| 17 | toggle | a latch, flipped by a rising edge on any port |
| 35 | selector | output i on when the index is i; port 0 steps, port i + 1 selects |
| 15 | counter | count / required; port 0 adds, port 1 resets |
| 16 | timer | count / length; advanced each tick; port 1 resets |

Every sensor type, the randomiser (19) and a board owned by a sequencer keep the stored
signal. Types without a handler in the game give {0, 0, -1}.

INFERRED: 30 ticks per second and timer lengths in ticks; that the first pass compares
against the stored signals; that the off flags are computed once before the first pass;
the order in which a wiring cycle is broken (lowest Thing index).

### Measured (PC, 378 levels, release build)

Census: 141,572 PSwitch Things. The AND / OR mode word holds 0 or 1 only (AND 5,671 / 56,
OR 4,084 / 5). 5,686 NOT gates are inverted, 108 are not. 28,278 microchips, each with a
board; 5,420 have port 0 wired. 4,298 of 4,599 battery outputs are stored off while the
battery is on: the files hold mostly unpowered outputs, and the first pass powers them.

| Run | Target visits | `__b` calls | `__bf` calls | Outputs unlike the file at the end | Things stopped by a fault |
|---|---|---|---|---|---|
| plain, 60 ticks | 75,140 | 204 | 36 | 24,612 | 597 (was 582) |
| `--visit-triggers`, 60 ticks | 467,601 | 222 | 37 | 24,665 | 598 (was 582) |
| `--visit-triggers --pulse-switches` | 770,915 | 3,052 | 1,345 | 9,486 | 1,794 |
| plain, 300 ticks | 265,180 | 226 | 37 | 24,469 | 598 |

Unit: one call of the script entry on one Thing; 144,608 outputs in all. In the plain run
18,100 Things end off under a microchip. `--pulse-switches` turns every output on for one
tick and off again, to reach the reactions the unvisited sensors would cause.

New faults the reactions reach: `Thing.Pos` in `Trigger_Speech_Bubble` (14 Things in the
plain run, 287 pulsed) and `PWorld.Frame` in `ScoreTweak` (1; 925 pulsed). 19 more unbound
natives are called in the plain run: get and set of the global setting (121 calls in 64
levels each), four Gooey frame natives, `Audio.IsSafeToTriggerMusic`, six speech-bubble
natives, two trigger messages, `Game.GotoScoreboard`, `Game.IsVcrPaused`,
`Thing.GetMicroChipProxyFromThing`, `Thing.GetWorldPositionForSensor`.

### Not modelled

Sensors (tag, player, impact, grab and the rest), the randomiser, the game's dirty flags
(every switch runs each tick here), sequencer boards and cursor chips, the three extra
conditions of `__bf`, object kinds 0x49, 0x53 and 0x54, the mesh, trigger-parent and
gameplay-data kind tests, the activating-player record, the force flag.

### Verification

`cargo test -p lbp-runtime -p lbp-level`: 21 unit tests in `switches.rs` and two world
tests (`stage3b_switch_reactions_go_to_listed_scripts_only`, the AND -> NOT -> script
chain); fmt, clippy and the CI test set pass. Not run in the viewer window; no switch
compared against the game in RPCS3.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Part 13: world, position, player and game state (2026-10-13, D262)

Stage 4 of `docs/rscript-plan.md`. After part 12 the scripts ran until they asked the
engine about the level itself: its game mode and frame, where a Thing is, who the players
are, what the score is. The runtime had no answer and the Thing stopped (597 Things in the
plain run). Now the level world keeps that state, and 41 more natives read and write it.

### What the game does (FOUND, 01.33 executable)

- PWorld: the serializer (0xb3f3c0..0xb3fd20) names `Frame`, the sim frame,
  `FrameLevelStarted`, `GameMode`, `IsPaused`, `WaterLevel` and two arrays of 8 s32,
  `GlobalSettingsThingUIDs` and `GlobalSettingsThingPriority`. The level file stores them.
- PPos holds a local and a world matrix. `SetLocalPosition` (0xf2e80) stores the local
  matrix, sets world = parent's world x local, then gives every descendant with a PPos
  world = its parent's world x its own local. `SetWorldPosition` (0xf26bc) stores the
  world matrix and works the local one out through the parent's inverse (0xf1d58), with
  the same pass over the descendants. `GetWorldPosition` (0x755874) returns the whole
  world matrix, not a position.
- `GetMicroChipProxyFromThing` (0x754498) returns the Thing's microchip Thing when it has
  one, else the Thing. `GetWorldPositionForSensor` (0x7556a0) returns the proxy's world
  matrix unless the Thing is on a sequencer. `IsInactive` (0x7533ac) reads the Thing's
  deactivated byte. `Game.IsOnSequencer` (0xf3d94): the parent is a circuit board whose
  parent has a PSequencer.
- Players: the game state holds an array of player records (found by player number,
  0x156b18). A record has a score (f32), a multiplier counter, a multiplier timer, the
  Thing the multiplier is shown at, a visible flag, the player number and a team number.
  The score natives read and write these fields. For a number without a record a getter
  gives 0 (team: -1) and a setter does nothing. The multiplier is counter / 5 + 1.
  `SetPlayerScore` (0x3970e4) clamps the value to 0.0 .. 4,000,000.0.
- `GetYellowheadFromPlayerNumber` (0x18ec14) walks the PWorld's list of PYellowHead parts
  for the one with that player number and returns its Thing, or null.
- Global settings: `GetGlobalSettingUID(type)` reads slot `type` of the UID array.
  `SetGlobalSettingUID(type, uid, priority)` (0xce554), with cur = the slot's priority:
  nothing when priority < cur, nothing when cur is 1; else the slot takes the UID and
  priority 0, and for a UID other than 0 the given priority. A global-settings tweaker
  asks for its slot and then looks whether the slot holds its own UID.
- `GetLevelSlotID` (0x16cb18) and `GetPrimaryLevelSlotID` (0x16ca08) build a new script
  SlotID object (Type, SlotNumber) from the slot the game state holds; never null.
- `IsVcrPaused` (0x18cf18) returns the PWorld's IsPaused byte. `IsSafeToTriggerMusic`
  (0x2c6a14) is true at once in play mode. `FindLockMesh` (0x1343dc) walks a level link's
  descendants for the first whose mesh has GUID 0xeb1.

### What the runtime does

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| Part names | `Pos`, `PhysicsTweak` and the other names that start with P keep their P; `MetaData`, `MicroChip`, `WormHole`, `TagSynchroniser` are found | bug fix |
| `PWorld.GameMode`, `Frame`, `WaterLevel` | the stored values; `Frame` + 1 at the end of each level tick | FOUND where stored; INFERRED when the frame steps |
| `PPos.WorldPosition` | get: the world matrix. Set: the world matrix alone, no local matrix, no descendants | get FOUND; set INFERRED (a raw write) |
| `PTrigger.RadiusMultiplier`, `AllZLayers`, `HysteresisMultiplier` | the values the trigger test uses | INFERRED |
| Local matrices | derived at load (the file has none): inverse(parent world) x world | FOUND rule, derived data |
| `SetLocalPosition__m`, `SetWorldPosition__m`, `GetLocalPosition__`, `GetWorldPosition__` | as the game; a null Thing or one without a PPos: nothing, or the identity | FOUND; identity INFERRED |
| VM m44 | element (row, column) at column * 4 + row, as the engine keeps a matrix | FOUND in the script tables (use only) |
| Microchip proxy | the outermost chip whose board the Thing sits on | FOUND native; INFERRED which chip |
| Player records | one record, player number 0, from the start; `add_player` gives the next record a Sackboy and makes a missing record | FOUND layout; INFERRED one local player numbered 0, team 0 |
| Yellowhead | the Sackboy `add_player` made for that number; null before | FOUND rule |
| Local player | a number with a record is local | INFERRED (one machine) |
| Score and multiplier group | the record's fields with the game's rules | FOUND |
| Global settings | the two arrays from the file (8 slots), then the priority rule | FOUND |
| SlotID | one object each for the level slot and the primary slot, made ahead; Type 0, SlotNumber 0 | FOUND that it is never null; contents INFERRED |
| `IsVcrPaused` | false | INFERRED, see below |
| `IsTutorialLevel`, `IsPSNDemoBuild` | false (a field the caller can set; the full game) | INFERRED |
| Party leader | the first record's number | INFERRED |
| `FindLockMesh` | depth first under the Thing, first mesh GUID 0xeb1 | FOUND, simplified |

`IsVcrPaused`: the stored IsPaused is true in 220 of the 236 levels that have a PWorld
(MEASURED), story levels included, and those levels play. So the stored byte is an editor
state that the game clears when play starts. Returning it would pause every script that
asks. The runtime returns false in play; where the game clears the byte was not located.

Recorded only: `World.SetPieChartMultiplier`, `Thing.UpdateScale`, `Thing.HideRenderMesh`,
`Thing.SetRenderMeshEditorTexture`, `Thing.SetRenderMeshEditorColour`,
`Thing.RemoveFromAnimatingDivergentList`, `Trigger_Speech_Bubble.NotifyNpcsOfMouthEnd`
(its callee is an empty function), `NoJoinMarker.SetGameNoJoin` and `Game.GotoScoreboard`
return nothing to the script; what they do in the game is display, HUD, network or game
flow. The world counts each call (`LevelWorld::engine_calls`), keeps the render-mesh ones
on their Thing and keeps a scoreboard request with its flag. `natives::RECORDED_ONLY`
lists them with the four audio natives, and `level_scripts` counts their calls apart.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Census of the 236 levels with a PWorld: game mode 0 in 190, 1 in 14, 2 in 32; IsPaused
true in 220; both global-setting arrays have 8 entries and all are 0.

| Run | Things stopped by a fault | Fault kinds | Reaction calls | Unbound natives called |
|---|---|---|---|---|
| plain: before stage 4 | 597 | 11 | 240 | 85 |
| plain: after the world and position half | 117 | 3 | 240 | 98 |
| plain: now | 4 | 2 | 262 | 83 |
| `--visit-triggers`: before | 598 | 11 | 259 | 99 |
| `--visit-triggers`: after the first half | 117 | 3 | 260 | 112 |
| `--visit-triggers`: now | 4 | 2 | 283 | 99 |
| with `--pulse-switches`: before | 1,794 | 11 | 4,397 | 102 |
| with `--pulse-switches`: after the first half | 117 | 3 | 5,611 | 116 |
| with `--pulse-switches`: now | 4 | 2 | 5,704 | 110 |

Units: scripted Thing; distinct fault text; one `SwitchPerformReaction` call (`__b` and
`__bf` together); distinct native without a binding.

- Native calls in the pulsed run: 3,650,429, of which 2,697,610 reach a binding (17,206 of
  those are recorded only) and 952,819 are defaulted. Plain run: 534,799, 445,907 (3,379)
  and 88,892.
- Levels with a fault: 221 before, 105 after the first half, 2 now. The two faults left:
  `Thing.GetCOM__` (no centre of mass, 3 Things in 1 level) and a null Thing passed to
  `SetShapeEditorColour` by the Pod (1 Thing).
- The level links now pass `Init`: 113 Things in 103 levels. With the trigger visit 189
  more Enter and 189 more Leave calls run (11,872 and 7,574), on Things that no longer
  stop.
- Global-setting slots held at the end, summed over the levels: 77 plain, 121 with the
  trigger visit, 188 pulsed (the files store none).
- `GotoScoreboard` requests: 3 plain (3 levels), 4 with the visit, 39 pulsed (23 levels).
- 1,107 of 6,951 scripted Things on a board sit on a chip inside another chip, and that
  inner chip has no PPos; the outermost chip always has one. That is why the proxy is the
  outermost chip.
- New natives the scripts now reach, largest first by levels: `Pad.IsJustPressed`
  (131,404 calls in 220 levels, pulsed run), `OffScreenIndicator.
  GetPlayerIndicatorActivityType` (65,702 / 220), `Game.GetGameFrame` (763 / 121),
  `ModalOverlay.IsPartyLeaderOverlayActive` (33,120 / 103), the level-link prompt,
  `Game.SetGlobalSettingsTo` and its audio, gravity and water forms.

### Not modelled

- Where `Frame` is stepped in the game's tick (here: at the end, so tick n reads the
  stored frame + n - 1).
- The Thing flag that makes world = local under a parent; the refresh of bodies and
  shapes after a transform write.
- The sequencer branch of the sensor position.
- The score history `SetPlayerScore` appends to, and every HUD or audio call the score
  natives make. Edit mode of `SetPlayerScore` and `IsSafeToTriggerMusic` (an error).
- The level's own slot (it is in the game's slot list, not in the level file): a level
  link that compares its target with the current slot compares with 0.
- Which levels are tutorial levels.
- `FindLockMesh` goes below every Thing; the game does not go below a Thing with a
  certain part (which part is not established).
- `Thing.IsDead` and `IsInLimbo` stay unbound: true for a Thing without a PCreature
  (FOUND), but for a creature the game reads a state the runtime does not have.
- The Sackbot and enemy group, Gooey, HUD, messaging, the slot database.

### Verification

`cargo test -p lbp-script-vm -p lbp-runtime`: 11 + 13 VM tests, 25 unit and 27 world
tests. The stage 4 world tests use synthetic levels: PWorld members and the frame, PPos
and PTrigger members, local matrices and the transform natives, a script reading a
position out of a matrix, inactive / proxy / sensor position / sequencer, player records
and queries, the score group, the global-setting priority rule, SlotID objects with the
game-state queries and the lock mesh, the recorded-only calls. fmt, the CI clippy line,
the CI test set and `cargo check` of both viewers pass. Not run in the viewer window;
nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).

## Part 14: the natives the stage 4 runs reached (2026-10-13, D263)

Between stages 4 and 5 of `docs/rscript-plan.md`. Part 13 let the scripts run on, and they
then called 110 natives that had no binding. This part binds the ones reached in the most
levels: 48 natives, 164 -> 212 in the table. 16 of the 48 are recorded only (29 in all).
It also makes a script cast test the class of the Thing.

### What the game does (FOUND, 01.33 executable)

- Pad: a Thing's PYellowHead gives a player number, the player's record a pad object
  (0x156b18). The pad holds two mode masks, the button word of this frame and the one of
  the frame before; the game copies the first over the second once a frame (0x3866e8).
  `IsPressed(thing, mode, buttons)` (0x388e2c): the mode is in the first mask and every bit
  of `buttons` is down. `IsJustPressed` (0x388358): the mode is in both masks, all bits are
  down now and were not all down the frame before. `IsJustReleased` (0x389400): the
  reverse edge. A null Thing, no PYellowHead, no record or no pad: false.
- `LAMS.IsInLams(key)` (0x159c1c): a binary search of the sorted rows of every loaded
  translation resource; the same search hands `Translate` its string.
- Global settings: for each one-float setting (water level, gravity, wave magnitude,
  murkiness, water bits) the PWorld keeps a start value, a target and a blend factor t,
  and the value in force is start + (target - start) * t. `Set...To(value, blend)`: with
  blend, start = the value in force, target = value, t = 0; without, start = target =
  value and t = 1.0 (0xce608 and its neighbours). The lighting group (lighting, darkness,
  fog, fog tint, colour correction) does the same with five values under one t (0xd9dac).
  `SetGlobalSettingsBlendFactor(type, t)` (0xd623c) writes t; the scripts step it
  themselves from `GetGameFrame`. The water getters return the blended value;
  `GetGlobalSettingsWaterLevel` (0x5ec6f8) returns it only above 0.0001, else 0.0. The
  water tint is one stored value.
- `GetGameFrame` (0x19f63c) is the PWorld's frame. `GetNumYellowHeadAvatars` (0x5ed9f8)
  is the length of the PWorld's PYellowHead list. `GetGameScoreDataType` (0x16aa4c) is 6
  in game mode 1, else a player count - 1. `IsSlotIDValidForScoring` (0x229bb8) reads
  Type and SlotNumber: false for (0, 0), else true for type 0, 1 or 8.
  `Messaging_ShouldFilterMessageStrings` (0x736260) is true unless the level slot's type
  is 2.
- `Slot.GetSlotGroup(slot)` (0x640500 -> 0x7477d8 -> 0x1a6cd8) looks the slot up in the
  slot database and builds a new SlotID from the group pair of the slot's record, or from
  (0, 0) when the slot is not found. It uses the builder of `GetLevelSlotID`, so it never
  returns null. The lookup finds nothing for the slot (0, 0).
- `Poppet.GetTextEntryType` (0x56a838) returns the constant 2 and
  `Game.IsTarsierDLCEnabled` (0x5e68d8) the constant 1. `Game.TrackSpeechKey` (0x5e67b0)
  calls an empty function.
- The messaging natives hand a request to one of five message managers: a speech bubble
  with or without a prompt (message types 1 and 2), subtitles (3), a level-link prompt
  (0x2b), a wormhole prompt (0x59), and a game event that closes a message.

### What the runtime does

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| `Pad.IsJustPressed`, `IsPressed`, `IsJustReleased` | the game's rule on `PlayerRecord::pad`; no pad until `LevelWorld::set_pad_buttons` writes one, so false | FOUND rule; the masks (both 1) and where in the tick the old buttons are copied INFERRED |
| `LAMS.IsInLams`, `Translate` | `LevelWorld::translations`, searched in order; no table: false, and no text | FOUND rule; which tables are loaded INFERRED |
| Global settings | `WorldState::water`, `gravity`, `water_wave_magnitude`, `water_murkiness`, `water_bits` (`Blend`), the tint, the lighting group, the 8 blend factors, the drain-sound flag | FOUND rules; start values INFERRED (at rest on the stored value) |
| `Game.GetGameFrame`, `GetNumYellowHeadAvatars` | `PWorld.Frame`; the Sackboys `add_player` made | FOUND |
| `Score_Board.GetGameScoreDataType` | 6 in game mode 1, else player records - 1 | FOUND; that the count is the players INFERRED |
| `Score_Board.IsSlotIDValidForScoring` | the rule on the slot the world made the SlotID from | FOUND, without a second test that can still say no |
| `Slot.GetSlotGroup` | for the empty slot (0, 0) a SlotID (0, 0) made ahead; any other slot is an error (no slot database) | FOUND |
| Script cast (`CASTsp`) | a Thing passes when its script class or a superclass has the name; it passed always before | bug fix |
| `Messaging_ShouldFilterMessageStrings` | true unless the slot type is 2 | FOUND; the type 2 answer (false) INFERRED |
| `Resource.LoadAnimatedTextureByFilename` | a handle for the GUID, as the texture loader | FOUND shape |
| `Poppet.GetTextEntryType`, `Game.IsTarsierDLCEnabled` | 2 and true | FOUND constants |
| `OffScreenIndicator.GetPlayerIndicatorActivityType` | 0, no indicator | INFERRED for one player on screen |
| `Game.GetNumKeyNames`, `GetKeyIndex` | 0 and -1: no Sackbot names | INFERRED (the reader keeps no names) |
| `ModalOverlay.IsPartyLeaderOverlayActive` | false: no overlay is ever open | INFERRED |
| `Game.GetNumKnownItemsCollectedAtStart` | 0: a fresh profile | INFERRED |

The level reader now keeps the stored wave magnitude, gravity, tint, murkiness, water bits
and the drain-sound flag (field order from community layouts, as the water level).
`level_scripts` hands every level the English base and patch tables through the loader
the story browser uses; the viewer does not load them yet.

Recorded only (16): the three trigger messages, the level-link and wormhole prompts,
`Messaging_NewGameEvent`, `HUD.RegisterOverlayCallback`, `HUD.PreloadGlyphs`,
`Game.TrackSpeechKey`, `Trigger_Speech_Bubble.NotifyNpcsOfMouthStart`,
`Thing.AddToAnimatingDivergentList`, `Thing.SetAnimLoop`, three Gooey natives
(`SetLastNavSelectionMode`, `PerFrameUpdate`, `SetStylesheetScalingFactor`) and
`Game.SetGlobalSettingsAudioTo`. Ten of them are kept with their arguments and the frame
in `LevelWorld::requests` (at most 4,096) for stage 5; `SetAnimLoop` is kept on its Thing.

Pad buttons: the button word has FOUND names for four bits only (0x100000 up, 0x200000
right, 0x400000 down, 0x800000 left). The other names are INFERRED and are not in the
code. The level scripts ask for mask 4 with mode 1.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Translation tables: 2 read, 29,477 keys (all in the base table; the patch table is empty).

| Run | Unbound natives called | Unbound calls | Recorded-only calls | Things stopped by a fault | Levels with a fault |
|---|---|---|---|---|---|
| plain: before | 83 | 88,892 | 3,379 | 4 | 2 |
| plain: now | 52 | 67,045 | 13,526 | 4 | 2 |
| `--visit-triggers`: before | 99 | 941,081 | 13,855 | 4 | 2 |
| `--visit-triggers`: now | 85 | 758,850 | 76,257 | 4 | 2 |
| with `--pulse-switches`: before | 110 | 952,819 | 17,206 | 4 | 2 |
| with `--pulse-switches`: now | 88 | 766,940 | 83,820 | 4 | 2 |

Units: distinct native without a binding; native call; scripted Thing; level.

- Native calls in the pulsed run: 4,064,366, of which 3,297,426 reach a binding (83,820 of
  those recorded only) and 766,940 are defaulted. Before: 3,650,429, 2,697,610 and 952,819.
- 44 natives left the unbound list of the pulsed run and 22 new ones joined it.
- No new fault. A first run of this part stopped more Things (26 plain, 35 with the visit,
  58 pulsed), for two reasons that are settled now:
  - `IsTarsierDLCEnabled` is true, so `TriggerLevelLink.IsSourceSlotLocked` goes on to
    `Slot.GetSlotGroup`, which was unbound and returned null (22 Things plain, 46
    pulsed). It is bound: the game never returns null there.
  - `Race_End.Enter` casts the world's current challenge to `Race_Start` and reads its
    colour index. The runtime let every cast pass, so a challenge of another class got
    through and the member read failed (8 Things with the trigger visit). A cast now
    tests the Thing's class chain and gives null for another class, as the script
    expects.
- The scoreboards go further with the trigger visit: 19 natives are new in 101 levels
  each, among them `Score_Board.PostScore`, `SetScoreLocked`, `CreateMagicWall`,
  `Cam.SetOverrideCamera` and `Game.StartAsyncSection`.
- Requests kept in the pulsed run, summed over the levels: 1,771 speech bubbles (95 with
  a prompt, 959 without, 717 subtitles), 2,058 game events, 216 level-link and 103
  wormhole prompts, 137 overlay callbacks, 297 audio settings, 1,806 speech keys.
- Faults left as before: `Thing.GetCOM__` (3 Things) and the Pod's null Thing (1).
- Most called without a binding (pulsed run, calls / levels): `Native.IsEnemyAI`,
  `Native.SetWalkInput`, `Thing.IsDead`, `Thing.IsInLimbo` 114,228 / 26 each,
  `Thing.GetBodyRoot` 65,490 / 27. In most levels: `Audio.StopSound` 5,236 / 143,
  `Audio.RemoveMixOverlay` 2,906 / 142, `Slot.GetLevelIcon` 113 / 103,
  `Game.SetLevelCompleted` 103 / 103.

### Not modelled

- Pad input: nothing feeds the pads yet (the viewer has to call `set_pad_buttons` each
  tick). The names of the button bits other than the d-pad, the meaning of the two mode
  masks, and the button hint `IsPressed` can show.
- Which translation tables the game has loaded while a level plays; the viewer loads none.
- Global settings: the stored start, target and blend factor of each setting (the start
  is taken as at rest); the 9 water tint colours and what the blend flag does to the
  tint; the lighting block, the physics gravity and the renderer values the game rebuilds
  from a change; the editor flag in the water level getter. Nothing draws by them yet.
- `GetPlayerIndicatorActivityType` for a player whose Sackboy is off screen or not
  spawned; Sackbot names for the key natives; the second test of
  `IsSlotIDValidForScoring`; a SlotID a script built itself (an error); the group of
  any slot other than the empty one (an error: it needs the slot database).
- A cast to a class whose script is not loaded gives null; before
  `add_class_defaults` has run every cast still passes.
- Left unbound on purpose: `Slot.GetLevelIcon` (needs the slot database and a default
  icon), `Game.GetNumKnownItemsInLevelAtStart` (a count of the level's items, 0 would be
  a wrong fact) and the 11 audio getters (the reader drops the stored audio settings).

### Verification

`cargo test -p lbp-script-vm -p lbp-runtime -p lbp-level`: 12 + 13 VM tests, 25 unit and
33 world tests. New: the pad rules on a `PadState` and through the world with
`set_pad_buttons`, LAMS keys against synthetic tables, the blends of every global-settings
type, the game-state queries with the scoring slot test, the recorded calls with their
kept arguments, the slot group, the script cast; the level reader test covers the new
PWorld fields. fmt, the CI clippy
line, the CI test set, the Python tests and `cargo check` of both viewers pass. Not run
in the viewer window; nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).

## Part 15: tag sensors and player sensors (2026-10-13, D264)

Between stages 4 and 5 of `docs/rscript-plan.md`. Part 12 ran the switch network, but every
sensor kept the signal the level file stores. Now the two most common sensors are computed
each tick: the tag sensor (switch type 4) and the player sensor (type 3), 22,580 of the
141,572 switches. The other sensors, the randomiser and the sequencers still keep the
stored signal.

### What the game does (FOUND, 01.33 executable)

- Both sensors run every tick at their place in the pass order (tag sensor 0x118d5c, player
  sensor 0x119910). Every output of the switch gets the same signal. A deactivated Thing
  gives off. The inversion of an inverted switch is applied to the finished signal, so an
  inverted sensor that finds nothing is on.
- The PWorld keeps a tag map: per (colour index, name) the tags with their x and y, sorted
  by x. The first thing the switch pass does (0xd5c1c) is to write every tag's position
  again and sort. So all sensors of a tick see the tags where they were when the pass
  started. A sensor finds its tags by the same colour index and the same name (0xceac0).
- A sensor or a tag on a microchip is placed by the chip's Thing, else by its own Thing.
  Distances are between world positions, x and y only.
- A tag's signal (0x12f6f4): off while its Thing is deactivated; else the signal wired to
  the Thing's input port 0; with nothing wired {1.0, 1, -1}.
- Range: a candidate at distance d counts when MinRadius^2 <= d^2 <= Radius^2. A negative
  Radius means no range test. Closeness: 1.0 with a negative Radius; (1 - d / Radius) / 0.75
  with MinRadius 0; else 1 - (d - MinRadius) / (Radius - MinRadius).
- Ternary of an activation a (0x11873c), with s = a * 3.75: off at s <= 0.25, on at
  s >= 0.75, in between what output 0 holds. With MinRadius 0 that is on within 0.85 of the
  Radius, off beyond 0.95, and a ring between the two that holds.
- Tag sensor. KeySensorMode 1 hands on the tag's own signal; the other modes measure
  closeness. Per tag in x order: in mode 1 a tag with activation 0 is skipped and one whose
  ternary is not 0 is counted; in the other modes a tag whose ternary is 0 is skipped. Then
  the team filter, the "own chip" rule (a tag on the sensor's own chip is skipped unless
  DetectUnspawnedPlayers is set) and the range. The tags that pass are kept by falling
  strength, cut to BulletsRequired entries; fewer than that is off. The output is the
  weakest kept one: its closeness clamped to 0..1 with the ternary rule above, or in mode 1
  its signal as it is.
- Team filter (0xf9a64), by the game mode: outside versus (1) and team (5) levels the
  filters 0, 1, 3 and 4 pass and 2 fails. In a versus level 1 passes the player who owns
  the sensor and 2 any other player; in a team level 1 to 4 compare teams.
- Player sensor. The candidates are the players' Sackboys. With PlayerMode 0 only the
  leader's counts. Needed in range: every candidate with RequireAll, else BulletsRequired.
  The activation is the closeness of the farthest one needed; the output's player is the
  nearest one.
- A sensor whose AngleRange is below 180 also tests a cone: the cosine of the angle
  between a direction of the sensor and the candidate against cos(AngleRange). A sensor
  whose IncludeTouching is not 0 also tests layers: the z spans of the sensor's body and
  the tag's body must overlap; for a player his layer must be one the sensor covers.

### What the runtime does

`crates/lbp-runtime/src/sensors.rs`. The level reader keeps the PSwitch fields Radius,
MinRadius, the key colour index and name, RequireAll, AngleRange, KeySensorMode,
DetectUnspawnedPlayers and PlayerMode, and the PSwitchKey part's colour index and name.

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| Tag map | `LevelWorld::refresh_tags` at the start of each pass; `LevelWorld::tags` | FOUND |
| Tag key | `TagKey`: colour index and an index into the level's names | FOUND that both must match; that equal name handles are equal strings INFERRED |
| Position of a sensor or a tag | the outermost chip's Thing, else its own (`sensor_proxy`) | FOUND rule; which chip INFERRED (part 13) |
| Tag signal | `LevelWorld::tag_signal` | FOUND |
| Range, closeness, ternary with hold | as the game | FOUND |
| Tag sensor | as the game, with the team filter and the own-chip rule | FOUND |
| Player sensor | over `LevelWorld::players`; no player, no candidate | FOUND rule |
| Leader for PlayerMode 0 | the first player record's number | INFERRED (as `GetPartyLeadersPlayerNumber`) |
| Player sensor points | the positions of the sensor's proxy Thing and the Sackboy's Thing | INFERRED: the game takes the centre of each one's body |
| Cone, AngleRange below 180 | not tested, every direction passes | INFERRED choice: the direction is not established |
| Layer test, IncludeTouching 1 | not tested, every layer passes | INFERRED choice: needs the bodies' z spans |
| Owner of the sensor's body | nobody (-1): versus filter 1 fails, filter 2 passes any tag with a player | INFERRED |

In the viewer the play-mode Sackboy is already the level world's player (`--level-scripts`
with `--play-sackboy`), so the player sensors follow him; nothing was added there.

`level_scripts`: the switch census has a `sensors` block. `--visit-triggers` now also
stands the player at every player sensor in turn (two ticks in its range, one far away).
`sensors_unlike_stored` counts the sensors whose output, while their Thing was not
deactivated, differed from the stored signal on some tick.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Census. Unit: Thing with a PSwitch of type 3 or 4, or with a PSwitchKey.

| | Player sensors | Tag sensors |
|---|---|---|
| All | 5,627 | 16,953 |
| Inverted | 227 | 1,677 |
| AngleRange below 180 (cone not tested) | 1,811 | 2,975 |
| IncludeTouching 1 (layer test not applied) | 212 | 489 |
| Either of the two | 1,932 | 3,353 |
| Radius negative / MinRadius not 0 | 47 / 89 | 467 / 130 |
| BulletsRequired above 1 | 360 | 418 |
| DetectUnspawnedPlayers | 192 | 23 |

Player sensors: RequireAll 408; PlayerMode 0 / 1 / 2: 4,280 / 759 / 588. Tag sensors:
KeySensorMode 0 / 1: 15,984 / 969; TeamFilter 0 / 1: 16,945 / 8, none of the 8 in a versus
or team level; 961 look for a tag their level does not have. Tags: 19,629, of which 5,316
have input port 0 wired. Every sensor and tag has a position. IncludeTouching holds 0 or 1
only on these two types.

| Run | Sensors unlike the stored signal: player / tag | Outputs unlike the file at the end | Reaction calls | Unbound natives called | Things stopped by a fault |
|---|---|---|---|---|---|
| plain: before | 0 / 0 | 24,612 | 262 | 52 | 4 |
| plain: now | 184 / 5,446 | 33,541 | 323 | 52 | 4 |
| `--visit-triggers`: before | 0 / 0 | 24,665 | 283 | 85 | 4 |
| `--visit-triggers`: now | 5,204 / 6,272 | 40,244 | 8,970 | 88 | 4 |
| with `--pulse-switches`: before | 0 / 0 | 9,486 | 5,704 | 88 | 4 |
| with `--pulse-switches`: now | 5,204 / 6,272 | 9,486 | 14,346 | 88 | 4 |

Units: sensor (output 0 differed on some tick while its Thing was not deactivated; the
pulse ticks are left out); switch output; one `SwitchPerformReaction` call (`__b` and
`__bf` together); distinct native without a binding; scripted Thing.

- By ternary alone the sensors that differ are 184 and 4,364 in the plain run, 5,203 and
  5,152 with the visit. The plain run has no player: a player sensor there is off, or on
  when it is inverted.
- The visit stood at all 5,627 player sensors. 360 of them ask for two to four players.
- Reaction calls with the visit: 283 -> 8,970, of which 5,989 go to script GUID 65238 and
  1,019 to 18256. The first tick of a level makes 178 calls (129 before).
- No new fault: the two fault kinds and the 4 stopped Things are the ones of part 14.
- With the visit three more natives are reached without a binding:
  `Game.PlayTutorialMonitorVideo`, `Profile.SetLBP2GameProgressionFlag`,
  `Game.TryToAwardPin`. Unbound calls 758,850 -> 1,002,475 (67,045 -> 73,329 plain); the
  sensor visit adds 16,881 level ticks to the run.
- Trigger events run with the visit: 14,016 Enter, 1,376 Inside, 9,048 Leave (11,872,
  1,295 and 7,574 before); the visit to the sensors is the new part of the run.
- Native calls in the pulsed run: 5,188,665, of which 4,178,077 reach a binding (102,810
  of those recorded only) and 1,010,588 are defaulted.
- The reader reads the same 378 levels with the same Thing counts as before.

### Not modelled

- The cone (1,811 player and 2,975 tag sensors): the test is known, the direction is
  not. It is built from the proxy's matrix and a rotation the game accumulates at run
  time. These sensors react all round here.
- The layer test (212 and 489 sensors): the formulas are known, the z span of a Thing's
  body is not kept. These sensors react on every layer here.
- A sensor or tag on a player's cursor. A level without cursor Things never takes that
  path.
- The player who owns a body: the player a tag without one would hand on, and the versus
  filters 1 and 2 (no sensor in the 378 levels uses them in a versus or team level).
- A tag the game skips by a Thing flag in one game state (the meaning of both is not
  settled).
- Player sensor: a player out of play, a player sitting in something (the game then
  measures to that Thing), and DetectUnspawnedPlayers (192 sensors), which counts players
  waiting at a checkpoint. The world has no checkpoint list and no such player state.
- When the game uses its snapshot copy of the tag map instead of the live one.
- The other sensor types, the randomiser, the sequencers.

### Verification

`cargo test -p lbp-runtime -p lbp-level`: 9 new unit tests in `sensors.rs` (range edges
and the hold ring, MinRadius, the N-th strongest tag, the tag's signal unwired, wired and
deactivated, KeySensorMode 1, inversion with the own-chip rule and the team filter, the
tag map, the player sensor's range, hold and inversion, player counts with RequireAll and
PlayerMode), one world test from level parts (`d264_sensors_read_the_level_parts`) and one
reader test (`switch_and_switch_key_keep_the_sensor_fields`). fmt, the CI clippy line, the
CI test set, the Python tests and `cargo check` of both viewers pass. Not run in the viewer
window; no sensor compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).

## Part 16: the message list (2026-10-13, D265)

Stage 5 of `docs/rscript-plan.md`, first PR. Part 14 kept every speech-bubble, prompt and
subtitle request with its arguments and did nothing with it. Now the level world runs the
game's message list by those calls: what is open, what a new request does to it, what
closes it. Nothing is drawn yet; the viewer reads the list in the next PR.

### What the game does (FOUND, 01.33 executable)

- The trigger messages and the two prompts go to message manager 4 of 5. A manager holds
  at most 8 messages (add 0x3e8dc0). A message keeps its type, up to five strings, its
  Thing, two counters that start at 0 and some flag bytes.
- Types, by the constant each native's core passes: bubble with a prompt 1 (0x7382e0),
  without 2 (0x7380f8), subtitles 3 (0x737320), level-link prompt 0x2b, wormhole prompt
  0x59, caption 0x5d (0x737f64) and 0x5e (0x737d7c), think 0x5f (0x737be8) and 0x60
  (0x737a00).
- A core first asks the manager for a message of its type about its Thing that is not
  closing (0x3e4124). When there is one, the call does nothing.
- With 8 messages held, the eighth is cleared to make room, unless its type's priority
  number is below the new type's. All nine types have priority 10 (type records filled
  by 0x3e3248).
- Bit 0 of a type's flags means one at a time: the new message is written over the first
  message of that type and keeps its id. Types 2, 3, 0x5e and 0x60 have the bit; the
  bubbles with a prompt and the two prompts do not, so those queue.
- Events (0x3e8aa0): event 3 reaches the messages about the Thing passed with it; events
  1, 2, 5, 6, 7 and 8 reach only the first message of the list, and only after one of its
  counters has passed a threshold; any other event reaches every message. A message then
  runs the action its type lists for the event (up to four pairs per type). All nine
  types list action 0x10 for event 3; the subtitles also for event 9. Types 1, 0x5d,
  0x5f, 0x2b and 0x59 list action 0x17 for event 2, types 2, 0x5e and 0x60 action 0x37.
- Action 0x10 (in the switch 0x3e528c): for a bubble or subtitle type it calls a function
  with a part of the message's Thing, then sets the message's closing byte. A closing
  message no longer blocks a new one of its type about the same Thing.
- `Messaging_NewGameEvent(manager, event, thing, flag)` (0x7353ec): an index above 4
  means manager 4. A speech bubble closes its own message with (4, 3, self, false).
- None of the nine types has a timed action. Nothing closes them by time.
- The request of a bubble with a prompt (0x7382e0): text, the actor name, a prompt
  string looked up by a LAMS key unless prevent close is set, the Thing, both player
  fields -1, "triggered once" in the byte the list sort reads, the bubble position, and
  prevent close in the byte actions 0x17 and 0x37 test before they do anything.
- Actor name (0x737910): an empty name is no name. When the level slot's type is 0 or 8
  the name is hashed as a translation tag and looked up in the loaded tables; a tag in no
  table gives a default string. For another slot type the name is used as passed.
- After each change the list is sorted and every message gets a state: one value with
  the "triggered once" byte set, one for the first other message when it heads the list,
  one for the rest (0x3e8928).

### What the runtime does

`crates/lbp-runtime/src/messages.rs`; `LevelWorld::messages` is a `Messages`.

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| Types | `MessageKind` (Bubble, BubbleNoPrompt, Subtitles, LevelLinkPrompt, WormHolePrompt) and `BubbleStyle` (Speech, Caption, Think); `message_type` gives the game's number | FOUND |
| Duplicate | a request of a type about a Thing that has an open message of the type is dropped | FOUND |
| One at a time | a bubble without a prompt (per style) or a subtitle is written over the open one of its type and keeps its id | FOUND |
| Full list | at most 8 (`MAX_MESSAGES`); a ninth clears the eighth first | FOUND rule; which message is the eighth INFERRED (below) |
| List order | the order the messages were added | INFERRED: the game sorts with a comparison that was not read |
| Close | event 3 for manager 4 removes the messages about the Thing; event 9 removes the subtitles | FOUND that action 0x10 marks them closing; INFERRED that they leave at once |
| Other events | counted, nothing happens (`events_without_effect`); events for managers 0 to 3 are counted apart | FOUND that no type here lists another event except 2; event 2 not modelled |
| Time limit | none | FOUND |
| Age | `Message::age(frame)`: `PWorld.Frame` now minus the frame at opening; a message written over starts again | INFERRED (the game counts in the message; the restart is FOUND only for type 1's request) |
| Text | the first argument as passed: the script has already translated it | FOUND (script use) |
| Actor | `LevelWorld::actor_name`: the game's rule; `Message::actor_tag` keeps the argument | FOUND; INFERRED that the default string is empty, and the slot type 0 (part 13) |
| Anchor | `LevelWorld::message_anchor`: the Thing's microchip when it sits on one, else the Thing, when that has a PPos; `message_position` gives its place now | INFERRED: where the HUD draws a bubble was not read; the bubble's script measures its off-screen distance at this Thing |
| Prompt | `Message::has_close_prompt`: a bubble with a prompt unless prevent close; `CLOSE_PROMPT_KEY` is the string's LAMS key | FOUND for type 1 |
| Player | none: a trigger message has no player | FOUND (both fields -1) |
| Bubble position | kept as passed (0 to 8) | meaning not established |

Read API for the viewer: `LevelWorld::messages.open` (the open `Message`s in list order),
`Messages::of_thing`, the `Message` fields `id`, `kind`, `style`, `text`, `actor`,
`actor_tag`, `thing`, `anchor`, `bubble_pos`, `triggered_once`, `prevent_close`,
`has_voice`, `frame`, the methods `Message::age`, `message_type` and `has_close_prompt`,
and `LevelWorld::message_position`. `Messages` also counts requests, messages opened,
written over and dropped per type (`by_type`, `totals`), messages closed and pushed out,
and keeps a bounded log of what entered the list when `log_limit` is set (reports only).

Natives: the four caption and think natives are bound (212 -> 216 in the table). The ten
`GameMessagingManager` natives that open or close a message are listed in
`natives::MESSAGING` and are no longer in `RECORDED_ONLY` (29 -> 23): the world now keeps
state by them. They still reach the world through `World::engine_call`, and
`LevelWorld::requests` still keeps each call with its arguments (14 kept names).

`level_scripts`: each played level and `play_all` have a `messages` block.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Units: request (one call of a messaging native); message (one entry of the list).

| Run | Requests | Opened | Written over | Dropped as duplicate | Closed | Open at the end | Levels with a message |
|---|---|---|---|---|---|---|---|
| plain | 92 | 57 | 35 | 0 | 0 | 57 | 47 |
| `--visit-triggers` | 1,690 | 1,036 | 621 | 33 | 1,000 | 36 | 192 |
| with `--pulse-switches` | 2,381 | 1,187 | 1,161 | 33 | 1,152 | 34 | 216 |

By type, requests / opened / written over / duplicate:

| Type | plain | visit | pulsed |
|---|---|---|---|
| Bubble with a prompt | 8 / 8 / 0 / 0 | 156 / 156 / 0 / 0 | 214 / 214 / 0 / 0 |
| Bubble without a prompt | 57 / 23 / 34 / 0 | 457 / 221 / 227 / 9 | 1,058 / 306 / 743 / 9 |
| Caption without a prompt | 4 / 3 / 1 / 0 | 30 / 29 / 1 / 0 | 59 / 34 / 25 / 0 |
| Subtitles | 0 | 719 / 326 / 393 / 0 | 719 / 326 / 393 / 0 |
| Level-link prompt | 23 / 23 / 0 / 0 | 215 / 191 / 0 / 24 | 218 / 194 / 0 / 24 |
| Wormhole prompt | 0 | 113 / 113 / 0 / 0 | 113 / 113 / 0 / 0 |

- The caption with a prompt and the two think forms are not called in any run.
- Text of the bubbles and subtitles that entered the list (opened or written over): a
  string of a loaded translation table 69 / 1,289 / 1,974; another string 0 / 1 / 1;
  empty 0 / 63 / 66. With the two English tables loaded no text is left untranslated.
- Actor names: 16 / 69 / 218 messages came with a name, and every one is a tag of the
  loaded tables. Every message that entered the list has an anchor with a position.
- Close events for manager 4: 0 / 1,740 / 2,318, all (4, 3). 740 and 1,166 of them
  closed nothing: the Thing's message had been written over by another Thing's, or was
  never opened. No event for another manager.
- The list was full once, in the pulsed run (one message cleared from the eighth place).
- Things stopped by a fault: 4 -> 4 in all three runs; fault kinds 2, levels with a
  fault 2, as before. Native calls, trigger events and reaction calls are the same as in
  part 15.
- Unbound natives called 52 -> 51, 88 -> 87, 88 -> 87 (the caption native); unbound
  calls 73,329 -> 73,325, 1,002,475 -> 1,002,445, 1,010,588 -> 1,010,529. Recorded-only
  calls 13,648 -> 13,560, 95,656 -> 92,256, 102,810 -> 98,170: the messaging natives
  left that list.

### Not modelled

- Event 2, the player's button: action 0x17 on a bubble with a prompt calls a function
  with a part of the Thing and sets the "triggered once" byte, unless prevent close is
  set; action 0x37 on a bubble without a prompt was not read. Whether the bubble's
  script is told was not read. So a prompt bubble here stays until its script closes it.
- How long a closing message stays in the list, and the function action 0x10 calls on
  the Thing.
- The sort order and the three states, so which of several queued bubbles is on screen.
- Messages about another Thing that the game closes when a one-at-a-time message is
  added (0x3e4af0, by a Thing flag that was not identified).
- What a subtitle's text goes through before it is kept (0x736f1c), and the flag set
  from its last argument.
- The text of the level-link and wormhole prompts (the HUD script builds it), bubble
  placement and bubble positions 1 to 8, art, fonts.
- The bubble script's own off-screen close: the three screen natives are still unbound.
- The other four managers and every message type but these nine.

### Verification

`cargo test -p lbp-script-vm -p lbp-runtime`: 12 + 13 VM tests, 40 unit and 35 world
tests. New: 6 unit tests in `messages.rs` (type numbers, duplicate per kind, queue and
written over, the ninth message, close events per manager and Thing, the log) and the
world test `d265_the_messaging_natives_open_and_close_the_worlds_messages` (every
messaging native on a synthetic level with made-up text: fields, anchor on a chip, actor
name through a synthetic table and in another slot type, caption and think types,
subtitles, prompts, close). fmt, the CI clippy line, the CI test set, the Python tests
and `cargo check` of both viewers pass. Not run in the viewer window; nothing compared
against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).

## Part 17: the viewer draws the messages (2026-10-13, D266)

Stage 5 of `docs/rscript-plan.md`, second PR. Part 16 gave the level world the game's
message list. The Bevy viewer now draws it while a level plays under `--level-scripts`: a
box per open speech bubble at its Thing, and the subtitle at the bottom of the screen.
Nothing new was read in the executable; the look and the placement are INFERRED.

### What the viewer does

- Tables. `level_play.rs` loads the English base and patch translation tables for its
  level world, patch first, with the loader the `level_scripts` example uses since part 14
  (`lbp_preview::story::load_strings`). `LAMS.IsInLams` and `LAMS.Translate` now answer in
  the viewer as in the measuring runs, so a bubble's text is the translated string. A
  table that cannot be read is left out: the viewer still runs, a script then passes its
  own text for a missing key (the "Missing key" string of the bubble script, part 16),
  and a bubble has no close prompt line. The header line prints how many tables loaded.
- What is drawn. `LevelWorld::message_overlay` (`lbp-runtime/src/message_view.rs`) picks
  from `messages.open`, in list order:

| Message | Drawn | Label |
|---|---|---|
| Bubble with or without a prompt, any style, with text and an anchor that has a position | a box above the anchor: the speaker's name when `Message::actor` has one, the text, and the close prompt's string when `has_close_prompt` | INFERRED: every open bubble is drawn (the game's sort gives queued bubbles different states, not read) |
| Caption and think styles | as a speech bubble | INFERRED |
| Bubble whose text is empty | not drawn, counted | INFERRED |
| Bubble whose anchor has no position | not drawn, counted | no place to draw it |
| Subtitles | one box at the bottom centre; of several with text, the last of the list | INFERRED placement; the list holds one at a time (part 16) |
| Level-link and wormhole prompt | not drawn, counted | the HUD script builds their text (not read) |

- Close prompt. `LevelWorld::close_prompt` looks `CLOSE_PROMPT_KEY` up in the loaded
  tables. It is shown as a small line in the bubble. No key is wired to it: the player's
  close button (event 2) is still not modelled, so a prompt bubble stays until its script
  closes it.
- Text. `display_text` takes the game's markup out of a string, because the viewer has
  neither the game's fonts nor its icons. MEASURED on the two English tables (29,477
  strings): the markup is tags in angle brackets, and there are seven of them.

| Tag | Places | In the viewer |
|---|---|---|
| emphasis (`em`, `i`) | 1,821 and 17 | left out, the text between kept |
| line break (`br`) | 280 | a new line |
| pad button or stick icon (`icon NAME`) | 299 | the icon's name in square brackets |
| font face, colour, size (`font`) | 68 | left out, the text between kept |
| a player's or a level slot's icon (`player`, `slot`) | 46 | left out, the text between kept |

  Anything else in angle brackets stays as text (three strings have words in them), and
  so do square brackets (stage directions in subtitles). Control characters other than
  the new line are dropped. What the game does with an unknown tag was not read.
  `Message::text` keeps the string as the script passed it.
- Placing. `place_bubbles` and `place_subtitle`, in screen pixels. A bubble's box is
  centred on its anchor's screen point with its lower edge 24 pixels above it. The
  bubbles of one anchor Thing form a stack, the first of the list lowest, 6 pixels
  apart. A stack that would leave the screen is moved back inside as a whole, 8 pixels
  from the edge, so a bubble whose Thing is off screen sits at the nearest edge. An
  anchor behind the camera places nothing. Stacks of different Things may overlap. The
  subtitle is centred, 48 pixels above the lower edge, its text at most 0.7 of the
  screen's width; a bubble's text wraps at 300 pixels. All of this is INFERRED.
- Painting. `speech_overlay.rs` in the viewer measures the lines with egui, asks the
  functions above for the places and paints boxes with the fill and outline of the
  viewer's own windows, behind them. The anchor goes from level units to the screen by
  the level's display matrix (`LevelDisplay`) and the camera. The boxes are also drawn
  under `--qa-hide-ui`: they belong to the level, not to a panel. This is not the game's
  bubble art.
- Counts. The "Level scripts" window has a line with the messages open, drawn, opened,
  written over and closed. The viewer prints one line on each tick on which a message
  entered the list or was closed (`Messages at tick N: ...`), with the anchor Thing and
  place of each drawn bubble: the check that needs no screenshot.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path
gamedata/lbp2/th_cosmos/final_levels/comp_01_rocket_funland/comp_01_rocket_funland.bin
--level-scripts`. That level opens one bubble without a prompt on the first tick, with
no player. Add `--level-focus-thing 246` to frame its Thing, or `--play-sackboy` to walk
into the triggers of a level (`--play-sackboy-at X,Y,Z` to start near a Thing).

### Measured (PC)

- The pulsed measuring run of part 16 again (378 levels, 60 ticks, `--visit-triggers
  --pulse-switches`): the whole `play_all` block is equal to the D265 run. Things stopped
  by a fault 4; requests 2,381, opened 1,187, written over 1,161, closed 1,152.
- Viewer, release build, the level above, 1280 x 720, no focus taken (`--qa-capture`):
  translation tables 2; `Messages at tick 1: open 1, opened 1, written over 0, closed 0;
  drawn: bubbles 1` at Thing 246. Two frames looked at: one framed on the Thing, with
  the box above the magic mouth; one from the Sackboy follow camera started at the
  Thing's place, where the Thing is above the frame and the box sits at the upper edge.
  The frames stay in ignored `local-analysis/`.

### Not modelled

- The game's bubble: art, fonts, the tail, where the HUD script puts it, bubble positions
  1 to 8, how a caption and a think bubble differ from speech.
- Which of several queued bubbles the game draws (the sort states of part 16).
- The player's close button. The prompt line is text only.
- Icons, emphasis, font faces, colours and sizes of the markup; the `%1`-style
  parameters inside the player and slot tags.
- The subtitle's formatting in the game (0x736f1c) and its place.
- The level-link and wormhole prompts.
- Other languages: the two English tables are the ones loaded (INFERRED, part 14).

### Verification

`cargo test -p lbp-runtime`: 46 unit and 36 world tests. New, all with made-up text: 6
unit tests in `message_view.rs` (markup tags and what is not a tag; which messages the
overlay draws; a bubble above its anchor and at each edge and corner; stacks of one Thing
and their order near the top; an anchor behind the camera; the subtitle's place) and the
world test `d266_the_overlay_reads_the_worlds_messages_and_tables` (the overlay through
the messaging natives, the close prompt through synthetic tables with the patch table
first, a moved anchor). fmt, the CI clippy line, the CI test set, the Python tests and
`cargo check` of both viewers pass; `cargo clippy -p lbp-bevy-viewer --all-targets -- -D
warnings` passes. Seen in the viewer window on one level, one bubble without a prompt.
Not seen by eye: a subtitle, a speaker's name, a close prompt line, a stack of several
bubbles, a caption. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).

## Part 18: global settings take effect (2026-10-13, D267)

Stage 5 of `docs/rscript-plan.md`, third PR. Part 14 gave the world the global settings
as start, target and blend factor. Scripts set targets in 93 levels, and the settings in
force moved in none: the blend factor is stepped by a script function that the engine
calls every frame, and the runtime did not call it. Now it does, the runtime hands out the
settings in force, and the viewer lights a played level again when a script changes the
lighting or the darkness.

### What the game does (FOUND, 01.33 executable and script listings)

- The PWorld's per-frame function (0xd5e84) goes through the eight global-settings slots
  in order. An empty slot is passed over. For a slot with a UID it looks the Thing up
  (0xce860, a search of the PWorld's sorted UID list). A Thing with a PScript part gets
  the script function `ApplyGlobalSettings__`, no arguments (0x88ff8c -> 0x620bd8). When
  no Thing has the UID, or the Thing has no PScript, the slot's UID and priority are set
  to 0. This is the only caller of the function: one reference in the executable, and no
  script calls it.
- On its first run (a PWorld flag byte) the same function writes every slot again with
  its own UID and priority, through the UID setter with the priority test off (0xce554).
  The setter clears the slot first and writes the priority back only for a UID other
  than 0. The one effect: an empty slot loses a stored priority.
- Place in the frame: the world update (0xeacd4) calls it at 0xeb2a4, after the switch
  pass (0xdd228, part 12), with four other calls between them.
- The script (the global-settings tweaker). `Enter` and `SwitchPerformReaction(true)`
  call `BeginApplyGlobalSettings`: it asks for its type's slot with
  `Game.SetGlobalSettingUID` and, when it then holds the slot, writes its values as new
  targets with blend on and keeps the frame. `ApplyGlobalSettings__`: seconds since that
  frame = frames / 30; while that is below its transition time (its `TransitionTime`
  member, or its length on a sequencer / 30) it writes the factor 3x^2 - 2x^3 of
  x = seconds / time, kept in 0..1, with `Game.SetGlobalSettingsBlendFactor`; after that
  it writes 1.0 on every call. While its `IsTweaking` member is set it writes its values
  without a blend instead. A lighting tweaker writes the factor of type 5; a water
  tweaker those of the water types.

Looked for and not found: the engine call that runs `Update__` on every scripted Thing.
One engine function calls a script function of that name by its name object (0x625780).
It is reached from one game-state object outside the world update (0x3f8d24), so it is
not the per-Thing call. The place of the new call relative to `Update__` stays INFERRED.

### What the runtime does

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| The call | `LevelScripts::apply_global_settings`: per slot in order, `ApplyGlobalSettings__` on the scripted Thing that holds it | FOUND |
| Slot holder | `LevelWorld::global_settings_holder`: the first Thing with the UID, when it has a script; else the slot is cleared | FOUND rule; that UIDs are unique INFERRED |
| First pass | an empty slot loses its stored priority | FOUND; that the flag byte is 0 for a loaded level INFERRED |
| Place in the tick | `tick_level`: after `switch_reactions` | FOUND after the switch pass; after `Update__` INFERRED (above) |
| No function | a holder whose script has no `ApplyGlobalSettings__` is passed over and keeps its slot | INFERRED (the call by name was not followed into the VM) |
| Stopped Thing | a holder stopped by a fault is not called | the schedule's own rule |
| Settings in force | `WorldState::settings_in_force` -> `SettingsInForce`: water level, gravity, wave magnitude, water tint, murkiness, the lighting group (5 values), water bits | FOUND blend rule (part 14); audio is not kept |
| Against the file | `LevelWorld::settings_at_load` (in force when the level was read) and `settings_unlike_load` (per type) | - |

Read API for a display: `host.world.pworld.settings_in_force()` each tick;
`SettingsInForce::light()` gives (lighting, darkness); `of_type` and `unlike` work per
setting type; `GLOBAL_SETTING_NAMES` names the eight types. `LightFollower` says when a
display should build a new light: when lighting or darkness is more than `RELIGHT_STEP`
(0.002, a PLACEHOLDER) from the pair shown, and once more when the pair has come to rest.
`StepReport::global_settings` counts the calls by type, the slots cleared and the slot
turns without a call. `WorldState::lighting_stored` keeps the five current values the
level file stores next to the group.

No new native. `Game.SetGlobalSettingsBlendFactor` (bound since part 14, never called
before) is now reached.

`level_scripts`: `play_all` has a `global_settings` block (and so has a level played
alone): the calls by setting type, the slots cleared, and by type the levels and ticks on
which the setting in force was unlike the one in force when the level was read.

### What the viewer does

`lbp-bevy-viewer --level-scripts`, for a level lit by the PWorld lighting rule (P3m: not
the Pod, no chosen preset, not the main block).

| Value in force | In the viewer | Label |
|---|---|---|
| Lighting | the stored presets blended at that position (`LevelSettings::at_lighting`) | the rule the viewer already uses for the file's own value (P3m, two live levels) |
| Darkness | exposure and fog colour x (1 - darkness), same function | as above; the exposure and fog split is INFERRED there |
| Fog | not applied | open: what the factor does to the settings block was not read |
| Fog tint | not applied | open, as fog |
| Colour correction | not applied | open: FOUND to be an index 0..7 into eight records the game holds; the records were not read |
| Gravity | not applied | open: its scale to physics units was not read |
| Water level, wave, tint, murkiness, bits | not applied | the viewer draws no water |
| Audio | not kept | no audio system |

- The level's main block, presets and light frame are kept at load
  (`lighting::LevelRelight`). `level_play.rs` asks `LightFollower` after each tick and
  builds the light again through the existing lighting path. A blend is followed step by
  step.
- The viewer prints one line per new light (`Lighting at tick N: lighting x, darkness
  y; not applied: ...`), and the "Level scripts" window has a global-settings line with
  the apply calls and the values that are not applied.
- A new light waits until the scene has been drawn for 60 frames. MEASURED: a new light
  in the first frame after the scene was spawned lost the GPU device, twice out of two
  runs; with the wait, 89 new lights on 89 ticks in a row did not. The cause is not
  established; the wait is a PLACEHOLDER guard.
- The scripts start ticking when the viewer starts, while the scene is still loading
  (as before this part). A change in the first seconds has happened by the time the
  scene is drawn; the scene is then lit once, with the values in force.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path
gamedata/lbp2/th_da_vinci/final_levels/cutscene_outro_theme1/cutscene_outro_theme1.bin
--level-scripts`. A global-settings tweaker there takes lighting from 0.4 to 1.0 and
darkness from 0 to 0.6 on the first ticks, with no player. Without `--level-scripts` the
level is lit from the file's values.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Units: tick (one `tick_level`); level; call (one `ApplyGlobalSettings__`). "Unlike" =
the setting in force after a tick is not the one in force when the level was read.

| Run | Ticks | Calls | Levels with a call | Levels with a setting unlike | Ticks with a setting unlike |
|---|---|---|---|---|---|
| plain | 22,680 | 4,562 | 66 | 48 | 2,819 |
| `--visit-triggers` | 104,826 | 80,255 | 109 | 81 | 40,159 |
| with `--pulse-switches` | 105,582 | 80,631 | 121 | 96 | 40,334 |

By setting type, levels unlike / ticks unlike / calls (calls are by the slot's type):

| Type | plain | visit | pulsed |
|---|---|---|---|
| Lighting group | 47 / 2,757 / 3,047 | 76 / 37,727 / 39,918 | 89 / 37,889 / 40,104 |
| of it: lighting or darkness | 42 / 2,434 | 70 / 32,464 | 85 / 32,610 |
| Gravity | 3 / 177 / 226 | 11 / 7,618 / 8,890 | 15 / 7,642 / 8,924 |
| Water level | 0 / 0 / 0 | 1 / 365 / 366 | 2 / 368 / 370 |
| Water tint, murkiness, bits | 0 | 0 | 1 level each, 1 or 2 ticks, 0 calls |
| Audio (not kept) | - / - / 1,289 | - / - / 31,081 | - / - / 31,233 |

- Before this part the settings in force were unlike the file in 0 levels in every run.
- Slots cleared 0; slot turns without the function 0; slot turns of a stopped Thing 0,
  in all three runs. So every slot holder in these runs is a Thing with the function.
- The five current lighting values the level file stores are equal to its stored start,
  target and blend factor taken together in all 378 levels. This fits the per-value
  blend of part 14.
- Things stopped by a fault: 4 -> 4 in all three runs; fault kinds 2, levels with a
  fault 2, the same two faults as before. The new call exposed no fault.
- Unbound natives called: 51 -> 51, 87 -> 87, 87 -> 87, the same natives with the same
  counts. Unbound calls 73,325, 1,002,445 and 1,010,529, unchanged.
- Native calls 557,861 -> 571,776, 5,130,313 -> 5,363,790 and 5,188,665 -> 5,423,370,
  all of the increase bound; natives called 190 -> 191, 253 -> 254, 255 -> 256 (the
  blend-factor native).
- Messages, trigger events and switch reactions are equal to parts 16 and 17.
- Viewer, release build, the level above, 1280 x 720, no focus taken (`--qa-capture`).
  Without `--level-scripts`: `3 stored presets blended at position 0.80 (PWorld lighting
  0.40, darkness 0.00)`. With it: `Lighting at tick 504: lighting 1.000, darkness 0.600
  (new light 1); not applied: colour correction 0.000, fog 0.500, fog tint 0.633`; the
  light's exposure goes from 1.134 to 0.437 and its sun, ambient and fog colours change.
  Two pairs of frames looked at (the whole level, and framed on Thing 168): the picture
  changes from a light green to a darker blue-green. In all four frames the viewer's own
  fog covers most of the picture, with and without the scripts, so little else can be
  judged from them. The frames stay in ignored `local-analysis/`.

### Not modelled

- Fog, fog tint and colour correction in the picture; what the game's light rebuild
  blends after a change (0xd9b70, two entries of a PWorld list) and how that relates to
  the preset rule.
- Gravity in `lbp-sim` (the unit is not read); water in the picture.
- Where the engine runs `Update__` in the frame, so the order of the new call and the
  update.
- Default audio settings the same per-frame function applies while the audio slot is
  empty, and the water values it refreshes for the renderer.
- The PWorld flag byte of the first pass as stored in a level file.
- Why an early new light loses the GPU device in the viewer.
- Running the level's scripts only once its scene is drawn, so that a change at the
  start can be watched.

### Verification

`cargo test -p lbp-runtime`: 49 unit and 37 world tests. New: 3 unit tests in
`global_settings.rs` (the settings in force and the per-type comparison; the light
follower's steps and coming to rest; the report sums) and the world test
`d267_the_slot_holders_apply_their_global_settings_each_tick` (a made-up script that
steps the lighting factor: the call each tick, the cleared slots, the first pass, a
holder without the function, a stopped holder, a level without a PWorld). fmt, the CI
clippy line, the CI test set, the Python tests and `cargo check` of both viewers pass;
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` passes. Seen in the
viewer window on one level: one change of lighting and darkness. Not seen by eye: a
blend followed step by step by real level data (89 lights in a row were built from a
test ramp), any other level. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer feature).

## Part 19: hidden, tinted and moved Things (2026-10-13, D268)

Stage 5 of `docs/rscript-plan.md`, fourth PR. Scripts hide a render mesh, set its colour
or tint and move Things. The world kept some of that and nothing drew by it. Now the
world lists the Things whose display values changed, and the Bevy viewer sets the draw
entities of a played level from that list: hidden, tinted, placed.

### What the game does (FOUND, 01.33 executable and script listings)

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Thing.HideRenderMesh__Q5Thing` | static `void (Thing)` | 0x642cd4 -> 0x754aac -> 0xebf98 | Nothing for a null Thing or one without a PRenderMesh. Else the visibility call with "off" and the mask 3 |
| `Thing.ShowRenderMesh__Q5Thing` | static `void (Thing)` | 0x642c94 -> 0x753e3c -> 0xebf98 | The same call with "on" |
| `PostItNote.IsVisibleInPlayMode__` | `bool ()` | 0x644d7c -> 0x5e9260 | Bit 0 of the visibility byte of the Thing's PRenderMesh; false for a null Thing or no PRenderMesh |
| `Thing.SetRenderMeshEditorColour__Q5Thingr` | static `void (Thing, v4)` | 0x642924 -> 0x753ed4 -> 0xecac4 | Nothing for a null Thing or no PRenderMesh. Else the colour, packed to 32 bits, into two words of the PRenderMesh |
| `Thing.SetRenderMeshEditorColourTint__Q5Thingr` | static `void (Thing, v4)` | 0x642978 -> 0x7549c8 | As above, into a third word, after the two |
| `Thing.SetShapeEditorColour__Q5Thingr` | static `void (Thing, v4)` | 0x642a20 -> 0x75495c | Nothing for a null Thing or no PShape. Else the packed colour into a word of the PShape |
| `Thing.SetShapeEditorColourTint__Q5Thingr` | static `void (Thing, v4)` | 0x6429cc -> 0x753a9c | As above, into the word before it |

- The visibility call (0xebf98) takes a PRenderMesh, on or off, and a mask. It clears the
  mask's bits in the PRenderMesh's visibility byte and, for "on", sets them again. Then,
  unless the Thing has a PShape or its mesh is one of two globe meshes (GUID 0x27bd,
  0x3e6e), it does the same for each child of the Thing that has a PRenderMesh and no
  PShape, and so on down from that child. A child without a PRenderMesh ends its branch.
- A new PRenderMesh starts with the byte 3 (its constructor), and with one start value in
  the two colour words and the tint word.
- The byte is the `visibilityFlags` field a level file stores for a RENDER_MESH part.
  Bit 0 is "visible in play mode": the game's own native of that name reads it. So the
  hide native leaves a mesh that is not visible in play mode, and a level file can store
  one that way.
- Use in the scripts. Hide and show: the level link, on its lock mesh. The editor colour:
  the global-settings tweaker, on its own Thing when it is created. The mesh tint and the
  shape colour: the scoreboard, the two race gates and the burning object; the
  scoreboard, when created, gives its shape a fixed colour and its mesh the shape's old
  colour as tint. The shape tint: the same four classes. The play-mode question: the
  note object, once.
- Read and not bound: `TriggerLevelLink.SetLockMeshVisible__b` (0x641520) makes the same
  visibility call on the link's lock mesh; no script calls it. `Thing.IsVisible__Q5Thing`
  (0x64b4b0) compares a frame number kept with the Thing's mesh against a global: whether
  the mesh was drawn, not a display setting.

INFERRED: the two colour words are the `editorColor` a level file stores, and the tint
has no stored form (a file has one colour per mesh); bit 1 of the byte is the editor's
(the community reader names the two bits play mode and edit mode); the order of the
channels in a packed colour (as the shape colour of part 2) and its rounding (the
conversion is vector code that the disassembly did not show in full). Not read: how the
renderer uses the byte, the editor colour and the tint.

### What the runtime does

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| Visibility byte | `ThingState::render_mesh_visibility`: the stored `visibilityFlags`; none without a PRenderMesh | FOUND |
| Hide, show | `LevelWorld::show_render_mesh`: the mask 3 on the Thing and down its children, by the rule above | FOUND |
| Hidden | `ThingDisplay::mesh_hidden`: bit 0 is clear. The level world runs in play mode | FOUND bit; that the game then draws nothing INFERRED |
| Editor colour | `ThingState::render_mesh_colour`: the stored colour until a script sets one; none without a PRenderMesh | FOUND target; INFERRED stored field |
| Mesh tint | `ThingState::render_mesh_tint`, only on a Thing with a PRenderMesh (before: on any Thing) | FOUND |
| Shape colour, shape tint | `ThingState::shape_colour`, `shape_tint`, only on a Thing with a PShape; a Thing without one is left alone (before: an error) | FOUND |
| Null Thing | the six setters do nothing (before: the shape colour and the mesh tint faulted) | FOUND |
| World matrix | `SetWorldPosition__m`, `SetLocalPosition__m`, the Things moved along with a parent, and a write of `PPos.WorldPosition` | unchanged rules (part 13) |

The dirty list (`lbp-runtime/src/display.rs`):

- `LevelWorld::take_display_dirty()` gives the Things whose display values changed since
  the last call, in Thing order, each with a `DisplayChange` (visibility, colour,
  transform), and empties the list. A Thing is listed when a write left a value that is
  not the value before it: the visibility byte, a colour as the packed colour the game
  keeps (a tint never set counts as white), the world matrix by its bits.
- `LevelWorld::thing_display(thing)` gives the values now (`ThingDisplay`),
  `display_at_load(thing)` the values the level file had. `ThingDisplay::moved_from` is
  the matrix that takes a Thing from its stored place to its place now: what a display
  puts in front of the matrix it placed a model with.
- `display_changed()` lists every Thing unlike the level file, for a display that starts
  after scripts already ran. `display_summary()` counts them (`DisplaySummary`).
- Not on the list: a player's Sackboy that the caller moves (`move_player`), the
  animation position and speed, the mesh texture.

Natives: 216 -> 219 in the table. New: `Thing.ShowRenderMesh__Q5Thing`,
`Thing.SetShapeEditorColourTint__Q5Thingr`, `PostItNote.IsVisibleInPlayMode__`. No
longer recorded only (`RECORDED_ONLY` 23 -> 21): `Thing.HideRenderMesh__Q5Thing` and
`Thing.SetRenderMeshEditorColour__Q5Thingr`.

`level_scripts`: `play_all` has a `display` block (and so has a level played alone): the
Things unlike the level file after the last tick and the levels with one, by kind; the
changes marked; the render meshes by stored visibility byte; the tints by kind.

### What the viewer does

`lbp-bevy-viewer --level-scripts`, for a level other than the Pod.

- The map. When the level scene is spawned, every draw entity of a Thing's own model
  gets a `LevelThing` component, and `thing_view::LevelThings` maps the Thing to its
  entities. A Thing's own model is its render mesh or one of its decorations, placed by
  its own matrix. The report lines say how many Things and entities are mapped.
- Each tick `level_play.rs` takes the dirty list and `thing_view::apply` sets the
  entities. For a scene that was just spawned it takes every mapped Thing that is hidden
  or unlike the level file instead, because the scripts tick while the scene loads. Like
  the light of part 18 it waits until the scene has been drawn for 60 frames.

| Change | In the viewer | Label |
|---|---|---|
| Render mesh hidden (bit 0 clear), by a script or in the level file | the mesh's entities are not drawn; the Thing's decorations stay | FOUND bit; INFERRED that nothing is drawn, and that decorations stay |
| Mesh tint | the mesh's entities get a material of their own; red, green and blue multiply the surface colour before lighting | INFERRED |
| Mesh tint's alpha | not used: a white tint with an alpha draws as no tint | open |
| World matrix | every entity of the Thing is moved by `ThingDisplay::moved_from` | the scene's own placing rule, moved along |
| Mesh editor colour set by a script | not drawn, counted | the scene builds the colour into the material when the level is loaded |
| Shape colour, shape tint | not drawn, counted | below |
| A moved Thing with a shape | not moved, counted | below |

- Shapes are not followed. The scene merges the shapes of one material and colour into
  shared meshes in world space, so one shape has no entity to hide, colour or move. A
  shape that a script recolours or moves keeps the look and place the level file gives
  it, and the viewer counts it ("no draw entity", "editor or shape colour"). Taking
  such shapes out of their batch is left for a later step.
- Also not followed: the Pod (`--pod-camera`: it places its planets by its own rules),
  meshes skinned per item (a mesh on a joint span, `--per-bone-placement`), the
  backdrop, sticker decals.
- Without `--level-scripts` the viewer draws a level as before, with every mesh.
- The viewer prints one line per tick on which the scene changed (`Things at tick N:
  ...`), with the first Things hidden and tinted, and the "Level scripts" window has a
  Things line: the check that needs no screenshot.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path gamedata/lbp2/dlc/move/levels/side_rts/side_rts_tutorial.bin
--level-scripts --level-focus-thing 108`. Thing 108 is a level link's lock mesh. Add
`--no-lbp-lighting` to look at it without the viewer's fog.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Units: Thing; level. "Unlike" = the value after the last tick is not the level file's.
The three runs (plain, `--visit-triggers`, with `--pulse-switches`) give the same counts
except where two numbers are given.

| What | Things / levels |
|---|---|
| Render meshes | 68,618 |
| Stored visibility byte 3 / 2 / 0 | 56,512 / 11,573 / 533 |
| Not visible in play mode in the level file (bit 0 clear) | 12,106 / 218 |
| Visibility byte unlike the file | 0 / 0 |
| Mesh editor colour unlike | 10 / 1 |
| Mesh tint unlike (white when never set) | 102 / 101 |
| of them white with an alpha / with a colour | 101 / 100 and 1 / 1 |
| Shape colour unlike | 102 / 101 |
| Shape tint unlike | 0 / 0 |
| World matrix unlike in any bit | 4,545 / 32 |
| World matrix unlike by more than the counting threshold | 1 / 1 (no mesh, no shape) |
| Changes marked: visibility / colour / transform | 0 / 214 / 4,604 (pulsed 4,606) |

- So in these runs the scripts change no visibility byte: the 113 hide calls (103
  levels) are on meshes the level file already stores as not visible in play mode. What
  the viewer hides under `--level-scripts` is the 12,106 meshes of the file.
- The 1,180 editor-colour calls (135 levels) set the colour the file stores, but for 10
  Things of one level.
- The tints are the scoreboards': 101 are white with the alpha 0.498, and the viewer
  draws no tint for them. One, in one level, has a colour (0.70, 1.0, 0.15). 46 more
  Things in 21 levels are given a white tint, which is no change. That level is also
  the one with the 10 new mesh colours.
- The 4,545 moved Things are the ones whose script writes its stored place back; one
  Thing ends elsewhere, in the Pod, and it has no mesh. So no drawn Thing moves here.
  The counting threshold is 0.01, or 0.00001 of the matrix's largest element for a Thing
  far from the origin (`MOVED_FAR`, `MOVED_FAR_RELATIVE`; not game values).
- Things stopped by a fault: 4 -> 4 in all three runs; fault kinds 2. The Pod's fault
  moved by one instruction: the shape-colour call on a null Thing is now nothing, and
  the script then reads a part of that null Thing.
- Unbound natives called 51 / 87 / 87 with the same calls as before; native calls
  571,776, 5,363,790 and 5,423,370, unchanged. Calls to recorded-only natives fall by
  1,293 in each run (the two natives that left the list). Messages, global settings,
  trigger events and switch reactions are equal to part 18.
- Viewer, release build, side_rts_tutorial, 1280 x 720, no focus taken
  (`--qa-capture`): `Thing map (level scripts): 200 Things with a model of their own,
  759 draw entities`; `Things at tick N (new scene): listed 24, with a draw entity 24;
  hidden 23, ... tinted 0 ... not followed: no draw entity 0, editor or shape colour 1,
  tint of alpha only 1`. Two pairs of frames looked at, without the viewer's fog
  (`--no-lbp-lighting`), without and with the scripts: framed on Thing 108 the padlock
  of the level link is gone and the link's disc stays; framed on the scoreboard (Thing
  975) two small white markers at its lower corners are gone and its colours are the
  same. On ghost_train (869 Things mapped, 232 hidden, 1 tinted), framed on the
  scoreboard (Thing 1151): its frame turns from tan to yellow-green, and the small
  markers around it are gone. The frames stay in ignored `local-analysis/`.

### Not modelled

- Shapes: a recoloured, tinted or moved shape in the picture (102 recoloured shapes in
  101 levels).
- The mesh editor colour a script sets, in the picture; the tint's alpha; how the
  game's shader uses the editor colour and the tint.
- What the game draws for a mesh with bit 0 clear, and for the decorations of a hidden
  mesh; bit 1 of the byte.
- A Thing flag that makes a local position the world position (part 13), and the
  refresh of bodies and shapes when a Thing moves.
- The animation position and speed scripts set (537 and 560 Things in the earlier
  count): a mesh's animation is not played per Thing.
- The mesh texture a script sets (kept, not drawn); `SetCollidable`,
  `SetBonesToSkinPoseLocal` (unbound).
- The Pod's planets under `--level-scripts`.

### Verification

`cargo test -p lbp-runtime`: 52 unit and 38 world tests. New: 3 unit tests in
`display.rs` (packed colours; what counts as unlike, hidden and moved far, and the
matrix from the stored place; the sums) and the world test
`d268_hidden_coloured_and_moved_things_are_listed_for_a_display` (the seven natives on
a made-up Thing tree: the hide rule down the children, a shape, a child without a mesh
and a globe mesh; the null Thing; colours with and without the part; a move with its
descendants; the list taken once; the summary). `lbp-script-vm`: the display natives in
a world without the state. `lbp-bevy-viewer`: `thing_view` test (a new scene, then a
hide, a tint and a move after it, on three entities with a decoration; a material of
its own in the scene list; a tint of alpha only). fmt, the CI clippy line, the CI test
set, the Python tests and `cargo check` of both viewers pass; `cargo clippy -p
lbp-bevy-viewer --all-targets -- -D warnings` passes; the viewer's tests pass but the
one that fails on main (`sackboy_play::tests::input_script_steps`). Seen in the viewer
window on two levels: meshes hidden and one mesh tinted, from the first set of the
scene. Not seen by eye: a mesh hidden, tinted or moved while the scene is up (no level
of these runs does it after its first ticks; the test does), a moved Thing. Nothing
compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer feature).

## Part 20: the script camera and the screen (2026-10-13, D269)

Stage 5 of `docs/rscript-plan.md`, fifth PR. A scoreboard takes the camera when a player
reaches it, and a speech bubble closes when its Thing has been off the screen for two
seconds. Both asked natives that were not bound. Now the world keeps the camera a script
lays over the follow camera and steps its blend, the screen natives answer from the
camera the picture was drawn with, and the Bevy viewer moves its camera to the override
and back.

### What the game does (FOUND, 01.33 executable and script listings)

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Cam.SetOverrideCamera__rr` | static `void (v4, v4)` | 0x5fc45c -> 0x7349f8 -> 0x3760e4 | Stores a position and a direction in the level's camera, as passed. Nothing without a level camera |
| `Cam.SetOverrideCameraActivation__bf` | static `void (bool, f32)` | 0x5fc348 -> 0x734bc4 -> 0x374cec | Sets the override's blend target to 1.0 or 0.0 and its blend rate to the float. It does not touch the blend |
| `Cam.GetCameraMatrix__` | static `m44 ()` | 0x609ed0 -> 0x7347e0 | Copies the render camera's camera matrix out: camera to world |
| `HUD.GetScreenSpacePosition__r` | static `(v4)`, returns the 2D point type | 0x601d30 -> 0x4b6e80 -> 0x2e7bdc, 0x2e74a0 | Projects a world point with the render camera's view-projection matrix and maps it to pixels |
| `HUD.GetXRes__`, `HUD.GetYRes__` | static `s32 ()` | 0x5fa5e0 -> 0x4b6a94, 0x5fa5a4 -> 0x4b6aac | Two globals; their values in the executable's data are 1280 and 720 |

- The override. The level's camera has one: a position, a direction, a blend, a blend
  target and a blend rate. Only the activation native writes the target and the rate,
  apart from the camera's constructor (zero) and a copy of the whole record. So the
  engine never ends an override; a script does. There is no owner and no priority: the
  last caller wins.
- Each camera update (the apply function 0x6eb650), in this order:
  1. The blend takes a step (helper 0x38eb40): the target when it is nearer than the
     rate, else the blend moved by the rate toward it. Linear.
  2. The weight is `0.5 - 0.5 cos(pi x blend)`.
  3. The view direction goes from the follow camera's to the override's on the arc
     between them (helper 0x3918ec): a plain linear mix when their dot product is above
     0.999, the sine form below it. An opposite pair takes a fallback axis.
  4. The eye is the follow camera's eye plus the weight times the way to the override's
     position.
  5. The camera aims at the eye plus 1000 times the direction.
- The follow camera keeps running underneath. While the blend is above 0.0001 the camera
  takes a plain lens: 60 degrees of vertical field of view (1.0472 rad), near 10, far
  40000 (0x378470, 0x38367c). The weights of the tracked players are multiplied by
  1 - blend (0x3764a4), so players stop steering the follow camera while the override is
  in.
- The camera matrix. Column W is the eye and column Z points backward: the look-at
  builder that fills it (0x2e7990) writes right, up, minus forward, eye.
- The screen position. `clip = M x (x, y, z, 1)`, whatever the point's own w; divided by
  `clip.w`, or by 0.0001 when that is exactly 0 (the constant at 0xd9c820);
  `x = (ndc.x + 1) / 2 x XRes`, `y = (1 - ndc.y) / 2 x YRes`. So: pixels, from the top
  left corner, y down. There is no test for a point behind the camera: its w is negative
  and it comes out mirrored through the middle of the screen. The native sets the fourth
  component to 0.
- Use in the scripts. The scoreboard, once its trigger fires, reads the camera matrix,
  takes column W as its camera position and minus column Z as its direction and turns
  the override on with the rate 0.05. From then on it eases its own position and
  direction toward the scoreboard each frame and hands them to `SetOverrideCamera`. When
  it is destroyed it turns the override off, with 0.05 again. The speech bubble, each
  tick while it is not a subtitle, asks for the screen position of its microchip's
  place; more than 100 pixels outside the screen (against `GetXRes` and `GetYRes`) adds
  one to a counter, anything else sets the counter to 0; past 60 the bubble closes its
  message, unless a flag of the bubble says otherwise.

INFERRED: the up vector under an override is world +Y (the look-at build after the blend
was not read); the third component of the screen position is the depth after the divide
(no script reads it); the two resolution globals keep their values in play (a store
through a pointer at video set-up was not ruled out); the two pointers through which the
render camera is reached name one record. Not read: which of the two camera update
functions that call the apply function runs in a level frame, and where that is against
the script update; the fallback axis of an opposite pair.

Read and not bound: `Native.SetSpeechBubbleActive__Q5Thingb` (0x649464 -> 0x6d4eb0 ->
0x55f70; 16 levels, 63 calls in the pulsed run). FOUND: it finds one part of the Thing
and makes one call with the bool. Which part, and what the call does, is INFERRED only
(the camera tweak, a speech-bubble camera zone), so it stays unbound and counted.
`Cam.SetCameraFromToUp__rrrffff` writes the render camera directly, with no blend; only
the Pod calls it, and it is left for stage 6.

### What the runtime does

`lbp-runtime/src/camera.rs`. State in `LevelWorld::camera` (`CameraState`).

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| The override | `OverrideCamera`: position and direction as passed, target, rate, blend | FOUND |
| The two setters | `World::set_override_camera`, `set_override_camera_activation`: the stores above; counted and nothing else in a level without a PWorld | FOUND |
| Blend step | `step(blend, target, rate)`, in `LevelWorld::step_camera` | FOUND rule |
| Place in the tick | `tick_level`: after `apply_global_settings`, before the frame count goes up | INFERRED |
| Weight | `ease(blend)`, `OverrideCamera::weight` | FOUND |
| Mix | `blend_pose`: eye linear, direction by `slerp`; both directions taken at length 1 and the result too | FOUND mix; the lengths INFERRED (the game stores the direction as passed; the scoreboard passes length 1) |
| Opposite directions | `slerp` turns about world +Y, or +X for a direction along Y | INFERRED (the game's axis was not read) |
| Threshold and lens | `OVERRIDE_THRESHOLD` 0.0001; `OVERRIDE_FOV_DEG` 60, `OVERRIDE_NEAR` 10, `OVERRIDE_FAR` 40000 | FOUND |
| Camera matrix of a pose | `CameraPose::matrix`: right, up, backward, eye; up from world +Y | FOUND layout; INFERRED up |
| Screen | `SCREEN` = 1280 x 720, whatever a window measures; `screen_position` | FOUND |
| The view | `CameraState::view` (`CameraView`: camera matrix and view-projection matrix, level units), written by whoever draws; scripts never write it | - |
| `GetCameraMatrix` without a view | the default follow camera (`FOLLOW_EYE` from the first player's Sackboy, looking down -z) under the override; the identity without a player | INFERRED stand-in |
| `GetScreenSpacePosition` without a view | the middle of the screen, counted in `screen_queries_without_view` | INFERRED stand-in |

- Read API for a display: `LevelWorld::camera_pose(follow_eye, follow_dir)` gives the
  camera to draw with while the override is in (`CameraPose`: eye and direction, level
  units) and `None` while the blend is at or below the threshold. The display hands in
  its own follow camera each frame, so the follow camera keeps moving under the blend.
- Write API: after drawing, the display writes `camera.view`. `view_projection` builds
  the second matrix from a camera matrix and a lens for a caller without a renderer.
- The stand-ins are chosen so that a run without a display behaves as before this part:
  a defaulted screen position was (0, 0), which is on the screen too, so no bubble
  closes for being off screen and the message counts stay equal.
- Counters: set calls, activation calls and those that turned the override on, ticks
  with the override in, the highest blend, camera matrix queries, screen queries (all,
  without a view, more than the bubble's margin off the screen).
- `FOLLOW_EYE` (296, 168, 1218) is the one follow pose measured in the game's memory in
  an emulator; it moved from the viewer into the runtime, and the viewer reads it there.

Natives: 219 -> 225 in the table: the six of the table above. None is recorded only.

`level_scripts`: `play_all` has a `camera` block, and a level played alone has it as
`camera.override` with the override's last values.

### What the viewer does

`lbp-bevy-viewer --level-scripts` (`level_play.rs`, system `script_camera`).

- Each frame, after the viewer's own camera system has placed the camera, that placement
  is taken as the follow camera: its eye and view direction go to level units through
  the level's display matrix.
- While the override is in, the camera is put at `camera_pose` of that follow camera,
  with world +Y up, and takes the 60 degree field of view. When the override is out the
  camera's own field of view comes back. The viewer's orbit state is not changed, so
  orbit, pan and zoom go on underneath and the view returns to them.
- Then the camera the picture is drawn with is written as `camera.view`. The scripts
  read it on the next ticks: the scoreboard's first position is where the viewer's
  camera was, and a bubble's off-screen test is against the picture.
- Not applied: the override lens's near and far planes (the viewer keeps its own, in
  display units); the players' weights in the follow camera.
- The viewer prints one line when the override comes in, is fully in, or goes out
  (`Camera at tick N: override ...`), with the camera's place in level units, and the
  "Level scripts" window has a camera line with the blend and the screen queries.
- A consequence to know: with `--play-sackboy` the Sackboy starts at the first
  checkpoint of the level file, and in some levels that checkpoint is the scoreboard's
  own (it sits inside the scoreboard's trigger). The scoreboard then fires on the first
  ticks and the camera goes to it and stays, as its script says. `--play-sackboy-at`
  starts him elsewhere.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path
gamedata/lbp2/th_da_vinci/final_levels/main_04_bravery_test/main_04_bravery_test.bin
--level-scripts --play-sackboy --play-sackboy-at -9168,2950,-400`. That starts the
Sackboy in the scoreboard's trigger. Without `--level-scripts` the follow camera stays.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Units: native call; tick (one `tick_level`); level. No view is written in these runs.

| What | plain | `--visit-triggers` | with `--pulse-switches` |
|---|---|---|---|
| Things stopped by a fault, before -> now | 4 -> 4 | 4 -> 4 | 4 -> 4 |
| Unbound natives called | 51 -> 48 | 87 -> 81 | 87 -> 81 |
| Unbound calls | 73,325 -> 62,000 | 1,002,445 -> 865,777 | 1,010,529 -> 872,549 |
| Native calls (unchanged) | 571,776 | 5,363,790 | 5,423,370 |
| `SetOverrideCamera` calls | 0 | 39,386 | 39,588 |
| Activation calls / of them on | 0 | 101 / 101 | 101 / 101 |
| Levels where the override comes in | 0 | 101 | 101 |
| Levels where it is fully in / still in after the last tick | 0 | 98 / 101 | 98 / 101 |
| Ticks with the override in | 0 | 39,386 | 39,588 |
| Camera matrix queries / levels | 0 | 101 / 101 | 101 / 101 |
| Screen position queries / levels | 3,775 / 28 | 32,360 / 61 | 32,730 / 70 |
| of them without a view / off the screen | 3,775 / 0 | 32,360 / 0 | 32,730 / 0 |

- The unbound natives that are gone are the six, with the calls the runs before counted
  for them; no native is new on the list and no other count on it changed.
- Fault kinds 2 and levels with a fault 2, as before. The messages, global settings,
  display, trigger events and switch reactions blocks are equal to part 19 in all three
  runs: the stand-ins keep a run without a display as it was.
- The override is in on 101 levels, the levels with a scoreboard whose trigger the
  visiting player reaches. In 3 of them the activation comes less than 20 ticks before
  the run ends, so the blend does not reach 1. No level turns it off: the scoreboard is
  not destroyed in these runs.
- With the rate 0.05 the blend takes 20 steps from 0 to 1 and 20 back, the last one
  landing on the target (MEASURED in the unit test, in f32).
- Viewer, release build, 1280 x 720, no focus taken (`--qa-capture`), the level and the
  start above: `Camera at tick 701: override fully in (blend 1.00, weight 1.00, target
  on); set calls 699, activations 1`, the camera at level (-9167.9, 3023.8, 800.0). Two
  frames looked at, without the panels. Without the scripts: the follow camera, the
  Sackboy left of the middle in front of the lower half of the scoreboard. With them:
  the camera is centred on the scoreboard, looks slightly up, and the scoreboard's
  frame fills the height of the picture. The scripts tick while the scene loads, so the
  override was fully in before the first frame was drawn: the blend itself was not
  seen. On comp_01_rocket_funland, framed on the bubble's Thing: 488 screen queries, 449
  of them before the scene was up (without a view), 0 off the screen, and the bubble
  stays open. The frames stay in ignored `local-analysis/`.

### Not modelled

- The follow camera itself: still the one measured pose in the viewer and the stand-in
  of the runtime. The game's follow camera (player weights, camera zones, cut scenes,
  the fades next to the override's) is engine code and a stage of its own.
- The up vector and roll under an override; the override's near and far planes in the
  viewer.
- Where the camera update runs in the game's frame.
- The speech-bubble camera (`SetSpeechBubbleActive`, the camera tweak's type).
- The scoreboard itself: its screens are not drawn, and the branch of its zoom that
  follows the winning player calls natives that are not bound and were never reached.
- The other camera and screen natives the draw functions of level scripts call
  (`Math.WorldToScreenSpace`, `Cam.IsPointOnScreen2D`, `HUD.GetScreenEdgeCrop` and
  others): nothing calls those functions yet.
- The Pod camera (`SetCameraFromToUp`): stage 6.
- Whether the screen is 1280 x 720 in every video mode, and what a 4:3 screen changes.

### Verification

`cargo test -p lbp-runtime`: 58 unit and 40 world tests. New: 6 unit tests in
`camera.rs` (the step: 20 steps each way, a rate larger than the gap, no rate; the ease;
the arc, its linear branch and an opposite pair; the pose at weights 0, a half and 1,
without a direction and with a position that is not a number; the camera matrix; the
screen position: the middle, the edges, y down, the point's w ignored, a point behind
the camera mirrored, w = 0) and two world tests:
`d269_the_override_camera_blends_in_and_the_screen_natives_use_the_view` (the six
natives on a made-up level: the stores, 20 ticks in and 20 out, the pose half way, the
camera matrix with and without a view and a player, the screen position with and without
a view, a level without a PWorld) and `d269_a_script_reads_its_screen_position_each_tick`
(a made-up script that asks for the screen position of its Thing each tick: 61 ticks
without a view, on the screen, and 61 ticks more than 100 pixels outside; the tick steps
the blend). The script keeps the numbers; the bubble's own counter and branch are not in
it. `lbp-script-vm`: the six natives in a world without the state, and their argument
checks. `lbp-bevy-viewer`: `the_view_for_the_scripts_is_in_level_units`. fmt, the CI
clippy line, the CI test set, the Python tests and `cargo check` of both viewers pass;
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` passes; the viewer's
tests pass but the one that fails on main (`sackboy_play::tests::input_script_steps`).
Seen in the viewer window on one level: the camera on the scoreboard. Not seen by eye:
the blend coming in or going out, a bubble closed for being off the screen. Nothing
compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer feature).

## Next

- Stage 6 of `docs/rscript-plan.md`: the Pod, with its camera (`SetCameraFromToUp`).
- From part 20: the game's follow camera (player weights, camera zones), the up vector
  under an override, where the camera update runs in the frame, the speech-bubble
  camera, the screen natives of the draw functions; in the viewer, a start for the
  Sackboy that is not the scoreboard's checkpoint, and a look at the blend coming in.
- From part 19: shapes that scripts recolour or move (taken out of their batch), the
  editor colour a script sets, the tint's alpha, the animation position per Thing, how
  the game's shader uses the editor colour and the tint.
- From part 18: what fog, fog tint and colour correction do to the picture; gravity for
  `lbp-sim`; where the engine runs `Update__`; the GPU device lost on an early new
  light; starting the scripts when the scene is drawn.
- From part 16: the player's close button (event 2), the list's sort order, how long a
  closing message stays.
- From part 17: where the game's HUD puts a bubble and what it looks like, a look at a
  subtitle and a stack of bubbles in the window, the markup's icons and emphasis.
- From part 14: the level's slot and the slot database, feeding the pads in the viewer,
  the scoreboard group the trigger visit now reaches,
  the creature state for `IsDead`.
- From part 15: the cone direction and the layer test of the tag and player sensors; the
  other sensor types, the randomiser, sequencers.
- The owner-player fallback of `GetSwitchActivationPlayer`.
- The z span of Sackboy's body for the layer rule (a memory read in RPCS3).
- Drive the viewer camera from `LevelWorld::camera` (part 7 runs the scripts and shows
  the camera values; the level-unit to display-unit mapping needs a PC capture).
- Script threads (`Thread.Start__` -> `Thread.StartImpl__`) for the Pod intro cutscene.
- PlanetThing `Update__rb` as a frame entry (its arguments are still unknown).
- Rookie Test: Gooey members;
  Pod: `Thing.FindThing__Q7StringA`.
- Resource object values, if the PC count shows scripts reading them.
