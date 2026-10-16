# Level scripts, part 26: the Pod shell layer under the script's camera (2026-10-13, D275)

Second PR of the "Pod picture" follow-up to stage 6, viewer only. With the shell layer
(`--pod-camera N --pod-shell HEX`) the viewer kept its own Pod camera under
`--level-scripts` and printed that the script's camera was "not applied" (part 23): the
shell level is placed each frame so that one camera can draw two levels, and that
placing knew only the rig's own two planets poses. Now the planets level is drawn with
the camera its world script writes, as without the shell, and the shell and the tent are
placed from that camera.

### What the game does

- FOUND (part 23): the planets camera is the one `PlanetsWorld` writes each tick.
- MEASURED before (P2b to P2k): the shell level is drawn over the planets through a
  camera of its own. Which engine camera that is, and how it moves when the pod computer
  opens, is not established: the shell eyes stay the viewer's fitted ones.
- Nothing new was read in the executable for this PR.

### What the viewer does

`lbp-bevy-viewer` (`pod_menu.rs`, `level_play.rs`, `main.rs`).

- `PodRig::layer_at(eye, turn, s, rot)` is the old `layer` with the planets pose passed
  in: a shell point `X` goes to `eye + scale * turn * rot^-1 * (X - s)`, so from a
  camera at `eye` turned by `turn` it shows where the shell eye `s` turned by `rot` sees
  it. `layer` calls it with the rig's own poses (the same arithmetic in the same order).
  `shell_root_at` and `tent_root_at` stand beside `shell_root` and `tent_root`.
- Under `--level-scripts` with `--pod-camera N` (`LevelMode::pod_scripted`), with or
  without the shell layer, the planets level hangs under the root that follows its world
  Thing (part 23) and the Thing map is on (part 25): Earth and Moon are placed by their
  own scripts in both paths.
- `script_camera` puts the camera where the script wrote it and then places the shell
  root and the tent root with `shell_root_at` / `tent_root_at`: the camera's eye and turn
  in display units, and the opening progress `t` of the viewer's own Puter state. One
  line says when this first happens. `pod_dolly` still moves `t` on the Puter key but
  leaves the camera, its field of view and the roots alone in this mode.
- The rig is built with the game's field of view (57.3 degrees) whenever the scripts
  run: the script's lens is 1.0 rad, and the shell eyes are fitted for the rig's field
  of view. The rig's own planets eyes are the two camera presets (they give the first
  frames before a tick, and Home), and the shell's size is set as under
  `--pod-game-camera`: its far side at half the resting camera's distance to the Earth.
- The Puter key (Space, or `--pod-open` at the start) is handed to the script world as
  `pod_computer_in_use` when it changes (`puter_change`): the stand-in `level_scripts
  --puter-in-use` uses. The planets camera then goes to the in-use preset while the
  shell dollies. The planets view stays view 0.
- The menu constants are not mixed in. Under `--level-scripts` `--pod-game-camera` no
  longer moves the planets to `EARTH_MENU` / `MOON_MENU` (`place_menu_planets`), no
  longer hangs the level under the constant view transform (`planets_root(t)`) and no
  longer turns per-bone placement on by default (it would put the stored places of the
  bone Things into the vertices): the capture with and without that flag is equal byte
  for byte. The "not applied" line is gone.
- Without `--level-scripts` nothing changes.

The table of part 23, last row, now reads: with `--pod-camera N --pod-shell HEX` the
camera is the script's, the planets hang under the root that follows the world Thing,
and the shell layer is placed from the script's camera.

### Measured (PC, release build)

Viewer, 1280 x 720, no focus taken (`--qa-capture`), the planets level with
`--pod-camera 1 --pod-shell 000027b9 --window-16-9 --preview-lighting --qa-hide-ui`.
The frames stay in ignored `local-analysis/rscript/stage6/shots/` (`d275-*`).

- Without `--level-scripts`, before (main at D274, the same build tree) and after, with
  and without `--pod-game-camera`: equal byte for byte, both pairs. The pair with
  `--pod-game-camera` also equals part 23's capture.
- With `--level-scripts` and `--level-scripts-hold-at` 30, 200, 440, 600. Printed: `Pod
  shell layer at tick 30: placed from the script's camera (... opening 0.00, shell scale
  6.0818)`; the camera, world Thing, Thing map and fog bias lines equal part 25's (`Script
  camera at tick 452: at rest; eye [0.0, -250.0, 1130.0]`).
- Looked at, tick 600 (camera at rest): the tent, its two side panels and the DualShock
  are where the capture without scripts has them. Through the tent's window: the Earth
  near (0.46, 0.48) of the frame, the Moon near 0.56 of the width, the lamp above
  them, the table and the figure at the lower right, the room's wall behind. The same
  places as in the `--pod-game-camera` capture without scripts; that one is washed out
  in orange (no fog bias without scripts), this one is clear with a light haze.
- Tick 30: the window shows the room's ceiling only (the camera looks steeply up; the
  planets are below the window). Tick 200: ceiling in the upper half of the window, the
  wall below, the top of the Earth at the window's lower left, beside the DualShock.
  Tick 440: as tick 600 with a small shift.
- In none of the four is a part of the shell seen cut by the planets or the room, or
  the other way round, and the fog does not swallow the view in the window.
- With `--pod-open` and hold at 600: `Pod computer at tick ...: in use true`, and the
  camera comes to rest at eye (0, -300, 900), direction (0, 0.316, -0.949): the in-use
  preset. Looked at: the tent is out of the frame, the DualShock is at the lower middle,
  the room is seen from below with the Earth just above the DualShock (its lower edge
  behind it) and the Moon to its right. This is view 0 under the in-use camera, not the
  game's menu view (the Earth is not at the menu view's place).
- With the default lighting of level mode (no `--preview-lighting`), ticks 30 and 600:
  the same framing; the tent is shaded the same in both.
- Under `--preview-lighting` the tent is shaded differently at tick 30 than at tick 600
  (greener): the shell root turns with the script's camera, and the preview light is
  not turned with it. The default lighting turns each layer's light with its root.

Not seen: the motion (stills only), the dolly between closed and open under scripts.
Not compared: any frame against the game, side by side.

### Not modelled

- The shell level's own camera in the game and its motion; the tent's fade and the
  pod's change-over (`PodTransition` in pod.bin's world script): `t` is the viewer's
  own dolly.
- The pod computer's menu views (view 1 and on) and their fog bias.
- The preview light of the shell turned with its root.
- A script camera that goes out while the shell layer is up: the roots stay where they
  were placed last. The planets level does not do that.

### Verification

`lbp-bevy-viewer`: `d275_layer_at_keeps_the_shell_projection_under_any_planets_camera`
(`shell_root_at` and `tent_root_at` equal `shell_root` and `tent_root` at the rig's own
poses, bit for bit; under three other cameras, one pitched like the intro's, one turned
and rolled, one far back, a shell point keeps its frame fraction at `t` 0, 0.5 and 1 and
its depth is the shell eye's times the scale; the tent keeps its place) and
`the_puter_state_is_handed_to_the_scripts_when_it_changes`. Neither needs game data.
fmt, the CI clippy line, the CI test set and the Python tests pass; `cargo check` of
both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass; the
viewer's tests pass but the one that fails on main
(`sackboy_play::tests::input_script_steps`). Seen in the viewer window: the stills
above. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
