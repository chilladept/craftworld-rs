# Level scripts, part 15: tag sensors and player sensors (2026-10-13, D264)

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
