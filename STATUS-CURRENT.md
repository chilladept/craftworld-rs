# Current state (overwrite each milestone) — after D266

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D266 the viewer draws the messages (2026-10-13)
- RScript plan stage 5, PR 2. `lbp-bevy-viewer --level-scripts` loads the English base and patch
  translation tables for its level world (as `level_scripts`) and draws `LevelWorld::messages`: a plain
  box per open bubble above its anchor Thing (speaker name, text, close prompt string of
  `CLOSE_PROMPT_KEY`) and the subtitle at the bottom centre. Without tables it still runs: no prompt line.
- `lbp-runtime/src/message_view.rs` (tested in CI): `LevelWorld::message_overlay` (what is drawn),
  `display_text` (markup out), `close_prompt`, `place_bubbles`, `place_subtitle`. The viewer's
  `speech_overlay.rs` measures and paints with egui. The viewer prints a `Messages at tick N` line on
  each change; the window has a messages line.
- MEASURED: the tables' markup is seven tags in angle brackets (emphasis 1,838 places, line break 280,
  icon 299, font 68, player and slot icons 46 of 29,477 strings). Line break -> new line, icon -> its
  name in square brackets, the rest left out.
- INFERRED: the box above the Thing, pushed back inside the screen at an edge; bubbles of one Thing
  stacked; every open bubble drawn; captions and think bubbles like speech; empty bubbles and the
  level-link and wormhole prompts not drawn; the look. The prompt is text only (event 2 not modelled).
- Checked: pulsed run equal to D265 (Things stopped by a fault 4); one level seen in the window without
  taking focus (one bubble at its Thing; clamped at the upper edge from the follow camera). Not seen: a
  subtitle, a name, a prompt line, a stack. Next: PR 3, global settings applied.
  `docs/level-scripts.md` part 17.

## Before: D265 the message list (2026-10-13)
- RScript plan stage 5, PR 1. `LevelWorld::messages` (`lbp-runtime/src/messages.rs`) is the game's list
  of open speech bubbles, captions, subtitles and level-link and wormhole prompts, run by the messaging
  natives. FOUND in the EBOOT (0x3e8dc0, 0x3e4124, 0x3e8aa0, 0x3e3248, 0x3e528c, the cores 0x7382e0 ..
  0x737a00, 0x737910): manager 4 holds at most 8; a request of a type about a Thing that has one is
  dropped; a bubble without a prompt or a subtitle is written over the one of its type; a ninth clears
  the eighth; event 3 closes the messages of its Thing; no time limit; the actor name is a translation
  tag in slot types 0 and 8. The four caption and think natives are bound (212 -> 216); the ten
  messaging natives are `natives::MESSAGING`, no longer recorded only (29 -> 23).
- Read API for the viewer: `messages.open` (`Message`: kind, style, text, actor, thing, anchor, bubble
  position, flags, frame), `Message::age`, `has_close_prompt`, `LevelWorld::message_position`.
- INFERRED: a closed message leaves at once; the list keeps the order of adding; the anchor is the
  Thing's microchip, else the Thing; age restarts when written over. Not modelled: the player's close
  button (event 2), which queued bubble is shown, placement.
- MEASURED, 378 levels, 60 ticks (plain / visit / pulsed): requests 92 / 1,690 / 2,381; opened 57 /
  1,036 / 1,187; written over 35 / 621 / 1,161; dropped 0 / 33 / 33; closed 0 / 1,000 / 1,152; levels
  with a message 47 / 192 / 216. Text from a loaded table in all but 1 non-empty case; every message has
  an anchor. Things stopped by a fault stay at 4. Next: PR 2, the viewer draws the list.
  `docs/level-scripts.md` part 16.

## Before: D264 tag sensors and player sensors (2026-10-13)
- The switch pass computes the tag sensor (type 4) and the player sensor (type 3) each tick; the other
  sensors keep the stored signal. FOUND in the EBOOT (0x118d5c, 0x119910, 0x11873c, 0xd5c1c, 0x12f6f4,
  0xf9a64): the tag map by (colour index, name), rebuilt when the pass starts; a tag's signal from its
  Thing's input port 0, on when unwired; range by Radius and MinRadius in x and y; closeness; on at 0.85 of
  the Radius, off at 0.95, hold between; the BulletsRequired-th strongest tag; KeySensorMode 1 hands on the
  tag's signal; RequireAll and PlayerMode for players; inversion afterwards. `lbp-level` keeps the sensor
  fields of PSwitch and the PSwitchKey colour and name. `lbp-runtime/src/sensors.rs`.
- INFERRED: no cone test (AngleRange below 180: 1,811 player and 2,975 tag sensors) and no layer test
  (IncludeTouching 1: 212 and 489); positions are the Things' positions; the leader is the first record.
- MEASURED, 378 levels, 60 ticks: 5,627 player and 16,953 tag sensors, 19,629 tags. Sensors that differ
  from the stored signal on some tick: 184 / 5,446 (plain), 5,204 / 6,272 with a player visiting every
  trigger and player sensor. Reaction calls 262 -> 323, 283 -> 8,970 with the visit, 5,704 -> 14,346
  pulsed. Things stopped by a fault stay at 4. Next: stage 5. `docs/level-scripts.md` part 15.

## Before: D263 the natives the stage 4 runs reached (2026-10-13)
- 48 natives bound (164 -> 212), 16 of them recorded only. FOUND in the EBOOT: a pad per player record
  with the pressed / just pressed / just released rules (0x388e2c, 0x388358, 0x389400); `LAMS.IsInLams`
  as a search of the loaded translation tables (0x159c1c); the global-settings blends, start + (target -
  start) * t per setting, with the setters, the blend factor and the water getters (0xce608, 0xd9dac,
  0xd623c, 0x5ec6f8); `GetGameFrame`, `GetNumYellowHeadAvatars`, the score data type, the scoring slot
  test; `Slot.GetSlotGroup` (0x7477d8: a SlotID (0, 0) for a slot the database lacks, never null).
  Speech bubbles, prompts, game events and audio settings are kept with their arguments
  (`LevelWorld::requests`) for stage 5.
- Bug fix: a script cast (`CASTsp`) tests the Thing's script class chain; every cast passed before.
- INFERRED: both pad mode masks are 1 and the old buttons are copied at the end of a tick; the English
  base and patch tables are the loaded ones (`level_scripts` only; the viewer loads none); every setting
  starts at rest on its stored value; no off-screen indicator, no Sackbot names, no modal overlay, a
  fresh profile.
- MEASURED, 378 levels, 60 ticks, every output pulsed: unbound natives called 110 -> 88, unbound calls
  952,819 -> 766,940, recorded-only calls 17,206 -> 83,820. Things stopped by a fault stay at 4 in all
  three runs (fault kinds 2, levels with a fault 2). No input is fed to the pads yet. Next: stage 5.
  `docs/level-scripts.md` part 14.

## Before: D262 world, position, player and game state (2026-10-13)
- RScript plan stage 4. `LevelWorld` keeps what the scripts ask the engine about the level. FOUND in the
  EBOOT: PWorld `GameMode`, `Frame`, `WaterLevel` and the two global-setting arrays (serializer 0xb3f3c0);
  PPos local and world matrices, where a write moves the descendants (0xf2e80, 0xf26bc); the microchip proxy
  (0x754498); player records with score, multiplier counter, timer, source, visible flag, number and team
  (0x156b18); the global-setting priority rule (0xce554); SlotID objects (0x16cb18). 41 natives bound
  (123 -> 164), 9 of them recorded only. Fixes: part names that start with P (`Pos`), the VM's m44 element
  order (column * 4 + row), `GetWorldPosition__` returns the matrix.
- INFERRED: one local player numbered 0, there from the start; `IsVcrPaused` false in play (the files store
  IsPaused true in 220 of 236 levels); the frame steps at the end of a tick; the proxy is the outermost chip;
  the level's own slot is 0 / 0.
- MEASURED, 378 levels, 60 ticks: Things stopped by a fault 597 -> 4 (plain), 598 -> 4 (trigger visit),
  1,794 -> 4 (every output pulsed); fault kinds 11 -> 2; levels with a fault 221 -> 2; reaction calls
  240 -> 262 (4,397 -> 5,704 pulsed). Left: `Thing.GetCOM__` (3 Things) and one null Thing in the Pod.
  Next: stage 5, show what the scripts do. `docs/level-scripts.md` part 13.

## Before: D261 switch network and reactions (2026-10-13)
- RScript plan stage 3b. `lbp-runtime/src/switches.rs`: each tick `LevelWorld::update_switches` computes
  every switch output in wiring order and applies it to its targets (FOUND in the EBOOT: 0x1244a4, 0x11ddf0,
  0x11bd58, 0x3ed2c, 0x409dc). Computed: microchip, circuit board, NOT, AND, OR, XOR, battery, direction,
  sign split, toggle, selector, counter, timer, inverted forms, microchip off. Stored signal kept: sensors,
  randomiser, sequencer boards. `SwitchPerformReaction__b` / `__bf` are called on the 16 script GUIDs the
  game lists.
- MEASURED, 378 levels, 60 ticks: 24,612 of 144,608 outputs differ from the stored value; 204 `__b` and
  36 `__bf` calls (3,052 and 1,345 with `--pulse-switches`); stopped Things 582 -> 597; 19 more unbound
  natives reached. Next: stage 4, player and game state. `docs/level-scripts.md` part 12.

## Before: D260 switch inputs (2026-10-13)
- RScript plan stage 3a. PSwitch is kept by `lbp-level`; `LevelWorld::wire_inputs` builds each Thing's
  input table from the switch outputs' targets (INFERRED build; table and readers FOUND in the EBOOT:
  0x7541e0, 0x754308, 0x7542a4, 0x75437c, 0x754ccc, 0x754244, 0x754af8). `IsSwitchTriggered__i` is true
  when an output is wired to the port, whatever its signal. 5 natives bound (123): the ternary form,
  `GetInput`, `GetOutput`, `Native.GetInThing`, `BaseWorld.IsScoreBoard`.
- MEASURED, 378 levels: 141,572 PSwitch Things, 177,800 targets, no port collision, 3,732 scripted Things
  with a wired port. Things stopped by a fault 1,722 -> 582; with the trigger visit unbound natives
  reached 94 -> 91. `docs/level-scripts.md` part 11.

## Before: D259 trigger events (2026-10-12)
- RScript plan stage 2. PTrigger is kept by `lbp-level`; `LevelWorld::update_triggers` tests the players
  against every trigger and `LevelScripts::tick_level` calls `Enter__Q5Thing`, `Inside__Q5Thing`,
  `Leave__Q5Thing`. FOUND in the EBOOT (0xe3bc0, 0x1364f0, 0x135fe0, 0x135a18, 0x1346fc): position in the
  trigger Thing's space; circle, box or sphere of the radius multiplier, times the hysteresis multiplier
  while something is inside; layer rule by the z span of Sackboy's body; Enter/Leave to the Thing's script
  or its parent's, Inside to the parent's. 2 natives bound (118). Viewer: `--level-scripts` with
  `--play-sackboy` feeds the Sackboy to the triggers (his z span is a PLACEHOLDER).
- MEASURED, 378 levels: 23,990 triggers, 4,662 scripted. `level_scripts --all --play=60 --visit-triggers`:
  11,430 Enter, 895 Inside, 7,357 Leave calls run; stopped Things stay 1,722; unbound natives reached
  65 -> 94. Next: stage 3, switch events. `docs/level-scripts.md` part 10.

## Before: D258 scripts run past unbound natives (2026-10-12)
- RScript plan stage 1 (`docs/rscript-plan.md`). An unbound native returns its type's default and is counted
  (`Bindings::default_unbound`; viewer and `level_scripts --play`; off in tests). `level_scripts --all
  --play=N` reports bound and unbound calls and the faults left over every level. 19 natives bound (116).
- MEASURED, 378 levels, 60 ticks, 13,547 scripted Things: stopped by a fault 4,940 -> 1,722; 76,269 calls
  to 64 unbound natives defaulted. Left: switch inputs 1,143, `PWorld.GameMode` 220, local transforms 199,
  TriggerLevelLink 113. Next: stage 2, trigger events. `docs/level-scripts.md` part 9.

## Before: D257 ledge-fall jump layer weight (2026-10-12)
- MEASURED on three RPCS3 ledge falls (lower-body vertex fits): the jump layer eases in with the 0x20a5f4
  follower from 0, peak += 0.125 (|vy| - peak), weight clamp(peak/10) times the blend-in; full weight about
  10 ticks after the edge (ours stopped near 0.77). w error 0.064 (D234 ramp 0.276). Landing tail at full
  weight. INFERRED: the follower holds under the landing tail. `docs/d250-air-lean-turn.md` (D257).

## Before: D256 standing foot placement (2026-10-12)
- FOUND in the EBOOT (foot pass 0x210dd8): standing (ground-relative speed under 1), each Foot joint is put
  on the floor point nearest its leg anchor, 25 to either side of the body, by a two-bone solve; in over 3
  ticks, out over 5. `FOOT_PLANT_*`, `Sackboy::foot_plant`, applied in `blend_frame`. MEASURED: foot spacing
  43.6 (game 43.4, was 51.9), standing rms 0.79 (was 1.30). `docs/d250-air-lean-turn.md` (D256).

## Before: D255 stick response, shortest jump, mirrored morphs (2026-10-12)
- FOUND in the EBOOT (0x38a578, 0x38aa10): stick x = (byte - 128) / 128 unclamped, response past the dead
  zone rescaled and squared, times both speed caps. Full right is 0.9778: MEASURED 29.34 / 22.00 to the
  right against 30.00 / 22.50 (Moon and Rookie Test). `stick_response`, `STICK_FULL_RIGHT`.
- MEASURED (d211): one- and two-tick presses peak at 33.1 and 90.4; `JUMP_MIN_TICKS` 1.
- Mirrored morphs applied (FOUND 0x1e42b4). Arm terms read: 0 without the arm buttons, no change.
  `docs/d250-air-lean-turn.md` (D255).

## Before: D254 mirrored poses (2026-10-12)
- FOUND in the EBOOT (pose mirror 0x1e42b4): a mirrored layer takes each bone's partner's local transform
  through one of six rules picked by the skeleton's mirror flip type; the zip-pull bones are copied, not
  reflected. `Rig::mirrored_trs` does the same. MEASURED on the left-to-right turn: vertex rms 2.3..4.2 ->
  0.5..1.4 on the turn frames, mirrored walk 0.7..1.6. `docs/d250-air-lean-turn.md` (D254).
- Walk start: MEASURED, no change needed. The walk phase is a free-running counter in the game too (phase
  at the first moving tick 0, 0, 3, 4, 4 over five captures); at equal phase our start fits at 0.9..1.7.

## Before: D250 jump air lean, D251 turn (2026-10-12)
- Air lean: FOUND in the EBOOT and applied. In the air and through the landing tail the upper body (Spine1
  and below, without the head) is lerped back toward stand frame 0 by s (1 - s), s easing 0.1 per tick
  (`AIR_EASE`, `Blend::UpperBody`). MEASURED on d234 `jump.bin`: Spine1 pitch within 0.05 degrees of the game
  on all flight ticks (was up to 5.5 off).
- Turn: first game captures of a direction reversal (`local-analysis/d252-turn/`, story level, flat). Pose,
  speed and clip timing matched already. Fixed: facing through the turn follows the speed angle (the
  72-degree hold is for the stop and lane change only), and the walk restarts on frame 1. Vertex rms through
  the turn 1.9..3.8 (was 48..98). `docs/d250-air-lean-turn.md`.

## Before: D249 short legs after a landing (2026-10-12)
- Two-bone leg solver (`BlendPose::move_chain_end`); `LANDING_SQUASH` and a twelve-tick `LANDING_DIP`,
  within 0.4 of the game on touchdown +3..+8 (one standing jump). `docs/d241-landing-dip.md` (D249).

## Before: D248 crouch spring found, not the landing squash (2026-10-12)
- `stretch_crouch` spring (EBOOT 0x2011d4) is in `sackboy_anim.rs` (`CrouchSpring`, tested) but unfed; as a
  landing kick it does not fit. Idle, steady walk and stop match the game within 0.4 deg lean / 0.4 height.
  No extra in-air body layer in the EBOOT. Vertex dumps log `sackboy pose N`; the stand phase at a script
  tick varies between viewer runs (+-1 height).

## Before: D246 idle stretch fixed at the cause (2026-10-12)
- The stretch was scale inheritance, not the clip weight: the stand clip breathes by scaling the spine bones
  and our skinning passed that scale to every bone below (legs, head, arms). `lbp-preview` skinning has
  `ScaleRule::Biped` (a `Bip...` bone keeps its parent's scale out of its own shape; its offset still moves),
  used by the Sackboy play figure only; `STAND_BASE_WEIGHT` is 1 again. Clean D234 captures: standing height
  per tick 201.1..206.4, ours 200.8..206.1 (RMS 0.34, was 2.5); 6-tick jump heights within 2.3 for ticks
  1..20, feet within 1. The D243 idle capture used for D242-D244 is not the neutral stand (do not tune
  against it). Open: landing dip after touchdown (game 195, ours 200), stop/turn
  poses, lean. `docs/d242-idle-stretch.md` (D246).

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- Mesh census 01.33 (4,261 GUIDs, after D188-D190): 4,259 viewable, 4,239 all-diffuse by
  default, 0 Complete. Files `local-analysis/b177-{default,legacy}.json`.
- Texture census 01.33: all 23,247 hashes decode (top mip, decodability only).
- Levels 01.33: 398 resources; lbp-level walks 236 in-band, Toolkit parity 236/236.
- D233: +0 newly viewable / +0 newly Complete (runtime feature).

## Open blockers
- Foot placement on slopes and steps (the standing spot is fixed in model space, not the contact point).
- Landing: only a standing jump is captured (ledge fall, running jump: same tables, unchecked). Upper arms
  2 to 3 degrees off late in the flight, unattributed (the arm terms are 0 there).
- Head-top bones 22/23/24/27/28.
- After a landing the body rests 0.5 to 1.5 above the spawn height (shape cast distances).
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; arm buttons (the arm clips); face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Emote clip frames vs game.
2. Popit: compare more frames, cursor mode, inventory grid pages (submode 11).
3. Camera lead, slopes (paused by the owner).

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu). Build trees are
  ~11 GB: reuse `target-pr*` through `CARGO_TARGET_DIR`.
- Git: remote origin github.com/chilladept/lbp2-asset-explorer (private), default-deny
  `.gitignore`, CI `.github/workflows/ci.yml`. Standing rules: CLAUDE.md.
- Native QA windows hidden or on the non-primary monitor; never steal focus.

## Where detail lives
Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`. Earlier milestone
docs under `docs/` (grep headings); format evidence `docs/formats.md`; rules `AGENTS.md`.
