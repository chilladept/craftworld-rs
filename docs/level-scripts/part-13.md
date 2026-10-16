# Level scripts, part 13: world, position, player and game state (2026-10-13, D262)

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
