# Level scripts, part 2: Rookie Test create/update faults (2026-10-08)

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
