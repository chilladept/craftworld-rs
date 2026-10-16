# Level scripts, part 4: the next four natives (2026-10-08)

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
