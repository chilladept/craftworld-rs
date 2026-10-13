# Script natives, first bindings (runtime plan item Rs)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Builds on the VM
(D128, `docs/script-vm.md`) and the G10 native-call census (D133,
`docs/d133-patch-scripts.md`; ranked export in the project files,
`lbp2/research/g10-native-top200.md`). Bindings are written from the names, counts and
general vector-library conventions only. No script bytecode, listing or engine code was
used or is committed.

## What it adds (`lbp-script-vm`)

- `signature::parse`: splits a mangled native name (`Name__` + parameter codes).
- `natives`: a table of bindings keyed by (declaring class, mangled name); a `World`
  trait for the engine side, whose methods default to an explicit "not available" error;
  `Bindings<W>`, which implements the VM `Host` and counts calls to unbound natives.
- `System.Yield__` suspends the VM: `Vm::start` / `Vm::resume` return
  `Step::Done(value)` or `Step::Yielded(task)`. `Vm::call` faults with `YieldInCall`.
- `Vm::has_code(class, function)`: whether a call resolves to script code.
- Example `native_coverage SCRIPT-DIR NEW-JSON`: the share of native call sites that have
  a binding, over all scripts and over gameplay scripts.

## Bound natives (212 in the table, plus `System.Yield__` in the VM)

| Group | Natives | Needs from `World` |
|---|---|---|
| Math | `floor__f`, `clamp__fff`, `atan2__ff`, `rotateX/Y/Z__f`, `translate__r`, `scale__r`, `mulPerElem__rr`, `length3__r`, `normalize3__r`, `length__p`, `NonDeterministicRandomBetween__ff` | random only |
| String | `Concat__Q6StringQ6String`, `StringFromInt__i`, `InfinitySymbol__` | nothing |
| LAMS | `Translate__i` | text by key |
| Thing | `GetThingUID__`, `GetParent/GetFirstChild/GetNextSibling__Q5Thing`, `GetWorld__` (also System), `GetWorldPosition__`, `GetCOM__`, `SetLocalPosition__m`, `IsInactive__` | thing tree, transforms |
| Switch inputs and outputs (docs/level-scripts.md part 11) | `Thing.IsSwitchTriggered__i`, `GetSwitchActivationBinary/Analogue/Ternary/Player__i`, `GetInput__Q5Thingi`, `GetOutput__Q5Thingii` | the output wired to each input port; a PSwitch's output targets |
| PartList | `HasPJoint/PSwitch/PTrigger/PYellowHead/PShape/PSpriteLight/PRenderMesh/PCameraTweak__Q5Thing` | parts |
| Triggers (docs/level-scripts.md parts 10 and 11) | `Native.GetNumInThings__Q5Thing`, `GetInThing__Q5Thingi` | Things inside a PTrigger |
| Script class (part 11) | `BaseWorld.IsScoreBoard__Q5Thing` | the Thing's script and its superclasses |
| Other | `Audio.PlaySound__Q5ThingQ7StringA`, `Resource.LoadTextureByFilename__g`, `Resource.LoadPlanByFilename__g`, `Game.GetNumPlayers__`, `Game.GetPlayOrEditMode__`, `Audio.GetDialogueList__`, `Thing.Get/SetShapeEditorColour__Q5Thing(r)` (docs/level-scripts.md part 2), `Thing.SetRenderMeshEditorColourTint__Q5Thingr`, `Native.SetAnimSpeed__Q5Thingf`, `Game.GetPlayerNumber__i` (part 4), `System.Yield__` (in the VM) | audio, resources, players, mode, shape colour |
| Pod camera and part 6 (docs/level-scripts.md part 6) | `Math.lerp__frr`, `SmoothStep__fff`, `Smooth__fffff`, `Smooth2D__pppff`, `Smooth3D__rrrff`; `Cam.SetCameraFromToUp__rrrffff`, `SetDOFAndFog__fff`, `ClearMotionBlur__`; `HUD.GetCinemaModeCount__`; `PodComputer.GetPodComputer__`, `GetPodComputerInUse__`, `GetCrashZoom__`, `GetPlanets__`; `Planets.GetCrafteroid__i`; `Planet.GetThing__`; `Profile.IsLBP2GameProgressionFlagSet__i`; `Thing.SetWorldPosition__m`; `Native.DebugPrint__i`, `SetAnimPos__Q5Thingf`; `LAMS.MakeLamsKeyID__Q6String` (Toolkit makeLamsKeyID); `Game.SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi`; `Gooey.CreateGooey__i`; interactive music commands (`Audio.SetInteractiveMusic__ggffiQ5Thing`, `SetInteractiveMusicStereoPairVolume__giff`, `StopInteractiveMusic__`); 11 state queries (`natives::STATE_QUERIES`) | camera, engine objects, profile flags, state defaults |
| Strings and math, stage 1 (docs/level-scripts.md part 9) | `String.Length/IsNullOrEmpty/Find/SubStr/ToUpper/StringFromStringA`, `StringA.Length/IsNullOrEmpty/Find/SubStr`, `Math.identity__`, `ceil__f`, `exp__f`, `pow__ff`, `sign__f`, `lerp__fff`, `Delt__fff`, `BiasCosine__f`, `Resource.LoadTextureByFilename__gb` | resources (texture) |
| World and position, stage 4 (docs/level-scripts.md part 13) | `Thing.GetLocalPosition__`, `GetMicroChipProxyFromThing__Q5Thing`, `GetWorldPositionForSensor__`, `Game.IsOnSequencer__Q5Thing`; changed: `GetWorldPosition__` returns the matrix, `SetLocalPosition__m` and `SetWorldPosition__m` move the descendants, `IsInactive__` reads the deactivated flag | local and world matrices, the microchip of a Thing |
| Players and score, stage 4 (part 13) | `Game.EPlayerNumberFromInt__i`, `DoesPlayerExist`, `IsPlayerConnected__i`, `GetYellowheadFromPlayerNumber`, `PlayerNumberIsLocalPlayer`, `IsLocalPlayerID`, `GetPartyLeadersPlayerNumber__`, `Set/GetPlayerTeamNumber`, `Set/GetPlayerScoreMultiplierCounter`, `GetPlayerScoreMultiplier`, `IncrementPlayerScoreMultiplierCounter`, `GetPlayerScoreMultiplierTimer`, `DecrementPlayerScoreMultiplierTimer`, `SetPlayerScoreMultiplierVisible`, `SetPlayerScoreMultiplierSource`, `Get/SetPlayerScore`; changed: `GetPlayerNumber__i` gives -1 outside the records, `SetPlayerScoreMultiplierTimer` does nothing without a record | player records (`natives::PlayerRecord`), the players' Sackboys |
| Game state, stage 4 (part 13) | `Game.Get/SetGlobalSettingUID`, `GetLevelSlotID__`, `GetPrimaryLevelSlotID__`, `IsVcrPaused__`, `IsTutorialLevel__`, `IsPSNDemoBuild__`, `Audio.IsSafeToTriggerMusic__i`, `TriggerLevelLink.FindLockMesh__` | the global-setting arrays, SlotID objects made ahead, mesh GUIDs |
| Recorded only, stage 4 (part 13; `natives::RECORDED_ONLY`) | `World.SetPieChartMultiplier__i`, `Thing.UpdateScale__`, `HideRenderMesh__Q5Thing`, `SetRenderMeshEditorTexture__Q5ThingQ8Resource`, `SetRenderMeshEditorColour__Q5Thingr`, `RemoveFromAnimatingDivergentList__`, `Trigger_Speech_Bubble.NotifyNpcsOfMouthEnd__`, `NoJoinMarker.SetGameNoJoin__`, `Game.GotoScoreboard__b` | `World::engine_call`: the call is counted and kept, nothing else happens |
| Pad, LAMS and game state, D263 (part 14) | `Pad.IsJustPressed/IsPressed/IsJustReleased__Q5Thingii`, `LAMS.IsInLams__i`, `Resource.LoadAnimatedTextureByFilename__g`, `Game.GetGameFrame__`, `GetNumYellowHeadAvatars__`, `GetNumKeyNames`, `GetKeyIndex`, `GetNumKnownItemsCollectedAtStart__`, `IsTarsierDLCEnabled__`, `Score_Board.GetGameScoreDataType__`, `IsSlotIDValidForScoring__Q6SlotIDi`, `Slot.GetSlotGroup__Q6SlotID`, `GameMessagingManager.Messaging_ShouldFilterMessageStrings__`, `OffScreenIndicator.GetPlayerIndicatorActivityType__i`, `ModalOverlay.IsPartyLeaderOverlayActive__`, `Poppet.GetTextEntryType__` | a pad per player record (`natives::PadState`), loaded translation tables, the level slot, 9 more state queries |
| Global settings, D263 (part 14) | `Game.SetGlobalSettingsTo__fffffb`, `SetGlobalSettingsGravityTo__fb`, `SetGlobalSettingsWater{Level,WaveMagnitude,Tint,Murkiness,Bits}To__fb`, `SetGlobalSettingsWaterDrainSoundsEnabled__b`, `SetGlobalSettingsBlendFactor`, `GetGlobalSettingsWater{Level,WaveMagnitude,Tint,Murkiness,Bits}__` | start, target and blend factor per setting |
| Recorded only, D263 (part 14) | `GameMessagingManager.Messaging_NewTriggerMessage`, `...NoPrompt`, `...Subtitles`, `Messaging_NewLevelLinkPrompt`, `Messaging_NewWormHolePrompt`, `Messaging_NewGameEvent`, `HUD.RegisterOverlayCallback`, `HUD.PreloadGlyphs`, `Game.TrackSpeechKey`, `Trigger_Speech_Bubble.NotifyNpcsOfMouthStart__i`, `Thing.AddToAnimatingDivergentList__`, `Thing.SetAnimLoop__b`, `Gooey.SetLastNavSelectionMode__b`, `PerFrameUpdate`, `SetStylesheetScalingFactor__f`, `Game.SetGlobalSettingsAudioTo` | `World::engine_call`; ten are kept with their arguments |
| Test (part 2) | `Check`, `CheckFalse`, `Fail`, `CheckEqual` on `i b j f p q r`, `CheckSimilar__fff`, each ending `Q7StringAi` | records each check |

Coverage against the G10 export, computed from its tables (unit: call instruction):

- **Gameplay top 100:** 953 of 1,356 calls are bound, across 40 of the 100 natives.
- **All-scripts top 200:** 2,981 of 11,999 calls are bound. Gooey (menu layout)
  dominates that list and is UI work, not Rs.

PC run after merge (D135, part 1, unit: call instruction, 426/426 scripts read):
23,747 calls, of which 6,043 go script to script. Of the 17,704 native calls, 2,347 are bound.
The 255 gameplay scripts make 2,472 native calls, and 690 of those are bound. There were 0
unparsed names. Unknown codes: `j` ×32 and `[` ×2. The top unbound gameplay natives were
the `Test.*` checks (about 530 calls), Yield (66, not counted as bound then), and
`DirectControlCombined.UpdateControlSeatOutput` (22).

Part 2 changes from that run:
- `j` is decoded as a 64-bit integer (INFERRED from `ShiftL__ji` and `MakeAgeString__j`).
- `[` marks an array of the next code (`DoInit__[Q12TweakSetting`).
- Yield counts as bound.
- Test scripts get their own bucket, out of gameplay.
- The `Test.*` natives are bound, so the game's own test scripts can check the VM
  (`script_tests` below).

PC run after part 2 (D137, unit: call instruction):
- **Coverage:** 2,970 of 17,704 native calls are bound. In gameplay, 756 of 1,935 calls
  across 254 scripts are bound. In the 1 test script, 524 of 537 are bound. No unknown
  codes remain.
- **`script_tests`:** 47 functions ran and 24 finished. Stops: Type 15 (mostly
  `SET_V4_X` and `GET_OBJ_MEMBER`), Unbound 3 (all `Test.Succeed`), Native 2, Register 2,
  Index 1.
- **Checks:** 532 passed and 3 failed. The failures are `NeverEvaluated` at test.cod:438
  and `TestS64` at :1551 and :1562.

Part 3:
- `Test.Succeed` is bound.
- Non-static test functions now run on a fresh instance placed in register 0 as `this`,
  the same `start_registers` setup the U12 Gooey host uses.
- This PR carries the same VM change as #34: `start_registers`, zeroed v4/m44 on a fresh
  register, and a typed default for a Void native result. It is identical text, so
  merging #34 later stays clean.

How a stage 4 binding states its evidence: a comment above the table entry gives the
handler and reader addresses in the 01.33 executable, the declared signature from the
script tables, what was FOUND there and what is INFERRED, and what is not modelled. A
native that only returns a default is never in the table; a native that is in the table
but whose effect is only a record is listed in `RECORDED_ONLY` and counted apart
(`native_calls_recorded_only` in the `level_scripts` report).

## Evidence

CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80 `cwlib/enums/BuiltinType.java` lists
V2, V3 and V4 as consecutive builtins, all four floats natively.

INFERRED (each is a place real scripts or RPCS3 can disagree):

| Rule | Choice | How it gets settled |
|---|---|---|
| Mangling | `b c i f` bool/char/s32/f32; `p q r` = V2/V3/V4 (following BuiltinType order); `m` m44; `g` GUID; `Q<len><name>` class or enum | `native_coverage` reports unknown codes and unparsed names; argument machine types in listings |
| Receiver | One staged value more than the signature's parameters = member call, receiver first | Arity faults on real scripts |
| `Get*__Q5Thing` | The Thing parameter is the subject, in both static and member form | Listings |
| Yield | Suspends the calling script until the next tick; returns nothing | 52 of 255 gameplay scripts call it; RPCS3 timing (X1) |
| Matrix layout | Column vectors in column-major memory order, translation at indices 12..14 as in level-file world matrices | RPCS3 comparison of a scripted transform |
| Angles | Radians | RPCS3 |
| `atan2(a, b)` | C order: `atan2(y, x)` | RPCS3 |
| `normalize3` | xyz scaled to length 1, w kept; zero vector faults | Script behaviour |
| `j` code | 64-bit integer; the Itanium C++ reading (unsigned 32-bit) is the alternative | Listings: machine type of the argument (s64 = 12) |
| `Test.*` natives | The last two parameters are message and line. `CheckEqual` on vectors compares the type's components (2, 3 or 4). `CheckSimilar(a, b, eps)` is `\|a - b\| <= eps`. A failed check is recorded and the script continues | The test scripts' own expectations |
| Switch natives | Separate fields: triggered, binary, analogue, player | Logic runtime (G-checks) |
| GUID values | Carried as s32 or pointer-sized values | Listings (`LC` before the call) |

UNKNOWN: everything else the census lists, including Gooey (menu layout, UI thread),
TutorialBase, PodComputer, the online search classes, and `Native.GetA/GetB`.

## Verification

Cloud (Linux):
- `cargo fmt --all --check`: clean.
- `cargo clippy -q -p lbp-script-vm --all-targets -- -D warnings`: clean.
- `cargo test -q -p lbp-script-vm`: 4 unit tests and 7 VM tests. They cover:
  - signature shapes, including a malformed `Q` length;
  - every table entry parsing with no unknown codes, and no duplicate entries;
  - math results (translation at 12..14, rotateZ maps +x to +y);
  - receiver split, null and missing receivers, and arity errors;
  - a world that lacks a feature saying so;
  - Yield over three ticks through `start`/`resume`, and `YieldInCall` from `call`;
  - `Math.floor` run through `Bindings`, and an unbound Gooey call counted.

Part 2 (cloud): fmt and clippy clean; `cargo test -q -p lbp-script-vm` gives 5 unit + 7 VM
tests. New coverage: `j` and `[` codes, and `Test.*` results (vector component counts,
tolerance, a mixed-type fault).

Not tested: any real script (they are on the PC only). No `World` implementation over a
real level exists yet.

## PC checks

1. `cargo run -q -p lbp-script-vm --example native_coverage -- <script dir> local-analysis/<new>.json`
   on the D133 script directory. Report the totals line, the unknown signature codes and
   the top unbound gameplay natives.
2. If unknown codes appear, list each code with an example mangled name (names only).
3. `cargo run -q -p lbp-script-vm --example script_tests -- <script dir> local-analysis/<new>.json`.
   It runs every no-argument function of each test script and reports the outcome
   counts and checks passed/failed. Report the summary line. For failed checks, report
   the first few class/function/line rows: each one is a VM or binding rule to look at.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Next

- Rt: a `World` over the level runtime. It needs the thing tree from the level reader,
  then PScript instance data as Thing members, and then the per-tick create/update calls
  with Yield tasks.
- Rs, continued: the next unbound gameplay natives from the PC coverage run.
