# Current state (overwrite each milestone) — after D276

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D276 the pod's mesh swap in the scene (2026-10-13)
- "Pod picture" PR 3, viewer and scene builder (`docs/level-scripts.md` part 27). The runtime reported
  the mesh a script puts on a PRenderMesh and the viewer counted it as not drawn. FOUND before: the one
  such call is `Pod.SetPodTransition__b` on pod.bin Thing 2, `pod.mol` (0x2f6c) under 1,
  `pod_platform.mol` (0x23959) at 1.
- `lbp-preview`: `LevelSceneOptions::mesh_alternates`, `add_alternate_meshes`, `SceneItem::alternate`:
  a Thing drawn with one mesh of a pair gets a second item for the other (not counted as drawn, out of
  the bounds). Viewer: `thing_view::SWAPPED_MESHES` (one pair), only under `--level-scripts` and never
  for the shell layer; the second set is spawned hidden; `thing_view::apply` shows the set of the mesh
  the Thing has now, under the play-mode visibility rule (`Followed::mesh_swapped`). QA option
  `--level-scripts-in-use-at N` sets `pod_computer_in_use` when N ticks have run (stand-in for D273's
  pad zone, press and host answer; MEASURED that "in use" alone runs pod.bin's change-over).
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d276-*`): without `--level-scripts`
  pod.bin alone and the shell picture are equal byte for byte before and after. pod.bin with scripts,
  in use from 600: `Things at tick 620: ... mesh swapped 1 ... [(2, "00023959")]`; the still at 610
  equals the capture without scripts byte for byte.
- Eye check: before, the cardboard pod with the DualShock in its opening; after, a flat round cardboard
  disc where the pod's floor was, the DualShock on it. Two small orange pieces at the pod's sides stay
  in mid-air (not entities of the pod Thing; not looked up). Nothing compared against the game.
- Left out: the fade (the pod pops), the swap in the Pod picture (the shell layer has no script world).
- Tests: one `lbp-preview` and two viewer tests. fmt, CI clippy, CI tests (611), Python tests, `cargo
  check` of both viewers and the viewer's clippy pass; the viewer's tests pass (58) but
  `sackboy_play::tests::input_script_steps` (fails on main).
- Next: Pod picture PR 4, the two Pod levels in one viewer session.

## Before: D275 the Pod shell layer under the script's camera (2026-10-13)
- "Pod picture" PR 2, viewer only (`docs/level-scripts.md` part 26). With `--pod-camera N --pod-shell
  HEX` under `--level-scripts` the viewer kept its own Pod camera and said the script's was "not
  applied". Now the planets level is drawn with the script's camera there too, and the shell and the
  tent are placed from it.
- Viewer: `PodRig::layer_at` / `shell_root_at` / `tent_root_at` (`pod_menu.rs`; `layer` calls
  `layer_at`, same arithmetic). `LevelMode::pod_scripted` (`--level-scripts` with `--pod-camera N`):
  the planets level under the world Thing's root, Thing map on, with or without the shell; the menu
  constants of `--pod-game-camera` (`place_menu_planets`, `planets_root(t)`, default per-bone
  placement) are left out there. The rig has the game's field of view and the camera presets.
  `level_play::script_camera` places the shell and tent roots from the script camera's eye and turn
  and the Puter state's `t`; `pod_dolly` only moves `t`; `tick` hands the Puter state to
  `pod_computer_in_use` when it changes.
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d275-*`): without `--level-scripts`
  the shell captures before and after are equal byte for byte, with and without `--pod-game-camera`.
  With `--level-scripts` the flag `--pod-game-camera` changes nothing (equal byte for byte).
- Eye check: at rest (tick 600) the tent's window shows the Earth near (0.46, 0.48) of the frame and
  the Moon near 0.56 of the width, the framing of the `--pod-game-camera` capture without scripts,
  and clear (fog bias). Ticks 30 and 200: the intro's ceiling and wall in the window. The shell does
  not cut into the planets and the fog does not swallow the view. `--pod-open`: the camera rests at
  the in-use preset, the tent is gone; view 0, not the game's menu view. Nothing compared against
  the game.
- Seen, not fixed: under `--preview-lighting` the tent's shade changes while the camera is pitched
  (the preview light is not turned with the shell root; the default lighting turns it).
- INFERRED: the shell's size rule. PLACEHOLDER: the Puter key as "in use"; `t` is the viewer's dolly.
- Left out: the shell level's camera in the game, the tent's fade and `PodTransition`, the menu
  views, the mesh swap (PR 3), the shared viewer session (PR 4).
- Tests: two new viewer tests (`d275_layer_at_...`, `the_puter_state_is_handed_...`). fmt, CI clippy,
  CI tests, Python tests, `cargo check` of both viewers and the viewer's clippy pass; the viewer's
  tests pass but `sackboy_play::tests::input_script_steps` (fails on main).
- Next: Pod picture PR 3, the pod's mesh swap.

## Before: D274 Earth and Moon by their own scripts, the fog bias, the video update (2026-10-13)
- "Pod picture" PR 1, after RScript plan stage 6. The planets level's Earth and Moon stayed at their
  stored places (their per-frame function was not called), `--preview-lighting` washed the played
  planets level out, and the place of the pod computer's video call was not known. Natives 255 -> 257.
- FOUND in the EBOOT (scoping for this PR; `docs/level-scripts.md` part 25): the planets object's
  per-frame function (0x4381d8) calls `Update__rb(v4, bool)` on every planet (0x42115c), then the
  function that ends in the world script's `UpdatePlanetsView`. The vector (0x421708) is a point of
  the planet to turn toward the camera; the bool is a byte a true result sets ("arrived"). The script
  puts the planet at its class's constant place (Earth (-1250, 0, 0), Moon (2000, 0, 0)) with the axis
  lengths it was created with, turned toward the point. `Math.transform` (0x5ee288): matrix times
  vector; `Math.ModAngleToWithinPI` (0x5eebb0): the angle less whole turns. The third `SetDOFAndFog`
  value is a fog bias: the Pod's measured fog start 25810 is the level's fog near 5810 plus 20000.
  `PodComputer.UpdateVideo__` is the last step of the pod computer's draw function (0x404598), called
  once a rendered frame after the scene (0x31e820).
- Runtime (`lbp-runtime/src/lib.rs`, `lbp-script-vm`): frame entries for `Earth` and `OrbitingMoon`
  (`PLANET_UPDATE`), `LevelScripts::planets` (`PlanetFrame`: point, arrived, calls),
  `set_planet_point`, the planets before the world script's entry; `LevelScripts::update_video`, once
  a tick after the global-settings pass (new, not moved: D271 had not added the call);
  `StepReport::planet_updates`, `planets_arrived`, `video_updates`; the two Math natives. The virtual
  call on a Thing (`CALLVsp`) now dispatches on the class of the Thing's own script
  (`Host::thing_script`): it was a plain call, and the planets' `GetPosition__` needs it.
- Viewer: under `--level-scripts --pod-camera N` without the shell layer the Thing map is on, so the
  planets' models follow their scripts inside the world Thing's root; `follow_fog` hands the bias to
  `LightingState::fog_bias` and `--preview-lighting` moves its fog out by it (`Fog::biased`). Without
  `--level-scripts` nothing changes (a capture is equal byte for byte to D272's).
- MEASURED, pod_planets alone, 600 ticks (`local-analysis/d274-planets-600.json`): Earth at (-1250, 0,
  0) with axes 0.9610 and Moon at (2000, 0, 0) with axes 2.1121 from tick 1 to 600; with the meshes'
  inverse bind scale (1.4052, 0.7312) that is the size read in the running game (P2k: 1.3505, 1.5444).
  1,200 planet calls, 600 video calls, 0 entries without a function (2 a tick before), 0 faults, 0
  unbound calls.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed; `d274-play-all-*.json`): Things
  stopped by a fault 3 / 3 / 3; unbound natives called 40 -> 41 / 71 -> 72 / 71 -> 72; unbound calls
  61,419 -> 61,420 / 862,919 -> 862,920 / 868,592 -> 868,593; native calls 573,134 -> 574,456 /
  5,365,247 -> 5,366,569 / 5,424,893 -> 5,426,259. Two levels differ: the planets level, and one
  cutscene level where a virtual call now reaches a subclass (`Gooey.StartFrameNamed`, unbound, once).
- Eye check (no focus taken; `local-analysis/rscript/stage6/shots/d274-*`): at rest the Earth's
  centre is near (0.46, 0.48) of the frame and the Moon near 0.56 of the width, the places P2k read
  from the closed Pod; the room is clear with a light haze under `--preview-lighting` (fog
  25804.6..78453.6 level units; 5804.6..58453.6 before). Nothing compared against the game side by
  side.
- PLACEHOLDER: the point while the host sets none, (0, 0, 1) (no turn; the game's idle vector was not
  read), so the side of the Earth that faces the camera is not the game's. INFERRED: the dispatch rule
  of the virtual call on a Thing; a tick is a frame and the video call is its last script call; the
  rounding of `ModAngleToWithinPI`; the argument registers of `Update__rb`.
- Left out: the planets' entries and turning; sticker decals of the planets (the scene has none: the
  viewer projects stickers on shapes only); depth of field; the bias in the default lighting's shader
  fog; the shell layer, the mesh swap, the shared viewer session (PR 2 to 4 of the Pod picture).
- Tests: `lbp-script-vm` `d274_planet_math_natives`, `virtual_calls_on_a_thing_dispatch_on_the_script_it_carries`;
  `lbp-runtime` 61 unit and 52 world tests (three new `d274_*`); two viewer tests. fmt, CI clippy, CI
  tests, Python tests pass; clippy and `cargo check` on the viewers pass; the viewer's tests pass but
  `sackboy_play::tests::input_script_steps` (fails on main).

## Before: D273 the pod computer is entered (2026-10-13)
- RScript plan stage 6 (the Pod), PR 4. The Pod's world script lets the leader engage the pod computer
  (flags, pad zone, button), asks the host for it, and then runs its change-over. Five natives of that
  path were unbound, the request was only recorded and the two Pod levels could not be played together.
  Natives 249 -> 255.
- FOUND in the EBOOT (read for this PR; `docs/level-scripts.md` part 24 has the table):
  `Native.GetIsInPadZone` (0x13bac0): our Pod, the primary player's head, the Thing in the list of a
  PWorld entry whose script passes a class test and a call by name, world z >= -300.
  `Input.IsJustClickedByID` (0x3890a0): an action number, not a pad mask; action 8 has context mask
  0x1001 and is bit 0x80 of the pad's button word, newly down; the string is the prompt's key only.
  `Native.GetPlayerColour` (0x139308): a stored 32-bit colour of the player as (R, G, B, A) / 255.
  `Game.CurrentLevelIsOurPod`: a byte written when a level is entered (0x1ab508), true for one slot id.
  `Video.PressedStartButton` (0x756520): bit 0x80000 of the first of seven pads, used up.
  `PodComputer.RequestHostUsePod` (0x3fa9cc): only in our own Pod; stores and sends the request. "In
  use" is set by the receiver of the host's message (0x3fa868 -> 0x18ed30), with a start-up or
  shut-down sound. `SetPodTransitionTexture` (0x4015e8) sets a texture on three engine resources.
- Runtime (`lbp-runtime/src/pod.rs`, `lbp-script-vm/src/natives.rs`): `LevelWorld::our_pod`,
  `in_pad_zone`, `request_host_use_pod`, `begin_session_tick` (the answer, first step of `tick_level`),
  `hand_session_to` and `tick_session` (several levels in one session: progression flags, "our Pod",
  "in use" and a waiting request, media requests, per player the colour and used-up buttons);
  `PadState::is_just_clicked`, `use_up`, `held_off`; `PlayerRecord::colour`, `colour_vector`.
  `RequestHostUsePod` left the recorded-only list; `SetPodTransitionTexture` joined it.
- `level_scripts`: `--our-pod` (else a level with a pod Thing is started as our Pod: the tool's
  stand-in, the game tests the slot), `--enter-pad-zone=TICK`, `--press=TICK:MASK,..`,
  `--player-colour=AARRGGBB`, `--with-level=PATH` (a second level in the session; `runs.session` has
  the path of events).
- MEASURED, pod.bin and pod_planets.bin in one session, 700 rounds, fresh profile
  (`local-analysis/d273-pod-session-700.json`): flags 3 on round 452; the Sackboy in the pad zone from
  460; the press and `RequestHostUsePod(true)` on 470; in use on 471 (start-up sound counted); the
  change-over 0.05 on 471 to 1.0 on 490, where the pod's mesh becomes GUID 145753; the planets camera
  leaves (0, -250, 1130) on 471 and rests at (0, -300, 900) on 496. No fault and no unbound call in
  either level. The other level order gives the same events on the same rounds.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed; `d273-play-all-*.json`): Things
  stopped by a fault 3 / 3 / 3; unbound natives called 41 -> 40 / 73 -> 71 / 73 -> 71; unbound calls
  61,480 -> 61,419 / 862,986 -> 862,919 / 868,663 -> 868,592; native calls the same. Only pod.bin
  differs. In the visit runs one Thing fewer has a shape colour unlike the file (the pod's, by the
  placeholder colour).
- INFERRED: the pad zone's list is the PTriggers and its class `Trigger_Pod` (the call by name is not
  made); the first player record is the primary player, the host and the Pod's owner; the host's
  answer comes at the start of the next tick; Start has the pad word's bit numbering; context bit 0x1
  is gameplay. PLACEHOLDER: a player's stored colour starts opaque white (the game's value and who
  writes it were not found).
- Left out: the viewer (`--level-scripts` still plays one level; nothing draws the change-over or feeds
  the viewer's Sackboy and pad in); the pod computer's own scripts and menus; the computer open on
  arrival and closed on leaving; the ownership test; the other 98 actions; one VM for both levels.
- Tests: `lbp-script-vm` `d273_pod_computer_natives`; `lbp-runtime` 61 unit and 49 world tests (three
  new `d273_*`). fmt, CI clippy, CI tests, Python tests pass; clippy on the Bevy viewer passes and its
  tests pass but `sackboy_play::tests::input_script_steps` (fails on main); its sources did not change.
  Nothing run in the viewer; nothing compared against the game.
- Next: the shared session and the change-over in the viewer, or the pod computer's own scripts.

## Before: D272 the Pod's camera and its moving world (2026-10-13)
- RScript plan stage 6 (the Pod), PR 3. The planets level's world script writes the camera itself each
  tick (`Cam.SetCameraFromToUp`) and moves its world Thing each tick. The runtime kept both and nothing
  drew by them. No native is new (249).
- FOUND in the EBOOT (scoping notes of stage 5 and 6): `SetCameraFromToUp` (0x2e9798) builds the camera
  matrix from eye, point and up with the look-at builder 0x2e7990 and writes it into the render camera,
  no blend; its four floats are stored as near, far, one value not read, and the field of view
  `Cam.GetFov__` returns (radians). The script passes 10, 100000, 0, 1.
- Runtime (`lbp-runtime/src/camera.rs`): `ScriptCamera` (eye, direction, up as passed, lens),
  `LevelWorld::script_camera` (stands from the call to the end of the next tick's count),
  `LevelWorld::display_camera` (which camera a frame is drawn with: a camera a script wrote, outright;
  else the follow camera under an override; else none), `DisplayCamera`, `CameraSource`,
  `CameraPose::matrix_with_up`; `GetCameraMatrix` without a view answers with the written matrix.
  `display.rs`: `LevelWorld::world_thing_moved`. `level_scripts`: five counts in the `camera` block and
  the camera after ticks 1, 226, 452 and the last for a level played alone.
- Viewer (`level_play.rs`): `script_camera` applies `display_camera` (eye, turn, field of view; near and
  far stay the viewer's); `follow_world` places the root of the planets level (`N M N^-1`) each tick.
  With `--pod-camera N` and no `--pod-shell` the level hangs under that root; with the shell layer the
  viewer keeps its own Pod camera and says so once (that layer is placed for it). Without
  `--level-scripts` no flag changed. New QA option `--level-scripts-hold-at N` (scripts stop after tick
  N and a capture waits for it). `thing_view.rs` counts a swapped mesh as not drawn.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): the `play_all` block equals D271 but
  for the new counts. Things stopped by a fault 3 / 3 / 3; unbound natives called 41 / 73 / 73; unbound
  calls 61,480 / 862,986 / 868,663. `SetCameraFromToUp` 60 / 60 / 62 calls in 1 level; levels with a
  written camera and an override in at once: 0.
- pod_planets alone: the camera after tick 1 eye (-500, -2000, 2000), after 226 (-250, -1125, 1565),
  from 452 on (0, -250, 1130) looking along (0, 0.132, -0.991), 57.3 degrees; with the pod computer in
  use (0, -300, 900). The viewer prints the same poses.
- Eye check (no focus taken, `local-analysis/rscript/stage6/shots/`): stills at ticks 30, 200, 440 and
  600: the camera comes from low, looking up at the ceiling, to a level view of the room with the Earth
  and the Moon near the middle. Under `--preview-lighting` they are washed out by the viewer's distance
  fog (the level is 31,000 units away); the default lighting shows the room clearly. With the shell
  layer the capture is equal byte for byte with and without scripts. Nothing compared against the game.
- INFERRED: a written camera stands for one tick and goes before an override; the look-at axes; the
  field of view is vertical; the whole planets level is drawn under its world Thing (measured in the
  running game before, P2k; the engine code was not read).
- Left out: the shell layer under the script's camera, Earth and Moon by their own scripts (they stay
  at their stored places, so the planets' framing is not the game's), the pod's mesh swap in the scene,
  near and far, depth of field, fog. Next: PR 4, the pod computer is entered. `docs/level-scripts.md`
  part 23.

## Before: D271 video, voice-over and progression flags (2026-10-13)
- RScript plan stage 6 (the Pod), PR 2. The planets level's intro thread (451 ticks of camera zoom,
  then a wait on "a voice-over line plays or the pod computer has a video handle") never ended: the
  unbound `Video.StartVideo` answered 0 and the script takes any handle but -1 as a playing video.
- Nothing plays. A video or voice-over request is kept with its arguments (`LevelWorld::media`: video
  GUID, line name, flags, frame) for a later video or audio system; the script gets the game's answer
  for "nothing started".
- FOUND in the EBOOT: `Video.StartVideo__Q5Thinggb` (0x67c620) answers -1 when no video starts;
  `Video.UpdateVideo__ib` (0x67c22c) false for a handle no video has; `Audio.RequestVoiceOver` (0x2de6b8)
  false when no line starts; `IsVoiceOverPlaying` (0x2de218) false while none plays; `StopVoiceOver`
  true; `SetMixOverlay` / `RemoveMixOverlay` (0x2cdb7c, 0x2cdaa0) false without the audio system object;
  `Profile.SetLBP2GameProgressionFlag__i` (0x15d22c) ORs a bit mask into one 32-bit word of the profile
  and `IsLBP2GameProgressionFlagSet__i` (0x15d2a8) wants every bit of its mask. The intro sets mask 3.
- Runtime: `LevelWorld::progression_flags` is one `u32` (was a set of numbers); `--flags` and the
  viewer's `--level-scripts-flags` take masks (`parse_progression_flags`). New
  `lbp-runtime/src/media.rs` (`MediaState`, `VideoRequest`, `VoiceOverRequest`). Natives 238 -> 249
  (`STATE_QUERIES` 26 -> 29, `RECORDED_ONLY` 23 -> 24, new `RECORDED_REQUESTS` of 5 with their answers).
  `level_scripts`: `media`, `progression_flags`, `thread_ticks`. Viewer: the option's parse only.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): Things stopped by a fault 3 / 3 / 3
  (unchanged); unbound natives called 44 -> 41 / 78 -> 73 / 78 -> 73; unbound calls 61,483 -> 61,480 /
  865,266 -> 862,986 / 872,042 -> 868,663. Video requests 1 in 1 level, voice-over requests 1 in 1
  level (all three); mix overlays removed 0 / 2,260 in 114 levels / 3,355 in 143; a script sets a
  progression flag in 0 / 3 / 3 levels. The rest equals D270.
- The Pod levels alone: pod_planets, 600 and 1,200 ticks: the thread starts on tick 1 and ends on tick
  452; the flag word goes 0 -> 3 on tick 452; no thread starts again; 0 unbound natives (was 4, 152
  calls); the camera rests on the not-in-use preset (the in-use one with `--puter-in-use`). pod.bin is
  as in D270; with `--flags=3` and a visiting player its update reaches `Native.GetIsInPadZone`.
- INFERRED: `HasPlayedLBP1AndUnlockedPod` / `Moon` false (no save data of the first game); the debug
  and build overrides of the flag test are off; `RequestVoiceOver` true when a line starts.
- Not added: the engine's per-tick `PodComputer.UpdateVideo__` call. FOUND who calls it (0x50373c, at
  the end of 0x404598, a function reached only through a table slot); who calls that slot and when was
  not found. Not bound: `Video.PressedStartButton__`. The flag word is per level world: one session
  for both Pod levels is PR 4. Next: PR 3, the viewer's Pod camera. `docs/level-scripts.md` part 22.

## Before: D270 script names, the pod Thing, the avatar (2026-10-13)
- RScript plan stage 6 (the Pod), PR 1. The Pod is two levels, `gamedata/levels/pod/pod.bin` (the shell)
  and `gamedata/lbp2/pod/levels/pod_planets.bin` (the planets view); the runtime plays each alone. The
  shell's world script was the one Thing outside TriggerAirStone that stopped on a fault: two unbound
  natives answered null and it read the PShape of the null Thing.
- FOUND in the EBOOT: `Thing.FindThing__Q7StringA` (0xd3360) compares a PWorld list's byte strings with
  the argument by the C string compare (whole string, case sensitive), first match wins, null with none.
  `Thing.GetPodThing__b` (0xd32dc) gives the first entry of a PWorld list whose resource GUID is 0x2f6c;
  one with 0x23959 only when the bool is true (with false the search ends there with null).
  `Game.GetAvatarFromPlayerID__i` shares the core of `GetYellowheadFromPlayerNumber` (0x18ec14).
  `Thing.SetRenderMeshMesh` (0x755580) sets a PRenderMesh's mesh from a resource whose type is a mesh.
  `FileList.AsGUID__i` / `AsInt__g` are the identity; `Game.IsKioskBuild__` is the constant false.
- Reader: `lbp-level` keeps the PScriptName's name (`PartData::script_name`: `ScriptName { stored, name }`,
  bytes up to the first NUL); it kept the length. Same bytes read; Toolkit parity 236 / 236.
- Runtime: `lbp-runtime/src/pod.rs` (`LevelWorld::find_thing`, `pod_thing`, `script_names`,
  `planets_background`, `PodState`); `ThingState::script_name`; `mesh_guid` follows `SetRenderMeshMesh`;
  a mesh change reaches the display list (`DisplayChange::mesh`, `ThingDisplay::mesh`). Natives
  225 -> 238 (2 recorded only: the pod decorations' fade and the request to use the pod computer).
  `level_scripts` has a `pod` block. The viewer is unchanged (it compiles and its tests are as on main).
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): Things stopped by a fault 4 -> 3 in all
  three (fault kinds 2 -> 1, levels with a fault 2 -> 1); left: 3 x `Thing.GetCOM__` (TriggerAirStone, 1
  level). Unbound natives called 48 -> 44 / 81 -> 78 / 81 -> 78; unbound calls 62,000 -> 61,483 /
  865,777 -> 865,266 / 872,549 -> 872,042. 173 Things in 66 levels have a script name (141 not empty);
  `FindThing` 1 call, 1 found; `GetPodThing` 61 / 64 / 66 calls, all found.
- The Pod levels alone, 600 ticks: pod.bin has no fault, its script is resumed on all 600 ticks, the
  checkpoint is set active, the pod keeps its mesh (transition 0). pod_planets is unchanged: the intro
  thread runs its 451 ticks of zoom and then waits on the video handle (PR 2).
- INFERRED: the two PWorld lists are the PScriptName and the PRenderMesh parts in Thing order; a null
  string finds nothing; a null resource clears a mesh; no start menu and no photo.
- Not bound (effect or answer not established): `PodComputer.SetPodTransitionTexture__g` (61 calls in
  pod.bin), `Game.CurrentLevelIsOurPod__`, `Native.GetPlayerColour__Q5Thing`, `Native.GetIsInPadZone`,
  `Input.IsJustClickedByID`. Next: PR 2 (video, voice-over, progression flags), PR 3 (the viewer's Pod
  camera). `docs/level-scripts.md` part 21.

## Before: D269 the script camera and the screen (2026-10-13)
- RScript plan stage 5, PR 5 (the last of stage 5). FOUND in the EBOOT: the level's camera keeps one
  override: a position and a direction (`Cam.SetOverrideCamera__rr`, 0x3760e4), a blend target and a
  blend rate (`Cam.SetOverrideCameraActivation__bf`, 0x374cec). Each camera update (0x6eb650) the blend
  steps linearly by the rate, is eased with 0.5 - 0.5 cos(pi x blend), and the eye is mixed linearly and
  the view direction on the arc from the follow camera; above a blend of 0.0001 the lens is 60 degrees,
  near 10, far 40000. Only a script ends it. `Cam.GetCameraMatrix__` (0x7347e0) is camera to world;
  `HUD.GetScreenSpacePosition__r` (0x2e7bdc, 0x2e74a0) gives pixels from the top left, y down, on the
  1280 x 720 of `HUD.GetXRes__` / `GetYRes__`, with no test for a point behind the camera.
- Runtime: `lbp-runtime/src/camera.rs`. `CameraState` has the `OverrideCamera`, the `CameraView` a
  display writes (camera matrix, view-projection) and counters; `step`, `ease`, `slerp`, `blend_pose`,
  `screen_position`, `view_projection`; `LevelWorld::step_camera` at the end of `tick_level`,
  `camera_pose(follow_eye, follow_dir)` for a display. Natives 219 -> 225 (the six above).
- Viewer: `--level-scripts` takes its own camera placement as the follow camera each frame, draws with
  the mixed camera and the 60 degree lens while the override is in, and writes the camera it drew with
  for the scripts (`level_play.rs`, `script_camera`). Orbit, pan and zoom go on underneath. It prints
  `Camera at tick N`.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): unbound natives called 51 -> 48 /
  87 -> 81 / 87 -> 81; unbound calls 73,325 -> 62,000 / 1,002,445 -> 865,777 / 1,010,529 -> 872,549; the
  override comes in on 0 / 101 / 101 levels (fully in on 98; 39,386 / 39,588 set calls); screen
  positions asked in 28 / 61 / 70 levels (3,775 / 32,360 / 32,730 calls). Things stopped by a fault
  4 -> 4; messages, global settings and display counts equal to D268.
- INFERRED: world +Y as up under the override; the step at the end of the tick; both directions at
  length 1 in the mix; without a display `GetCameraMatrix` is the default follow pose (or the identity)
  and a screen position is the middle of the screen, so that a run without a display counts as before.
- Not bound: `Native.SetSpeechBubbleActive__Q5Thingb` (which part it finds and what the call does are
  not established). `Cam.SetCameraFromToUp` is the Pod's: stage 6.
- Checked in the window without taking focus (main_04_bravery_test, the Sackboy started in the
  scoreboard's trigger): with the scripts the camera is centred on the scoreboard, at the override's
  position; without them the follow camera. Not seen: the blend in motion (the scripts tick while the
  scene loads, so it was fully in at the first frame), a bubble closed for being off the screen.
  Open: with `--play-sackboy` some levels start the Sackboy at the scoreboard's own checkpoint, and the
  camera then goes to the scoreboard at once and stays. Next: stage 6, the Pod.
  `docs/level-scripts.md` part 20.

## Before: D268 hidden, tinted and moved Things (2026-10-13)
- RScript plan stage 5, PR 4. FOUND in the EBOOT: `Thing.HideRenderMesh` / `ShowRenderMesh` clear and
  set two bits (mask 3) of the PRenderMesh's visibility byte, the stored `visibilityFlags`, and go down
  the Thing's children that have a mesh and no shape (0xebf98); bit 0 is "visible in play mode"
  (`PostItNote.IsVisibleInPlayMode__`, 0x5e9260); the mesh colour, mesh tint, shape colour and shape tint
  setters store packed colours and do nothing for a null Thing or a missing part.
- Runtime: `lbp-runtime/src/display.rs`. `LevelWorld::take_display_dirty` lists the Things whose
  visibility byte, colours or world matrix changed (`DisplayChange`); `thing_display` and
  `display_at_load` give the values (`ThingDisplay`, with `mesh_hidden` and `moved_from`);
  `display_changed`, `display_summary`. Natives 216 -> 219 (`ShowRenderMesh`, `SetShapeEditorColourTint`,
  `IsVisibleInPlayMode`); hide and the mesh colour are real bindings (`RECORDED_ONLY` 23 -> 21).
- Viewer: `--level-scripts` tags the draw entities of each Thing's own model (render mesh, decorations)
  and keeps a Thing-to-entity map (`thing_view.rs`). Each tick it hides a mesh whose play-mode bit is
  clear, gives a tinted mesh a material of its own (red, green and blue multiply the surface colour) and
  moves a Thing's entities by the change of its world matrix. A new scene starts with every mesh hidden
  that the level file stores as not visible in play mode. It prints `Things at tick N`.
- Not followed: shapes (merged into shared meshes per material and colour: a recoloured or moved shape
  keeps the file's look and place), an editor colour set by a script (built into the material at load),
  the tint's alpha, the Pod, per-item skinned meshes.
- MEASURED (378 levels, 60 ticks, the three runs alike): 68,618 render meshes, 12,106 in 218 levels
  stored as not visible in play mode (byte 2: 11,573, byte 0: 533); visibility bytes changed by scripts
  0 (the 113 hide calls are on meshes already stored hidden); mesh colour unlike the file 10 Things / 1
  level; mesh tint 102 / 101 (101 white with an alpha, 1 with a colour); shape colour 102 / 101; world
  matrix unlike in any bit 4,545 / 32, beyond rounding 1 Thing (no mesh). Things stopped by a fault
  4 -> 4; unbound natives unchanged.
- INFERRED: a mesh without bit 0 is not drawn in play and its decorations stay; the tint as a multiply
  before lighting; the two colour words are the stored `editorColor`; the packed channel order.
- Checked in the window without taking focus (side_rts_tutorial, unlit): the level link's padlock and
  the scoreboard's two corner markers are gone with the scripts. On ghost_train the one tint with a colour (0.70, 1.0, 0.15 on the scoreboard's mesh) turns the scoreboard's frame yellow-green. Not seen: a change while the
  scene is up (covered by the `thing_view` test). Next: PR 5, camera and screen natives.
  `docs/level-scripts.md` part 19.

## Before: D267 script global settings take effect (2026-10-13)
- RScript plan stage 5, PR 3. FOUND in the EBOOT: the PWorld's per-frame function (0xd5e84, after the
  switch pass in the world update) calls `ApplyGlobalSettings__` on the scripted Thing of each
  global-settings slot and clears a slot whose Thing is gone. The script steps its blend factor there,
  which is why no setting moved before.
- Runtime: `LevelScripts::apply_global_settings` in `tick_level`; `LevelWorld::global_settings_holder`;
  `lbp-runtime/src/global_settings.rs` with `WorldState::settings_in_force` (`SettingsInForce`),
  `LevelWorld::settings_at_load` / `settings_unlike_load`, `GlobalSettingsReport`
  (`StepReport::global_settings`) and `LightFollower`.
- Viewer: `--level-scripts` builds the level's light again when lighting or darkness in force changes,
  by the rule it already uses for the file's values (`LevelSettings::at_lighting`). Not applied: fog,
  fog tint, colour correction (mapping not read), gravity, water. It prints `Lighting at tick N`.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): calls 4,562 / 80,255 / 80,631; a
  setting in force unlike the level file in 48 / 81 / 96 levels (0 before); lighting group 47 / 76 /
  89, gravity 3 / 11 / 15, water level 0 / 1 / 2. Things stopped by a fault 4 -> 4; unbound natives
  unchanged.
- INFERRED: the call's place after `Update__` (the engine's per-Thing `Update__` call was not found).
- Checked in the window without taking focus: cutscene_outro_theme1, lighting 0.4 -> 1.0, darkness
  0 -> 0.6. Open: a new light in the first frame after the scene is spawned lost the GPU device (a
  60-frame wait avoids it, cause unknown); the viewer's fog covers most of those frames.
  Next: PR 4, hidden, moved and tinted Things. `docs/level-scripts.md` part 18.

## Before: D266 the viewer draws the messages (2026-10-13)
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
