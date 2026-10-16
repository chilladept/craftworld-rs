# Level scripts, part 11: switch inputs (2026-10-13, D260)

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
