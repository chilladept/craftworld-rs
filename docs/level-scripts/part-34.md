# Level scripts, part 34: the script camera's lens (2026-10-14, D283)

Tenth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change, no
new native. The camera a script writes with `Cam.SetCameraFromToUp` now draws with the
script's near and far planes as well as its field of view, and the override camera
with the game camera's lens. Part 31 left both planes out: the viewer drew with its own
near 0.01 and far 100 display units (64 and 636,705 level units in the Pod).

### What was found

A static reading of the 01.33 EBOOT (evidence only, nothing of it is copied):

- FOUND: the render camera's lens is five floats: an ortho factor, the vertical field of
  view in radians, the aspect (width over height), near and far. The projection builder
  makes a perspective projection when the ortho factor is 0 (OpenGL style, clip z -1 to
  1) and an orthographic one when it is above 0.
- FOUND: `Cam.SetCameraFromToUp` writes near, far, ortho factor and field of view from
  its four floats, in that order, on every call, then rebuilds the projection (only when
  near and far are both above 0). Part 23's "third float" is the ortho factor. The Pod
  passes 10, 100,000, 0 and 1.0 on every tick: near 10, far 100,000, a perspective
  camera, 57.3 degrees of vertical field of view. Nothing in the lens is per level.
- FOUND: the native does not write the aspect. The Pod frame keeps the aspect the
  render camera has, which is the game camera's: 16:9, or 4:3 under a display flag.
  INFERRED: 16:9 here (the flag is INFERRED to be a non-widescreen output setting).
- FOUND: the override camera (`Cam.SetOverrideCamera*`, part 20) has no lens of its own.
  The game camera's lens is near 10, far 40,000 plus a camera zone's pull-back, and 60
  degrees of vertical field of view by default, eased by camera zones. While the
  override is in, the zones' step is skipped: 60 degrees, near 10, far 40,000. Part 20
  said these three are the override's plain lens; near 10 and far 40,000 are the game
  camera's constants always (corrected there).
- FOUND: an engine letterbox exists. `HUD.EnterCinemaMode` and `LeaveCinemaMode` move a
  counter, and the bars drawn are each smoothstep(amount) x 0.17 + 0.01 of the screen's
  height (18 percent at full amount). The code that moves the amount toward the counter
  was not found. The Pod's scripts never call Enter or Leave (only `GetCinemaModeCount`).
  INFERRED: the black bar above the pod in the intro (ticks 30 to 200, part 31) is the
  level's own black rectangle (Thing 12, part 27), not the letterbox. Not compared
  against a frame of the game.

### Units

The runtime keeps all four floats of the call (`ScriptCamera`: `near`, `far`,
`lens_value`, `fov_y`), in the units the script passed. INFERRED: the game's world
units are level units (the eye, the point looked at and every Thing's position are
compared in them throughout parts 23 to 33). The viewer draws in display units: each
level has a display matrix, a uniform scale and a translation (`level_home`). The
planes are lengths, so they take the scale only: display = level x scale.

MEASURED: in the Pod (the planets level's display matrix, which both levels are drawn
in since part 31) the scale is 1.5706e-4 display units per level unit (100 / 636,705).
So Bevy gets near 0.0015706 and far 15.706 display units, and a vertical field of view
of 1.0 radian (57.30 degrees). Bevy's `PerspectiveProjection::fov` is the vertical one,
like the game's. The field of view was already the script's since part 23, so it does
not change.

In Bevy the projection has no far plane (an infinite, reversed-depth projection): the
far plane only culls whole draw entities by their bounds, and a mesh that crosses it is
drawn whole. The near plane clips per pixel as in the game.

### What the viewer does

`lbp-bevy-viewer` (`level_play.rs`) only.

- `script_lens`: the lens of a camera the runtime says the frame is drawn with
  (`DisplayCamera`). The field of view is the camera's when usable, else the viewer's;
  near and far are the camera's times the display scale when both are above 0 and far is
  beyond near (as the game's rebuild), else the viewer's. With an ortho factor other
  than 0 the viewer keeps its lens whole (PLACEHOLDER: the orthographic branch is not
  drawn; no script seen passes one). The aspect is never written: it stays the window's,
  16:9 under `--window-16-9`.
- The override camera gets the game camera's lens through the same function (the
  runtime's `DisplayCamera` already carries 60 degrees, 10 and 40,000). The zone
  pull-back is not modelled (no camera zones in the viewer).
- `script_camera` keeps the viewer's lens (field of view, near and far) while a script
  camera or an override has it, and puts it back once when it goes out
  (`wanted_lens`); a level without either never has its lens written.
- Printed: the `Script camera at tick N` line now gives the passed near, far and ortho
  factor and the lens drawn with: `drawn with vertical field of view 57.30 deg, near
  0.00157 and far 15.706 display units (10.0 and 100000 level units, scale 1.5706e-4),
  aspect 1.7778`. The `Pod levels under one camera` line gives `camera near plane 10, far
  plane 100000`.

### Measured (PC, release build)

Same setup as part 33: 1280 x 720, no focus taken (`--qa-capture`, window hidden),
`--level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --window-16-9
--preview-lighting --qa-hide-ui`, the d133 snapshot with the overlay. No key was sent
to a window. Frames in ignored `local-analysis/rscript/stage6/shots/` (`d283-*`); the
same fifteen commands with main's build first (`d283-base-*`), which gave the d282
frames byte for byte.

- Depth, from the bounds of the shown draw entities along the view (level units): at
  rest (452 to 600) the shell reaches from 221 behind the camera to 2,161 in front, the
  planets level and its room to 69,306 in front; held at 610, 70,739; at 200, 72,847; at
  30, 21,498. All below 100,000: no entity is culled by the far plane in any of these
  frames, so nothing of the planets level disappears.
- Equal byte for byte to part 33's (the lens moves nothing there): the closed Pod with
  `--play-sackboy` held at 470, the walk and press script held at 580, the intro held
  at 30 and at 200; of the eleven equality captures, the four without scripts, the
  shell under scripts closed at 600, in use from 600 at 640, the press on 600 at 641,
  the planets level with `--pod-camera 1` at 600, and pod.bin alone at 640.
- Changed, two of the eleven. In use from 600 held at 610: 3 pixels, at the frame's
  bottom left (x 198 to 209, y 702 to 716), all by less than 32 of 255 in every
  channel: a sliver of the shell between 10 and 64 level units from the camera that the
  old near plane cut. The planets level alone, scripts, held at 600 (no `--pod-camera`,
  so the level does not hang under its world Thing and the camera stands among its
  own Things): 140,640 pixels (15.3 percent) in the left 477 columns. The old near plane
  cut through a large object right in front of the camera (a strip of the wall behind
  showed at the left edge, and the shape in the middle was cut on its left); with near
  10 that object is drawn whole and covers the strip.
- Looked at, against the game's closed frame (ignored `local-analysis/d156-rpcs3-pod-default.png`,
  picture 1120 x 618; fractions of the picture, by eye, about 0.01): the tent's window
  0.34 to 0.68 across and 0.26 to 0.72 down (game 0.34 to 0.68, 0.25 to 0.73), Sackboy
  0.47 to 0.53 across (game 0.47 to 0.525; he sits in the viewer and stands in the game,
  so his height differs). The D283 still at 470 is the D282 still byte for byte, so the
  two are equally close to the game: the lens moved nothing in this frame. The match of
  the field of view was already part 31's.

Not seen: an override level with the new lens in a window, the motion. Not compared:
the intro against the game.

### Left out

- The orthographic branch (an ortho factor above 0) and `Cam.SetCameraFromTo` (it stores
  the lens after the rebuild, so its lens takes effect on the next one; no Pod script
  uses it).
- The camera zones' field of view and far pull-back for the game camera; the 4:3 flag.
- The letterbox: the code that moves its amount was not found; no Pod script uses it.
- Per-pixel far clipping (Bevy culls whole entities only); the game's depth range.
- The runtime's own comment on `ScriptCamera::lens_value` still says its use was not
  read (the runtime is untouched in this PR).

### Verification

`lbp-bevy-viewer`, none needs game data:
`d283_the_lens_is_the_natives_arguments_in_display_units` (the Pod's call gives 1.0
radian, near 10 and far 100,000 times the scale, 0.0015706 and 15.706 at the Pod's
scale; applied to a 16:9 lens the aspect stays 16:9 and the projection is a perspective
one, the top and side edges at tan(0.5) over the depth; an ortho factor of 0.5 keeps
the viewer's lens; planes the game would not rebuild with keep the viewer's; a field of
view no lens can have keeps the viewer's), `d283_the_override_lens_is_the_game_cameras`
(60 degrees, 10, 40,000), `d283_a_level_without_a_script_camera_keeps_the_old_lens`
(nothing written without a script camera; under one the script's lens, the viewer's
kept; out, the viewer's back once, then nothing). fmt, the CI clippy line, the CI test
set (612 pass), `cargo check` of both viewers and `cargo clippy -p lbp-bevy-viewer
--all-targets -- -D warnings` pass; the viewer's tests pass (84) but the one that fails
on main (`sackboy_play::tests::input_script_steps`). The runtime is untouched, so the
378-level runs were not repeated.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
