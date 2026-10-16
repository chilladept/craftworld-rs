# Level scripts, part 33: his z span, his arrival and the pod computer's user (2026-10-14, D282)

Ninth PR of the "Pod picture" follow-up to stage 6 (input option C), viewer only: no
runtime change, no new native. Three placeholders of part 32 are replaced by what the
01.33 EBOOT does (a static reading; the code is evidence only, nothing of it is copied):
the viewer's own Sackboy in the Pod now has the game's z span for the triggers' layer
rule, arrives by the game's checkpoint spawn rule, and at the pod computer is held,
turned and drawn the way the game's animation code treats its user. Only with
`--play-sackboy --level-scripts --pod-camera N --pod-shell HEX`.

### What was found

- His z span. FOUND (01.33): the two floats the trigger test reads are absolute world
  z, not offsets from Sackboy: the trigger's z plus its z offset must lie between them.
  They belong to the creature's current state record. The creature's constructor sets
  them to -500 and 100, and every creature update writes them again from the same two
  constants. For a Sackboy or Sackbot three paths can change them: around the centre z
  c of a box of the level, min(-500, c - 100) to max(100, c + 100); max 500 under a
  global flag; max 300 from another path (its condition was not fully read). INFERRED:
  none of these changes the range in normal play (any box centre from -400 to 0 leaves
  it as it is, and the flag paths go with a state that makes the trigger test reject him
  anyway). So the range is -500 to 100 in every lane. pod.bin's trigger at z -193.7 holds
  him in all three lanes, and the lane decision is the pad zone's own z rule (z at least
  -300, part 24) alone. That fits the measurement of part 32 (engaged from lane 0).
- His arrival. FOUND (01.33, the checkpoint spawn): his position is the checkpoint's
  world translation; z is the larger of the checkpoint's z and the z of a box of the
  checkpoint Thing, then 50 is added; then x is moved by a per-player offset by spawn
  slot (0, +80, -40, +40, -80; the first player takes slot 0). There is no height
  offset: he appears at the checkpoint's height. INFERRED: he falls from there (no floor
  search was seen). In the Pod (checkpoint Thing 4 at (-3.05, 540.76, -50)) that is
  (-3.05, 540.76, 0), lane 0, falling onto pod.bin's floor. MEASURED in the game on
  arrival: (-5.37, -477.97); the 2.3 in x is not explained by this code.
- The pod computer's user. FOUND: no native of the Pod's scripts moves him or changes
  his layer. While the pod computer is in use by him (the primary player) the Sackboy
  animation code switches his input context to a pod context, turns him to face angle
  pi (into the screen), plays enter, hold and leave clips, and applies a root offset to
  his drawn body of (target - his position) times a blend, the blend being the clip's
  progress 0 to 1. The target is a constant (-12.469, -482.979, -93.170) whose x is
  replaced by the x of the Thing named "pod_controller" (pod.bin's Thing 5). INFERRED:
  the offset is drawn only; no write to his physics position was found. INFERRED: his
  walking and jumping input is not applied in the pod context (no explicit lock was
  found). Part 32's "the game moves him to z -200" (INFERRED from one memory read) is not
  explained by this code: the drawn offset goes to z -93.2, and nothing writes his z.
  Which value that read saw is open.
- FOUND (the level file): pod.bin's "pod_controller" Thing stands at x 17.846, not at
  the constant's -12.469. The game takes the Thing's x at the time (the pod is a moving
  body, MEASURED 3.6 from its stored x), so 17.8 is the target's x as far as the file
  tells; the gap of 30.3 to the constant is not explained.

### What the viewer does

`lbp-bevy-viewer` (`level_play.rs`, `sackboy_play.rs`).

- The span: `level_play::POD_PLAYER_Z_SPAN`, the fixed range -500 to 100 whatever his z
  (it replaces part 32's `POD_PLAYER_HALF_DEPTH`, 200 either way). The play mode's own
  span on other levels (`PLAYER_HALF_DEPTH`, 100 either way, part 10, PLACEHOLDER) is a
  runtime matter and is left as it is.
- The arrival: `sackboy_play::checkpoint_arrival` with the shell level's first
  checkpoint, slot 0, and no box (the box of the checkpoint Thing was not identified:
  INFERRED the checkpoint's own z). He starts there, in the air, and falls in pod.bin's
  physics scene. Nothing is between: the floor finder's first top below the start in
  lane 0 is the floor, y -546.8. The data spawn (`data_spawn`, the Moon's x offset -1.11
  and the floor below) stays for the fitted figure and other levels; FOUND slot 0 has no
  x offset, so the Moon's -1.11 is a next step there.
- At the pod computer (the session's "in use", he being its only player):
  `level_play::tick` writes `PodSackboy::in_use` and the blend after each script tick.
  His movement input is not applied (`pod_input`: no walk, jump or lane change, from a
  script or the keys); the engage button still reaches the pad. His facing eases to pi
  at the picker's own rate, 0.4 per tick (`pod_facing`), and back to the picker's angle
  the same way afterwards (INFERRED). His figure is drawn at his physics position moved
  toward the target by the blend (`engage_drawn`); his physics position and so his place
  for the triggers stay. The target is the constant with x from the "pod_controller"
  Thing (`engage_target`). The blend: PLACEHOLDER, the change-over value of pod.bin's
  script (0.05 on the tick of the host's answer, 1.0 nineteen ticks later), since the
  clips and their timing are not available; 0 when not in use. His pose is the picker's
  with no input (stand, then the idle chain): PLACEHOLDER for the enter, hold and leave
  clips.
- Printed: `Pod Sackboy start: checkpoint spawn (FOUND, 01.33): checkpoint Thing 4
  (-3.05, 540.76, -50), z max(checkpoint z, box z) + 50 ..., he starts at (-3.05,
  540.76, 0), lane 0, and falls; the floor below is y -546.8 ...`, `Pod Sackboy engage
  target: (17.846, -482.979, -93.170): x from Thing 5 "pod_controller" ...`, the span in
  the `before tick 1` line, and `Pod Sackboy at tick N: at the pod computer: ...` and
  `... no longer in use ...` when that changes. The scripted log adds `shown` with the
  facing shown when it is not the picker's.

### Measured (PC, release build)

Same setup as part 32: 1280 x 720, no focus taken (`--qa-capture`, window hidden),
`--level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --level-scripts
--play-sackboy --window-16-9 --preview-lighting --qa-hide-ui`, the d133 snapshot with
the overlay. No key was sent to a window. Frames in ignored
`local-analysis/rscript/stage6/shots/` (`d282-*`).

- Arrival, `--play-sackboy-input "-:1"`: tick 1 at (-3.0, 540.8, 0) in the air, out of
  the pad zone; in the pad zone on tick 25 at y -331.3 while still falling (the box
  reaches up to -267.4); on the floor on tick 28 at y -476.4 (vy -72.6 on tick 27). He
  rests at -476.4, 0.5 above part 32's placed start: the capsule after a fall. Against
  the game's arrival (-5.37, -477.97): 2.3 in x, 1.6 in y.
- Walking, `"-:460,r:12,e:1,-:30,l:12,-:20,e:1,-:60"`: out of the zone on tick 471 at x
  190.6; the press on 472 at x 220.0 ignored; back in on 514 at x 154.0; the press on
  535 taken; in use on 536 (at x -11.0); the change-over 0.05 on 536 and 1.0 on 555. The
  shown facing: 46.1 degrees on 537, 151.1 on 540, 177.8 on 545, 180.0 from 560.
- Input while in use, the same script then `"-:30,r:20,b:1,-:30"`: he stays at x -11.0,
  z 0 from tick 536 to 620.
- Lanes, `"-:460,b:1,-:40,b:1,-:40,e:1,-:20,f:1,-:30,e:1,-:60"`: out of the zone on 504
  at z -305 (the z rule; with the fixed span he stays in the trigger's list), the press
  on 542 at z -400 ignored, in on 566 at z -295, the press on 594 taken, in use on 595.
  The same ticks as part 32.
- Stills taken twice with equal bytes: standing held at 470 (he sits by then: the idle
  chain, which in part 32 had not reached that tick) and in use held at 580.
- Looked at: closed at 470 (sitting in front of the DualShock, middle of the tent's
  floor), in use at 545 and 580 (his back to the camera, his head at the bottom middle in
  front of the DualShock), the lane run at 600 (five ticks in use, turning, his back
  seen).
- Unchanged, equal byte for byte to part 32's captures on the same commands (no
  `--play-sackboy`, eleven): without scripts closed, `--pod-open`, `--pod-game-camera`,
  `--pod-t 0.5`; the shell under scripts closed at 600, in use from 600 held at 610 and at
  640, the press on 600 held at 641; the planets level alone at 600 and with
  `--pod-camera 1`; pod.bin alone in use from 600 at 640.

Not seen: the way out of the pod computer (no script path for closing, part 30), so the
return of his input and facing was not run in the viewer (only in the tests). Not
compared: any of these frames against the game.

### Left of option C

- The enter, hold and leave clips of the pod computer and their timing (the blend is the
  change-over value); the bone poses on the pod_controller mesh (IK to his hands,
  INFERRED from a call not read in full).
- The 30.3 between the "pod_controller" Thing's x and the target constant's; the 2.3 in
  x and 1.6 in y of his arrival against the game; the box of the checkpoint spawn.
- What part 32's memory read of z -200 with the pod computer open was.
- The play mode's span on other levels (PLACEHOLDER 100 either way) and its data spawn x
  offset (-1.11, where FOUND slot 0 has none).
- Without `--play-sackboy` the scripted Pod picture still has part 30's undrawn record.
- The pod as a moving body and its strings; the floor's rise under a walking Sackboy; a
  body in the lane on a lane change; the Popit, the engage prompt, his costume and
  colour, his shadow.

### Verification

`lbp-bevy-viewer`, none needs game data:
`d282_the_span_is_the_fixed_range_in_every_lane` (pod.bin's trigger numbers: his span is
-500 to 100 in lanes 0, -200 and -400; listed in all three; the pad zone in at 0 and
-200, out at -400 by the z rule), `d282_the_arrival_is_the_checkpoint_spawn_rule` (a
synthetic checkpoint: +50 in z, a box in front wins, the five slots' x, no height
offset), `d282_the_pod_computer_user_is_held_turned_and_drawn_toward_the_target` (offset
0 at blend 0 and the target at 1, blend 0 when not in use, input dropped while in use and
back after, facing pi at the picker's rate and back);
`d281_the_pad_zone_is_decided_by_where_the_sackboy_stands` changed: the back lane is now
in the trigger's list and out of the zone by the z rule. fmt, the CI clippy line, the CI
test set (612 pass), `cargo check` of both viewers and `cargo clippy -p lbp-bevy-viewer
--all-targets -- -D warnings` pass; the viewer's tests pass (81) but the one that fails
on main (`sackboy_play::tests::input_script_steps`). The runtime is untouched, so the
378-level runs were not repeated.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
