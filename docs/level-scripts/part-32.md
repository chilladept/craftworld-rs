# Level scripts, part 32: the viewer's own Sackboy in the Pod (2026-10-13, D281)

Eighth PR of the "Pod picture" follow-up to stage 6 (input option C), viewer only: no
runtime change, no new native. Until now the Sackboy of the scripted Pod picture was a
record put on the pod computer's trigger before the first tick, not drawn (part 30,
PLACEHOLDER). With `--play-sackboy` he is now the viewer's playable Sackboy: he stands
on pod.bin's own floor, is drawn in the tent, walks and changes lanes, and the pad zone
is decided by where he stands. Without `--play-sackboy` nothing changes: the record of
part 30 stays, still a placeholder.

### What was found

- The Pod's floor is pod.bin's own collision. FOUND (the level file): the pod Thing
  (Thing 2) has a shape of 54 vertices with depth, under a matrix that lays the
  outline's z along the level's y. In the plane of its matrix the outline is a slab
  from y -167 to -152: the "floor at -160" of the earlier notes. Projected through the
  whole matrix (`lbp_sim::build_spawn_scene_with`, the rule `level_spawn_feet` already
  used for the fitted figure) it is the pod's cross-section, y -1525 to 1282, with a
  floor inside. MEASURED (the floor finder on that scene): the floor is at y -546.8
  under x -4.2 and -544.3 under x 338. With Sackboy's rest height over a floor, 69.9
  (MEASURED on the Moon; the Rookie Test gave 68.8), he stands at -476.9 and -474.4.
  MEASURED in the game (RPCS3 memory, 2026-10-09): -477.97 on arrival at x -5.37 and
  -475.53 at x 338.10. So the game's "y -478" is this floor, 1.1 off at both places,
  with the same rise to the right. No height is fitted.
- The pod computer's trigger. FOUND (the level file, Thing 1, class `Trigger_Pod`): a
  box (type 1), radius multiplier 1, under a matrix with scales 167.3 and 264.7 at
  (15.8, -532.2, -193.7), one layer, no z offset. By the trigger rule of part 10 it
  holds a Sackboy whose position is within x -151.5 to 183.1 and y -796.9 to -267.4
  and whose z span covers -193.7. The pad zone then wants his z at -300 or more (part
  24).
- His z span. FOUND (01.33, the trigger test 0x135fe0): the span the layer rule tests
  is two floats (offsets 0x164 and 0x168) of an object the Sackboy's creature part
  points to (offset 0xbf0). Their values were not read. MEASURED in the game: the pod
  computer was engaged by a Sackboy standing in the front lane (z 0), so his span
  reaches at least 193.7 behind the lane's middle. The play mode's 100 (part 10,
  PLACEHOLDER) does not.
- Where the game puts him. MEASURED (same read): on arrival he stands at (-5.37,
  -477.97) in lane 0; the checkpoint (Thing 4) is at (-3.05, 540.76, -50). Whether he
  drops from the checkpoint or is set down was not measured. With the pod computer
  open his z was -200: INFERRED the game moves him one lane back on engaging.

### What the viewer does

`lbp-bevy-viewer` (`sackboy_play.rs`, `level_play.rs`, `main.rs`). Only with
`--play-sackboy --level-scripts --pod-camera N --pod-shell HEX`
(`sackboy_play::pod_shell`).

- The physics scene is the shell level's, not the level given
  (`sackboy_play::pod_scene`): pod.bin's shapes with the projected outline, so the
  floor above is what he stands on. Every body is held where the file has it and the
  two string joints are left out. PLACEHOLDER: the pod is a moving body in the game
  (MEASURED 3.6 from its stored x); the viewer does not draw the shell from the
  physics, so a pod that swung would leave him beside the picture.
- The start: the data spawn rule of the fitted figure (`data_spawn`, INFERRED): the
  checkpoint's x moved by the Moon's spawn offset (-1.11), the lane nearest the
  checkpoint's z (0), on the floor below, his rest height above it: (-4.16, -476.91,
  0), 1.2 and 1.1 from the game's measured place. He starts standing there; no drop.
- The figure: the play figure (`sackboy_play::spawn_figure`, his body mesh and the
  clip bank) is a child of the shell's frame root and is placed by the shell's display
  matrix (`Figure::display`), so he stands in the shell level's own frame (part 31)
  and is lit by the shell's light. The fitted Pod figure is not spawned then.
- The camera: the script's (part 31). The play mode's follow camera is not applied in
  the Pod.
- His place for the scripts: after every physics tick his position is kept
  (`PodSackboy::at`, written in `FixedFirst`), and before every script tick it goes to
  the shell world (`level_play::stand_sackboy`: `add_player` the first time, then
  `move_player`), with a z span of 200 either way (`POD_PLAYER_HALF_DEPTH`,
  PLACEHOLDER: a lane step, at least the 193.7 above). The tick's trigger pass then
  keeps the trigger's list, and `Native.GetIsInPadZone` answers from the list and his
  z. So script tick N sees him where physics tick N left him. He is not added to the
  planets level's world.
- The pad: as part 30 (`feed_pad`), bit 0x80 on the tick of a press. The press comes
  from the engage key, which with him is Z (the play mode's Square key) or gamepad
  Square, since Space is his jump; from `--level-scripts-press-at N`; or from `e` in
  `--play-sackboy-input` (the engage button on the first tick of that step). No other
  button of his pad is written to the button word: which bit is which button is not
  established (part 24).
- No Popit in the Pod (its Square is the engage button there).
- `--level-scripts-hold-at N`: his input and his pose are held with the script ticks
  (`PodSackboy::held`), so a still of a standing Sackboy is the same bytes each run.
  The ticks that ran before his figure was loaded are kept and the animation picker
  catches up on them, so his pose on a tick does not depend on the load time.
- Printed: `Pod Sackboy start: data spawn: ... floor y -547 ...; he stands 69.9 above
  it at (-4.16, -476.91), lane 0; MEASURED in the game on arrival: (-5.37, -477.97)`,
  `Pod Sackboy before tick 1: Thing 19 of the shell level, the viewer's own Sackboy at
  [-4.2, -476.9, 0.0] (on the floor)`, `Pod Sackboy at tick N: in / out of the pad
  zone, standing at ...` when that changes, and a press that is ignored says where he
  stands.

### Measured (PC, release build)

Viewer, 1280 x 720, no focus taken (`--qa-capture`, window hidden), `--level-guid
00018fdb --pod-camera 1 --pod-shell 000027b9 --level-scripts --play-sackboy
--window-16-9 --preview-lighting --qa-hide-ui`, the d133 snapshot with the overlay, a
fresh profile. No key was sent to a window. The frames stay in ignored
`local-analysis/rscript/stage6/shots/` (`d281-*`).

- Standing, `--play-sackboy-input "-:1"`: on the floor from the first tick at (-4.2,
  -476.9, 0); in the pad zone on tick 1; he stays there for 600 ticks.
- Walking, `"-:460,r:12,e:1,-:30,l:12,-:20,e:1,-:60"`: out of the pad zone on tick 471
  at x 189.5 (the box ends at 183.1); the press on 472 at x 218.8 is ignored (`the
  Sackboy is not in the pad zone`); he stops at x 347.9; back in the zone on 514 at x
  152.9; the press on 535 at x -12.1 is taken; in use on 536; the change-over 0.05 on
  536 and 1.0 on 555. On the way his y stays -476.9; the floor finder says -474.4 at x
  338 and the game has -475.5 there: his controller did not follow the floor's rise of
  2.5 (not looked into).
- Lanes, `"-:460,b:1,-:40,b:1,-:40,e:1,-:20,f:1,-:30,e:1,-:60"`: one lane back (z
  -200 by tick 480) he stays in the zone; on the way to the back lane he is out of it
  on tick 504 at z -305, by the z rule; the press on 542 at z -400 is ignored; coming
  forward he is in on 566 at z -295; the press on 594 is taken; in use on 595.
- Stills taken twice with equal bytes: standing held at 100, and held at 600 (by then
  he sits, the play mode's idle chain).
- Against part 31's frames of the same ticks only his box differs: closed at 600,
  5,976 pixels in (602, 504) to (673, 634); open (press on 600, held at 641), 6,310
  pixels in (597, 616) to (676, 720); value 0.5 (held at 610), 6,515 pixels. He casts
  no shadow in these frames.
- Compared by eye against the game's closed frame (`local-analysis/d156-rpcs3-pod-default.png`,
  part 31's fractions of the picture, about 0.01): the standing Sackboy is 0.47 to
  0.525 wide and 0.706 to 0.885 high in the game, 0.470 to 0.523 and 0.700 to 0.881 in
  the viewer (held at 470). The game's frame has the engage prompt with him at that
  place, as the viewer has him in the pad zone there.
- Unchanged, equal byte for byte to part 31's captures on the same commands (no
  `--play-sackboy`): without scripts closed, `--pod-open`, `--pod-game-camera`, `--pod-t
  0.5`; with the shell under scripts closed at 600, in use from 600 held at 610 and at
  640, the press on 600 held at 641; the planets level alone at 600; pod.bin alone in
  use from 600 at 640. The play mode on one level (pod.bin alone, 300 ticks): the
  positions of all ticks equal the build before; its frames differ from run to run, as
  before (the pose there depends on the load time).

Looked at: closed at 470 (he stands in the middle of the tent's floor in front of the
DualShock), closed at 500 after the walk (at the right, in
front of the tent's right edge of the window), closed at 600 (sitting), open at 580
(the tent gone, his head at the bottom middle in front of the DualShock).

Not seen: the keys themselves (the script's `e` sets the press where the key's latch
would), the motion, the way back. Not compared: the open frame and any frame of him
walking against the game.

### Left of option C

- Without `--play-sackboy` the scripted Pod picture still has part 30's undrawn record
  on the trigger (PLACEHOLDER). The play mode is not made the default there.
- The game moves him to z -200 with the pod computer open (INFERRED from one read); the
  viewer leaves him where he stood, and he can still walk while it is in use.
- Whether he drops from the checkpoint on arrival; the 1.2 in x between the data spawn
  and the measured place.
- His z span (PLACEHOLDER 200; two floats of the game not read); the pod as a moving
  body and its strings; the floor's rise under a walking Sackboy.
- In the back lane he stands at x -4.2 inside the DualShock's shape (Thing 10, x -209
  to 244, y -549 to -351 in that lane): the play mode's lane change tests for a floor,
  not for a body in the lane. Not looked at in a frame.
- The Popit in the Pod, the engage prompt, his costume and the player's colour, his
  shadow, the other buttons of his pad in the button word.
- Everything parts 29 to 31 left out.

### Verification

`lbp-bevy-viewer`, none needs game data:
`d281_the_pad_zone_is_decided_by_where_the_sackboy_stands` (pod.bin's trigger numbers:
in at the game's measured arrival, out at its measured place after the walk, the box's
sides, a jump, the lane behind, the back lane, the z rule at -310 and -300; one
Sackboy all along; not in with the play mode's span),
`d281_a_press_outside_the_pad_zone_is_ignored_with_his_place`,
`d281_the_pod_wants_the_play_mode_and_the_scripted_shell`,
`d281_the_engage_key_of_a_script_is_a_click_on_its_steps_first_tick`,
`d281_the_pod_scene_has_the_projected_floor_and_holds_every_body` (a shape laid out as
the pod Thing's: the floor through the whole matrix in all three lanes and not in the
plane; every body held, no joint; the start on it within 1.2 of the game's y). fmt, the
CI clippy line, the CI test set (612 pass) and the Python tests (36) pass; `cargo check`
of both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`
pass; the viewer's tests pass (78) but the one that fails on main
(`sackboy_play::tests::input_script_steps`). The runtime is untouched, so the 378-level
runs were not repeated. Seen in the viewer window: the stills above. Compared against
the game: his place and size in the closed frame, by eye, against one earlier RPCS3
frame.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
