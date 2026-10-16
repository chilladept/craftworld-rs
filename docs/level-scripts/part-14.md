# Level scripts, part 14: the natives the stage 4 runs reached (2026-10-13, D263)

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
