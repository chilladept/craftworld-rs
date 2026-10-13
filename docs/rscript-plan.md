# RScript interpreter: plan to real levels

2026-10-12 America/Chicago. Goal (user, 2026-10-13 decision card): run the game's own
level scripts in the Rust + Bevy app so levels start to play. Scripts are interpreted in
our VM (user decision 2026-10-08), not replaced by native code. This plan follows
`docs/script-vm.md`, `docs/script-natives.md` and `docs/level-scripts.md` (parts 1-8).

## Where it stands

- The VM runs all 203 opcodes and passes 512 of 513 checks of the game's test script.
- `lbp-runtime` runs `OnCreate__`, then per tick resumes yielded scripts and threads and
  calls `Update__` (or a class's frame entry). The viewer does this at 30 Hz with
  `--level-scripts`.
- 99 natives are bound. An unbound native faults the script and its Thing stops.
- Nothing a script changes is drawn, and no engine event other than create and update
  reaches a script.

## Measured gap (2026-10-12)

Static census over the 49 script classes that the 378 readable levels use (13,547
scripted Things, D150), following script-to-script calls from the entry functions.
Unit: distinct native (declaring class, mangled name) without a binding. Names and
counts only; the listings stay in ignored `local-analysis/`.

| Reached from | Unbound natives |
|---|---|
| `OnCreate__`, `Update__`, `.init__`, `.ctor__` | 272 |
| every function of the classes in use | 583 |

By declaring class, create/update reach: Game 75, Thing 39, Native 16, Audio 15, Gooey
13, Score_Board 13, GameMessagingManager 11, Math 10, String and StringA 10, HUD 6.

Which classes matter (instances / levels; unbound natives reached from create/update):

| Class | Instances / levels | Unbound |
|---|---|---|
| PostItNote, Launcher, TriggerMusic, SmokeMachine and 14 more Tweakable classes | 7,900+ / up to 173 | 1 each: `Resource.LoadTextureByFilename__gb` |
| Trigger_Speech_Bubble | 1,469 / 161 | 60 |
| TriggerGlobalSettings | 1,180 / 135 | 18 |
| ScoreTweak | 1,106 / 48 | 22 |
| WorldMooCamera (the world script of 220 levels) | 220 / 220 | 37 |
| Score_Board | 102 / 101 | 80 |
| TriggerLevelLink | 113 / 103 | 30 |
| Pod | 1 / 1 | 31 |

The event entries the classes define, which nothing calls yet (unit: distinct class of
49): `Enter__Q5Thing` 18, `Render__` 18, `SwitchPerformReaction__b` 17, `Leave__Q5Thing`
11, `OnDestroy__` 8, `Inside__Q5Thing` 3. `UpdateUI__` (23) is the Popit tweak menu and
is out of scope here.

Reading: most scripted Things do nothing on create or update. They wait for a trigger
or a switch. So natives alone will not make a level play; the events are the larger
gap, and the natives a class needs can only be ranked honestly once its events fire.

## Stages

Each stage is one PR, with a PC run of `level_scripts --all` before and after.

1. **Run past unbound natives, and count them at run time.** An unbound native returns
   the default of its declared return type and is counted instead of stopping the Thing
   (a mode of `Bindings`, on for the viewer and `--play`, off in the VM tests).
   `level_scripts --all --play=N` then reports, over every level, calls made per
   unbound native and the faults that remain. Bind the pure ones in the same PR:
   String and StringA (length, find, substring, upper case, empty test, conversion),
   the 10 Math natives, and `LoadTextureByFilename__gb`.
   Result: a ranked list from real runs, and create/update finishing on most Things.
2. **Trigger events.** `Enter__`, `Leave__` and `Inside__` from the PTrigger part
   against the player Sackboy in the viewer's play mode (radius and layer rule from the
   executable, through the Ghidra bridge). This wakes speech bubbles, level links,
   collect triggers, music and global settings.
3. **Switch events.** `SwitchPerformReaction__b` and the switch-input natives from the
   level's switch and logic parts. Needs the logic network evaluated each tick; scope
   it from the PSwitch data first.
4. **Player and game state.** Players, the player's Thing (`GetYellowheadFromPlayerNumber`),
   score and multipliers, pause and mode queries: the Game group that ScoreTweak,
   WorldMooCamera and Score_Board need.
5. **Show what scripts do.** Speech bubbles and subtitles (GameMessagingManager) in the
   viewer UI with LAMS text, global settings (lighting, fog) applied to the scene,
   world transforms and tints applied to drawn Things, the script camera.
6. **Pod.** `Thing.FindThing`, the Pod world script and PodComputer threads, video and
   voice-over natives as recorded no-ops, up to the Pod intro sequence.

Audio natives stay recorded no-ops until there is an audio system. Online, Popit tweak
UI (`UpdateUI__`) and photo natives are not planned.

## Progress

- Stage 1 done (D258, `docs/level-scripts.md` part 9): over 378 levels at 60 ticks, Things
  stopped by a fault 4,940 -> 1,722 of 13,547; 76,269 calls to 64 unbound natives are
  defaulted and counted. Largest stops left: switch inputs 1,143, `PWorld.GameMode` 220,
  local transforms 199, TriggerLevelLink 113.
- Stage 2 done (D259, part 10): PTrigger kept by the reader; the inside test, hysteresis,
  layer rule and the Enter/Inside/Leave targets FOUND in the executable. 23,990 triggers
  in 378 levels, 4,662 with a script on their Thing or its parent. With a player visiting
  each trigger: 11,430 Enter, 895 Inside and 7,357 Leave calls run, no new fault, 29 more
  unbound natives reached (switch inputs and outputs lead). The viewer feeds the
  play-mode Sackboy to the triggers.
- Stage 3a done (D260, part 11): switch inputs. PSwitch kept by the reader; each Thing's
  input table built from the outputs' targets (INFERRED; 177,800 targets, no port
  collision); the switch-input natives follow the rules FOUND in the executable, and
  `GetSwitchActivationTernary`, `GetInput`, `GetOutput`, `GetInThing` and `IsScoreBoard`
  are bound (123). Things stopped by a fault 1,722 -> 582.
- Stage 3b done (D261, part 12): the switch network runs each tick in wiring order with the
  output rules FOUND in the executable for the logic types (gates, battery, toggle,
  selector, counter, timer, microchip and board, microchip off); sensors and the
  randomiser keep the stored signal. `SwitchPerformReaction__b` / `__bf` are called by the
  game's edge rule on the 16 script GUIDs the game lists. 378 levels, 60 ticks: 24,612
  outputs change, 204 + 36 reaction calls run (3,052 + 1,345 with every output pulsed),
  stopped Things 582 -> 597, 19 more unbound natives reached. Still open: sensors.
- Stage 4 done (D262, part 13): the level world keeps the PWorld values (game mode,
  frame, water level, the global-setting arrays), local and world matrices that move the
  descendants, player records (score, multiplier, team) and the global-setting priority
  rule, all FOUND in the executable; 41 natives bound (123 -> 164), 9 of them recorded
  only. 378 levels, 60 ticks: Things stopped by a fault 597 -> 4 (1,794 -> 4 with every
  output pulsed), fault kinds 11 -> 2, levels with a fault 221 -> 2. INFERRED: one local
  player numbered 0, `IsVcrPaused` false in play, the level's slot (0). Still open:
  pad input, the game frame native, the Sackbot group, the level's slot.
- D263 (part 14): the 48 natives the stage 4 runs reached in most levels (164 -> 212, 16
  of them recorded only). Pads per player record with the pressed, just pressed and just
  released rules, `IsInLams` against loaded translation tables, the global-settings
  blends (start, target, t), the game frame, the slot group of the empty slot, all FOUND
  in the executable; no input is fed yet. Speech-bubble, prompt and audio-setting requests
  are kept with their arguments for stage 5. A script cast now tests the Thing's class.
  Pulsed run: unbound natives called 110 -> 88, unbound calls 952,819 -> 766,940; Things
  stopped by a fault stay at 4. Still open: feeding the pads, the slot database, the
  Sackbot group.
- D264 (part 15): the tag sensor and the player sensor are computed each tick by the rules
  FOUND in the executable (tag map refreshed at the start of the pass, range, closeness,
  the on / hold / off thresholds, the N-th strongest tag, RequireAll, inversion
  afterwards); the reader keeps the sensor fields and the PSwitchKey. 5,627 player and
  16,953 tag sensors in 378 levels. With a player visiting every trigger and player
  sensor: reaction calls 283 -> 8,970 (5,704 -> 14,346 pulsed), Things stopped by a fault
  stay at 4. INFERRED: no cone test (4,786 sensors with AngleRange below 180) and no
  layer test (701 with IncludeTouching 1). Still open: those two, the other sensors, the
  randomiser, sequencers.
- Stage 5 begun, PR 1 (D265, part 16): the level world keeps the game's message list.
  The messaging natives open and close speech bubbles, captions, subtitles and the
  level-link and wormhole prompts by the rules FOUND in the executable (a duplicate is
  dropped, a bubble without a prompt or a subtitle is written over, 8 at most, event 3
  closes the messages of its Thing, no time limit); the caption and think natives are
  bound (212 -> 216). Pulsed run over 378 levels: 2,381 requests, 1,187 messages opened,
  1,161 written over, 33 dropped, 1,152 closed, a message in 216 levels; Things stopped
  by a fault stay at 4. INFERRED: a closed message leaves at once, list order, the
  anchor Thing. Follows: PR 2 the viewer draws the list (tables loaded, bubbles at the
  anchor, subtitles), PR 3 global settings applied, PR 4 hidden, moved and tinted Things,
  PR 5 camera and screen natives. Still open: the player's close button.
- Stage 5, PR 2 (D266, part 17): the viewer draws the message list. `--level-scripts`
  loads the English translation tables for its level world and shows each open bubble
  as a plain box above its anchor Thing (speaker name, text, close prompt line) and the
  subtitle at the bottom centre. `lbp-runtime/src/message_view.rs` decides what is drawn,
  takes the markup tags out of the text (seven tags, MEASURED on the tables) and places
  the boxes (kept on the screen, bubbles of one Thing stacked), with tests in CI.
  INFERRED: placement, look, that every open bubble is drawn. The pulsed run is equal
  to D265 (Things stopped by a fault 4). Follows: PR 3 global settings applied, PR 4
  hidden, moved and tinted Things, PR 5 camera and screen natives. Still open: the
  player's close button, the game's bubble placement and art.
- Stage 5, PR 3 (D267, part 18): global settings take effect. FOUND: the PWorld's
  per-frame function calls `ApplyGlobalSettings__` on the scripted Thing of each
  global-settings slot, after the switch pass, and clears a slot whose Thing is gone;
  the script steps its blend factor there. `LevelScripts::apply_global_settings` does
  that each tick, `WorldState::settings_in_force` hands out the values, and the viewer
  lights a played level again when lighting or darkness changes (the P3m preset rule).
  Over 378 levels the settings in force now differ from the level file in 48 / 81 / 96
  levels (plain / trigger visit / pulsed; 0 before), 4,562 / 80,255 / 80,631 calls;
  Things stopped by a fault stay at 4, no new unbound native. INFERRED: the call's
  place after `Update__` (the engine's `Update__` call was not found). Not applied:
  fog, fog tint, colour correction, gravity, water. Follows: PR 4 hidden, moved and
  tinted Things, PR 5 camera and screen natives.
- Stage 5, PR 4 (D268, part 19): hidden, tinted and moved Things. FOUND: hide and show
  clear and set two bits of the PRenderMesh's visibility byte, down the Thing's children
  with a mesh and no shape; bit 0 is "visible in play mode" (the game's own native of
  that name reads it); the colour setters store packed colours and do nothing for a
  null Thing or a missing part. `lbp-runtime/src/display.rs` keeps a dirty list
  (`take_display_dirty`: visibility, colour, world matrix) with the values now and at
  load. Natives 216 -> 219 (show, shape tint, the play-mode question); hide and the
  mesh colour are no longer recorded only. The viewer maps each Thing to the draw
  entities of its own model (`thing_view.rs`) and hides, tints and places them; a mesh
  that is not visible in play mode is hidden. Shapes are merged into shared meshes and
  are not followed. Over 378 levels: 12,106 meshes in 218 levels are stored as not
  visible in play mode; the scripts change no visibility byte, one mesh colour in 10
  Things, tints in 102 (101 of them white with an alpha, not drawn), shape colours in
  102, and no drawn Thing moves. Things stopped by a fault stay at 4. INFERRED: that a
  mesh without bit 0 is not drawn, the tint as a multiply. Follows: PR 5 camera and
  screen natives.
- Stage 5, PR 5 (D269, part 20): the script camera and the screen. FOUND: the level's
  camera keeps one override (position, direction, blend, target, rate);
  `SetOverrideCamera` stores the pair, `SetOverrideCameraActivation` the target and the
  rate; each camera update the blend steps linearly, is eased with half a cosine, and
  the eye is mixed linearly and the direction on the arc from the follow camera; the
  lens is then 60 degrees, near 10, far 40000; only a script ends it. `GetCameraMatrix`
  is camera to world; `GetScreenSpacePosition` gives pixels from the top left of a
  1280 x 720 screen, with no test for a point behind the camera. New
  `lbp-runtime/src/camera.rs` (`OverrideCamera`, `CameraView`, `step`, `ease`,
  `blend_pose`, `screen_position`, `LevelWorld::step_camera` and `camera_pose`). Natives
  219 -> 225. The viewer hands its follow camera in each frame, draws with the mixed
  camera while the override is in, and writes the camera it drew with for the scripts.
  Over 378 levels: unbound natives called 51 -> 48 / 87 -> 81 / 87 -> 81, unbound calls
  73,325 -> 62,000 / 1,002,445 -> 865,777 / 1,010,529 -> 872,549; the override comes in
  on 101 levels with the trigger visit (0 plain); screen positions are asked in 28 / 61
  / 70 levels; Things stopped by a fault stay at 4. INFERRED: world +Y as up, the step's
  place at the end of the tick, the answers when nothing draws (the default follow pose,
  the middle of the screen). Not bound: `SetSpeechBubbleActive` (its part and effect are
  not established). Stage 5 is done with this PR; follows stage 6, the Pod, with
  `SetCameraFromToUp`.
- Stage 6 begun, PR 1 (D270, part 21): script names, the pod Thing, the avatar. The Pod
  is two levels (the shell and the planets view); the runtime plays each alone. FOUND:
  `Thing.FindThing` compares a Thing's script name with the argument byte for byte, case
  sensitive, first match wins, null with none; `Thing.GetPodThing` gives the first Thing
  whose render mesh is one of two pod meshes (the platform only when asked for it);
  `Game.GetAvatarFromPlayerID` is the player's Sackboy; `SetRenderMeshMesh` sets a
  PRenderMesh's mesh from a mesh resource. `lbp-level` keeps the PScriptName's name
  (it kept the length). New `lbp-runtime/src/pod.rs`; a mesh change reaches the display
  list. Natives 225 -> 238, 2 of them recorded only. Over 378 levels: Things stopped by
  a fault 4 -> 3 in all three runs (fault kinds 2 -> 1, levels with a fault 2 -> 1):
  the Pod's world script gets its pod Thing and is resumed every tick; the three left
  are `Thing.GetCOM__` (TriggerAirStone, one level). Unbound natives called 48 -> 44 /
  81 -> 78 / 81 -> 78, unbound calls 62,000 -> 61,483 / 865,777 -> 865,266 / 872,549 ->
  872,042. 173 Things in 66 levels have a script name; the one `FindThing` call finds
  its Thing. INFERRED: the two PWorld lists are the PScriptName and PRenderMesh parts in
  Thing order; a null string finds nothing. Not bound (effect or answer not
  established): `SetPodTransitionTexture`, `CurrentLevelIsOurPod`, `GetPlayerColour`,
  `GetIsInPadZone`, `IsJustClickedByID`. Follows: PR 2 video, voice-over and
  progression flags (the intro ends), PR 3 the viewer's Pod camera.
- Stage 6, PR 2 (D271, part 22): video, voice-over and progression flags. The planets
  level's intro thread never ended: the unbound `Video.StartVideo` answered 0 and the
  script takes any video handle but -1 as "a video is playing". FOUND: `StartVideo`
  answers -1 when no video starts; `RequestVoiceOver` answers false when no line
  starts and `IsVoiceOverPlaying` is false while none plays; the two flag natives take
  a bit mask on one 32-bit word of the profile (set is an OR, the test wants every
  bit); the mix overlay natives answer false without an audio system. Nothing plays:
  a video or voice-over request is kept with its arguments (`LevelWorld::media`, new
  `lbp-runtime/src/media.rs`) for a later video or audio system, and the script gets
  the game's answer for "nothing started". `LevelWorld::progression_flags` is one
  `u32` (it was a set of numbers); `--flags` takes masks. Natives 238 -> 249. The
  intro now ends on tick 452 (451 ticks of zoom), sets the mask 3 and is not started
  again; no unbound native is called in the planets level. Over 378 levels: Things
  stopped by a fault stay 3 / 3 / 3; unbound natives called 44 -> 41 / 78 -> 73 /
  78 -> 73, unbound calls 61,483 -> 61,480 / 865,266 -> 862,986 / 872,042 -> 868,663.
  INFERRED: `HasPlayedLBP1AndUnlockedPod` / `Moon` false (no save data of the first
  game). Not added: the engine's per-tick `PodComputer.UpdateVideo__` call (FOUND who
  calls it, a function reached only through a table slot; not found who calls that
  slot and when). Not bound: `Video.PressedStartButton__`. Follows: PR 3 the viewer's
  Pod camera, PR 4 the pod computer is entered.
- Stage 6, PR 3 (D272, part 23): the viewer shows what the planets level's script does.
  `Cam.SetCameraFromToUp` was bound and kept since part 6; FOUND: it writes the render
  camera directly (eye, point, up; near, far, one value not read, field of view in
  radians), no blend. The runtime now says which camera a frame is drawn with
  (`LevelWorld::display_camera`): a camera a script wrote on the last tick, outright;
  else the follow camera under an override; else the display's own. The viewer puts
  its camera there (`level_play.rs`), and with `--pod-camera N` hangs the planets level
  under a root that follows the script-moved world Thing
  (`LevelWorld::world_thing_moved`). No native is new (249). Over 378 levels the runs
  equal D271 (Things stopped by a fault 3 / 3 / 3, unbound natives called 41 / 73 / 73);
  one level writes the camera and none has a written camera and an override at once.
  Captures of the intro at ticks 30, 200, 440 and at rest (tick 452 on: eye (0, -250,
  1130), 57.3 degrees). INFERRED: a written camera stands for one tick and goes before
  an override; the field of view is vertical; the whole level is drawn under its world
  Thing (measured before in the running game, the engine code not read). Left out:
  the pod shell layer under the script's camera (with `--pod-shell` the viewer keeps
  its own Pod camera), Earth and Moon by their own scripts, the pod's mesh swap in the
  scene (counted), depth of field and fog. Follows: PR 4 the pod computer is entered.
- Stage 6, PR 4 (D273, part 24): the pod computer is entered. Read first, in the
  executable: the six natives of the path and who sets "in use". FOUND:
  `GetIsInPadZone` is "our Pod, the primary player, in the list of a trigger whose
  script passes a class test, world z >= -300"; `IsJustClickedByID` takes an action
  number (8: context mask 0x1001, bit 0x80 of the pad word, newly down), the string is
  only the prompt's key; `GetPlayerColour` unpacks a stored 32-bit colour of the
  player; `CurrentLevelIsOurPod` is a byte set when a level is entered from one slot;
  `PressedStartButton` uses up bit 0x80000 of the first pad that has it;
  `RequestHostUsePod` only stores and sends the request, and the receiver of the host's
  message sets "in use" with a start-up or shut-down sound. Runtime: those rules
  (`pod.rs`, `PadState`, `PlayerRecord::colour`), the request answered at the start of
  the next tick, and one session for several levels (`LevelWorld::hand_session_to`,
  `tick_session`: flags, "our Pod", "in use", media requests). Natives 249 -> 255.
  `level_scripts --with-level` plays the two Pod levels together: flags 3 on round 452,
  Sackboy in the pad zone from 460, press on 470, request on 470, in use on 471, the
  change-over 0.05 a tick to 1.0 on 490 where the pod's mesh becomes GUID 145753, the
  planets camera at rest on its in-use preset (0, -300, 900) on 496. Over 378 levels:
  Things stopped by a fault 3 / 3 / 3; unbound natives called 41 -> 40 / 73 -> 71 /
  73 -> 71; unbound calls 61,480 -> 61,419 / 862,986 -> 862,919 / 868,663 -> 868,592.
  INFERRED: the trigger class is `Trigger_Pod`; one local player is primary, host and
  owner; the answer comes on the next tick; the pad bit numbering of Start.
  PLACEHOLDER: the stored colour starts opaque white (the game's value was not found).
  The tool starts a level as our Pod when it has a pod Thing (a stand-in; the game
  tests the slot). Left out: the viewer (one level there still), the pod computer's own
  scripts and menus, the other 98 actions. Stage 6 is done as planned.

## Rules

- Each native binding states its evidence: declared types from the script tables, use in
  scripts, and the executable through the Ghidra bridge (2,662 script natives named).
  Decompiled code is evidence only and stays in ignored `local-analysis/`.
- Default-returning natives are never counted as bound. Reports keep three numbers:
  bound, defaulted, faulted.
- No script bytecode, listing, constant or game asset is committed.

Counts: +0 newly viewable, +0 newly Complete (plan only).
