# Level scripts, part 18: global settings take effect (2026-10-13, D267)

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
