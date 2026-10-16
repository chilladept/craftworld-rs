# Level scripts, part 20: the script camera and the screen (2026-10-13, D269)

Stage 5 of `docs/rscript-plan.md`, fifth PR. A scoreboard takes the camera when a player
reaches it, and a speech bubble closes when its Thing has been off the screen for two
seconds. Both asked natives that were not bound. Now the world keeps the camera a script
lays over the follow camera and steps its blend, the screen natives answer from the
camera the picture was drawn with, and the Bevy viewer moves its camera to the override
and back.

### What the game does (FOUND, 01.33 executable and script listings)

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Cam.SetOverrideCamera__rr` | static `void (v4, v4)` | 0x5fc45c -> 0x7349f8 -> 0x3760e4 | Stores a position and a direction in the level's camera, as passed. Nothing without a level camera |
| `Cam.SetOverrideCameraActivation__bf` | static `void (bool, f32)` | 0x5fc348 -> 0x734bc4 -> 0x374cec | Sets the override's blend target to 1.0 or 0.0 and its blend rate to the float. It does not touch the blend |
| `Cam.GetCameraMatrix__` | static `m44 ()` | 0x609ed0 -> 0x7347e0 | Copies the render camera's camera matrix out: camera to world |
| `HUD.GetScreenSpacePosition__r` | static `(v4)`, returns the 2D point type | 0x601d30 -> 0x4b6e80 -> 0x2e7bdc, 0x2e74a0 | Projects a world point with the render camera's view-projection matrix and maps it to pixels |
| `HUD.GetXRes__`, `HUD.GetYRes__` | static `s32 ()` | 0x5fa5e0 -> 0x4b6a94, 0x5fa5a4 -> 0x4b6aac | Two globals; their values in the executable's data are 1280 and 720 |

- The override. The level's camera has one: a position, a direction, a blend, a blend
  target and a blend rate. Only the activation native writes the target and the rate,
  apart from the camera's constructor (zero) and a copy of the whole record. So the
  engine never ends an override; a script does. There is no owner and no priority: the
  last caller wins.
- Each camera update (the apply function 0x6eb650), in this order:
  1. The blend takes a step (helper 0x38eb40): the target when it is nearer than the
     rate, else the blend moved by the rate toward it. Linear.
  2. The weight is `0.5 - 0.5 cos(pi x blend)`.
  3. The view direction goes from the follow camera's to the override's on the arc
     between them (helper 0x3918ec): a plain linear mix when their dot product is above
     0.999, the sine form below it. An opposite pair takes a fallback axis.
  4. The eye is the follow camera's eye plus the weight times the way to the override's
     position.
  5. The camera aims at the eye plus 1000 times the direction.
- The follow camera keeps running underneath. While the blend is above 0.0001 the camera
  takes a plain lens: 60 degrees of vertical field of view (1.0472 rad), near 10, far
  40000 (0x378470, 0x38367c). Corrected in part 34: near 10 and far 40,000 are not
  special to the override; they are the game camera's constants always. The override
  has no lens of its own; what it changes is that the camera zones' step is skipped, so
  the field of view stays at 60 degrees and far gets no zone pull-back. The weights of
  the tracked players are multiplied by 1 - blend (0x3764a4), so players stop steering
  the follow camera while the override is in.
- The camera matrix. Column W is the eye and column Z points backward: the look-at
  builder that fills it (0x2e7990) writes right, up, minus forward, eye.
- The screen position. `clip = M x (x, y, z, 1)`, whatever the point's own w; divided by
  `clip.w`, or by 0.0001 when that is exactly 0 (the constant at 0xd9c820);
  `x = (ndc.x + 1) / 2 x XRes`, `y = (1 - ndc.y) / 2 x YRes`. So: pixels, from the top
  left corner, y down. There is no test for a point behind the camera: its w is negative
  and it comes out mirrored through the middle of the screen. The native sets the fourth
  component to 0.
- Use in the scripts. The scoreboard, once its trigger fires, reads the camera matrix,
  takes column W as its camera position and minus column Z as its direction and turns
  the override on with the rate 0.05. From then on it eases its own position and
  direction toward the scoreboard each frame and hands them to `SetOverrideCamera`. When
  it is destroyed it turns the override off, with 0.05 again. The speech bubble, each
  tick while it is not a subtitle, asks for the screen position of its microchip's
  place; more than 100 pixels outside the screen (against `GetXRes` and `GetYRes`) adds
  one to a counter, anything else sets the counter to 0; past 60 the bubble closes its
  message, unless a flag of the bubble says otherwise.

INFERRED: the up vector under an override is world +Y (the look-at build after the blend
was not read); the third component of the screen position is the depth after the divide
(no script reads it); the two resolution globals keep their values in play (a store
through a pointer at video set-up was not ruled out); the two pointers through which the
render camera is reached name one record. Not read: which of the two camera update
functions that call the apply function runs in a level frame, and where that is against
the script update; the fallback axis of an opposite pair.

Read and not bound: `Native.SetSpeechBubbleActive__Q5Thingb` (0x649464 -> 0x6d4eb0 ->
0x55f70; 16 levels, 63 calls in the pulsed run). FOUND: it finds one part of the Thing
and makes one call with the bool. Which part, and what the call does, is INFERRED only
(the camera tweak, a speech-bubble camera zone), so it stays unbound and counted.
`Cam.SetCameraFromToUp__rrrffff` writes the render camera directly, with no blend; only
the Pod calls it, and it is left for stage 6.

### What the runtime does

`lbp-runtime/src/camera.rs`. State in `LevelWorld::camera` (`CameraState`).

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| The override | `OverrideCamera`: position and direction as passed, target, rate, blend | FOUND |
| The two setters | `World::set_override_camera`, `set_override_camera_activation`: the stores above; counted and nothing else in a level without a PWorld | FOUND |
| Blend step | `step(blend, target, rate)`, in `LevelWorld::step_camera` | FOUND rule |
| Place in the tick | `tick_level`: after `apply_global_settings`, before the frame count goes up | INFERRED |
| Weight | `ease(blend)`, `OverrideCamera::weight` | FOUND |
| Mix | `blend_pose`: eye linear, direction by `slerp`; both directions taken at length 1 and the result too | FOUND mix; the lengths INFERRED (the game stores the direction as passed; the scoreboard passes length 1) |
| Opposite directions | `slerp` turns about world +Y, or +X for a direction along Y | INFERRED (the game's axis was not read) |
| Threshold and lens | `OVERRIDE_THRESHOLD` 0.0001; `OVERRIDE_FOV_DEG` 60, `OVERRIDE_NEAR` 10, `OVERRIDE_FAR` 40000 | FOUND |
| Camera matrix of a pose | `CameraPose::matrix`: right, up, backward, eye; up from world +Y | FOUND layout; INFERRED up |
| Screen | `SCREEN` = 1280 x 720, whatever a window measures; `screen_position` | FOUND |
| The view | `CameraState::view` (`CameraView`: camera matrix and view-projection matrix, level units), written by whoever draws; scripts never write it | - |
| `GetCameraMatrix` without a view | the default follow camera (`FOLLOW_EYE` from the first player's Sackboy, looking down -z) under the override; the identity without a player | INFERRED stand-in |
| `GetScreenSpacePosition` without a view | the middle of the screen, counted in `screen_queries_without_view` | INFERRED stand-in |

- Read API for a display: `LevelWorld::camera_pose(follow_eye, follow_dir)` gives the
  camera to draw with while the override is in (`CameraPose`: eye and direction, level
  units) and `None` while the blend is at or below the threshold. The display hands in
  its own follow camera each frame, so the follow camera keeps moving under the blend.
- Write API: after drawing, the display writes `camera.view`. `view_projection` builds
  the second matrix from a camera matrix and a lens for a caller without a renderer.
- The stand-ins are chosen so that a run without a display behaves as before this part:
  a defaulted screen position was (0, 0), which is on the screen too, so no bubble
  closes for being off screen and the message counts stay equal.
- Counters: set calls, activation calls and those that turned the override on, ticks
  with the override in, the highest blend, camera matrix queries, screen queries (all,
  without a view, more than the bubble's margin off the screen).
- `FOLLOW_EYE` (296, 168, 1218) is the one follow pose measured in the game's memory in
  an emulator; it moved from the viewer into the runtime, and the viewer reads it there.

Natives: 219 -> 225 in the table: the six of the table above. None is recorded only.

`level_scripts`: `play_all` has a `camera` block, and a level played alone has it as
`camera.override` with the override's last values.

### What the viewer does

`lbp-bevy-viewer --level-scripts` (`level_play.rs`, system `script_camera`).

- Each frame, after the viewer's own camera system has placed the camera, that placement
  is taken as the follow camera: its eye and view direction go to level units through
  the level's display matrix.
- While the override is in, the camera is put at `camera_pose` of that follow camera,
  with world +Y up, and takes the 60 degree field of view. When the override is out the
  camera's own field of view comes back. The viewer's orbit state is not changed, so
  orbit, pan and zoom go on underneath and the view returns to them.
- Then the camera the picture is drawn with is written as `camera.view`. The scripts
  read it on the next ticks: the scoreboard's first position is where the viewer's
  camera was, and a bubble's off-screen test is against the picture.
- Not applied: the override lens's near and far planes (the viewer keeps its own, in
  display units; applied from part 34); the players' weights in the follow camera.
- The viewer prints one line when the override comes in, is fully in, or goes out
  (`Camera at tick N: override ...`), with the camera's place in level units, and the
  "Level scripts" window has a camera line with the blend and the screen queries.
- A consequence to know: with `--play-sackboy` the Sackboy starts at the first
  checkpoint of the level file, and in some levels that checkpoint is the scoreboard's
  own (it sits inside the scoreboard's trigger). The scoreboard then fires on the first
  ticks and the camera goes to it and stays, as its script says. `--play-sackboy-at`
  starts him elsewhere.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path
gamedata/lbp2/th_da_vinci/final_levels/main_04_bravery_test/main_04_bravery_test.bin
--level-scripts --play-sackboy --play-sackboy-at -9168,2950,-400`. That starts the
Sackboy in the scoreboard's trigger. Without `--level-scripts` the follow camera stays.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Units: native call; tick (one `tick_level`); level. No view is written in these runs.

| What | plain | `--visit-triggers` | with `--pulse-switches` |
|---|---|---|---|
| Things stopped by a fault, before -> now | 4 -> 4 | 4 -> 4 | 4 -> 4 |
| Unbound natives called | 51 -> 48 | 87 -> 81 | 87 -> 81 |
| Unbound calls | 73,325 -> 62,000 | 1,002,445 -> 865,777 | 1,010,529 -> 872,549 |
| Native calls (unchanged) | 571,776 | 5,363,790 | 5,423,370 |
| `SetOverrideCamera` calls | 0 | 39,386 | 39,588 |
| Activation calls / of them on | 0 | 101 / 101 | 101 / 101 |
| Levels where the override comes in | 0 | 101 | 101 |
| Levels where it is fully in / still in after the last tick | 0 | 98 / 101 | 98 / 101 |
| Ticks with the override in | 0 | 39,386 | 39,588 |
| Camera matrix queries / levels | 0 | 101 / 101 | 101 / 101 |
| Screen position queries / levels | 3,775 / 28 | 32,360 / 61 | 32,730 / 70 |
| of them without a view / off the screen | 3,775 / 0 | 32,360 / 0 | 32,730 / 0 |

- The unbound natives that are gone are the six, with the calls the runs before counted
  for them; no native is new on the list and no other count on it changed.
- Fault kinds 2 and levels with a fault 2, as before. The messages, global settings,
  display, trigger events and switch reactions blocks are equal to part 19 in all three
  runs: the stand-ins keep a run without a display as it was.
- The override is in on 101 levels, the levels with a scoreboard whose trigger the
  visiting player reaches. In 3 of them the activation comes less than 20 ticks before
  the run ends, so the blend does not reach 1. No level turns it off: the scoreboard is
  not destroyed in these runs.
- With the rate 0.05 the blend takes 20 steps from 0 to 1 and 20 back, the last one
  landing on the target (MEASURED in the unit test, in f32).
- Viewer, release build, 1280 x 720, no focus taken (`--qa-capture`), the level and the
  start above: `Camera at tick 701: override fully in (blend 1.00, weight 1.00, target
  on); set calls 699, activations 1`, the camera at level (-9167.9, 3023.8, 800.0). Two
  frames looked at, without the panels. Without the scripts: the follow camera, the
  Sackboy left of the middle in front of the lower half of the scoreboard. With them:
  the camera is centred on the scoreboard, looks slightly up, and the scoreboard's
  frame fills the height of the picture. The scripts tick while the scene loads, so the
  override was fully in before the first frame was drawn: the blend itself was not
  seen. On comp_01_rocket_funland, framed on the bubble's Thing: 488 screen queries, 449
  of them before the scene was up (without a view), 0 off the screen, and the bubble
  stays open. The frames stay in ignored `local-analysis/`.

### Not modelled

- The follow camera itself: still the one measured pose in the viewer and the stand-in
  of the runtime. The game's follow camera (player weights, camera zones, cut scenes,
  the fades next to the override's) is engine code and a stage of its own.
- The up vector and roll under an override; the override's near and far planes in the
  viewer.
- Where the camera update runs in the game's frame.
- The speech-bubble camera (`SetSpeechBubbleActive`, the camera tweak's type).
- The scoreboard itself: its screens are not drawn, and the branch of its zoom that
  follows the winning player calls natives that are not bound and were never reached.
- The other camera and screen natives the draw functions of level scripts call
  (`Math.WorldToScreenSpace`, `Cam.IsPointOnScreen2D`, `HUD.GetScreenEdgeCrop` and
  others): nothing calls those functions yet.
- The Pod camera (`SetCameraFromToUp`): stage 6.
- Whether the screen is 1280 x 720 in every video mode, and what a 4:3 screen changes.

### Verification

`cargo test -p lbp-runtime`: 58 unit and 40 world tests. New: 6 unit tests in
`camera.rs` (the step: 20 steps each way, a rate larger than the gap, no rate; the ease;
the arc, its linear branch and an opposite pair; the pose at weights 0, a half and 1,
without a direction and with a position that is not a number; the camera matrix; the
screen position: the middle, the edges, y down, the point's w ignored, a point behind
the camera mirrored, w = 0) and two world tests:
`d269_the_override_camera_blends_in_and_the_screen_natives_use_the_view` (the six
natives on a made-up level: the stores, 20 ticks in and 20 out, the pose half way, the
camera matrix with and without a view and a player, the screen position with and without
a view, a level without a PWorld) and `d269_a_script_reads_its_screen_position_each_tick`
(a made-up script that asks for the screen position of its Thing each tick: 61 ticks
without a view, on the screen, and 61 ticks more than 100 pixels outside; the tick steps
the blend). The script keeps the numbers; the bubble's own counter and branch are not in
it. `lbp-script-vm`: the six natives in a world without the state, and their argument
checks. `lbp-bevy-viewer`: `the_view_for_the_scripts_is_in_level_units`. fmt, the CI
clippy line, the CI test set, the Python tests and `cargo check` of both viewers pass;
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` passes; the viewer's
tests pass but the one that fails on main (`sackboy_play::tests::input_script_steps`).
Seen in the viewer window on one level: the camera on the scoreboard. Not seen by eye:
the blend coming in or going out, a bubble closed for being off the screen. Nothing
compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer feature).
