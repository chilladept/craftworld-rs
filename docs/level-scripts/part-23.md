# Level scripts, part 23: the Pod's camera and its moving world (2026-10-13, D272)

Stage 6 of `docs/rscript-plan.md` (the Pod), third PR. The planets level of the Pod
(`gamedata/lbp2/pod/levels/pod_planets.bin`) writes the camera itself each tick, and moves
its world Thing each tick. The runtime kept both and nothing drew by them. Now the
runtime says which camera a frame is drawn with, and the Bevy viewer puts its camera
there and hangs the planets level under the moved world Thing. No native is new.

### What the game does (FOUND, 01.33 executable and script listings)

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Cam.SetCameraFromToUp__rrrffff` | static `void (v4, v4, v4, f32, f32, f32, f32)` | 0x5fc494 -> 0x734668 -> 0x2e9798 | Builds the camera matrix from an eye, a point to look at and an up vector with the look-at builder of part 20 (0x2e7990: right, up, backward, eye) and writes it and its inverse into the render camera. Stores the four floats in the render camera and rebuilds the projection (0x2e8c58) |

- The four floats, in the order passed: the near plane, the far plane, a value that goes
  to the projection builder, and the field of view. The field of view lands in the field
  `Cam.GetFov__` returns (part 20), in radians.
- The call does not go through the level's camera. There is no blend and no override
  state: the render camera has the new matrix at once. `Cam.GetCameraMatrix__` copies
  that matrix out.
- One script calls it: the planets level's world script, once a tick, from its per-frame
  entry (part 6). It passes an eye and a point picked from presets ("pod computer not in
  use", "in use"), eased toward them, with the up vector (0, 1, 0) and the floats 10,
  100000, 0 and 1. While the intro thread of part 22 runs, the pair is mixed from an
  outer pair, eye (-500, -2000, 2000) looking at (-100, -800, 1000), to the not-in-use
  preset by the thread's zoom value, which goes from 0 to 1 over 451 ticks.
- In the same entry the script writes the world Thing's matrix
  (`Thing.SetWorldPosition__m`, part 13): twice the size about the camera's eye, a pitch,
  an offset and a yaw of its current view. The offset of the idle view is 15,000 units
  along -z, so the world Thing ends about 31,000 units from where the level file has it.
- It also calls `Cam.SetDOFAndFog__fff` each tick (80000, 100000, 20000 at rest) and
  `Cam.ClearMotionBlur__` on a snap. Both are kept and not applied.

INFERRED:

- The field of view is the vertical one. The script passes 1.0, and 57.3 degrees of
  vertical field of view were measured in the running game (`docs/p2k-pod-game-camera.md`).
- How the look-at builder makes the three axes: right is forward x up, the matrix's up
  is right x forward, each at length 1 (the usual rule; FOUND is the order of the
  columns).
- How long a written camera stands. Here: for the frame it was written in. The engine
  code that writes the render camera in a frame without such a call was not read.
- That the whole planets level is drawn under its world Thing. FOUND is that the script
  moves that Thing. MEASURED before (`docs/p2k-pod-game-camera.md`, memory reads in an
  emulator): the Earth's and the Moon's matrices in the running game are the world
  Thing's matrix times a plain place and size. The level's other Things are not children
  of the world Thing in the level file, so the runtime's own tree does not move them.
  Which engine code applies the world Thing's matrix to them was not read.

Not read: what the third float does in the projection builder. (Part 34: FOUND, it is
the render camera's ortho factor; 0, as the Pod passes, is a perspective camera.)

### What the runtime does

`lbp-runtime/src/camera.rs`. Everything that needs no window is here.

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| The call | kept as passed (`CameraState::from_to_up`), with `PWorld.Frame` at the call (`from_to_up_frame`) | FOUND |
| The camera of a call | `ScriptCamera`: eye, direction to the point at length 1, the up vector as passed, near, far, the third float, the field of view in radians | FOUND; vertical INFERRED |
| Its matrix | `ScriptCamera::matrix`, `CameraPose::matrix_with_up`: right, up, backward, eye | FOUND layout; INFERRED axes |
| No direction | eye and point the same, or an eye that is not a number: no camera | INFERRED guard |
| Up vector without a direction, or along the view | world +Y, then +X as the right | INFERRED guard |
| A field of view outside 0 to 180 degrees | no field of view: the display keeps its own | INFERRED guard |
| How long it stands | `LevelWorld::script_camera`: from the call to the end of the next tick's count, so a display that draws between ticks sees the camera of the tick that just ended, and one tick without a call ends it | INFERRED |
| Which camera a frame is drawn with | `LevelWorld::display_camera(follow eye, follow direction)` | below |
| `GetCameraMatrix` without a view | the written camera's matrix while it stands; else as in part 20 | FOUND |
| The world Thing | `LevelWorld::world_thing_moved`: the matrix that takes the world Thing from its stored place to its place now | the rule of part 19 |

Which camera wins (`display_camera`, in this order):

1. A camera a script wrote itself on the last tick: that camera outright, with its up
   vector and its lens (`CameraSource::Script`). FOUND: no blend.
2. Else, while an override is in (part 20): the follow camera mixed with the override,
   world +Y up, the override's 60 degree lens (`CameraSource::Override`).
3. Else nothing: the display's own camera stands.

The order of 1 and 2 is INFERRED: the first writes the render camera directly, the
second is the level camera's own update, and which writes last in a frame that has both
was not read. No level has both (measured below), so the order decides no picture today.

Counters: `script_camera_ticks` (ticks on which a script wrote the camera) and
`script_camera_ticks_with_override`.

`level_scripts`: the `camera` block of `play_all` has the calls, the levels with a call,
the ticks with a written camera and those with the override in too. A level played alone
lists the camera a display would draw with after ticks 1, 226, 452 and the last one.

### What the viewer does

`lbp-bevy-viewer --level-scripts` (`level_play.rs`).

- `script_camera`, each frame after the viewer's own camera system: it asks
  `display_camera` and, when there is an answer, puts the camera there
  (`place_camera`): the eye through the level's display matrix, the turn taken from the
  runtime's camera matrix, the field of view from the answer. With no answer the
  viewer's own camera stands and its own field of view comes back. The orbit state is
  not touched. Then the camera drawn with is written for the scripts, as in part 20.
- `follow_world`, each tick: the root of a level that hangs under its world Thing gets
  the matrix `N M N^-1`, with `N` the level's display matrix and `M` from
  `world_thing_moved`. Like the light of part 18 it waits until the scene has been drawn
  for 60 frames, because the scene's light turns with the root.
- Not applied: the near and far planes of the script's lens (the viewer keeps its own,
  in display units; applied from part 34), depth of field, fog and motion blur.
- One line is printed when the written camera comes in, comes to rest, moves again or
  goes out (`Script camera at tick N: ...`, with eye, direction, up and field of view),
  and one when the root is first placed (`World Thing at tick N: ...`). The "Level
  scripts" window has both.

How it combines with the Pod flags. None of them changes without `--level-scripts`.

| Flags with `--level-scripts` on the planets level | Camera | Planets |
|---|---|---|
| none of the Pod flags | the script's camera | where the level file has them: no root, so the picture is not the Pod's. The Thing map of part 19 is on |
| `--pod-camera N` | the script's camera. The fitted home view of `--pod-camera` shows only until the first tick with a written camera | under a root that follows the world Thing. The flag still picks the Pod's lighting, and the Thing map stays off |
| `--pod-camera N --pod-shell HEX`, with or without `--pod-game-camera` | the viewer's own Pod camera; the script's camera is not applied, and one line says so | the viewer's own rule (`--pod-game-camera`: the fixed view transform of P2k) |

- With the shell layer the script's camera is left out on purpose. That layer is placed
  each frame for the viewer's own Pod camera, so that one camera can draw two levels;
  under another camera it would be drawn from the wrong place. Driving the shell layer
  from the scripts belongs to the PR that enters the pod computer.
- `--window-16-9` and `--preview-lighting` mean what they meant. Under
  `--preview-lighting` the viewer's distance fog now has 31,000 level units between the
  camera and the room, and the picture is washed out (below). The default lighting of
  level mode fogs in its shader and shows the room clearly.
- New QA option `--level-scripts-hold-at N`: the scripts stop after tick N, and a
  `--qa-capture` waits for that tick, so a capture shows a chosen tick. Without it the
  scripts tick while the scene loads, as before, and the first drawn frame is some
  seconds into the intro.

Left out:

- Earth and Moon by their own scripts. Their per-frame entry takes arguments that are
  not established (part 6) and is not called, so they stay at their stored places under
  the root and do not spin. The menu places of P2k are not applied on this path.
- The pod's mesh swap (`SetRenderMeshMesh`, in the dirty list since part 21). Following
  it means loading another mesh while the scene is up, which is not cheap, and no run
  swaps a mesh yet (the swap happens when the pod computer is in use). The viewer
  counts such a Thing in its Things line ("another mesh").
- The general Thing map for the Pod's planets (hide, tint, move per Thing): no script
  of that level changes those values.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path gamedata/lbp2/pod/levels/pod_planets.bin --pod-camera 1
--level-scripts --window-16-9`. Add `--level-scripts-flags 2` to skip the intro.

### Measured (PC, release build)

`level_scripts --all --play=60`, 378 levels, 13,547 scripted Things, plain / with
`--visit-triggers` / with `--visit-triggers --pulse-switches`, inputs as in part 11.
Outputs under ignored `local-analysis/`: `d272-play-all-{novisit,visit,visit-pulse}.json`
(D271: `d271-play-all-*`).

| 60 ticks per level | D271 | D272 |
|---|---|---|
| Things stopped by a fault | 3 / 3 / 3 | 3 / 3 / 3 |
| Fault kinds, levels with a fault | 1, 1 | 1, 1 |
| Natives in the table | 249 | 249 |
| Unbound natives called | 41 / 73 / 73 | 41 / 73 / 73 |
| Unbound calls | 61,480 / 862,986 / 868,663 | the same |
| Native calls | 573,134 / 5,365,247 / 5,424,893 | the same |
| Recorded-only calls | 12,328 / 91,027 / 96,943 | the same |
| `SetCameraFromToUp` calls, levels | - | 60 / 60 / 62, 1 level |
| Ticks with a written camera | - | 60 / 60 / 62 |
| Levels with a written camera and the override in | - | 0 / 0 / 0 |

- The whole `play_all` block is equal to D271 in all three runs but for the five new
  counts of its `camera` block. The runtime's behaviour did not change.
- The one level with a written camera is the Pod's planets level. The 101 levels with
  an override (part 20) have none.

The planets level alone (`d272-planets-{60,600,600-inuse}.json`): the camera a display
draws with, level units.

| After tick | Eye | Direction | Field of view |
|---|---|---|---|
| 1 | (-500, -2000, 2000) | (0.248, 0.744, -0.620) | 57.3 degrees |
| 60 | (-434.4, -1770.6, 1885.9) | (0.208, 0.653, -0.728) | 57.3 |
| 226 | (-250, -1125, 1565) | (0.101, 0.391, -0.915) | 57.3 |
| 452 and 600 | (0, -250, 1130) | (0, 0.132, -0.991) | 57.3 |
| 600, pod computer in use from tick 100 | (0, -300, 900) | (0, 0.316, -0.949) | 57.3 |

Up is (0, 1, 0), near 10, far 100000 and the third float 0 on every tick. The eye after
tick 226 is half way between the outer eye and the preset, which fits a zoom value of
225 / 450. The world Thing after tick 600 is at (0, 4733.1, -30793.1) with axes of
length 2.

Viewer, release build, 1280 x 720, no focus taken (`--qa-capture`), the planets level
with `--pod-camera 1 --level-scripts --window-16-9 --preview-lighting --qa-hide-ui` and
`--level-scripts-hold-at` 30, 200, 440 and 600. The frames stay in ignored
`local-analysis/rscript/stage6/shots/`.

- Printed, held at tick 30: `Script camera at tick 30: in; eye [-467.8, -1887.2,
  1943.9], direction [0.229, 0.701, -0.676] ... field of view 57.3 deg` and `World Thing
  at tick 30: ... translation [467.8, 6370.4, -31607.1] and x axis of length 2.000`.
  Held at 600: `Script camera at tick 452: at rest; eye [0.0, -250.0, 1130.0],
  direction [0.000, 0.132, -0.991]`. The eye at tick 30 fits a zoom value of 29 / 450,
  and the resting pose is the one `level_scripts` reports.
- Looked at, tick 30: the camera looks steeply up; the room's ceiling fills most of the
  picture, and the Earth, the Moon and the hanging lamp are cut by the lower edge.
- Tick 200: the ceiling is the upper third; the Earth and the Moon are left of the
  middle; the table with the figure is at the lower right.
- Ticks 440 and 600: the room is seen nearly level; the Earth is a little left of and
  above the middle, the Moon to its right, the lamp above them, the table at the right.
  The two frames differ by a small shift: the zoom is nearly over at 440.
- All four are washed out in orange: the fog of `--preview-lighting` over the distance
  the world Thing puts between camera and room. It is the viewer's fog, not a reading
  of the game's. The frame at tick 600 without `--preview-lighting` (the default
  lighting of level mode) shows the same framing clearly.
- The same level without `--level-scripts`: the fitted globe view of `--pod-camera`, the
  Earth large and left of the middle, the Moon and the table to the right, no fog.
- With `--pod-shell 000027b9 --pod-game-camera`, with and without `--level-scripts`:
  the two captures are equal byte for byte, and the line `Script camera ...: not
  applied` is printed.

Not seen: the motion itself (stills only); the first drawn frames of a run without the
hold option. Not compared: any of this against the game. The Earth and the Moon are at
their stored places here, which is not where the game's own scripts put them, so the
frames are not expected to match the game's framing of the planets.

### Not modelled

- Earth and Moon placed and spun by their scripts; the views of the pod computer's
  menus (they come from its state stack, which nothing runs).
- The pod shell layer under the script's camera; the pod's change-over (mesh swap,
  fade, transition texture).
- Near and far planes, depth of field, fog and motion blur of the Pod's camera; a fog
  for `--preview-lighting` that fits a level moved this far.
- The third float of the call; whether the field of view is vertical in the game's
  projection builder (measured so in one game state). Both FOUND in part 34: the ortho
  factor, and the vertical field of view.
- Which of a written camera and an override wins a frame that has both.
- The engine code that draws the planets level under its world Thing.

### Verification

`cargo test -p lbp-runtime`: 61 unit and 46 world tests. New: 2 unit tests in
`camera.rs` (`a_script_camera_is_an_eye_a_direction_an_up_vector_and_a_lens`: the pose
and the lens from a made-up call, the matrix and its axes, an up vector that rolls the
camera, one that is not at a right angle, none and one along the view, no direction, a
field of view no lens can have; `a_script_camera_stands_for_its_tick_and_goes_before_the_override`:
nothing, an override alone, a written camera over it, the tick after, a tick without a
call, a call every tick, the counters, a level without a PWorld) and the world test
`d272_a_written_camera_stands_for_its_tick_and_the_world_thing_moves_the_level` (the
native through three ticks of the schedule, the matrix a script reads back, the tick
without a call; the world Thing where the file has it, moved by a script, absent).
`lbp-bevy-viewer`: `a_script_camera_in_level_units_places_the_viewer_camera` (eye,
direction, up and field of view in display units; the point looked at in the middle of
the picture and the top edge at half the field of view; the matrix handed back to the
scripts; a rolled camera; the root of a level under its moved world Thing). fmt, the CI
clippy line, the CI test set, the Python tests and `cargo check` of both viewers pass;
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` passes; the viewer's
tests pass but the one that fails on main (`sackboy_play::tests::input_script_steps`).
Seen in the viewer window: four stills of the intro and its rest pose, and the frames
for comparison. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer feature).
