# Level scripts, part 71: Sackboy at the pod computer in the scripted Pod (2026-10-15, D320)

The game's Pod has its player at the pod computer, seen from behind, holding the DualShock
(GPU-captured frames 8980 and 9601, lane B). The viewer's scripted Pod drew the DualShock alone:
without `--play-sackboy` the session's Sackboy was part 30's undrawn record, and with it he stood
in the play mode's idle pose, drawn toward the engage target by the change-over value (part 33's
placeholders). Now the scripted Pod has the viewer's own Sackboy by default and he plays the pod
computer's own clips by the game's rule.

### What was found

A static reading of the 01.33 EBOOT (evidence only, nothing of it is copied):

- FOUND: the pod computer's clips. The Sackboy animation object's loader reads a GUID table in
  order: button_o, button_x, idle, into, stick_l, out (`sack_boy_pinched_anims/pod_computer_*`,
  GUIDs 000042b0, 000042b1, 000042b2, 000042b3, 000042b4, 000042e9). MEASURED (the files): into
  31 frames, idle 21, out 31, all at 30 fps.
- FOUND: the pod computer user's step (the function part 33 read for the target), once a tick
  with a frame counter n:
  - The tick he becomes the user the into clip starts at frame n = 0, one frame a tick. The pod
    layer's weight over his own pose is (n + 1) / 3 and the drawn offset's blend (n - 5) / 11,
    each clamped to 0..1. When the clip has played out the idle clip loops from frame 0, weight
    and blend 1. Part 33's "blend = the clip's progress" was this ramp.
  - The pad's cross and circle (bits 0x40, 0x20) start the button_x and button_o clips while he
    holds; the stick clip drives further layers. Neither is modelled (the viewer's pad writes only
    the engage bit, part 30).
  - The tick he stops being the user the out clip starts at frame 0: blend 1 - (n - 5) / 10,
    weight (frames - n) / 3, clamped; when it has played out the layer is off.
  - The into clip waits while a creature state is not 2 (INFERRED: a standing Sackboy's; the
    viewer's stands).
- FOUND: the offset (target - his position) x blend is sent to his root bone's translation in the
  model's axes (x, -z, y). The root bone's rotation gets the facing turn (a quaternion product).
- MEASURED (his head on the game's frames against the viewer's stills with the offset drawn as
  part 33 had it, at the target's depth; the DualShock the same size in both): the viewer's
  Sackboy was 1.21 times the game's (head 80 against 66 px wide at tick 616, 74 against 61 at
  625 / 619). At the camera's distance that puts the game's Sackboy about 300 units deeper than
  the target's z -93.2, so about -300, just in front of the DualShock ("pod_controller", z -354
  in the game's memory, 2026-10-09). His x on the frames is the target's within 5 px.
- MEASURED (the game's memory, 2026-10-09, `local-analysis/results/2026-10-09-pod-spawn-memory.md`):
  at the pod computer his z was -200 and his x unchanged.
- INFERRED: the depth comes out of the offset reversed, z - (target z - z) x blend, with z the
  -200 of the memory read: -306.8 at blend 1. Neither the reversed sign nor what moves him to -200
  is established in the code; together they give the measured depth and keep x and y by the FOUND
  rule.

### What the viewer does

`lbp-bevy-viewer` (`sackboy_play.rs`, `level_play.rs`, `main.rs`):

- The scripted Pod (`--level-scripts --pod-camera N --pod-shell HEX`) has the viewer's own
  Sackboy without `--play-sackboy`: the play mode in pod.bin (parts 32 and 33), his figure the
  Sackboy mesh with the default costume and colours as before. `--no-pod-sackboy` leaves him out
  (part 30's undrawn record, the frames of main byte for byte); `--play-sackboy` overrides it.
- The clip bank loads the three pod computer clips after the picker's (`POD_CLIPS`).
- `PodUserAnim` steps the found rule once a script tick (`level_play::tick`, after "in use" is
  read): clip, frame, weight and offset blend (`PodSackboy::user`). His pose is the picker's
  blend with the pod clip's pose lerped over it by the weight, the whole body, before the root's
  rotation is put back (part 33's facing pi stays).
- His figure is drawn at `engage_drawn`: x and y toward the target by the blend (FOUND), z from
  `POD_USER_LANE_Z` (-200) reversed by the blend (INFERRED above), while the pod layer is on.
  PLACEHOLDER: no walk back to that lane, he is drawn there from the first tick of the layer; his
  physics place and so his place for the triggers stay where he stood.
- Printed: the clips' frame counts in the `Play figure` line, and `Pod Sackboy at tick N: pod
  computer layer pod_computer_into from frame 0 (weight 0.33, offset blend 0.00)` when the layer
  changes clip.
- Gone: `engage_blend` (the change-over value as the blend).

### Verification

Viewer stills, hidden window, level lighting, in use from 600 (`local-analysis/d320-earth/shot.ps1`),
against the game's frames (`local-analysis/d316-zoom/rt8980_up.png`, tick 616 by the dissolve
clock, and `rt20_9601_up.png`, tick 619, part 69). His head box by eye on 5x crops with a 10 px
grid (`local-analysis/d320-sackboy/`), in px of 1280 x 720:

| tick | game head x, top | main | target's depth (part 33's rule) | D320 |
|---|---|---|---|---|
| 616 | 632-698 (66), 606; centre 665 | not drawn | 630-710 (80), 611 | 630-695 (65), 604; centre 662 |
| 619 | 624-685 (61), 620; centre 655 | not drawn | - | 622-683 (61), 619; centre 653 |
| 625 | (619's) | not drawn | 614-688 (74), 623 | 620-680 (60), 615 |

- Pose: at 616 the left hand up at the left stick and the right arm out over the right stick, as
  in 8980; at 619 both hands on the sticks (game 610-635 and 680-695 across, 665-700 down; viewer
  610-640 and 680-693, 667-705), as in 9601. The body below the head is cut by the frame's edge in
  both.
- Closed (600): he stands in front of the DualShock (the play mode's arrival, part 33). 605: his
  back to the camera, smaller, at the DualShock (the lane placeholder above). 640, 660: the idle
  clip.
- `--no-pod-sackboy` at 616 is main's still byte for byte.

Tests (`lbp-bevy-viewer`, no game data): `d320_the_pod_computer_layer_runs_into_idle_and_out`;
`d281_the_pod_wants_the_play_mode_and_the_scripted_shell` and
`d282_the_pod_computer_user_is_held_turned_and_drawn_toward_the_target` changed for the default
and the depth. fmt, `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`, the viewer's
tests (160) but the one that fails on main (`sackboy_play::tests::input_script_steps`). The
runtime is untouched. Stills stay in ignored `local-analysis/rscript/stage6/shots/d320-d320sb*`.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
