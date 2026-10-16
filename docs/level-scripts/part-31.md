# Level scripts, part 31: the Pod shell level in its own frame, under the script camera (2026-10-13, D280)

Seventh PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change, no
new native. Until now the shell level (pod.bin) was placed by the viewer's own rig under
`--level-scripts`: a shell eye fitted to the DualShock, moved with the change-over
value, and a tent eye with a fitted tilt (parts 26, 28, 29; all PLACEHOLDER). Now the
shell level stands in its own level frame and the camera the planets level's script
writes draws both levels. Nothing is fitted.

### What the game does

Nothing new was read for this PR. It rests on part 29's measurement:

- MEASURED (two captured frames, part 29): the camera position the game hands the pod's
  draws is (0, -250, 1130) with the Pod closed and (0, -299.5, 902.4) with the pod
  computer open. Those are the eyes of the planets level's script camera at rest
  (`Cam.SetCameraFromToUp`: (0, -250, 1130) not in use, (0, -300, 900) in use). The
  pod's vertex program takes the vertex position as a world position, and the tent's
  heights over the camera in the closed frame are its level heights plus 250.
- INFERRED from that: the shell level is drawn in its own level frame, by the planets
  level's script camera; the two levels share one camera and one frame, and the planets
  level is what is moved (it hangs under its world Thing, part 23). Between the two end
  states nothing was captured.
- Not established: the engine code that draws two levels with one camera; the camera's
  near and far planes as the game applies them to the shell.

### What the viewer does

`lbp-bevy-viewer` (`level_play.rs`, `main.rs`, `pod_menu.rs`). Only under
`--level-scripts --pod-camera N --pod-shell HEX`.

- `level_play::shell_frame_root(planets, shell)`: the root of the shell level. Each
  level has a display matrix of its own (level units to display units, a uniform scale
  and a translation). The script camera stands at the planets level's matrix times its
  eye. A shell entity stands at the shell's matrix times its level place, under the
  root; with the root `planets * shell^-1` it stands at the planets level's matrix times
  its level place. So level point X of the shell and level point X of the camera's frame
  are one point: the root is the identity of the level frame, written in display
  units. It has no turn and one scale (here 0.7788: the planets level's display scale
  over the shell's).
- Both shell roots (the tent's and the one with the DualShock, the strings, the bar and
  the platform) are spawned with that root and nothing moves them afterwards.
  `script_camera` no longer places them; it places the camera only. The roots stand in
  the frame before the first tick too, when the camera is still the rig's own planets
  eye (the closed preset, the same point of the frame).
- Gone under the scripts: the fitted shell eyes and their move with the change-over
  value, the tent's fitted tilt and pull (`TENT_TILT_CLOSED`, `TENT_PULL`), the rule
  for the shell's size (its far side at half the camera's distance to the Earth), the
  turn of the shell root with the camera. `PodRig::layer_at`, `shell_root_at` and
  `tent_root_at` (part 26) and `shell_layer_root` (part 29) are removed; `PodRig::layer`
  holds their arithmetic again, for the path without scripts.
- The change-over value still drives what the game's script drives with it: the tent's
  dissolve and the pod's mesh (parts 27 to 29). The shell level's place no longer
  follows it. `PuterState::t` is still written from it for the viewer's own readers
  (the Pod fog of the default lighting, the fitted figure's turn, the overlay).
- The dissolve's line (`follow_dissolve`, part 29) is unchanged in code and simpler in
  meaning. Its level-to-world matrix, the tent root's transform times the shell's
  display matrix, is now the planets level's display matrix: the tent, the camera and
  the line are in one frame, and the height over the line is level y minus camera y
  minus `1150 - 1550 v`. Part 29's INFERRED point (one frame for both levels) is what
  the whole picture now rests on.
- The shell's light: its display scale is set to the planets level's (in the shared
  frame a display unit of the shell is the planets level's). The preview light and the
  default light no longer turn with the camera, because the root has no turn: the
  shade issue of part 26 is gone with it.
- `level_play::layer_depths`, a check without a screenshot: when the script camera
  comes to rest, or the ticks are held for a capture, one line gives the depth range of
  both levels along the view (from the bounds of the shown draw entities), the box
  around the shell, the camera's near and far planes, and the smallest distance from
  that box to the planets level's surfaces. The last is taken on the triangles (a grid
  of 45 points on each): the planets level's room is a box around everything, so
  bounds alone say nothing.
- Printed at the start: `Pod shell level: drawn in its own level frame under the
  planets level's script camera (root scale 0.7788, ...)`. The `Pod shell layer at tick
  N: placed from the script's camera` line of part 26 is gone.
- PLACEHOLDER, left as it was: the fitted Pod figure (`--select-guid`), which is placed
  in the shell's space from the rig's fitted eyes; under the scripts it is then not
  where the game's camera would show it. None of the captures here has a figure.
- Without `--level-scripts` nothing changes: the rig places both roots as before.

### Where the planets level sits

MEASURED (the viewer's line, level units, camera at rest on tick 452, closed):

- The world Thing is moved by a matrix with translation (322, 5861, -31354) and scale
  2 (part 23's line). The planets level's room surrounds both levels: its bounds reach
  from 46,071 behind the camera to 69,306 in front.
- The shell's box is (-1276, -1521, -846) to (1219, 3438, 1184), around the origin; the
  camera at (0, -250, 1130) is inside it, in the tent's opening. Along the view the
  shell reaches from 221 behind the camera to 2,161 in front.
- The nearest point of the planets level's surfaces to the shell's box is 14,421 away
  (83,365 triangles looked at). Open (tick 640): 14,518; on the held ticks 605, 610,
  615 and 619: 14,476 to 14,551. So no surface of the planets level comes near the
  shell, and the shell cannot cut into the planets or the room, at either end or
  between.
- Planes: the viewer's near plane is 0.01 display units, 64 level units here; the far
  plane 636,705. The script's own 10 and 100,000 are still not applied (part 23).
  Nothing of the shell was seen cut by the near plane in the stills; the parts of the
  tent behind the camera are behind it in the game's frame too.
- Fog: the preview fog with the scripts' bias runs from 25,805 to 78,454 level units
  (part 25). The whole shell is within 2,400 of the camera: no fog on it. The planets
  and the room are fogged by their true distance, as before; the bias works on one
  scale for both levels now. Under the default lighting the shader's fog took the
  shell's depth six times too long (the rig's scale, 6.08, on the shell's own display
  scale), so with the pod computer open (fog start 5,810) the shell's far parts had a
  light haze; now the depth is the true one and the shell is short of the start in
  both states.

### Measured (PC, release build)

Viewer, 1280 x 720, no focus taken (`--qa-capture`, window hidden), `--level-guid
00018fdb --pod-camera 1 --pod-shell 000027b9 --window-16-9 --preview-lighting
--qa-hide-ui`, the d133 snapshot with the overlay. The frames stay in ignored
`local-analysis/rscript/stage6/shots/` (`d280-*`). The baselines are main's build
(45a59a1) on the same commands (`d280-base-*`, and part 30's `d279-*`).

- Without `--level-scripts`: equal byte for byte to main's, in four forms: closed,
  `--pod-open`, `--pod-game-camera`, `--pod-t 0.5`.
- One level with scripts and no shell layer (the planets level held at 600; pod.bin
  alone, flags 3, in use from 600, held at 640): equal byte for byte to part 30's.
- With the shell layer under scripts every frame differs from part 30's, by design.
  Closed at 600 was taken twice, with equal bytes. In use from 600 held at 640, the
  press on 600 held at 641 and `--pod-open` held at 600 give one frame, the same bytes.
- Printed, in use from 600: `Pod change-over at tick 601: opening, value 0.05`, `Pod
  tent dissolve at tick 601: value 0.05, 3 draw entities; cut above level y 820.4
  (camera y -252.1 + 1150 - 1550 x value)`, `Pod change-over at tick 620: open`, the
  swap on 620, `Pod levels under one camera at tick 628 (camera at rest ...)`.

Looked at, against two frames of the game in RPCS3 taken earlier (ignored
`local-analysis/d156-rpcs3-pod-default.png`, closed, and `d156-rpcs3-pod-open-t2.png`,
the pod computer's main menu; both hold the game's picture in a 1120 x 618 area of the
frame, so places are given as fractions of the picture). Read off by eye, about 0.01:

| | game | viewer |
|---|---|---|
| closed: the tent's window, left to right | 0.34 to 0.68 | 0.34 to 0.68 |
| closed: the tent's window, top to bottom | 0.25 to 0.73 | 0.26 to 0.72 |
| closed: the Earth's middle | (0.46, 0.47) | (0.46, 0.48) |
| closed: the Moon's middle | (0.56, 0.47) | (0.56, 0.47) |
| closed: the DualShock's top edge | 0.68 | 0.68 |
| open: the DualShock, left to right | 0.41 to 0.61 | 0.40 to 0.61 |
| open: the DualShock's top edge | 0.84 | 0.84 |
| open: the platform's far rim | 0.98 | 0.97 |

- Closed (600): the tent, its two side panels with the wall art, the roof's ring, the
  floor and the DualShock at the foot of the back wall are where the game's frame has
  them, with no fitted tilt. Through the window: the Earth, the Moon, the lamp, the
  table with the small figure, the room's wall. The game's frame also has Sackboy, the
  prompt and a statue at the right, which the viewer does not draw here.
- Open (640): the tent is gone, the DualShock is at the bottom middle and the platform's
  far rim is a flat arc at the frame's bottom edge, as in the game's frame. Behind it
  the viewer shows the room with the Earth and the Moon in view 0; the game's frame is
  the main menu's view of the planets (part 26: the menu views are not modelled). So
  the open picture matches in the shell and not in the planets.
- 605 (value 0.25): the tent in place with its roof cut away; the DualShock at the foot
  of the back wall, its grips at the floor's cut-outs, as closed.
- 610 (0.5): the script camera is on its way (it moves forward and pitches up, with an
  overshoot): the tent's walls are cut at about a third to a half of the frame, the
  window's lower edge and the floor still meet the DualShock as closed.
- 615 (0.75): low walls with the circuit band at the frame's bottom corners; the
  DualShock between them, half below the frame's edge (the camera's overshoot).
- 619 (0.95): no tent in the frame; the DualShock at the bottom middle.
- Through 605 to 619 the DualShock keeps its place against the tent's floor and back
  wall: both hang under the same fixed root, so nothing can move between them. Part
  29's fault (the DualShock sliding over the floor at 615 and 619) is gone.
- New in the picture, ticks 30 and 200 (the planets level's intro): the camera is
  outside the pod there, below it, and the frame shows the cardboard pod from outside,
  hanging in the room under a black bar, larger at 200 than at 30. With the rig these
  frames showed the tent's inside with the ceiling in its window. INFERRED: this is
  the game's fly-in to the Pod; not compared against a frame of the game.
- Default lighting (no `--preview-lighting`), closed at 600 and open at 640: the same
  framing; closed, the tent is lit and the window clear, close to the game's frame.

Not seen: the motion (stills only), the way back. Not compared: any frame between the
end states, or of the intro, against the game.

### Left out

- The planets level's menu views and their fog bias (the open picture's planets).
- The script camera's near and far planes.
- The fitted Pod figure under the scripts (PLACEHOLDER above); the viewer's own Sackboy
  in the Pod (part 30), who would now stand in the frame the camera is in.
- The black bar above the pod (drawn black; part 27) and the string ends, now in the
  intro's frames.
- Everything part 29 and part 30 left out of the dissolve and the input.

### Verification

`lbp-bevy-viewer`, none needs game data: `d280_the_shell_root_is_the_shells_own_level_frame`
(a shell level point is drawn where the planets level's display matrix has that level
point; the tent's top is 1528 over and 1130 in front of the closed eye; no turn; no
root without an inverse), `d280_the_dissolve_line_is_level_y_minus_camera_y_in_the_shared_frame`
(the matrix handed to the dissolve is the planets level's display matrix and the plane
gives level y minus the line), `d280_the_depth_range_is_from_the_corners_of_the_bounds`
and `d280_the_nearest_surface_to_the_shell_box_is_measured_on_the_triangles`. Removed
with their code: `d275_layer_at_keeps_the_shell_projection_under_any_planets_camera`
and `d278_the_tent_root_stays_closed_under_the_script_camera`. fmt, the CI clippy line,
the CI test set (612 pass) and the Python tests (36) pass; `cargo check` of both viewers
and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass; the viewer's
tests pass (73) but the one that fails on main
(`sackboy_play::tests::input_script_steps`). The runtime is untouched, so the 378-level
runs were not repeated. Seen in the viewer window: the stills above. Compared against
the game: the closed and the open still, by eye, against two earlier RPCS3 frames.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
