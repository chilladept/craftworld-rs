# Development log

## 2026-10-14 - D284 the Pod's second mask, by the mesh's own UV

Pod picture PR 11, viewer only. The Pod tent's dissolve now has the card materials' second mask, `pod_mask.tex`:
sampled at the mesh's raw UV0 (FOUND: identity transform in the captured draws), it raises the line one band
where it is white, and at value 1 the rule's last term keeps only those parts. Each draw bucket records which
input holds raw UV0 (MEASURED against the mesh: `pod_card` its primary coordinates, `pod_card_edge` its second
sample's, the glass none), so the transition mask moves from UV1 to UV0 on the card edge. At alpha 0 the
platform's card edge keeps the dissolve at value 1; its UV0 lies wholly on the mask's white strip and patch, so
its rim stays whole. Changed frames: the band's pattern at ticks 610 and 615; the raised parts are out of view at
610 to 619 and 640 equals D283. The viewer's Sackboy draws his coplanar parts in another order once the material
holds one more image (two closed stills). The game frame `open-t1` shows the player's green line and traces,
not drawn yet. `docs/level-scripts.md` part 35.

## 2026-10-14 - D283 the script camera's lens

Pod picture PR 10, viewer only. A camera a script writes with `Cam.SetCameraFromToUp` now draws with the
script's near and far planes as well as its field of view (FOUND in 01.33: the native writes near, far, ortho
factor and vertical field of view every call, not the aspect; the Pod passes 10, 100,000, 0, 1.0 rad), and the
override camera with the game camera's lens (near 10, far 40,000, 60 degrees; part 20's wording corrected). The
planes go from level units to display units by the level's display scale: in the Pod near 0.0015706 and far
15.706 display units (were 64 and 636,705 level units). Nothing of the Pod is beyond 100,000; the frames with
the script camera are equal byte for byte except 3 pixels at tick 610 and the planets level alone (a near
object the old near plane cut). The intro's black bar is INFERRED to be the level's Thing 12, not the engine's
cinema bars. `docs/level-scripts.md` part 34.

## 2026-10-14 - D282 his z span, his arrival and the pod computer's user

Pod picture PR 9 (input option C), viewer only. In the Pod with `--play-sackboy`: his z span for the
triggers' layer rule is the FOUND absolute range -500 to 100 of the 01.33 creature update (was 200 either
way), so the trigger holds him in every lane and the pad zone's z rule decides the lane; he arrives by the
FOUND checkpoint spawn rule (checkpoint + 50 in z, slot 0, no height offset) at (-3.05, 540.76, 0) and falls,
in the pad zone on tick 25, on the floor on tick 28 at y -476.4; while the pod computer is in use his movement
input is dropped, he turns to face pi and his figure is drawn toward the engage target (x from the
"pod_controller" Thing, 17.846) by the change-over value (placeholder for the clips). Without
`--play-sackboy` eleven captures are equal byte for byte. `docs/level-scripts.md` part 33.

## 2026-10-13 - D281 the viewer's own Sackboy in the Pod

Pod picture PR 8 (input option C), viewer only. With `--play-sackboy` under `--level-scripts --pod-camera N
--pod-shell HEX` the Sackboy plays in the shell level: a physics scene of pod.bin whose floor is the pod
Thing's own shape projected through its matrix (he stands at y -476.9; the game's measured -477.97; nothing
fitted), the play figure under the shell's frame root, the script's camera, his place handed to the shell
world before every tick so the pad zone follows where he stands, the engage button from Z / gamepad Square,
`e` in `--play-sackboy-input` or `--level-scripts-press-at`. Walk and lane runs: out of the zone past the
trigger's side and behind z -300, presses there ignored, taken back inside. His z span (200) and the held pod
are placeholders. Without `--play-sackboy` every capture is equal byte for byte; part 30's undrawn record
stays there. `docs/level-scripts.md` part 32.

## 2026-10-13 - D280 the Pod shell level in its own frame, under the script camera

Pod picture PR 7, viewer only. Under `--level-scripts` with the shell layer pod.bin is no longer placed by the
rig's fitted shell eyes: both shell roots are the shell's own level frame (`level_play::shell_frame_root`) and
the planets level's script camera draws both levels, as the two measured end states say. Closed and open
stills match two earlier RPCS3 frames within about 0.01 of the picture (tent window, Earth, Moon, DualShock,
platform rim); the DualShock no longer moves against the tent's floor in the change-over. The nearest
planets-level surface is 14,400 units from the shell's box. Without scripts equal byte for byte.
`docs/level-scripts.md` part 31.

## 2026-10-13 - D279 the pod computer entered through the pad zone and the press

Pod picture PR 4b. With the shell level in the viewer's session a Sackboy (not drawn, placeholder) stands on
the pod computer's trigger (`LevelWorld::pod_trigger_spot`) and the Puter key is bit 0x80 of the pad for one
tick: pod.bin's script asks the host and "in use" comes a tick later (press 600, in use 601, open 620).
Closing asks the host (placeholder). QA option `--level-scripts-press-at N`. Closed and open stills equal
D278's byte for byte; the dissolve's band is white (placeholder player colour). `docs/level-scripts.md` part 30.

## 2026-10-13 - D278 the Pod tent's top-down dissolve

Pod picture PR 5, viewer only. The tent of the shell's pod Thing is cut from the top down while pod.bin's
change-over runs: a discard above `camera y + 1150 - 1550 v` (FOUND in the pod's materials; the camera is
the script camera's, MEASURED at the end states) and a band under the line tinted by the transition mask
(`pod_dissolve.rs`, a dissolve block in the viewer material). The tent no longer slides away. Closed and
open stills equal D277's byte for byte. Left: the second mask, the platform's edge at 1, the input.
`docs/level-scripts.md` part 29.

## 2026-10-13 - D277 both Pod levels in one viewer session

Pod picture PR 4a. Under `--level-scripts` with the shell layer pod.bin has its own script world and the two
levels tick through `tick_session`; "in use" is set on both worlds; the shell has its own Thing map and the
swapped pair, so the mesh swap shows in the Pod picture; the opening progress is the script's change-over
value (`LevelWorld::pod_transition`, PLACEHOLDER as the shell's opening). In use from 600: swap on 620, the
tent gone and the platform under the DualShock at 640. Captures on one level and without scripts are equal
byte for byte. Left: the tent's dissolve, the Sackboy as input. `docs/level-scripts.md` part 28.

## 2026-10-13 - D276 the pod's mesh swap in the scene

Pod picture PR 3. The scene of a level played by `--level-scripts` loads both meshes of the pair the Pod's
script swaps (`pod.mol`, `pod_platform.mol`), the second hidden; `thing_view::apply` shows the one the
Thing has now. QA option `--level-scripts-in-use-at N`. pod.bin alone: swap printed on tick N + 20; the
still after it shows the platform in the tent's place (no fade; two small pieces stay in mid-air).
Without `--level-scripts` captures are equal byte for byte. `docs/level-scripts.md` part 27.

## 2026-10-13 - D275 the Pod shell layer under the script's camera

Pod picture PR 2, viewer only. `PodRig::layer_at` places the shell and the tent from any planets camera;
under `--level-scripts` with `--pod-shell` the planets level is drawn with the script's camera under the
world Thing's root (Thing map on), the Puter key sets `pod_computer_in_use`, and `--pod-game-camera`'s
menu constants are left out. Stills (ticks 30, 200, 440, 600, open): the planets view is in the tent's
window, nothing cut. Without `--level-scripts` captures are equal byte for byte. `docs/level-scripts.md`
part 26.

## 2026-10-13 - D274 Earth and Moon by their own scripts, the fog bias, the video update

Pod picture PR 1. FOUND (EBOOT 01.33): the engine calls each planet's `Update__rb(point, snap)` before
the world script's `UpdatePlanetsView`; the third `SetDOFAndFog` value is a fog bias on the fog near;
`PodComputer.UpdateVideo__` runs once a rendered frame after the update. Runtime: two frame entries with a
host point (PLACEHOLDER (0, 0, 1)) and "arrived", `update_video` at the end of a tick, `Math.transform` and
`ModAngleToWithinPI` (255 -> 257), virtual calls on a Thing dispatch on its own script. Viewer: the Thing
map follows the planets under the world Thing's root; `--preview-lighting` moves its fog out by the bias.
Planets level: Earth (-1250, 0, 0) axes 0.961, Moon (2000, 0, 0) axes 2.112, as in the game (P2k). 378
levels: faulted Things 3 / 3 / 3, unbound natives called 41 / 72 / 72, unbound calls 61,420 / 862,920 /
868,593 (one more, a cutscene level). `docs/level-scripts.md` part 25.

## 2026-10-13 - D273 the pod computer is entered

RScript plan stage 6 (the Pod), PR 4. FOUND (EBOOT 01.33, read for this PR): the pad zone is "our Pod, the
primary player, in a pod-computer trigger, world z >= -300"; `IsJustClickedByID` takes an action number (8 =
bit 0x80 newly down, context mask 0x1001); `GetPlayerColour` unpacks a stored ARGB word; "our Pod" is set
when a level is entered from one slot; `RequestHostUsePod` only sends a request and the host's answer sets
"in use" with a sound. Runtime: those rules (`pod.rs`, `PadState`, `PlayerRecord::colour`), the answer at
the start of the next tick, one session for several levels (`hand_session_to`, `tick_session`). Natives
249 -> 255. `level_scripts --with-level/--enter-pad-zone/--press/--player-colour/--our-pod`. MEASURED, the
two Pod levels together: flags 3 on round 452, press and request on 470, in use on 471, change-over to 1.0
and pod mesh GUID 145753 on 490, planets camera on its in-use preset on 496. 378 levels: Things stopped by
a fault 3 / 3 / 3, unbound natives called 40 / 71 / 71, unbound calls 61,419 / 862,919 / 868,592.
INFERRED: trigger class `Trigger_Pod`, one local player is primary, host and owner. PLACEHOLDER: the stored
colour (opaque white). Left out: the viewer, the pod computer's menus. `docs/level-scripts.md` part 24.

## 2026-10-13 - D272 the Pod's camera and its moving world

RScript plan stage 6 (the Pod), PR 3. The planets level's world script writes the camera itself each tick
(`Cam.SetCameraFromToUp`, bound and kept since part 6) and moves its world Thing each tick; nothing drew by
either. FOUND (EBOOT 01.33, scoping notes): the native (0x2e9798) writes the render camera directly from
eye, point and up, no blend; its floats are near, far, one value not read, and the field of view in
radians. Runtime (`camera.rs`): `ScriptCamera`, `LevelWorld::script_camera` (stands for the tick it was
written on), `LevelWorld::display_camera` (a written camera outright; else the follow camera under an
override; else the display's own), `CameraPose::matrix_with_up`; `display.rs`:
`LevelWorld::world_thing_moved`. Viewer (`level_play.rs`): `script_camera` applies the answer (eye, turn,
field of view); `follow_world` places the root of the planets level from the moved world Thing (with
`--pod-camera N`, no `--pod-shell`); with the shell layer the viewer keeps its own Pod camera (said once).
New QA option `--level-scripts-hold-at N`. No native is new (249). MEASURED, 378 levels, 60 ticks (plain /
trigger visit / pulsed): equal to D271 (Things stopped by a fault 3 / 3 / 3, unbound natives called
41 / 73 / 73, unbound calls 61,480 / 862,986 / 868,663); `SetCameraFromToUp` 60 / 60 / 62 calls in 1
level, 0 levels with a written camera and an override at once. pod_planets: eye (-500, -2000, 2000) after
tick 1, (0, -250, 1130) from tick 452 on, 57.3 degrees. Captures (no focus taken) at ticks 30, 200, 440,
600: from a steep look up at the ceiling to a level view of the room with Earth and Moon near the middle;
washed out by the viewer's fog under `--preview-lighting`, clear under the default lighting. INFERRED: how
long a written camera stands, that it goes before an override, vertical field of view, the level drawn
under its world Thing. Left out: the shell layer under the script's camera, Earth and Moon by their own
scripts, the pod's mesh swap in the scene (counted), depth of field and fog. Nothing compared against the
game. +0 viewable, +0 Complete. `docs/level-scripts.md` part 23.

## 2026-10-13 - D271 video, voice-over and progression flags

RScript plan stage 6 (the Pod), PR 2. The planets level's intro thread (451 ticks of zoom, then a wait on
"a voice-over line plays or the pod computer has a video handle") never ended: the unbound
`Video.StartVideo` answered 0 and the script takes any handle but -1 as a playing video. FOUND (EBOOT 01.33,
through the Ghidra bridge): `StartVideo` (0x67c620) answers -1 when no video starts; `UpdateVideo`
(0x67c22c) false for a handle no video has; `Audio.RequestVoiceOver` (0x2de6b8) false when no line starts,
`IsVoiceOverPlaying` (0x2de218) false while none plays; `SetMixOverlay` / `RemoveMixOverlay` false without
the audio system object; `Profile.SetLBP2GameProgressionFlag__i` (0x15d22c) ORs a bit mask into one 32-bit
word of the profile and `IsLBP2GameProgressionFlagSet__i` (0x15d2a8) wants every bit of its mask. Nothing
plays: a request is kept with its arguments (`LevelWorld::media`, new `lbp-runtime/src/media.rs`) for a
later video or audio system, and the script gets the game's answer for "nothing started".
`LevelWorld::progression_flags` is one `u32` (was a set of numbers); `--flags` takes masks. Natives
238 -> 249 (new list `RECORDED_REQUESTS`). MEASURED, 378 levels, 60 ticks (plain / trigger visit / pulsed):
Things stopped by a fault stay 3 / 3 / 3; unbound natives called 44 -> 41 / 78 -> 73 / 78 -> 73; unbound
calls 61,483 -> 61,480 / 865,266 -> 862,986 / 872,042 -> 868,663; video requests 1 and voice-over requests 1
in 1 level. pod_planets alone, 600 and 1,200 ticks: the thread ends on tick 452, the flag word goes 0 -> 3,
no thread starts again, 0 unbound natives (was 4, 152 calls). INFERRED: `HasPlayedLBP1AndUnlockedPod` /
`Moon` false. Not added: the per-tick `PodComputer.UpdateVideo__` call (FOUND who calls it, 0x50373c at the
end of 0x404598; that function is reached only through a table slot, and who calls the slot and when was
not found). Not bound: `Video.PressedStartButton__`. Viewer: the parse of `--level-scripts-flags` only.
+0 viewable, +0 Complete. `docs/level-scripts.md` part 22.

## 2026-10-13 - D270 script names, the pod Thing, the avatar

RScript plan stage 6 (the Pod), PR 1. The Pod is two levels, the shell (`pod.bin`) and the planets view
(`pod_planets.bin`); the runtime plays each alone. FOUND (EBOOT 01.33, through the Ghidra bridge):
`Thing.FindThing__Q7StringA` (0xd3360) walks a PWorld list and compares each entry's byte string with the
argument by the C string compare (whole string, case sensitive), first match wins, null with none;
`Thing.GetPodThing__b` (0xd32dc) gives the first entry of a PWorld list whose resource GUID is 0x2f6c, and
one with 0x23959 only for true (for false the search ends there with null);
`Game.GetAvatarFromPlayerID__i` shares the core of `GetYellowheadFromPlayerNumber` (0x18ec14);
`Thing.SetRenderMeshMesh` (0x755580) sets a PRenderMesh's mesh from a resource whose type is a mesh;
`FileList.AsGUID__i` / `AsInt__g` return their argument; `Game.IsKioskBuild__` is false. `lbp-level` keeps
the PScriptName's name (`ScriptName { stored, name }`, it kept the length; Toolkit parity 236 / 236). New
`lbp-runtime/src/pod.rs` (`find_thing`, `pod_thing`, `PodState`); a mesh change reaches the display list
(`DisplayChange::mesh`). Natives 225 -> 238, two recorded only (`SetFadePodDecorations__f`,
`RequestHostUsePod__b`). MEASURED, 378 levels, 60 ticks (plain / trigger visit / pulsed): Things stopped by
a fault 4 -> 3 in all three (fault kinds 2 -> 1): the Pod's world script gets its pod Thing and is resumed
every tick; left: 3 x `Thing.GetCOM__` (TriggerAirStone). Unbound natives called 48 -> 44 / 81 -> 78 /
81 -> 78; unbound calls 62,000 -> 61,483 / 865,777 -> 865,266 / 872,549 -> 872,042. 173 Things in 66 levels
have a script name; `FindThing` 1 call, 1 found; `GetPodThing` 61 / 64 / 66 calls, all found. pod.bin alone,
600 ticks: no fault, resumed on all 600; pod_planets unchanged (the intro thread waits on the video handle).
INFERRED: the two lists are the PScriptName and PRenderMesh parts in Thing order; a null string finds
nothing. Not bound (effect or answer not established): `SetPodTransitionTexture__g`,
`CurrentLevelIsOurPod__`, `GetPlayerColour`, `GetIsInPadZone`, `IsJustClickedByID`. The viewer is unchanged.
+0 viewable, +0 Complete. `docs/level-scripts.md` part 21.

## 2026-10-13 - D269 the script camera and the screen

RScript plan stage 5, PR 5, the last of stage 5. FOUND (EBOOT 01.33, through the Ghidra bridge and a decode
of the vector code): the level's camera keeps one override. `Cam.SetOverrideCamera__rr` (0x3760e4) stores a
position and a direction as passed; `Cam.SetOverrideCameraActivation__bf` (0x374cec) sets a blend target
(1.0 or 0.0) and a blend rate, and nothing else writes them, so only a script ends an override. Each camera
update (0x6eb650) the blend steps linearly by the rate (0x38eb40), the weight is 0.5 - 0.5 cos(pi x blend),
the eye is mixed linearly and the view direction on the arc from the follow camera (0x3918ec); above a blend
of 0.0001 the lens is 60 degrees, near 10, far 40000. `Cam.GetCameraMatrix__` (0x7347e0) copies the render
camera's camera matrix (column W the eye, column Z backward). `HUD.GetScreenSpacePosition__r` (0x2e7bdc,
0x2e74a0) projects a point with the view-projection matrix to pixels from the top left, y down, dividing by
0.0001 when w is exactly 0 and with no test for a point behind the camera; `HUD.GetXRes__` / `GetYRes__`
return two globals that hold 1280 and 720. New `lbp-runtime/src/camera.rs`: `OverrideCamera`, `CameraView`,
`CameraPose`, `CameraState` (moved, with the view and counters), `step`, `ease`, `slerp`, `blend_pose`,
`screen_position`, `view_projection`, `LevelWorld::step_camera` (at the end of `tick_level`), `camera_pose`,
`camera_matrix`, `screen_position`. Natives 219 -> 225 (the six named). `level_scripts` reports
`play_all.camera`. Viewer: a `script_camera` system after the viewer's camera system takes that placement as
the follow camera, draws with the mixed camera and the 60 degree lens while the override is in, and writes
the camera it drew with for the scripts; `FOLLOW_EYE` moved into the runtime. MEASURED, 378 levels, 60 ticks
(plain / trigger visit / pulsed): Things stopped by a fault 4 -> 4 in all three; unbound natives called
51 -> 48 / 87 -> 81 / 87 -> 81; unbound calls 73,325 -> 62,000 / 1,002,445 -> 865,777 / 1,010,529 -> 872,549;
the override comes in on 0 / 101 / 101 levels (fully in on 98), 39,386 / 39,588 set calls; screen positions
asked 3,775 / 32,360 / 32,730 times in 28 / 61 / 70 levels; messages, global settings and display counts
equal to D268. INFERRED: world +Y as up under an override, the step at the end of the tick, directions at
length 1 in the mix, and the answers without a display (the default follow pose or the identity; the middle
of the screen), chosen so that a run without a display counts as before. Not bound:
`Native.SetSpeechBubbleActive__Q5Thingb` (its part and effect are not established). Viewer seen without
taking focus on main_04_bravery_test with the Sackboy started in the scoreboard's trigger: the camera is
centred on the scoreboard at the override's position; without the scripts the follow camera. The blend in
motion was not seen (the scripts tick while the scene loads). Open: with `--play-sackboy` some levels start
the Sackboy at the scoreboard's own checkpoint, so the camera goes to the scoreboard at once. Tests: 6 unit
tests and two world tests in `lbp-runtime`, the natives without a world in `lbp-script-vm`, one view test in
the viewer. +0 viewable, +0 Complete. `docs/level-scripts.md` part 20.

## 2026-10-13 - D268 hidden, tinted and moved Things in the viewer

RScript plan stage 5, PR 4. FOUND (EBOOT 01.33, through the Ghidra bridge): `Thing.HideRenderMesh` (0x642cd4)
and `ShowRenderMesh` (0x642c94) reach one visibility call (0xebf98) with the mask 3: it clears, and to show
sets, two bits of the PRenderMesh's visibility byte (the stored `visibilityFlags`; a new PRenderMesh has 3)
and goes down the Thing's children that have a PRenderMesh and no PShape, not below a Thing with a PShape or
one of two globe meshes. `PostItNote.IsVisibleInPlayMode__` (0x644d7c -> 0x5e9260) answers with bit 0 of that
byte, which names the bit. `SetRenderMeshEditorColour` (0x642924) stores a packed colour in two words of the
PRenderMesh, `SetRenderMeshEditorColourTint` (0x642978) in a third; `SetShapeEditorColour` (0x642a20) and
`SetShapeEditorColourTint` (0x6429cc) in two words of the PShape; all do nothing for a null Thing or a
missing part. New `lbp-runtime/src/display.rs`: `LevelWorld::take_display_dirty` (Things whose visibility
byte, colours or world matrix changed, with a `DisplayChange`), `thing_display` / `display_at_load`
(`ThingDisplay`: `mesh_hidden`, `unlike`, `moved_from`, `moved_far`), `display_changed`, `display_summary`,
`pack_colour`; `ThingState::render_mesh_visibility` and `shape_tint`, `render_mesh_colour` starts as the
file's. Natives 216 -> 219 (`Thing.ShowRenderMesh__Q5Thing`, `Thing.SetShapeEditorColourTint__Q5Thingr`,
`PostItNote.IsVisibleInPlayMode__`); hide and the mesh colour are real bindings (`RECORDED_ONLY` 23 -> 21);
the shape colour and the mesh tint no longer fault on a null Thing. `level_scripts` reports
`play_all.display`. `lbp-preview`: `SceneItem::decoration`. Viewer: `thing_view.rs` (`LevelThing` on the
draw entities of a Thing's own model, the `LevelThings` map built in `spawn_level`, `apply`), a
`follow_things` system after the tick, a tint uniform in the material (`dual::set_thing_tint`). Under
`--level-scripts` a mesh whose play-mode bit is clear is hidden (from the level file or by a script), a tint
multiplies the surface colour, a moved Thing's entities follow. Shapes are merged into shared meshes and are
not followed; a script's editor colour, the tint's alpha and the Pod are not followed either. MEASURED, 378
levels, 60 ticks, the three runs alike: 68,618 render meshes, visibility byte 3 / 2 / 0 = 56,512 / 11,573 /
533, so 12,106 meshes in 218 levels are stored as not visible in play mode; scripts change 0 visibility
bytes (113 hide calls on meshes already stored hidden), the mesh colour of 10 Things in 1 level (1,180
calls), the tint of 102 Things in 101 levels (101 white with an alpha, 1 with a colour), the shape colour of
102 in 101; 4,545 world matrices differ in some bit, 1 beyond rounding (a Thing without a mesh). Things
stopped by a fault 4 -> 4 in all three (the Pod's fault moved one instruction on); unbound natives and
native calls unchanged. INFERRED: a mesh without bit 0 is not drawn in play, the tint as a multiply, the
colour words as the stored `editorColor`, the packed channel order. Viewer seen without taking focus on
side_rts_tutorial, unlit: the level link's padlock and two markers at the scoreboard are gone with the
scripts. On ghost_train the one tint with a colour (0.70, 1.0, 0.15 on the scoreboard's mesh) turns the scoreboard's frame yellow-green. Tests: 3 unit tests and one world test in `lbp-runtime`, the natives without a world in
`lbp-script-vm`, one `thing_view` test in the viewer. +0 viewable, +0 Complete. `docs/level-scripts.md`
part 19.

## 2026-10-13 - D267 script global settings take effect

RScript plan stage 5, PR 3. FOUND (EBOOT 01.33, through the Ghidra bridge): the PWorld's per-frame function
0xd5e84, called from the world update after the switch pass, runs the script function `ApplyGlobalSettings__`
on the scripted Thing of each of the eight global-settings slots (0x88ff8c -> 0x620bd8, its only caller) and
clears a slot whose Thing is gone or has no script; on its first run an empty slot loses a stored priority.
The global-settings script steps its blend factor there (script listings). New
`LevelScripts::apply_global_settings` in `tick_level`, `LevelWorld::global_settings_holder`, and
`lbp-runtime/src/global_settings.rs`: `SettingsInForce` (`WorldState::settings_in_force`: water level,
gravity, wave, tint, murkiness, the lighting group, water bits), `LevelWorld::settings_at_load` and
`settings_unlike_load`, `GlobalSettingsReport`, `LightFollower`. `level_scripts` reports
`play_all.global_settings`. Viewer: `--level-scripts` keeps the level's main block, presets and light frame
(`lighting::LevelRelight`) and builds the light again when lighting or darkness in force moves, by the P3m
rule (`LevelSettings::at_lighting`); fog, fog tint, colour correction, gravity and water are not applied
(mapping not read). MEASURED, 378 levels, 60 ticks, plain / trigger visit / pulsed: calls 4,562 / 80,255 /
80,631 in 66 / 109 / 121 levels; a setting in force unlike the level file in 48 / 81 / 96 levels (0 before)
on 2,819 / 40,159 / 40,334 ticks; lighting group 47 / 76 / 89 levels, gravity 3 / 11 / 15, water level
0 / 1 / 2; slots cleared 0. Things stopped by a fault 4 -> 4 in all three; unbound natives unchanged; no new
native (the blend-factor native is now reached). INFERRED: the call's place after `Update__` (the engine's
per-Thing `Update__` call was looked for and not found), unique UIDs, the first-pass flag. Viewer seen
without taking focus on cutscene_outro_theme1: lighting 0.4 -> 1.0, darkness 0 -> 0.6, one new light.
MEASURED there: a new light in the first frame after the scene is spawned lost the GPU device (2 of 2 runs);
a 60-frame wait avoids it (PLACEHOLDER, cause open). Tests: 3 unit tests and one world test with a made-up
script. +0 viewable, +0 Complete. `docs/level-scripts.md` part 18.

## 2026-10-13 - D266 the viewer draws speech bubbles and the subtitle

RScript plan stage 5, PR 2. `lbp-bevy-viewer --level-scripts` loads the English base and patch translation
tables for its level world (`lbp_preview::story::load_strings`, as `level_scripts`) and draws the open
messages of `LevelWorld::messages`: a plain box per bubble above its anchor Thing with the speaker's name,
the text and the close prompt's string (`CLOSE_PROMPT_KEY`), and the subtitle at the bottom centre. New
`lbp-runtime/src/message_view.rs` holds what needs no window: `LevelWorld::message_overlay`, `close_prompt`,
`display_text`, `place_bubbles`, `place_subtitle`; new viewer module `speech_overlay.rs` measures and paints
with egui. MEASURED on the two English tables (29,477 strings): the markup is seven tags in angle brackets
(emphasis 1,838 places, line break 280, icon 299, font 68, player and slot icons 46); a line break becomes a
new line, an icon its name in square brackets, the others are left out. INFERRED: the box 24 pixels above
the anchor's screen point, moved back inside the screen at an edge; bubbles of one Thing stacked, the first
lowest; every open bubble drawn; captions and think bubbles like speech; empty bubbles and the level-link
and wormhole prompts not drawn; the look. Nothing new read in the EBOOT. The prompt is text only: event 2
is still not modelled. Without tables the viewer runs and shows no prompt line. The viewer prints a
`Messages at tick N` line on each change. PC: the pulsed run's `play_all` block equals D265 (Things stopped
by a fault 4); one level seen in the window without taking focus (one bubble at Thing 246; at the upper edge
from the follow camera). Tests: 6 unit tests and one world test with made-up text. Not seen in the window:
a subtitle, a name, a prompt line, a stack. +0 viewable, +0 Complete. `docs/level-scripts.md` part 17.

## 2026-10-13 - D265 the level world keeps the game's message list

RScript plan stage 5, PR 1. New `lbp-runtime/src/messages.rs`: `LevelWorld::messages` holds the open speech
bubbles, captions, subtitles and level-link and wormhole prompts, and the messaging natives run it through
`World::engine_call`. FOUND (manager add 0x3e8dc0, find 0x3e4124, event 0x3e8aa0, sort 0x3e8928, type
records 0x3e3248, actions 0x3e528c, cores 0x7382e0 / 0x7380f8 / 0x737320 / 0x737f64 / 0x737d7c / 0x737be8 /
0x737a00, actor name 0x737910): manager 4 of 5, at most 8 messages; a request of a type about a Thing that
has a message of the type does nothing; types 2, 3, 0x5e and 0x60 are written over the open one of the
type; with 8 held the eighth is cleared (all nine types have priority 10); event 3 runs action 0x10 on the
messages of its Thing, which marks them closing; no type has a timed action; a trigger message has no
player; the prompt bubble carries a prompt string unless prevent close is set. INFERRED: a closing message
leaves the list at once; the list keeps the order of adding (the sort comparison was not read); a message
written over restarts its age; the anchor is the Thing's microchip, else the Thing; a tag in no table
gives no actor name. Not modelled: event 2 (the player's button, actions 0x17 and 0x37), the sort states,
the closing of other Things' bubbles by 0x3e4af0, subtitle text formatting. Natives: the caption and think
forms bound, 212 -> 216; `natives::MESSAGING` lists the ten messaging natives, which leave `RECORDED_ONLY`
(29 -> 23); `KEPT_REQUESTS` 10 -> 14. `level_scripts`: a `messages` block per level and in `play_all`. PC
run, 378 levels, 60 ticks (plain / visit / pulsed): requests 92 / 1,690 / 2,381, opened 57 / 1,036 /
1,187, written over 35 / 621 / 1,161, dropped as duplicate 0 / 33 / 33, closed 0 / 1,000 / 1,152, open at
the end 57 / 36 / 34, levels with a message 47 / 192 / 216; unbound natives called 52 -> 51, 88 -> 87,
88 -> 87; Things stopped by a fault stay at 4 in all three runs. No viewer change. +0 viewable, +0
Complete. `docs/level-scripts.md` part 16.

## 2026-10-13 - D264 tag sensors and player sensors run in the switch pass

Between RScript plan stages 4 and 5. `lbp-level` keeps the PSwitch fields the sensors read (Radius,
MinRadius, the key colour index and name, RequireAll, AngleRange, KeySensorMode, DetectUnspawnedPlayers,
PlayerMode) and the PSwitchKey colour index and name; the same 378 levels read with the same Thing counts.
`lbp-runtime/src/sensors.rs`: `LevelWorld::refresh_tags` rebuilds the tag map (colour, name) -> tags by x
at the start of each pass (FOUND 0xd5c1c); `tag_signal` (FOUND 0x12f6f4: the Thing's input port 0, on when
unwired, off when deactivated); the tag sensor (FOUND 0x118d5c: x window, team filter 0xf9a64, own-chip
rule, range, closeness, the BulletsRequired-th strongest, KeySensorMode 1 handing on the tag's signal) and
the player sensor (FOUND 0x119910: PlayerMode, RequireAll, BulletsRequired, closeness of the farthest one
needed, the nearest player on the output); the ternary with its hold band (FOUND 0x11873c); inversion
afterwards. INFERRED: no cone test for AngleRange below 180 (1,811 player and 2,975 tag sensors; the
direction is not established) and no layer test for IncludeTouching 1 (212 and 489; needs the bodies' z
spans); positions are the Things' positions; the leader is the first player record; the sensor's body has
no owning player. `level_scripts`: a sensor census, and `--visit-triggers` also stands the player at every
player sensor. PC run, 378 levels, 60 ticks: 5,627 player and 16,953 tag sensors, 19,629 tags; sensors
unlike the stored signal on some tick 184 / 5,446 plain and 5,204 / 6,272 with the visit; reaction calls
262 -> 323 plain, 283 -> 8,970 with the visit, 5,704 -> 14,346 pulsed; unbound natives called 52 -> 52,
85 -> 88, 88 -> 88; Things stopped by a fault stay at 4 in all three runs. Not done: the cone direction,
the layer test, cursors, DetectUnspawnedPlayers on player sensors, the other sensors, the randomiser,
sequencers. +0 viewable, +0 Complete. `docs/level-scripts.md` part 15.

## 2026-10-13 - D263 the natives the stage 4 runs reached are bound

Between RScript plan stages 4 and 5. 48 natives bound from the stage 4c research notes, 164 -> 212 in the
table, 16 of them recorded only (29 in all). New state: `PlayerRecord::pad` (`PadState`: two mode masks,
buttons now and old) with `LevelWorld::set_pad_buttons` and `end_pad_frame`, so `Pad.IsJustPressed`,
`IsPressed` and `IsJustReleased` follow the game's rule (FOUND 0x388358, 0x388e2c, 0x389400) and are false
until input is fed; `LevelWorld::translations` for `LAMS.IsInLams` and `Translate` (FOUND 0x159c1c;
`level_scripts` loads the English base and patch tables, 29,477 keys); the global-settings blends
(`Blend`: start, target, t) for water level, gravity, wave, murkiness and water bits, the tint, the
lighting group and the 8 blend factors, with 9 setters, the blend factor and 5 water getters (FOUND
0xce608, 0xd9dac, 0xd623c, 0x5ec6f8); `lbp-level` keeps the stored wave, gravity, tint, murkiness, bits and
drain-sound flag. From kept state: `GetGameFrame`, `GetNumYellowHeadAvatars`, `GetGameScoreDataType`,
`IsSlotIDValidForScoring`, `Messaging_ShouldFilterMessageStrings`, `LoadAnimatedTextureByFilename`.
Constants: `Poppet.GetTextEntryType` 2 and `IsTarsierDLCEnabled` true (FOUND literals); no off-screen
indicator, no key names, no modal overlay, no items known at start (INFERRED for one local player).
Recorded with their arguments in `LevelWorld::requests`: the trigger messages, the level-link and wormhole
prompts, game events, overlay callbacks, speech keys, the audio settings. PC run, 378 levels, 60 ticks,
every output pulsed: unbound natives called 110 -> 88, unbound calls 952,819 -> 766,940, recorded-only
calls 17,206 -> 83,820; Things stopped by a fault stay at 4 in all three runs. Two faults of a first run
are settled: `IsTarsierDLCEnabled` true led the level links to `Slot.GetSlotGroup`, now bound (FOUND
0x640500 -> 0x7477d8: a new SlotID from the slot's group pair, (0, 0) for a slot the database lacks, never
null; the world answers the empty slot); and `Race_End.Enter` read `Race_Start.ColorIndex` on a Thing of
another class because every script cast passed. `LevelWorld::thing_is` now tests the Thing's class chain.
Not done: feeding the pads and the tables in the viewer, the slot database,
`GetNumKnownItemsInLevelAtStart`, the audio getters. +0 viewable, +0 Complete. `docs/level-scripts.md`
part 14.

## 2026-10-13 - D262 the level scripts get world, position, player and game state

RScript plan stage 4. `LevelWorld` now answers what the scripts ask the engine about the level. First half:
`part_name` kept dropping the P of `Pos` (fixed, with four mismatched part names); `lbp-level` keeps the PWorld
frame, game mode, pause flag, water level and the two global-setting arrays; `PWorld.GameMode` / `Frame` /
`WaterLevel`, `PPos.WorldPosition` and three PTrigger members are real state; local matrices are derived at
load and `SetLocalPosition__m` / `SetWorldPosition__m` move the descendants (FOUND 0xf2e80, 0xf26bc);
`GetWorldPosition__` returns the matrix; the VM indexes an m44 as column * 4 + row (FOUND in the script
tables: column W is the position in 115 uses); `GetLocalPosition__`, `GetMicroChipProxyFromThing`,
`GetWorldPositionForSensor__` and `Game.IsOnSequencer` are bound. Second half: player records (score,
multiplier counter, timer, source, visible, number, team; FOUND 0x156b18) with the player queries and the score
group, `SetPlayerScore` clamped to 0.0 .. 4,000,000.0 (FOUND 0x3970e4); `Get/SetGlobalSettingUID` with the
priority rule (FOUND 0xce554); SlotID objects for `GetLevelSlotID__` / `GetPrimaryLevelSlotID__`;
`IsVcrPaused`, `IsTutorialLevel`, `IsPSNDemoBuild`, `IsSafeToTriggerMusic`, `FindLockMesh`; nine calls that are
recorded and do nothing else (`natives::RECORDED_ONLY`). 123 -> 164 natives in the table. INFERRED: one local
player numbered 0, `IsVcrPaused` false in play (IsPaused is stored true in 220 of 236 levels), the frame steps
at the end of a tick, the proxy is the outermost chip, the level's slot is 0 / 0. PC run, 378 levels, 60
ticks: Things stopped by a fault 597 -> 4 (598 -> 4 with the trigger visit, 1,794 -> 4 with every output
pulsed); fault kinds 11 -> 2; levels with a fault 221 -> 2; reaction calls 240 -> 262 (4,397 -> 5,704 pulsed).
Left: `Thing.GetCOM__` (3 Things) and one null Thing in the Pod. Not done: pad input, `Game.GetGameFrame`, the
Sackbot group, HUD, messaging, the level's slot. +0 viewable, +0 Complete. `docs/level-scripts.md` part 13.

## 2026-10-13 - D261 the switch network runs and switch reactions reach the scripts

RScript plan stage 3b. `lbp-runtime/src/switches.rs`: `LevelWorld::update_switches` orders the switches by
their wiring once (cycles broken, a back wire reads last tick's value) and each tick computes every output,
writes it where the 3a input natives read it, and applies it to its targets with the game's edge rule (FOUND
0x1244a4, 0x11ddf0). Computed types (rules FOUND in 0x11bd58 and its jump table): microchip, circuit board,
NOT / node, AND, OR, XOR, battery, direction, sign split, toggle, selector, counter, timer, with the inverted
rule; sensors and the randomiser keep the stored signal. A microchip that is off forces the outputs of
everything on its board to {0, 0, -1} (Thing+0x7f). `SwitchPerformReaction__b(on)` is called on the target
Thing's own script when its script GUID is one of the 16 the game lists, `__bf(on, 1.0 or 0.0)` for one GUID;
a Thing that the game classifies by another part or by its switch type first gets no call (FOUND 0x3ed2c).
`lbp-level` keeps the PSwitch logic fields and the PMicrochip board. `level_scripts`: logic census, switch
counters, `--pulse-switches`. PC run, 378 levels, 60 ticks: 24,612 of 144,608 outputs differ from the stored
value; 204 `__b` and 36 `__bf` calls run (222 and 37 with the trigger visit, 3,052 and 1,345 with every
output pulsed); stopped Things 582 -> 597 (the reactions reach two new faults); 19 more unbound natives
reached. Not done: sensors, the randomiser, dirty flags, sequencer boards. +0 viewable, +0 Complete.
`docs/level-scripts.md` part 12.

## 2026-10-13 - D260 switch inputs reach the level scripts

RScript plan stage 3a. `lbp-level` keeps PSwitch (outputs with signal and targets, type, inverted,
behavior, manualActivation). `LevelWorld::wire_inputs` builds each Thing's input table from the outputs'
targets (INFERRED; the table itself and its readers FOUND in the EBOOT: 0x7541e0, 0x754308, 0x7542a4,
0x75437c, 0x754ccc, 0x754244, 0x754af8, 0x6d558c). `IsSwitchTriggered__i` is "an output is wired", Binary is
"ternary not 0", Analogue the activation or 0.0, Player the signal's player. Newly bound:
`Thing.GetSwitchActivationTernary__i`, `GetInput__Q5Thingi`, `GetOutput__Q5Thingii`,
`Native.GetInThing__Q5Thingi`, `BaseWorld.IsScoreBoard__Q5Thing` (123). `level_scripts`: switch census. PC
run, 378 levels: 141,572 PSwitch Things, 144,608 outputs, 177,800 targets, 136,337 Things with an input
table, no port collision, 3,732 scripted Things with a wired port; stopped Things 1,722 -> 582; unbound
natives reached with the trigger visit 94 -> 91. Not done: the logic network is not evaluated and
`SwitchPerformReaction__b` is not called. +0 viewable, +0 Complete. `docs/level-scripts.md` part 11.

## 2026-10-12 - D259 trigger events reach the level scripts

RScript plan stage 2. `lbp-level` keeps PTrigger. `LevelWorld::add_player`/`move_player` put a player's
Sackboy in the script world; `update_triggers` runs every trigger against the players and
`LevelScripts::tick_level` calls `Enter__Q5Thing`, `Inside__Q5Thing` and `Leave__Q5Thing`. Rule FOUND in
the EBOOT (0xe3bc0, 0x1364f0, 0x135fe0, 0x135a18, 0x1346fc): position in the trigger Thing's space; types
0 circle, 1 box, 4 sphere; radius multiplier times the hysteresis multiplier while something is inside;
the trigger's z plus offset must lie in the z span of Sackboy's body unless all layers; Enter and Leave go
to the Thing's script or its parent's, Inside to the parent's. Bound: `Native.GetNumInThings__Q5Thing`,
`PartList.HasPYellowHead__Q5Thing` (118). Viewer: the play-mode Sackboy is the player. `level_scripts`:
trigger census and `--visit-triggers`. PC run, 378 levels: 23,990 triggers, 4,662 scripted; visiting each:
11,430 Enter, 895 Inside, 7,357 Leave calls run, stopped Things unchanged at 1,722, unbound natives
reached 65 -> 94. +0 viewable, +0 Complete. `docs/level-scripts.md` part 10.

## 2026-10-12 - D258 scripts run past unbound natives, counted

RScript plan stage 1. `Bindings::default_unbound`: a native without a binding returns the default of its
return type and is counted as unbound, never as bound; on in the viewer and `level_scripts --play`, off in
tests (`--strict` in the tool). `level_scripts --all --play=N` plays every level on its own VM and reports
bound and unbound calls, each unbound native, and the faults left. 19 natives bound (116): String and StringA
length, find, substring, empty test, upper case, conversion; Math identity, ceil, exp, pow, sign, lerp,
Delt, BiasCosine (rules FOUND in the EBOOT; StringA forms INFERRED); `LoadTextureByFilename__gb`. PC run,
378 levels, 60 ticks, 13,547 scripted Things: stopped by a fault 4,940 -> 1,722; bound calls 26,186 ->
51,030; 76,269 calls to 64 unbound natives defaulted. Largest stops left: switch inputs 1,143,
`PWorld.GameMode` 220, local transforms 199. +0 viewable, +0 Complete. `docs/level-scripts.md` part 9.

## 2026-10-12 - D257 ledge-fall jump layer weight

Walking off a ledge, the game's jump layer is at full weight about 10 ticks after the edge; ours stopped at
0.75 to 0.78 (the D234 ramp). MEASURED on three RPCS3 falls in Rookie Test (two at full speed, one leaving at
about 12 per tick), lower-body vertex fits per tick of lerp(ground pose, jump pose, w): the fitted w follows
the 0x20a5f4 rule with the peak as a plain eased follower from 0, peak += 0.125 (|vy| - peak), weight =
clamp(peak/10) times the blend-in, with no tuned constant. Error in w over 35 fall ticks: 0.064 (D234 ramp
0.276, the same rule with a peak that rises at once 0.323); lower-body vertex rms 1.76 against 4.12, free-fit
floor 1.60. The landing tail then plays at full weight times its fade (MEASURED). INFERRED from the second
drop in each capture (he leaves the ground again 5 to 9 ticks after landing and starts at 0.83 to 0.94): the
follower holds while the landing tail plays and that fall has no new blend-in. A jump from the ground is
unchanged (the take-off sets the peak to full). `JUMP_AIR_*` removed (`lbp-sim`); `pose_fit` has a `STACK`
probe. Not viewed in the viewer: the blank level has no ledge. `docs/d250-air-lean-turn.md` (D257).

## 2026-10-12 - D256 standing foot placement

The game's feet stand 43.4 apart, ours stood 51.9. FOUND in the EBOOT (foot pass 0x210dd8, solver 0x6c15b8):
while he is on the ground and his ground-relative speed is under 1, each Foot joint is put by a two-bone leg
solve on the floor point nearest its leg anchor (LimbAnchorPos, 25 to either side of the body); the placement
eases in over 3 ticks and out over 5. MEASURED: our stand clip has the Foot joints at +29.9 and -28.7 (58.6
apart), which is what the rule needs to explain the gap. `FOOT_PLANT_*` and `Sackboy::foot_plant` (`lbp-sim`),
applied in the viewer's `blend_frame` with the D249 leg solver. MEASURED after: spacing 43.6, standing vertex
rms against the game 1.30 -> 0.79, legs 4.63 -> 1.52. Depth target (the body's depth line) is MEASURED, the
lane-change gate INFERRED. `docs/d250-air-lean-turn.md` (D256).

## 2026-10-12 - D255 stick response, shortest jump, mirrored morphs; arm terms read

Four of the open Sackboy items, three from data already on disk. Speed to the right: MEASURED again in the
d211 Moon reads, not level-specific (29.34 on the ground, 22.00 in the air, against 30.00 and 22.50 to the
left; same acceleration). FOUND in the EBOOT (0x38a578, 0x38aa10): the stick byte less 128 is divided by 128
with no clamp, then the part past the dead zone is rescaled and squared, and the result scales both speed
caps; with the dead zone 0.3 (INFERRED, a run-time setting) full right gives 0.9778, which is both measured
ratios. `stick_response`, `STICK_FULL_RIGHT`; the keys, the script and the pad stick use it. Shortest jump:
the d211 presses seen for one and two ticks rise 12.0 and 12.0 + 19.3 and then fall (peaks 33.1 and 90.4);
`JUMP_MIN_TICKS` 3 -> 1. Mirrored morphs: FOUND in the pose mirror (0x1e42b4), morph i takes the weight of
morph `mirrorMorphs[i]`; `Rig::mirrored_morphs`, used by `lerp_mirrored`. Air ease arm terms: FOUND (0x1fb0e4,
0x2097e0), they are the eased held-arm-button weights (0.2 per tick) and stay 0 without those buttons or a
second player holding him, so they are not the 2 to 3 degrees left on the upper arms; no change. MEASURED,
open: standing foot spacing is 43.4 in every game capture (both levels) against our 51.9 (feet 4.1 and 3.3
further out); the resting height after a landing varies 0.5 to 1.5 above the spawn height (shape cast
distances; landing with no gap sank him 2.7 at times, not kept). `docs/d250-air-lean-turn.md` (D255).

## 2026-10-12 - D254 mirrored poses use the game's per-bone flip types

The left-to-right turn fitted worse than the right-to-left one (a front torso bone, the zip pull, up to 15
units off; hands and fingers about 5). FOUND in the EBOOT (pose mirror 0x1e42b4, called from 0x1ebb48): the
game mirrors each bone in its own local frame, taking the partner bone's local position, rotation and scale
through one of six rules picked by the skeleton's mirror flip type; types 6 and up copy the partner unchanged
(the two zip-pull bones). Ours reflected skinning matrices across x = 0, which needs a left/right symmetric
bind (Sackboy's hands differ by 6 units) and reflected the zip pull. `Rig::mirrored_trs` now applies the
game's rule when the skeleton has flip types. MEASURED on the left-to-right turn capture: vertex rms (free
yaw) on the turn frames 2.3..4.2 -> 0.5..1.4, mirrored steady walk 0.7..1.6, the zip-pull joint 15.1 -> under
1.7. `docs/d250-air-lean-turn.md` (D254). Walk start: MEASURED, no change. The game's walk phase at the first
moving tick differed between five captures (0, 0, 3, 4, 4), as the free-running 13-tick counter gives; with
our counter on the game's phase the whole start (speed 3 to 30) fits at 0.9..1.7.

## 2026-10-12 - D250 jump air lean (upper body eased toward stand in the air); D251 turn facing and walk restart

D250: the +4 degree air lean was one joint (Spine1, up to 5.5 degrees) plus the upper arms. FOUND in the EBOOT
(0x20e28c state +0x6ec easing 0.1 toward 1 in the air, arm pass 0x208dec, masked lerp 0x1eaccc toward stand
frame 0 with weight s (1 - s) on Spine1 and below except the head). In `sackboy_anim.rs` as `AIR_EASE` and
`Blend::UpperBody`; `lbp-preview` has `lerp_masked` / `subtree_mask`. MEASURED: Spine1 pitch now within 0.05
degrees of the game on all 27 flight ticks. D251: first game captures of a direction reversal (three
right-to-left, one left-to-right, one stop). Pose, speed and clip timing already matched; the facing did not
(the game eases toward the speed angle through the turn, ours held 72 degrees on the stick's side, up to 111
degrees off) and the walk restarts on frame 1, not 0. Both fixed; vertex rms through the turn 1.9..3.8 (was
48..98). `docs/d250-air-lean-turn.md`.

## 2026-10-12 - D249 short legs after a landing: two-bone leg solver, landing tables to twelve ticks

`BlendPose::move_chain_end` (lbp-preview) moves a thigh-calf-foot chain's end joint, keeping the bone lengths,
the knee's bend plane and the foot's rotation. The Sackboy play figure raises both ankles by `LANDING_SQUASH`
(0, 0, 7.6, 6.0, 4.4, 3.0, 1.6, 0.5) and `LANDING_DIP` continues past its two under-floor ticks with the
body's settle (-6.6, -3.3, -0.5, 2.7, 0.1, -2.8, -2.4, -1.4, -0.8, -0.3). MEASURED against d234 `jump.bin`:
touchdown +3..+8 height, pelvis height and lowest vertex within 0.4 of the game (height was 8 off). Both
tables are one capture's series; the game's foot pass (0x210dd8) inputs are untraced. `docs/d241-landing-dip.md`.

## 2026-10-12 - D248 lean/height measured (no change); crouch spring found, not the landing squash

MEASURED tick-aligned against the D234 captures: idle, steady walk and stop match the game within 0.4 deg of
lean and 0.4 units of height; nothing changed. Read-only Ghidra pass for an extra in-air body layer: none
(phys+0x41 and creature +0x874..+0x880 are the wall-cling state and its head look, clip-mix weight 0x80c is
grab only); open candidates for the +4 deg air lean are the head-look tilt from the physical body's rotation
(0x200c6c) and the peak follower / landing-branch conditions of 0x20a5f4. The `stretch_crouch` spring
(0x2011d4) is now in `sackboy_anim.rs` as `CrouchSpring` with tests but unfed: fitted as a landing kick it
misses the game by 3.9+ units, shortens the spine where the game does not, and peaks two ticks late. The
squash looks like feet held on the floor (foot pass 0x210dd8) under a body still rising for ten ticks.
`LANDING_DIP` unchanged. Vertex dumps now log a pose line per tick shown. `docs/d241-landing-dip.md` (D248).

## 2026-10-12 - D247 landing starts on the touchdown tick, two-tick dip; stop pose measured

Stop pose MEASURED tick-aligned against the D234 walk capture: fit mean 1.56 against 1.51 for the steady walk
(the floor of the method), so it matches the game and nothing changed; the old "fit 3-5" was the fit tool not
converging at 72 deg yaw. The turn still needs a game capture with a direction reversal. Landing (d234
`jump.bin`): the game starts the tail on the tick the feet reach the floor, ours a tick later because
`SackboyBody::grounded` was the floor probe taken before the move; it is now the contact after the move
(controller step unchanged). `LANDING_DIP` = -25.9, -15.9, 0 (was -21.2, -23.5, -10.7, 0). Touchdown +1/+2:
height game 206.0/202.9, ours 206.3/202.6 (was 201.9/206.3); lowest point game -29.0/-16.2, ours -29.0/-16.1
(was -20.2/-26.6, then -10.9 where the game is at 0). Flight ticks unchanged. `docs/d241-landing-dip.md`.

## 2026-10-12 - D246 body scale stays on the scaled bone; stand clip at full weight

Owner: the idle still stretches the body too much. Cause FOUND in the rig data: the stand clip breathes by
scaling the two spine bones (up to 9% along the spine) and the viewer multiplied plain matrices, so both
legs, the chest, neck, head and arms inherited that scale (head and hands 0.87..1.12 over a breath at full
weight). Sackboy's skeleton is a 3ds Max Biped (bones named `Bip...`) plus plain bones (zip pulls, eyes,
jaw, tongue); Biped links do not pass scale on. New `ScaleRule::Biped` in `lbp-preview` skinning: a Biped
bone sits in its parent's unscaled frame with its offset multiplied by the parent's scale, and its own scale
applies to its own vertices only; other bones inherit as before (the zip pull's clip scale is the inverse of
the chest's, so it has to). Used by the Sackboy play figure only; every other preview keeps plain
inheritance. GPU skinning gets a scale node under each joint for such a rig. `STAND_BASE_WEIGHT` is back to
1: D242-D244 lowered it to hide this stretch, and the capture they tuned against (D243 idle: height
198.3..200.9) is not the neutral stand (right hand 51 units above the pelvis, body leaning; no stand or
mood-stand clip fits it better than 14 units per vertex).

MEASURED against the clean D234 captures (one vertex buffer per tick): standing mesh height per tick, game
201.1..206.4, ours 200.8..206.1, tick-aligned RMS 0.34 (before: 198.3..204.4, RMS 2.5; order-free vertex
fit of the stand clip 0.73 against 1.62 with inherited scale, noise floor 0.7). Jump, 6-tick press, ticks
1..20: max 2.3 / RMS 1.4 (before 2.4 / 1.5; the jump clips carry no body scale), feet within 1 unit of the
game's on every tick up to the apex (242.9 / 241.9), take-off step 201.3 -> 214.4 (game 201.3 -> 214.5).
Rigid-bone check on our dump: thighs, feet, head, hands at scale 1.000 (before 0.96..1.03), eyelids still
close to 0.08 in a blink. The D245 height test now uses the clean capture and our own controller. Open: the
landing (the game dips to 195 for three ticks after touchdown, ours stays at 200; unchanged by this), what
the D243 idle pose is, and the game's own bone composition is not traced (rule INFERRED from vertex data).
`docs/d242-idle-stretch.md`. +2 tests.

## 2026-10-12 - D245 jump pose mixes the tap and held clips

Owner: the take-off tucks the legs too early. The EBOOT jump layer (0x20a5f4, 0x209fc8) confirms our frame
rule (tick 1..5, then `8 * (1 - vy / 73)`), so the handoff's guess of a different vy scale was wrong. What we
lacked: the pose is `lerp(jump_small_*, jump_*, held ticks / 10)`, the count frozen when the button is
released, and the layer is at full weight after a take-off. We drew jump_stand alone at 0.7-0.9. Added
jump_small_stand / jump_small_walk (56 clips), `JUMP_HOLD_FULL` = 10. Against the D243 capture (6-tick hold)
the mesh height per tick is within 4.6 (rms 2.3; jump_stand alone was 20 low) when the pose is driven with the
capture's vertical speed; in the viewer, on our own 6-tick arc, it measured max 7.0 / rms 4.8. Later check: the
D243 capture is not a clean 6-tick standing jump (feet apex +277, 106 units of sideways drift); against the
clean D234 capture (same 200 ms press) the viewer's height is within 2.4 and its feet rise within 1.8, so the
arc was left alone. Ledge falls keep the D234 weight ramp (not re-measured). +0 / +0.

## 2026-10-12 - D244 idle weight reaches the viewer

Owner: the idle still stretches vertically in the viewer. The viewer's pose blender forced the base layer to
weight 1, so `STAND_BASE_WEIGHT` (D242) never changed the picture. Our dumped idle vertices: head top travelled
18.5 units per breath (game 2.6); with the weight honoured 6.1 at 0.3 and about 4.6 at 0.25. `STAND_BASE_WEIGHT`
= 0.25 (chest and head-top joint travel on the game's 2.2 / 2.9). docs/d242-idle-stretch.md. +0 / +0.

## 2026-10-12 � D242 idle breathing at half weight

Owner: the body stretches vertically too much in the idle animation. Game idle vertices vs our stand clip,
per joint: same period, but head/chest move 2-3 units (ours 7-11) and sit 3-6 higher, fitting the stand
clip at about 0.5 over the bind pose. `STAND_BASE_WEIGHT` = 0.5 (inferred weight, measured amplitude).
docs/d242-idle-stretch.md. +0 / +0.

## 2026-10-12 � D241 landing dip matched to the game's vertices

Owner note: Sackboy "extra bouncy, stretching vertically". Game vs ours, same jump: the vertical arc matches
within 3 units (centroid +234 apex both), but on landing our drawn figure sank 35 + 5.5 below rest for
about 15 ticks, the game's lowest vertex reaches 29 and is back at rest 3 ticks later. `LANDING_DIP` is
now 4 ticks (-21.2, -23.5, -10.7, 0). docs/d241-landing-dip.md. +0 / +0.

## 2026-10-11 — D239 Popit Personal page tiles

"Tools and Used Objects" now draws its 14 function tiles (names from the game's tooltips, icons from the
64x64 plan textures: glyph = R - B) and the focusable Gameplay Kits header with its item.
docs/d235-popit.md pass 6. +0 / +0.

## 2026-10-11 — D235 Popit overlay (create and play menus)

`--play-sackboy` draws the Popit: gradient card, SDF icon grid in the script's order (create 3x3 with
the cursor focused, play 6/4,0/7,3), focus tile, tooltip pill, "Close Popit" prompt, cord, 10-tick
smoothstep open and a camera nudge. Item data from poppet.ff (Ghidra pass); sizes/colours measured
on RPCS3 frames. docs/d235-popit.md. +0 / +0.

## 2026-10-11 — D234 Sackboy animation fitted to the game's own vertices

RPCS3's CPU-skinned Sackboy vertices fitted to our clips and blended play poses (pose_fit,
LBP_DUMP_VERTS). Walk right 0.8 and walk left 1.5 match; the jump layer sat at full weight in
the air where the game shows 0.7-0.9, so the air weight is now 0.7 rising to 0.9 (jump fit
2.4-2.9 -> 1.3-1.5, landing arms 12 -> 2). docs/d234-anim-vertex-fit.md. +0 / +0.

## 2026-10-11 — D233 Sackboy moods, emotes and head look (not compiled yet)

Mood d-pad presses (levels 1-3, opposite cancels), anticipation -> emote -> to_neutral, mood
stands by eased weight, three additive look layers (neutral: no look inputs modelled). 33
new clips. cargo/python were refused by the session gate: fmt, clippy and tests still to
run. docs/d233-sackboy-moods-headlook.md. +0 / +0.

## 2026-10-11 — D232 open items from Ghidra pass 2

Idle roll 0..249, lane layered after jump, idle only after 300 quiet ticks, seed 42 and a
tick-0 blink (all FOUND); landing k = 1 and walk_z sign confirmed. Crouch input, emotes and
head look traced, not implemented. docs/d232-sackboy-anim-rules.md. +0 / +0.

## 2026-10-11 — D232 QA fixes: Sackboy facing, mirror, landing, blank spawn

Clip root rotation (the facing channel, +90 in walk/idle clips) is held at bind so the
picker's yaw no longer doubles; left travel mirrors layers via the skeleton mirror table;
stop/turn hold +-72 deg; landing tail starts at 13-14; spawn no longer inside the floor;
--qa-record. docs/d232-sackboy-anim-rules.md. +0 / +0.

## 2026-10-11 — D232 Sackboy animation follows the EBOOT rules

Weighted walk layers over stand with one 13-tick phase, eased facing (0.4/tick), vy-driven jump
frame and landing tail, stop/turn/lane ramps, idle chains and blink (lbp-sim SackboyAnim), blended
per bone in the viewer (lbp-preview BlendPose). docs/d232-sackboy-anim-rules.md. +0 / +0.

## 2026-10-11 — D231 Sackboy plays his own clips in play mode

--play-sackboy draws mesh 0000043f by default and picks stand / walk bands / lane / jump / land
squash per tick (lbp-sim sackboy_anim, rules INFERRED) from a 9-clip bank on one rig; hard cuts.
Replaces the Sackbot walk 0001a697. docs/d231-sackboy-animations.md. +0 viewable / +0 Complete.

## 2026-10-10 — P3m levels are lit by their presets blended by PWorld lighting

Stored presets are spread over PWorld `lighting` and blended; `darkness` scales exposure and fog colour.
Matches the live blocks of tutorial_23, Bravery Test and the Moon level to 4 digits. Replaces preset 0
(also for baked AO) and the main block, whose blue 1000..4000 fog drowned levels under --preview-lighting.
Level mode is LBP-lit by default (P2w). docs/p3m-level-lighting.md. +0 viewable / +0 Complete.

## 2026-10-10 — P3l cube-map reflections (output port 6)

Reflection graphs with one BC1/BC3 cube map (59 graphs) now draw: six faces as a mipped cube
texture, sample steps on it read R.xyz with the TXB bias as gradient scale. Face order, face
orientation and handedness INFERRED. Several cubes in one graph are named, not drawn. Docs:
docs/p3l-cube-reflection.md. Not rendered in the cloud; PC verifies a cube-map gmat.

## 2026-10-10 — P3k baked AO (backdrop lightmap)

Under --lbp-lighting the backdrop static mesh's lightmap (B8 GTF, SMH UV1) is bound as an R8
mipped texture; the texel enters as shadowmap.x (sun and rim) and ao = bakedAOScale + bakedAOBias
(PLevelSettings, now parsed) scales ambient and sun (fpdec arithmetic; the binding is INFERRED). `--no-baked-ao` for A/B. A material
that already uses UV1 keeps it and the AO is named, not drawn. Docs: docs/p3k-baked-ao.md.
Not rendered in the cloud; PC verifies Bravery Test / tutorial_23 backdrops against RPCS3.

## 2026-10-10 — P3i reflection (output port 6)

Under --lbp-lighting the port-6 graph is evaluated per fragment at the reflection vector R
(2D maps, ReflectionBlur bias) and added x luma(light); cube maps and diffuse-graph materials
are named, not drawn. View space, luma weights, scale 1 INFERRED. Docs: docs/p3i-reflection.md.
Not rendered in the cloud; PC verifies metal/glass, census.

## 2026-10-10 — P3h glow (output port 4)

Under --lbp-lighting port 4 is loaded through the diffuse route (port 4 stood in for port 0)
and added unlit before the sqrt output: a solid colour or one baked map on a free UV set.
Docs: docs/p3h-glow.md. Not rendered in the cloud; PC verifies lamps/screens, census.

## 2026-10-10 - P3j shader scalars (bump x2, cosine power x22)

The shader constants are fixed transforms of the stored GfxMaterial scalars (PC CGB patch lists,
644 gmats, 0 exceptions): BumpLevel = 2 x stored, CosinePower = 22 x stored (blur = stored - 1 lands
with P3i). Surface now applies them; gloss powers and bump strength change. PC verifies.

## 2026-10-10 — P3g dynamic sun shadows

`--lbp-lighting` adds one cascaded Bevy sun-shadow map (3 cascades, 1024) that darkens the
sun term, sun gloss and specular; Pod (shell and planets) excluded; `--no-lbp-shadows` opts out. Docs:
docs/p3g-sun-shadows.md. Not rendered in the cloud; PC verifies acne/peter-panning.

## 2026-10-09 — P2x level fog from the level's own settings

Under the LBP-shaped lighting a level's fog uses its FogNear/FogFar/FogColor (game formula from
the PC Ghidra read); the Pod keeps its fitted live-game constants. +0 viewable / +0 Complete;
PC verifies shots (docs/p2x-level-fog.md).

## 2026-10-10 — P3f gloss weights use the stored colour .w

Sun, ambient, rim and rim2 .w for the gloss term = stored .w x the game factor (rim and rim2
separately); rgb unchanged. Docs: docs/p3f-gloss-weights.md. Gloss only under --lbp-lighting.

## 2026-10-10 — P3d gloss colour maps

Port 2 wired from one sampler (2,132 graphs) multiplies the gloss term under --lbp-lighting;
UV from the diffuse coordinates or a free UV1. Docs: docs/p3d-gloss-map.md. PC verifies shots.

## 2026-10-09 — P3a alpha-test cut-out under the LBP-shaped lighting

Materials with a stored AlphaTestLevel discard fragments below it when the LBP surface extras are on
(docs/p3a-alpha-test.md). +0 viewable / +0 Complete; PC verifies a cut-out level and the Pod.

## 2026-10-10 — P3e PWorld lighting fields

World keeps the lighting/colourCorrection/fog/fogTint/darkness f32 and the GlobalSettings
target/from + blend; the settings probe prints them. Question: which backdrop preset a level
selects. Docs: docs/p3e-world-lighting.md. No viewer change.

## 2026-10-10 — P3b Pod lighting from the backdrop plan's settings

Pod `--lbp-lighting` takes preset 0 of the PLevelSettings holder in the backdrop plan
(live source, probes #104/#105), also under plain --preview-lighting (its main block was the wrong holder). Docs: docs/p3b-backdrop-settings.md. PC verifies against RPCS3.

## 2026-10-09 — D194 level shapes take their own PShape.color

Shape batches split per (gfx material, PShape.color) and, under the level defaults, load
with that colour as the instance colour. Targets the 68 grey shapes in Bravery Test
(gfx 00012ba7, 00018401). --level-report prints shape-batch reasons and Things.
+0 viewable / +0 Complete expected; PC verifies.

## 2026-10-10 — P3c output-port census example

Read-only example counting what feeds each material output port (specular, glow, reflection
planning). No viewer change. Docs: docs/p3c-output-ports.md. PC runs it on the corpus.

## 2026-10-09 — P2v game normals for model meshes

Model meshes are lit with the game's decoded normals instead of normals generated from our
triangles (fallback per vertex; skinned per-bone copies and shapes unchanged). Per-mesh detail
reports the agreement with generated normals. +0 viewable / +0 Complete; PC verifies shots.

## 2026-10-09 — P2u mipmaps and anisotropic filtering

Game textures get a CPU box-filtered mip chain (linear-light for colour) and a trilinear
sampler (anisotropy 1, as the game); point-sampled textures stay nearest. Pure viewer change, +0 viewable /
+0 Complete. Sampler matches the Pod capture (docs/p2u-mipmaps.md); PC verifies shots.

## 2026-10-09 — P2w level scenes use the LBP-shaped lighting by default

Level mode turns on the LBP-shaped lighting (RSX-fitted on the Pod) unless --preview-lighting
(Lambert) or --no-lbp-lighting (unlit) is given. Moon-level block/preset still INFERRED until the
requested capture (docs/p2w-level-lighting-default.md). +0 viewable / +0 Complete; PC verifies.

## 2026-10-09 — D193 sampler port folds under the nested instance tint

Folded sampler ports (D96) and the nested ThingColor tint now combine instead of refusing;
both rewrites are exact. Targets the last 22 neutral Things in Bravery Test (mesh 00019e98).
+0 viewable / +0 Complete expected; PC verifies.

## 2026-10-09 — B179 PC verification of D192
- STATUS.md B179: #95 checks, Bravery Test 35 -> 22 neutral, census unchanged, captures.

## 2026-10-09 — D192 material-less primitives hidden

The game skips primitives that have no material (Ghidra, confirmed), so the viewer now hides
them instead of drawing them grey; levels stop counting them as neutral. --level-report names
the reason per neutral mesh. +0 viewable / +0 Complete.

## 2026-10-09 — D191 level scenes use asset defaults

Levels now draw Things with the new preview defaults and their editorColor tint. Bravery Test
neutral Things 195 -> 35; tint values not yet checked visually. +0 viewable / +0 Complete.

## 2026-10-09 — D188-D190 preview defaults, mixed-UV graphs, material fixes

Single-mesh previews now default to animation base values and a white instance colour; mixed
UV0/UV1 blend graphs and a few small material cases are supported. Corpus all-diffuse 3,798 ->
4,239 of 4,261 meshes (0 lost). +0 viewable / +0 Complete.

## 2026-10-09 — D160-D175 Pod vs RPCS3, threads, physics

Pod closed window, side-panel wall art and in-use camera matched to RPCS3; script threads and
viewer level scripts; MULm44 order fixed; lbp-sim physics merged (Rookie Test stays put).
+0 viewable / +0 Complete.

## 2026-10-08 — D151-D159 Pod menu, decorations, runtime ticks

Pod main menu renders over the real Pod level with a solved camera; 150 level decorations
drawn; level scripts run per tick. +150 viewable (decorations) / +0 Complete.

## 2026-10-08 — D146-D150 level polish, UI objects, runtime

Sackboy shown upright facing the camera; Rookie Test joints skinned; three Da Vinci
levels render; main-menu script runs cleanly; level-script runtime resolves all 13,547
scripted things. +0 / +0.

## 2026-10-08 — D145 script VM arrays

All 47 of the game's test functions run; 512/513 checks pass. S9 waits for My Moon to
unlock. +0 / +0.

## 2026-10-08 — D143, D144 X1 measured; s64/m44 VM fixes

RPCS3: the game updates at 30 Hz (one tick per 2 vblanks). Script VM: 479/480 test checks
pass. S9 waits on getting past the Introduction level. +0 / +0.

## 2026-10-08 — D142 test counting; s64 constants are zigzag

Test* functions: 476 checks pass, 4 fail. The script s64 constant table is zigzag-encoded
(explains the TestS64 failures). +0 / +0.

## 2026-10-08 — D141 script VM objects

The game's own test script: 850 checks pass, 7 fail (was 826/27). +0 / +0.

## 2026-10-08 — D140 script menu in the viewer

The viewer draws the main menu from the game's own script (five English buttons;
placeholder layout). +0 / +0.

## 2026-10-08 — D136 Gooey trace

The game's main-menu script now runs headless and lays out its real English buttons
(Story, Community, Recent Activity, Store, Me). +0 / +0.

## 2026-10-08 — D139 VM operand fixes

The game's test script: 44/47 functions finish, 826 checks pass, 27 fail. +0 / +0.

## 2026-10-08 — D138 script tests part 3

The game's test script: 37/47 functions finish, 589 checks pass, 11 fail. Gooey trace
(#34) still held. +0 / +0.

## 2026-10-08 — D137 script tests

VM binds 2,970/17,704 native calls; the game's own test script passes 532 checks, fails
3. Gooey trace (#34) still held; RPCS3 input awaits the user's OK. +0 / +0.

## 2026-10-08 — D135 native bindings

40 VM native bindings merged: 2,347 of 17,704 native calls bound (690/2,472 in gameplay
scripts). PR #34 Gooey trace held back (no labels yet). +0 / +0.

## 2026-10-08 — D134 U10 name sentinel

Unnamed type rows (0xFFFFFFFF) accepted: all script listing checks now 0. +0 / +0.

## 2026-10-08 — D133 real scripts

patch.edat decrypted in RPCS3: 426/426 scripts read, Toolkit 3/3, G10 native-call census,
branch rule confirmed (from current instruction). +0 viewable / +0 Complete.

## 2026-10-08 — D132 strings and fonts browser

--text-browser merged (21/21 tables, 583,648 keys); OS font fallback fixes missing glyphs
such as ™. +0 viewable / +0 Complete.

## 2026-10-08 — D131 inventory browser

--popit browser merged: 12,161 items on 27 pages, 9,971/9,971 distinct icons decode.
+0 viewable / +0 Complete.

## 2026-10-08 — D129, D130, RPCS3 setup

Item details: all 225 stopped plans now read (12,161/12,245; 84 Vita excluded). Story slot
browser merged (164 slots, 164 icons). RPCS3 firmware + 01.33 update installed; game boot
and patch.edat decryption blocked. +0 viewable / +0 Complete.

## 2026-10-08 — D127, D128

Item details reader: 11,936 of 12,245 plans complete on 01.33. Script VM crate merged
(synthetic tests; real scripts wait for patch.edat). +0 viewable / +0 Complete.

## 2026-10-08 — D125, D126

Runtime decisions doc merged (AGENTS scope amendment, RPCS3 plan). RScript reader merged;
0 scripts readable on 01.33 (bytes likely in encrypted patch.edat), G10 blocked.

## 2026-10-08 — D124 U4/U6 follow-ups

testrnp.slt is an expected text-form exclusion (U4 census 15 complete + 1 excluded);
UI demo no longer rebuilds every frame. +0 viewable / +0 Complete.

## 2026-10-08 — Level v2 + queued PRs (D115-D123)

tutorial_23 now draws its workshop backdrop, inverse-bind props, opt-in bevels and level
lighting. Merged skin/morphs, animation keys, UI layer, Sackboy animation preview (GPU =
CPU), slot/pin readers. +0 viewable / +0 Complete.

## 2026-10-08 — First level on screen (D113, D114)

tutorial_23 renders in the Bevy viewer from 01.33 data: 14/14 meshes at POS and 9/9
visible shapes (straight extrusion, labelled approximate). Level mode flags and
--qa-hide-ui. +0 viewable / +0 Complete.

## 2026-10-08 — Batch: first levels (D106-D108) + D109-D112

Level oracle (385/398 parse; first target tutorial_23) and new lbp-level crate (236/236
in-band levels parity, 4,180 Things bit-exact). Shape research. Merged .trans reader,
bone parsing, runtime research, font glyphs. +0 viewable / +0 Complete.

## 2026-10-08 — Batch: local D103, D104, D105 merged

D103 off-grid bakes (labelled preview approximation). D104 UV word 2 = UV0 (from the
game's compiled shaders). D105 evaluator on instance-color graphs. Unsupported 53 -> 34;
corpus all-diffuse 3,695 -> 3,700. +0 viewable / +0 Complete.

## 2026-10-08 — Batch: local D100, D101, D102 merged

D100 general static graph evaluator on the Repeat grid. D101 four-leaf trees without
shared pairs, COLOR-only trees, CONST leaves. D102 matrix masks and animated x COLOR.
Unsupported 102 -> 53; corpus all-diffuse 3,683 -> 3,695. +0 viewable / +0 Complete.

## 2026-10-08 — Batch: local D97, D98, D99 merged

D97 BLEND masks (red sRGB, alpha linear; 9 under instance color). D98 remap word and
volume slices: all 23,247 textures decode. D99 MULTIPLY trees and off-grid alpha MIX.
Unsupported 138 -> 102; corpus all-diffuse 3,669 -> 3,683. +0 viewable / +0 Complete.

## 2026-10-08 — Batch: local D94, D95, D96 merged

D94 general BLEND threshold rule (mask colour space still unknown; 0 changes). D95
large B8 and cube textures: 23,244 of 23,247 decode. D96 sampler-port folds under the
instance color: unsupported 144 -> 138. +0 viewable / +0 Complete.

## 2026-10-07 — Batch: local D91, D92, D93 merged

D91 GTF A8R8G8B8/B8 decode: textures 23,215 of 23,247 decode (+242). D92 2+ animated
leaves under animation base: unsupported 162 -> 144. D93 census counts shown in both
viewers. Corpus all-diffuse unchanged by default. +0 viewable / +0 Complete.

## 2026-10-07 — Batch: local D88, D89, D90 merged

D88 textured THING_COLOR under the instance color (8). D89 ADD shapes (4). D90 texture
census: 22,973 of 23,247 texture hashes decode. Unsupported materials 174 -> 162; corpus
all-diffuse 3,665 -> 3,669. +0 viewable / +0 Complete. Parity 0 mismatches.

## 2026-10-07 — Batch: local D85, D86, D87 merged

D85 opt-in corpus census modes. D86 constant THING_COLOR folds under a supplied color
(8). D87 CONST4, COLOR x COLOR, TEX with CONST UV scale (25). Unsupported 207 -> 174;
corpus all-diffuse 3,657 -> 3,665. +0 viewable / +0 Complete. Parity 0 mismatches.

## 2026-10-07 — Batch: local D81-D84 + audit fixes merged

D81 corpus census on 01.33: 4,257 of 4,261 mesh GUIDs viewable, 3,657 all-diffuse, 0
Complete. D82 legacy CLAMP 5 (0 users). D83 two-texture alpha MIX (unsupported 209 ->
207). D84 BLEND phase 1 (0 selected). CI pinned to Rust 1.99.0. +0 viewable / +0 Complete.

## 2026-10-07 — Batch: cloud D79, D80, D78 merged

D79 FNTb header reader (01.33 unsupported resources 1 -> 0; clippy fix on PC). D80
THING_COLOR research doc. D78 animation-base opt-in for 2+ texture products: route
unsupported 258 -> 209, parity 49/0. +0 viewable / +0 Complete.

## 2026-10-07 — Batch: cloud D77, D76, D75 merged

D77 Vita 0x4431 headers parse (01.33 unsupported 117 -> 1). D76 BLEND research doc.
D75 two textures x COLOR: route census unsupported 386 -> 258, Toolkit parity 128/0.
+0 viewable / +0 Complete; D75 cohort 0 -> 44 all-diffuse.

## 2026-10-07 — Batch: cloud D72, D71, D73 merged

First on-demand cloud batch, verified on the PC and merged: D72 GitHub Actions CI, D71
FARC/MAP/SFO bounds tests, D73 01.33 revision-gate tests. +0 viewable / +0 Complete.
D74 closed as duplicate of D71.

## 2026-10-07 — D69 01.33 becomes the working input

Disc + update 01.33 read as layers; patch MAP wins per GUID; parsers accept heads to
LBP2_MAX 1016 (66/66 Toolkit parity). 58,124 assets, 8,159 materials all parse; new
48-mesh cohort 48/45/0 (+48 viewable in cohort, +0 Complete). Viewer launcher uses 01.33.
[update-overlay.md](docs/update-overlay.md).

## 2026-10-07 — Cloud-session prep

Tests create ignored local-analysis/ in setUpModule (fresh-clone fix); added
[cloud-sessions.md](docs/cloud-sessions.md) (credit scope, cloud vs PC split, multi-agent rules).
Linux check in a cloud container: all crates test/fmt clean, non-GUI clippy clean.
Not tested: Windows rerun beyond unittest.

## 2026-10-07 — D68 official update 01.33 overlay data

All 32 official BCUS98245 updates verified and extracted to ignored
`game-data/a1-update-0133` (disc untouched). Patch MAP: 21,270 new GUIDs, 1,014 replaced;
9,665 resources at revisions 952..1016. No code change; +0 viewable/+0 Complete.
[update-overlay.md](docs/update-overlay.md).

## 2026-10-07 — D67 third-texture role swap

Three-texture products whose third channel was not displayed now swap leaf roles
(commutative product). Cohorts: 30 -> 38 and 5 -> 7 full diffuse; +0 viewable/+0
Complete. Corpus unsupported materials 366 -> 235 since D63.
[third-sample-preview.md](docs/third-sample-preview.md).

## 2026-10-07 — D66 per-mesh material budget

The 8-material cap is now 64 loads within a 640 MiB retained-bytes budget; Bevy draws
9 -> 65. 22 many-material meshes: groups 157 -> 449 of 504, full diffuse 0 -> 5;
+0 viewable/+0 Complete. [material-budget-preview.md](docs/material-budget-preview.md).

## 2026-10-07 — D65 independent third-texture transform

Nested three-texture products no longer need the third sample to share the second's
transform; Bevy derives its coordinates from a shared channel. 48-mesh cohort full
diffuse 0 -> 29; +0 viewable/+0 Complete. Oracle parity 48/0 mismatches.
[third-sample-preview.md](docs/third-sample-preview.md), [STATUS.md](STATUS.md).

## 2026-10-07 — D64 corpus route census and same-sample multiply

New selection-only census over 5163 materials ranks blockers by dependent meshes.
Same sample on both multiply ports now previews (texel squared). 26-mesh cohort
full diffuse 0 -> 23; +0 viewable/+0 Complete. Fixed D63 label mojibake in linked.rs.
[route-census.md](docs/route-census.md), [STATUS.md](STATUS.md).

## 2026-10-07 — D63 unwired diffuse as explicit black

Materials with no diffuse wire preview as a labelled black solid (Toolkit generator
evidence; game default unknown). Other channels stay flagged, Complete blocked.
48-GUID cohort: +0 viewable/+0 Complete; all-options full diffuse 47→48.
[unwired-diffuse-preview.md](docs/unwired-diffuse-preview.md), [STATUS.md](STATUS.md).

## 2026-10-07 — D62 git baseline and subtype-0 animation base

Local git repo with default-deny ignore; baseline commit. Recorded the unlogged D61
(supplied instance color, texture review). Opt-in animation base now covers one-curve
scale/offset samplers. 48-GUID cohort: +0 viewable/+0 Complete; base mode full
diffuse 44→45. Only blocker left with all options on: a material with no diffuse
wire. [STATUS.md](STATUS.md), [animation-base-preview.md](docs/animation-base-preview.md).

## 2026-10-06 — D60 MIRROR and independent product axes

Added typed Mirror mapping and separate two-source GPU sampler axes. Repeat-grid,
product MIX/shared chain restrictions remain. Candlestick gains1278triangles,
3/4 to4/4diffuse groups.40GUID cohort:+0newly viewable/+0Complete, full-diffuse
35 to36. App labels both samplers and retains Viewable/Complete measured counts.
92tests/fmt/Clippy/builds pass; one second-monitor native test/one image.
Commands/failures/limits:STATUS.md,[mirror-preview.md](docs/mirror-preview.md).
Originals read-only; no publication.

## 2026-10-06 — D59 interior-only BORDER preview

Added guarded nearest alpha-MIX preview for BORDER axes whose whole selected UV
channel stays inside1/4096margin. Original mode/range bits retained,explicit app
label and separate display axes;no guessed bordercolor or general border support.
Same39GUIDs:+0newly viewable/+0Complete;polaroid gains32triangle group1/2→2/2,
full-diffuse34→35. 92tests/fmt/Clippy/both builds pass after fixing a missing
synthetic fixture field. One native test verified second-monitor placement;one
image viewed. Commands/evidence/limits:STATUS.md,docs/interior-border-preview.md.
Originals read-only,no proprietary bundling/publication.

## 2026-10-06 — D58 same-sample alpha MIX

Added fixed COLOR/texture MIX weighted by that texture's alpha,raw metadata and
nearest-only derived baking/frontends/UI label. Target1and existing32/48pinned
metadata comparisons0mismatches;91tests/fmt/Clippy/both builds pass after fixing
an initial misplaced label match arm. Local polaroid remains blocked by BORDER
wrapping. Same39GUID cohort39viewable/34all-diffuse/0Complete unchanged:
+0newly viewable/+0verifiedComplete,0improved assets;corpus totals unknown.
Headless,0windows/0images. Evidence/commands:docs/alpha-mix-preview.md,STATUS.md.
Original inputs read-only,no proprietary bundling/publication.

## 2026-10-06 — D57 constant COLOR/single-texture MIX

Added bounded constant COLOR/single-sampler diffuse MIX,using existing UV/wrap/
display-buffer and UI labels/counts. Exact weight/input direction retained;RGB
mixes in linear space,alpha linearly. Same38GUID baseline:38viewable/34full-diffuse/
0Complete unchanged,+0newly viewable/+0verifiedComplete. Bouncepad gains one
232triangle casing group0/2→1/2. Target1and existing32/48pinned comparisons have
0mismatches;diffuse eligibility26→27. 90tests/fmt/Clippy/both builds pass.
Headless;0windows/0images. Sources/commands/limitations:docs/single-mix-preview.md,
STATUS.md. No original writes/proprietary bundling/publication.

## 2026-10-06 — D56 local animation base snapshots

Added opt-in stored matrix-animation base snapshots with explicit local-row/time
labels, raw static/base metadata retention and bounds. Bevy toggling reloads and
resets verification; snapshots remain separate from Complete. Same37GUIDs:
geometry boy gains2groups/223triangles4/6→6/6;static full-diffuse34,opt-in35,
+0newly viewable/+0verifiedComplete. 89tests/fmt/Clippy/both builds pass;target2
and existing48/32pinned records0mismatches. Game time/global uniform behavior
remain unknown. One native test verified second-monitor placement;one image
checked labels/counts and rendering. Evidence/commands:docs/animation-base-preview.md,STATUS.md.
Original inputs read-only; no proprietary bundling or publication.

## 2026-10-06 — D55 static COLOR/product MIX

Added bounded static COLOR/two-texture product MIX,with exact stored weight/input
direction and linear RGB/alpha before output encoding. Existing nearest grid
rendering/UI labels reused;Complete remains separate. Same37GUIDs:lamp gains
one190triangle group3/4→4/4,+0newly viewable/+0verifiedComplete,all-diffuse33→34.
89tests/fmt/Clippy/both builds pass;targeted static/instance records and existing
48/32pinned comparisons0mismatches,46/26eligible. Neon ThingColor hypothesis
rejected;no invented colors. Headless checks,0images/native windows.
Evidence/commands/limits:docs/static-mix-preview.md,STATUS.md. No original writes/
proprietary bundling/recovered-source use/publication.

## 2026-10-06 — D54 independent U/V wrapping

Bevy now preserves mixed Repeat/ClampToEdge axes on static single/tinted diffuse
bindings. App labels each axis;legacy eframe explicitly retains geometry without
substituting a wrapping mode. Multi-texture products remain Repeat-only. Same37GUID
baseline:3assets gain3groups/81triangles,+0newly viewable/+0verifiedComplete,
all-diffuse31→33.87tests/fmt/Clippy/both builds pass;target1and existing48/32pinned
metadata comparisons0mismatches. Headless descriptor checks;no native/image use.
Evidence/commands/limits:docs/axis-wrap-preview.md,STATUS.md. No original writes/
proprietary bundling/recovered-source use/publication.

## 2026-10-06 — D53 four-texture diffuse chain

Bevy supports a fixed three-multiply/four-leaf chain with exact first/fourth and
second/third coordinate sharing. Fourth texture reuses primary UVs; all four GPU
images removed on replacement. Handkerchief00007a6d gains1group/98triangles,
2/3→3/3supported. Same37GUID baseline:+0newly viewable/+0verifiedComplete,
all-diffuse30→31. App labels four samplers and keeps manual geometry/texture
confirmations separate.86tests/fmt/Clippy/both builds pass;48/32pinned metadata
comparisons0mismatches,45/26eligible. Evidence/native verification/limitations:
docs/four-product-preview.md,STATUS.md. No original writes/proprietary bundling/
recovered-source use/publication.

Native second-monitor check shows the ornate center motif,3/3supported and1ready
for review while Complete remains unset. Exactly one image viewed;stderr empty.

## 2026-10-06 — D52 three-texture nested products

Bevy supports exactly two static diffuse multiplies/three distinct texture leaves
when the third sampler shares exact channel/transform with an inner leaf. Three
original textures retained, two coordinate arrays; shader samples the third image
and cleanup owns it. Handkerchief00007a6d gains2groups/160triangles,0/3→2/3;
same37GUID baseline:+0newly viewable/+0verifiedComplete,37viewable/30all-diffuse
unchanged. App labels all three samplers and2/3supported; center remains neutral,
Complete checks unavailable.86tests/fmt/Clippy/both builds pass;48/32pinned metadata
comparisons0mismatches,44/26eligible. Second-monitor native capture/one image.
Initial E0308 from misplaced label arm fixed before successful verification;
Windows sandbox execution/path failures resolved with established reviewed workaround.
No original writes/proprietary bundling/recovered source/publication. Evidence,
memory ceilings and limitations:docs/nested-preview.md,STATUS.md.

## 2026-10-06 — D51 independent UV0/UV1 product support

Bevy now supports different original UV channels on static two-texture diffuse
products. Grid baking stays same-channel; GPU adapter preserves both coordinate
arrays and indices. Four assets gain4groups/1577triangles; same37GUID baseline
+0newly viewable/+0verifiedComplete,all-diffuse26→30. Circuit board4/4supported,
app labels UV1×UV0 and1ready for review with Complete unset.86focused tests,
fmt/Clippy/both builds pass;48/32pinned comparisons0mismatches. Second-monitor
native check,one image inspected. Legacy geometry fallback and all budgets retained.
No original writes/proprietary bundling/recovered-source use/publication. Exact
commands/limitations:latest STATUS.md,docs/channel-pair-preview.md.

## 2026-10-06 — D50 independent static matrix products

Bevy now samples two transformed textures independently for static affine/matrix
diffuse products; D49 integer grids unchanged. Original matrices/UVs/textures stay
intact. Secondary GPU image ownership covered by cleanup regression; legacy eframe
explicitly retains geometry without this route. Same37distinctGUIDs:oneasset gains
one2000triangle group; full diffuse25→26, +0newly viewable/+0verifiedComplete.
Tutorial floor5/5supported, app enables review with Complete unset.85focused tests,
fmt/Clippy/both builds pass; targeted pinned oracle0mismatches. Initial helper-name
compile error corrected. One native image inspected after second-monitor placement.
No original writes/proprietary bundling/recovered code use/publication. Exact scope,
commands/remaining limits in STATUS.md and docs/dual-preview.md.

## 2026-10-06 — D49 bounded two-texture diffuse products

Added static two-sampler multiply using bounded periodic grids, independent repeat
scales, linearRGB/alpha product and explicit nearest filtering in both frontends.
Original textures/UVs/raw sampler bits retained; matrix/other-channel/nested/animated
routes remain unsupported. Same37GUID cohort:7assets gain13groups, full diffuse
coverage23→25; +0newly viewable/+0verifiedComplete. Tutorial floor now4/5supported,
Viewable/unverified in app.85focused tests,fmt/Clippy/both builds pass;48/32pinned
metadata records0mismatches. Native test placed on second monitor before visibility,
one image inspected. No original writes, proprietary bundling/recovered code use
or publication. Exact commands/counts/limitations in latest STATUS.md.

## 2026-10-06 — D48 static UV matrix compatibility

Added bounded static subtype-one 2×3 UV matrix sampling to shared CPU preview,
used and labeled by both frontends. Raw UVs/coefficients preserved; animated,
wired and unknown subtypes remain explicit unsupported. One tutorial-floor asset
gains one968triangle texture group; +0newly viewable/+0verifiedComplete in same
37GUID cohort (37viewable,23all-diffuse). App shows1/5supported and remains
Viewable/unverified; Complete requires textures and correct geometry.83focused
tests,fmt,affected-target Clippy and both builds pass. Reused48/32pinned material
records:0mismatches,36/24eligible. Native test placed on second monitor before
showing; one image inspected. Launcher starts hidden too. No publication/input
writes/recovered source use; exact evidence and limits in STATUS.md/matrix-preview.

## 2026-10-05 — C4 BC3 completed

Added independent BC3 top-mip decoding, raw BlockFormat labels and bounded sample
selector. 35 Toolkit samples agree exactly; BC1 regressions retained. 57 Rust tests,
Clippy/fmt/native build and four Python checks pass; native RGBA/alpha inspected.
Documented synthetic Toolkit reversed-color bug; Pillow corroborates the public
rule while differing in low-endpoint expansion. No decoder translation or data writes.

## 2026-10-05 — C3 completed after resumption

Clear TEX/DDS BC1 top mip verified: 15 pinned Toolkit samples match exact pixels
and retained DDS fields; GTF and refreshed C1 regressions pass. 53 workspace Rust
tests, three Python tests, fmt, Clippy and native build pass. Native RGBA/raw-zero
mips, alpha and unsupported DXT5 captures inspected. Older C1 oracle lacked DDS
fields; refreshed reference output resolves that comparison failure. Updated
contracts and next C4 task; no BC3/higher-mip expansion or original data writes.

## 2026-10-05 — C3 partial, user-requested stop

Added TEX/DDS BC1 top mip through shared bounded envelopes/pixels, explicit DDS
variant rejection, raw metadata and mip-zero handling. Four original TEX tests
added (11 decoder tests total); seven viewer tests/fmt/affected-target Clippy pass.
15 pinned Toolkit TEX samples match exact Rust pixels/dimensions/mips/no_srgb.
Stopped to conserve user usage before native TEX build/QA and GTF oracle regression.
Resume instructions/evidence in STATUS.md and docs/tex-preview.md. Not marked complete.

## 2026-10-05 — C2 first BC1 texture preview

Added CPU-only lbp-texture, strict bounded GTF envelopes and BC1 top-mip RGBA,
approved read callbacks/cancellation, synthetic tests and metadata-only parity helper.
Native preview adds Fit/zoom, RGBA/channel/alpha checkerboard, explicit unsupported
states and one-resource CPU/GPU cache. Investigated 15/16 initial rounding mismatches:
normalized nearest RGB565 endpoints now agree with 19 distinct pinned Toolkit samples.
Workspace 49 tests, final relevant tests/fmt/Clippy/native build pass. Four native
screenshots inspected on RTX 3080/Vulkan. Unknown tails/higher mips untouched;
no swizzle copied, original writes, extraction, protected decode or publication.
Next C3 extends existing BC1 pixels to clear TEX/DDS. Details: docs/texture-preview.md.

## 2026-10-05 — C1 texture census and pinned oracle

Added bounded diagnostic Rust example and authored Java/Python oracle adapters.
13,624 texture headers: 11,633 GTF / 1,991 TEX, all supported census probes succeed.
9,817,800 diagnostic bytes read; 22 clear samples decode in pinned Toolkit with
191 matching fields. Deterministic census/oracle repeats; only ignored metadata
and digests emitted. Four diagnostic Rust tests and three Python tests added;
catalog tests/fmt/Clippy and Java synthetic guards pass. Product GUI unchanged.
Documented local mip-size surplus, demonstrated Toolkit's synthetic mip-1 failure,
and flagged RPCS3-derived uncompressed swizzle for external-oracle use only.
Next: plain 2D GTF BC1 top-mip decoder and preview. No original writes, extraction,
protected decoding or publication. Detailed evidence and fixtures: docs/textures.md.

## 2026-10-05 — B2 source attachment and bounded hex

Split source/SFO into UI-independent lbp-io and shared it with scanner/catalog.
Added source fingerprint/table/candidate validation, stability stamps and approved
ranges; UI uses cancellable attachment/hex workers. Hex pages stop at candidate EOF,
cap payload reads at 4096 bytes and stop at protected markers. Independent virtual
archive-entry browsing preserves orphan entries, aliases and conflicting hashes.
Eight new tests; workspace and final catalog checks pass (36 Rust tests total),
with fmt/Clippy/native build. Existing source
validated with 4,704,862 metadata bytes; 31,534 FARC entries match. Native entry and
resource hex views inspected on RTX 3080/Vulkan. No local orphan hashes; orphan
support verified synthetically. No original writes, extraction, payload decode or
publication. Next C1 identifies clear texture variants and compares Toolkit oracle.

## 2026-10-05 — B1 native metadata explorer

Added UI-independent lbp-catalog and native lbp-viewer, pinned eframe/egui/wgpu
dependencies. Snapshot-only browsing supports aliases, archive/type/status filters,
exact GUID/hash search, selected/raw metadata, outgoing/incoming references and
history. Cancellable worker handles JSON/indexing; lists virtualize rows. Nine
authored tests cover catalog bounds/references, cancellation and UI state. All 28
workspace Rust tests, fmt, Clippy and native build pass. Full existing inventory
count parity verified; debug load approximately 4.5 s. Native Vulkan screenshots
inspected on RTX 3080. No manual interactive QA or payload previews claimed.
Original files untouched, snapshots/screenshots/toolchain ignored, no Git changes.
Next B2 source attachment/hex/orphan archive entries, before texture milestone C.

## 2026-10-05 — A2 resource metadata and community oracle

Built unmodified pinned Toolkit cwlib from source with ignored local Temurin/Maven;
added authored read-only Java harness, source-pin checks and metadata comparisons.
Verified every local MAP/FARC field and 24 clear binary samples/123 normalized
dependencies. Census found revisions 941–951 beyond the initial 940 sample maximum;
corroborated each with Toolkit before extending support.

Added bounded resource metadata parser, explicit partial static-mesh handling,
raw dependency preservation and catalog links. Schema 2 inspects each location once,
keeps aliases/candidates, separates extension hints and magic, budgets work and
publishes diagnostic partial output with exit 2 when appropriate. All 31,534 FARC
entries inspected using 1,462,918 metadata bytes: 16,515 ordinary parsed, 21 static
partial, 13,862 identified/unparsed, 1,136 unknown. 116,739 raw dependencies, 46
unresolved in this MAP. Two full inventories byte-identical. 19 Rust and seven
Python tests pass; fmt/Clippy pass. Original files unchanged; no protected decoding,
payload implementation or GUI. Failed approaches/cleanup recorded in STATUS.

## 2026-10-05 — A1 directory scanner

Implemented four Rust crates, bounded SFO/MAP/FARC readers, read-only directory
catalog, alias-preserving candidate resolution, extension hints and deterministic
atomic/no-overwrite JSON output. Retained pinned Toolkit provenance and notices.
Added 11 passing Rust synthetic tests; fmt and Clippy (-D warnings) pass. Full
field parity with the earlier Python table probe and byte-identical inventories
verified on BCUS98245: 36,854 MAP rows, 31,534 FARC entries, 32,232 archive matches.

Installed ignored project-local GNU Rust plus verified MSYS2 assembler/runtime
tools after discovering no MSVC installation. Selectively copied three clear
SFO/MAP/FARC files to ignored game-data/a1-disc, checked metadata and marked copies
read-only. Original image untouched. Resource headers/dependencies, Java oracle,
payload decoders and GUI deferred to subsequent milestones. Updated usage,
architecture, roadmap, scanner contract and handoff. No root Git initialization.

## 2026-10-05 — Reconnaissance and foundation

Read local AI modding guides/templates and worked example; inspected existing workspace
and read-only game image; verified SFO identity, ISO tree, MAP/FARC tables and 19
distinct clear resource samples. Researched public community projects and active forks,
recorded license/provenance constraints and source pins. Added project rules, status,
research ledger, architecture, format map, community inventory, roadmap and notices;
extended default-deny ignore rules. Added bounded metadata probes and seven passing
synthetic tests, including an external TEX-only synthetic oracle.

Failed sample alias experiment was corrected and superseded; see STATUS. No game
files were modified/extracted, no protected payload decoded, no game executable run,
no renderer/runtime built, no Rust workspace initialized, and no Git history created.

## 2026-10-05 — D1 independent mesh-prefix diagnostics

Added lbp-mesh and bounded metadata cohort/oracle/parity diagnostics. 36 selected
prefixes match counts/raw fields/offsets/payload digests/IEEE bounds; 61 Rust tests,
Clippy/fmt pass. Recovered float JSON presentation mismatch via canonical bits.
No topology inferred, proprietary fixtures, asset export or original data writes.

## 2026-10-05 — D2 first MSH wireframe

Added independent primitive/descriptor/bone-layout traversal, explicit topology,
bounded list/strip/restart geometry IR and native Mesh tab with cancellation/cache,
metadata, orbit/zoom/reset. 36 geometry and36 prefix regressions match pinned
Toolkit. Native list/strip/static views inspected; fixed missing QA-tab allowlist
and added early magic preflight. Workspace65tests then final seven mesh/nine viewer
checks, fmt/Clippy/native build pass. No original data writes or full mesh/pose claim.

## 2026-10-05 — D3 material reference navigation/census

Added alias/conflict-preserving GUID/hash material candidate menus and metadata
navigation. Header-only census5,163GMTs;32sampleheaders/56normalizeddeps match
Toolkit. Final67Rusttests,5Pythontests, Clippy/fmt/native build pass. Native controls
inspected. Fixed source encoding/line-ending edit artifacts; no shader parser or
original writes. Future material payload work requires a separate verified contract.
## 2026-10-05 — D4 guarded material prefix evidence

- Added authored MaterialOracle/run-material-oracle and metadata-only consistency
  report plus3synthetic tests. Pinned Toolkit count/stream helpers only; no full
  material deserialize, shader byte export or product material parser.
- 32 samples/232,687 archived bytes accepted; 56 slot references match prior header
  dependencies. Ten shader offsets per sample, eight wraps per axis, raw nulls kept.
- Synthetic truncation133 exposed Toolkit fixed-width padding; final-offset guard
  rejects every fixed/packed truncation. Final Java synthetic checks and3Pythontests
  pass. Rust unchanged, prior67tests/fmt/Clippy/build not repeated.
- Added material-prefix provenance/applicability/conflicts/next-task contract;
  private outputs ignored, originals/references unchanged, no commit/publication.

## 2026-10-05 — D5 independent material prefix reader

Added lbp-resource::material and shared binary decoder factored from authored mesh
code; mesh now uses shared decoder/error variants. No new crate/UI/shader evaluator.
32material metadata samples match exactly,36mesh regressions match. Four new
synthetic integration tests plus full71workspace tests, fmt/Clippy/build pass.
Initial sandbox fmt/native executable denied OS access; approved run succeeded.
Updated provenance/applicability/budgets and next native metadata-tab task. Original
inputs/read-only boundary and ignored private diagnostic reports preserved.

## 2026-10-05 — D6 native material metadata inspection

Added Material tab in MAP/archive views, bounded validated cancellable worker,
diagnostic metadata/cache and texture-slot candidate navigation. Shared authored
GUID/hash grouping with meshes; raw nulls/aliases/conflicts retained. No shader or
opaque-byte export/graph rendering. Three new synthetic cases;74workspace tests,
13viewer tests/fmt/Clippy/native build pass. Real GMT/unsupported native captures
inspected. Fixed nonexistent demo helper and headless texture-delta cleanup errors.
Updated handoff/provenance/usage and next bounded graph-metadata reconnaissance.

## 2026-10-05 — D7 guarded material graph evidence

Added authored MaterialGraphOracle and optional run-material-oracle -Graph mode,
metadata consistency report/3synthetic tests. Unmodified pinned per-box/wire reader
corroborates32samples/139boxes/118wires; animation metadata/digests and prefix fields
agree. Prefix32sample/56reference regression passes, graph repeat byte-identical.
Recorded ignored parameter count, extreme signed zigzag bug and guarded animation
mask/division boundary. No Rust/UI code change or proprietary input writes.
Added material-graph provenance/applicability/test/next-independent-reader contract.

## 2026-10-05 — D8 independent optional graph metadata

Added material/graph.rs and optional inspect_graph API sharing bounded prefix
expansion without altering prefix schema/UI. Raw boxes/wires, animation metadata/
digests, slot/range flags and unknown tails retained; correct extreme signed zigzag.
Five new synthetic cases and79workspace tests/fmt/Clippy/native build pass. Graph32
andprefix32canonical comparisons pass. Corrected ambiguous test integer type and
old prefix report label. No original data writes or graph execution/GUI changes.
Updated source provenance/contracts and next native metadata-table task.

## 2026-10-05 — D9 native graph metadata tables

Added optional Material graph mode, virtualized lists/selected raw metadata,
slot candidate menus and diagnostic JSON. Independent graph error retains prefix;
cancellation/reset/cache checks plus unknown/range/layout fixtures pass. No parser
changes.15viewer/81workspace tests/fmt/Clippy/build pass; real head935native Vulkan
capture inspected. Input fingerprints/attributes and ignore boundaries verified.
Updated handoff/research/architecture/format/roadmap. Next bounded material-tail
oracle investigation, not shader evaluation or textured rendering.
Final lifecycle refinement queues a graph request made during prefix loading once,
without automatically retrying cancelled/failed jobs. All final checks repeated
and pass; native capture preceded this lifecycle-only refinement.

## 2026-10-05 — D10 guarded material tail oracle

Added authored isolated parameter-animation-array oracle, -Tail runner, diagnostic
consistency report and4synthetic validation cases.32tails fully accounted for;
30empty arrays and2single animations (241values/keys each), allsound0. Prefix/graph
metadata unchanged; repeated report byte-identical. Java self-tests and22Python
tests pass. Rust/UI unchanged. Recorded per-rule source pin/thresholds, strict
budgets/cancellation, raw/unknown outcomes, null/empty ambiguity and unknown sound/
animation semantics in material-tail.md. Inputs/reference/ignore boundaries clean.
Next optional independent tail reader and parity, leaving GUI for a later task.

## 2026-10-05 — D11 independent optional material tail diagnostics

Added tail.rs/inspect_tail and shared internal graph decode/payload continuation,
reusing bounded animation metadata helper. Prefix/graph APIs and schemas stable;
raw sound/array summaries/unknown tail counts only. Expanded material_parity with
--tail canonical comparison. Four synthetic integration cases,85workspace tests,
fmt/Clippy/native build pass; tail32/graph32/prefix32exact. Inputs/reference/ignore
boundaries unchanged; no UI or semantics implementation. Next D12 native optional
metadata inspection, preserving prefix/graph on tail failure.

## 2026-10-05 — D12 optional native material tail inspection

Added explicit tail mode, raw sound and virtualized parameter-animation metadata/
JSON. Single-expansion successful requests, bounded sequential fallback and
independent prefix/graph/tail errors; retain earlier cache on deeper cancellation.
Two new synthetic cases;17viewer/87workspace tests/fmt/Clippy/build pass. Three
native captures inspected: empty plus both241-key populated tails (masks4/8).
No parser/oracle change; original input/reference/ignore boundaries unchanged.
Updated handoff and next bounded coverage census rather than semantics/rendering.

## 2026-10-05 — D13 bounded material metadata coverage

Added diagnostic census and metadata-only bounded follow-up selection.5,163unique
clear material locations/5,536MAProws;all stages parsed,44,060,339bytesread,zero
residualtails,18revisions/28rawsounds/arraysup to7. Repeated report byte-identical.
FourRust/twoPython cases;91Rust tests/fmt/Clippy/build and24Python tests pass.
Initial32follow-up cap omitted7-recordarray;40selection covers all primaryfeatures.
Protected snapshot magic/orphan/duplicate tests hardened and final census rerun.
No parser/viewer change or new external validation. Next40sample pinned-oracle
comparison, preserving unknown semantics and selected-input scope boundaries.

## 2026-10-05 — D14 broader material oracle parity

Validated40follow-up samples through pinned isolated readers and complete Rust
metadata parity;0mismatches/failures, repeated external output byte-identical.
Combined72unique samples cover18revisions/28rawsounds/7arraycounts and29of76graph
count shapes. Remaining largest component-array boundary is1143versus603validated.
No source change or new format rule/conflict. Inputs/reference/ignore boundaries
unchanged; no commits/publishing. Next bounded remaining-shape/boundary oracle set.


## 2026-10-05 — D15 count-shape and boundary parity

Added deterministic metadata-only selection/union audit and three synthetic cases.
48 samples/430,103 bytes: complete pinned-oracle/Rust equality, zero mismatches or
failures, repeated manifest/oracle byte-identical. Combined 120 samples cover all
76 observed count pairs and maximum 1,143-component array. All 27 Python tests pass;
no Rust/parser/adapter/UI changes. Input/publication boundaries unchanged. Next
native diagnostic graph diagram with raw labels and linked metadata.


## 2026-10-05 - D16 native material graph diagram

Added bounded authored graph drawing, selection, pan/zoom/Fit and invalid-link list;
reexported shared parser caps. Four synthetic tests; 95 workspace tests and final
21 viewer tests/fmt/Clippy/build pass. Two final native graphs visually inspected.
Resolved harness/module/encoding issues; game data remains unchanged. Next cached
material diagnostic JSON export. Evidence: docs/material-diagram.md.


## 2026-10-05 - D17 cached material diagnostic export

Added explicit private JSON export, versioned stage envelope/schema, exclusive output
and bounded writer; no asset rereads. Four synthetic cases and99workspace tests/
fmt/Clippy/build pass. Native export repeats identically and matches pinned oracle
fields. Schema validator installed only under ignored analysis; real/invalid schema
checks pass. Inputs/publication boundaries unchanged. Next cached texture export.


## 2026-10-05 - D18 cached texture diagnostic export

Added cached header/count JSON projection and schema; shared private writer/helpers
now serve Material and Texture. Two new texture cases;101workspace tests/fmt/Clippy/
build pass. Native GTF/TEX output matches prior validated metadata and shared oracle
fields; channel-independent repeat and unchanged Material export verified. Schemas
reject pixels/invalid stages. Inputs/boundaries unchanged. Next cached mesh summaries.


## 2026-10-05 - D19 cached mesh diagnostic summaries

Added cached prefix/topology/material-reference JSON projection and schema; excludes
geometry arrays. Two new tests;103workspace tests/fmt/Clippy/build pass. Native
list/strip metadata matches prior parser output and pinned canonical oracle;
unsupported static result explicit. Repeated JSON identical, schema mutations reject
payload. Inputs/boundaries unchanged. Next bounded route Back/Forward browsing.

## 2026-10-05 - D20 bounded route history

Added shared validated transitions and128-visit Back/Forward across MAP rows, archive
entries andOverview, tab restoration and filter-clearing restoration. Cancels old
inspector jobs and prevents stale detail rendering. Five new authored tests;
108workspace tests/fmt/Clippy/build pass, two native captures inspected. Source
fingerprints/protection boundaries unchanged. Next keyboard route shortcuts.

## 2026-10-05 - D21 keyboard Back/Forward

Added exact Alt+Left/Right navigation, consumed repeated events and visible help.
Three new input/editor tests;111workspace tests/fmt/Clippy/build pass. Native help
and synthetic route roundtrip inspected; no manual native key dispatch claim.
Inputs unchanged. Next evidence-based32-bit TEX/DDS top-mip support.

## 2026-10-05 - D22 BGRA8 texture preview

Added independently specified32-bit standard-mask DDS pixels and row pitch; strict
legacy LINEARSIZE exception corroborated locally. Four new tests;115workspace
tests/fmt/Clippy/build pass. One fresh pinned BGRA oracle and67combined texture
samples match exactly. Native two-channel captures/schema/deterministic metadata
repeat pass. Inputs unchanged. Next animation metadata reconnaissance/oracle.


## 2026-10-06 - D23 animation reconnaissance

Added snapshot-only census/selector and guarded pinned Java prefix/full-reader oracle.
748locations/21cohorts;35samples,33modern prefixes,23full-reader field matches,
10locator skips,2old-layout rejections. Three Python tests/Java synthetics pass;
deterministic repeat proven. Product unchanged; next independent prefix reader.


## 2026-10-06 - D24 animation prefix parser

Added independent CPU ANMb prefix and shared authored integer helper. Four tests;
119workspace tests/fmt/Clippy/build pass.33exact pinned-oracle prefix matches,
2explicit old layouts, deterministic repeat and32material regression matches.
Inputs unchanged. Next bounded native Animation metadata inspector.


## 2026-10-06 - D25 native Animation inspection

Added MAP/entry Animation prefix view, bounded source worker/cancel/cache/reset and
scrollable raw diagnostics. Four viewer tests plus long protection-signature test;
124workspace tests/fmt/Clippy/build pass. Existing protection helper caller-length
mismatch fixed after failing test. Four native captures inspected, supported cached
fields match pinned oracle, old844explicitunsupported. Inputs unchanged. Next cached
Animation diagnostic export.


## 2026-10-06 - D26 cached Animation diagnostic export

Added explicit cached Animation JSON projection/schema, shared private saver/UI and
QA flag. Two new authored tests;126workspace/43viewer tests/fmt/Clippy/build pass.
Four native runs: oracle/schema/outcome/repeat pass; five invalid mutations rejected.
One native PNG inspected. Original data unchanged. Next level/plan reconnaissance.


## 2026-10-06 - D27 level/plan census and reader reconnaissance

Added snapshot-only level_plan_report.py plus four synthetic tests and provenance
map.282levels/7342plans counted; deterministic census and48plan samples spanning28
modern cohorts. Study pinned RLevel/RPlan/Serializer/Thing/InventoryItemDetails;
recommend plan prefix oracle first. No asset reads/parsing or Rust changes. Initial
canonicalization permission failure recorded and resolved with scoped execution.


## 2026-10-06 - D28 external plan-prefix oracle

Added authored bounded PlanOracle adapter/runner, sourceprovenance and ignoreallowlist.
48localmodern prefix probes pass and three reports byte-identical; synthetic guarded
reader/dependency/overflow/budget cases pass. Initial relative-seek bug fixed before
corpus read. No original writes, fullThing reader, copiedsource or Rust changes.


## 2026-10-06 - D29 independent plan prefix

Added CPUplan metadata/digest reader and bounded read-only oracle-parity example.
Four synthetic tests;130workspace tests/fmt/Clippy/build pass.48oraclefields exact,
zero mismatch/failures and deterministic repeat. No originals/dependencies/GUI changed.


## 2026-10-06 - D30 native Plan inspection

Added Plan tab/worker/cache/raw/hash/bounds UI and navigation/source reset integration.
Fourauthored tests;134workspace tests/fmt/Clippy/build pass. Foursupported/oldnative
captures exit0 and inspected; supportedcached fields matchpinned oracle. No format/
dependency/original changes. Next cached Plan diagnostic export.


## 2026-10-06 - D31 cached Plan diagnostic export

Added explicitcached Plan projection/schema, sharedprivate save/UI/QAcommand and
twoauthored tests.136workspace tests/fmt/Clippy/build pass. Fournative reports validate,
supportedfields equal oracle, deterministicrepeat and invalidschema mutations pass.
Largecapture inspected. Originals unchanged; next inventoryscalar prefix evidence.


## 2026-10-06 - D32 bounded plan inventory-prefix evidence

Added authored externaladapter/runner/ignoreallowlist.48prefixprobes+SlotID class
agreement and syntheticguards pass, deterministicrepeat and priorplanstage unchanged.
Documented pinned s64 highbit arithmeticshift conflict; retain encodeddatebits.
No independentRust/GUI/Thing parsing or originalwrites in this milestone.


## 2026-10-06 - D33 independent inventory-prefix stage

Added optionalCPUPlanInventoryInspection/rawmetadata, privateprefixhelper and bounded
read-only parity example. Fournewsynthetictests;140workspace tests/fmt/Clippy/build.
48inventory/48base fields exactlymatch oracle; repeat and unchangedD29prefixreport.
Fixednew syntheticzlib encodingcollision, no parserrule weakening. Originalsunmodified.


## 2026-10-06 - D34 optional native Plan inventory diagnostics

Added optional Plan inventory UI/worker cache and lifecycle coverage. 143 Rust tests,
fmt/Clippy/build and native oracle checks pass; five native runs exit0. Base-only v1
exports remain byte-identical. No original writes, new format rules or dependencies.
Details and next stage-aware export task: docs/plan-inventory-inspector.md.


## 2026-10-06 - D35 Plan export version 2

Added cached base/inventory projections and stage-aware outcomes/refusals; retained
historical v1 schema.145Rusttests/fmt/Clippy/build pass. Five native exports schema-valid,
oracle-equal when supported, deterministic repeat; partial/mutation checks pass.
Originals unchanged. docs/plan-export-v2.md; next deterministic resource sorting.


## 2026-10-06 - D36 resource list sorting

Added stable MAP row/path/GUID/SHA1 presentation order and CLI selector, preserving
aliases, selection and navigation.147tests/fmt/Clippy/build pass; five snapshot-only
native sort checks match independent expectations. No original writes/format changes.
Docs: resource-sorting.md. Next corresponding archive-entry sorting.


## 2026-10-06 - D37 archive list sorting

Added table/offset/size/SHA1 archive-entry ordering and CLI option, preserving original
indices/orphans/aliases.149tests/fmt/Clippy/build and five snapshot-only native checks
pass. Global crossarchive/tie semantics documented; originals unchanged. Next folder
prefix browsing. docs/archive-sorting.md.


## 2026-10-06 - D38 folder-prefix browser

Added literal MAPprefix children/counts/breadcrumbs and CLI startupfolder.152tests,
fmt/Clippy/build pass; four snapshot-only native count/order/selection checks match.
Originals unchanged; unusual/budget-exhausted rows preserved. docs/folder-browsing.md.
Next optional neutral surface mesh preview using already verified geometry.


## 2026-10-06 - D39 neutral mesh surfaces

Added optional authored diagnostic flat surfaces from verified stored geometry;
wireframe remainsdefault.154tests/fmt/Clippy/build pass; four native runs and unchanged
metadata/schema regression pass. Averagedepth/appearance limits documented; originals
unchanged. docs/mesh-surface.md. Next focus/preset viewing controls.

## 2026-10-06 — D40 material-linked textured mesh

Superseded further UI polish with first working direct-diffuse mesh slice. Independent
UV parser matches36oracle samples; one floor texture matches exact pixel metadata.
Added bounded linked worker, ambiguity/unsupported outcomes and unlit UV0 preview.
153 Rust tests/67viewer, fmt/Clippy/build pass; four native regressions preserve prior
JSON bytes/schema. Documented pinned partial-UV getter conflict and resolved compile/
lint/adapter failures. Originals/reference/ignore/noGit boundaries intact.
Next D41: primitive ownership and multiple direct-diffuse material groups.

## 2026-10-06 — Bevy frontend decision

Recommend staged Bevy rendering atop D40's verified CPU assets. Recorded shared
preview/renderer boundaries, source safety, version-verification requirements and
native acceptance checks in docs/bevy-transition.md. D41 becomes the Bevy floor
preview; multi-primitive materials follow as D42. No frontend implementation/build
or dependency mutation in this documentation pass; current explorer left intact.

## 2026-10-06 — next-chat handoff review

Checked current CPU loader boundaries and official Bevy/bevy_egui/Windows setup
documentation. Refined D41 ordering: minimal toolchain/render gate first, then shared
loader regression, known asset, minimal controls and adapter/lifecycle evidence.
Recorded winding/UV/color/alpha/normal display-policy checks. No Rust/dependency/data
changes or new test/build claims in this review.

## 2026-10-06 — user research-policy revision

Updated AGENTS.md to permit scoped local decryption/decompilation of the user's
own copy, superseding earlier prohibitions. Preserved read-only originals, ignored
research outputs and the asset/code-free distribution requirement. Recovered source
is evidence only, never copied/adapted/translated product code. No protected data or
executable was processed; no parser behavior changed. Bevy milestone unchanged.

## 2026-10-06 — complete revised handoff prompt

Saved docs/next-chat-prompt.md with the full D41 implementation/verification scope
and revised decryption/decompilation research permissions. Retained read-only inputs,
independent product code and exclusion of original assets/recovered source from
distribution. No Rust, dependency or original-data changes.

## 2026-10-06 — D41 native Bevy preview

Proved Bevy 0.19.1/bevy_egui 0.42.0 native depth rendering on existing GNU Rust,
then extracted CPU-only lbp-preview and kept eframe diagnostic behavior/export intact.
Added asynchronous validated source loading, GUID/source reload/cancel, perspective
camera controls, explicit outcomes, independent Mesh/Image/material adapter and owned
scene/GPU cleanup. No proprietary assets/code/keys or executable research involved.
157 workspace /71 focused tests, fmt, Clippy and both native builds pass. Inspected
synthetic orientation/sampler/depth/opaque-alpha, real floor, unsupported multi-primitive
fallback and static errors. Eframe export byte-identical to D40. Resolved sandbox
access/API/Clippy failures and remaining manual-interaction/platform limits in STATUS.
See docs/bevy-preview.md for exact launch, policy/provenance and user walkthrough.

## 2026-10-06 — D42 primitive/material groups

Retained CPU primitive triangle spans without changing metadata exports. Shared
bounded per-group direct-diffuse loading caches successes/failures; Bevy merges
shared bindings for draws while retaining per-primitive diagnostics and neutral
unsupported geometry. Added partition/cache/adapter tests and multi-group cleanup
coverage. Pinned oracle: 36 meshes/82 primitives, zero mismatches. 160 workspace
tests, fmt/Clippy/native builds pass; mixed synthetic, board, Bevy/eframe floor
captures inspected; eframe JSON unchanged. Product contains no original LBP code,
assets or keys; no decryption/recovered-source reuse/publication. See
[contract](docs/primitive-groups.md) and STATUS.md for exact checks/limits.

## 2026-10-06 — D43 diffuse-only projection

Shared preview now selects one unambiguous direct texture diffuse route from a
larger graph, records/lists ignored nodes/wires, and accepts either zero offset
sign. Other branches remain unevaluated; selected sampler inputs/animation/transforms
and unsupported operations reject. Pinned helper parity: 32 established plus a
separate 48 linked sample set, zero mismatches. Bevy/eframe bottle cap is visibly
textured; two-group garage light retains neutral unsupported center. 160 workspace
tests, fmt/Clippy/builds pass; floor metadata unchanged. No proprietary inputs
modified or code/assets/keys bundled. [Contract](docs/diffuse-projection.md).

## 2026-10-06 — D44 direct-diffuse UV1

Shared preview explicitly selects raw UV0/UV1 without fallback or array mutation;
frontends label selected channel. Existing UV/graph reader bounds unchanged.
32/48 material route/channel comparisons and36 mesh UV digests match pinned oracle.
Bevy Valkyrie shows two distinct UV1/BC3 texture groups; UFO retains unsupported
neutral groups. Eframe limit explicit; UV0 floor export unchanged. 161 tests,
fmt/Clippy/build pass after closing only task-owned locked D43 preview. No proprietary
inputs modified or code/assets/keys bundled. [Contract](docs/uv1-preview.md).


## 2026-10-06 — Clickable launcher and searchable Bevy asset menu

Added Launch Asset Explorer.cmd and a checkout-relative PowerShell launcher.
Bevy now browses catalog rows with search, virtual scrolling, direct selection and
show/hide files without current previews. Unchecked meshes remain visible; failures
are source-scoped, aliases preserved and cancelled jobs do not install outcomes.
Unsupported initial selection retains attachment; GUID is optional for browsing.
Seven viewer plus14 shared/catalog tests, fmt/Clippy/build and native captures pass.
Launcher works from Windows temp directory. No game bytes/code/keys bundled or
original inputs modified. D45 affine sampling remains deferred.
[Usage and limitations](docs/bevy-preview.md).


## 2026-10-06 — D45 static diffuse UV scale/offset

Added bounded static affine sampling on UV0/UV1 with raw UV/material bits preserved
and separate derived coordinates. Both frontends label transforms; animated/matrix/
wired sampler routes stay unsupported. Existing32/48 oracle records match with zero
route/channel/parameter mismatches, enabling three additional material samples.
Toro2× strap and Clivehalf-scale groups visibly textured; floor export unchanged.
78 focused tests, fmt/Clippy/build pass. Authored native offset/reflection fixture
added. No proprietary inputs modified or code/assets/keys bundled.
[Contract and evidence](docs/affine-preview.md).

## 2026-10-06 — Compatibility status and second-monitor test windows

Bevy browser now displays Viewable/Complete labels, textured primitive coverage,
and source-session totals for distinct attempted GUIDs. Complete requires visual
confirmation of all textures and correct geometry; neutral groups/skipped geometry
block it. Reload resets that asset's confirmation; source changes/restart clear
counts. Corpus totals remain unknown. No support expansion: +0 viewable/+0 complete.
Bevy starts hidden, then centers on a non-primary monitor before showing. Recorded
sparse image viewing and future compatibility-delta reporting in AGENTS.md.
Nine headless viewer tests, fmt, Clippy and rebuilt launcher target pass. Physical
placement/visual UI interaction untested; no images viewed or native programs
opened. Sandbox path/manifest denial resolved by reviewed local execution.
[Status definitions and limits](docs/bevy-preview.md).

## 2026-10-06 — D46 static texture × COLOR diffuse projection

Added one bounded static texture/color multiply route in either input order.
Both frontends label tint; raw pixels/UVs retained, optional bounded derived display
pixels use linear tint with cancellation. Other graphs stay explicit unsupported.
Pinned helper oracle48/32samples:zero mismatches, four new eligible material hashes.
Same36-asset baseline: +0newly viewable/+0newly complete; three existing meshes gain
six textured primitive groups. Separate native vulture shows gray tint/4×tiling.
81focused tests, fmt/Clippy/both builds pass; native capture selected non-primary
monitor, one image viewed. Initial fixture/Clippy/sandbox failures fixed; no game
inputs modified/proprietary assets/code/keys distributed or publication.
[Rules, counting scope and limitations](docs/tinted-preview.md).

## 2026-10-06 — D47 supported static COLOR-only diffuse

Added explicit solid material IR and both renderer paths without texture/UV reads
or fallback images. Static finite unanimated direct COLOR→DIFFUSE only; raw bits
preserved. App separates textured/solid groups and reports ready-for-review assets;
Complete remains explicit visual verification. Same36-asset baseline:13improved
assets/23solid groups, all-diffuse15→23, +0newviewable/+0verified complete.
Pinned48/32route/color comparisons have zero mismatches.81focused tests, fmt,
Clippy and both builds pass. Fez native capture verified red/black solid groups on
non-primary monitor; one image inspected. No game input changes/proprietary assets,
code/keys bundled or publication. [Contract and limits](docs/solid-color-preview.md).
