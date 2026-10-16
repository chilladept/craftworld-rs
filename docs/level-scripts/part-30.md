# Level scripts, part 30: the pod computer is entered through the pad zone and the press, in the viewer (2026-10-13, D279)

Sixth PR of the "Pod picture" follow-up to stage 6 (PR 4b): the viewer, one function
moved into the runtime. No new native. Until now the Puter key of the Pod picture set
"pod computer in use" on both worlds itself (part 28's stand-in). Now the key is the
engage button on the pad the scripts read, and pod.bin's own script does the rest, by
the path of part 24: pad zone, click, `RequestHostUsePod(true)`, the host's answer one
tick later. No drawn Sackboy.

### What was added

`lbp-runtime` (`pod.rs`):

- `LevelWorld::pod_trigger_spot()`: the world position of the first Thing with a
  PTrigger, a position and the script class `Trigger_Pod`, with the trigger's z offset.
  It is the spot finder of the `level_scripts` example's `--enter-pad-zone`, moved; the
  example calls it. PLACEHOLDER for a player who walks there: the middle of the trigger
  is a spot inside it, not a place the game puts anyone.

`lbp-bevy-viewer` (`level_play.rs`, `pod_menu.rs`, `main.rs`), only with the shell
level in the session (`--level-scripts --pod-camera N --pod-shell HEX`):

- The session's Sackboy. Before the shell level's first tick `add_player` puts a
  Sackboy on that spot, one unit of depth either way (`place_sackboy`). The first tick
  puts him in the trigger's list, and `Native.GetIsInPadZone` is true for him from then
  on (the spot's z is -193.7 in pod.bin, MEASURED; the pad zone wants -300 or more).
  PLACEHOLDER: he stands there from the start and is not drawn.
- The pad. Before every tick the button word of player 0's pad is written on both
  worlds (`feed_pad`; both, as the tool's `--press`: the pad is the player's, not a
  level's): bit 0x80 on the tick of a press, 0 on every other (`pad_word`). So a press
  is one tick long, which is what the game's "just clicked" wants (down now, up the
  frame before).
- The keys. `pod_dolly` no longer sets the Puter state's `open` under the scripts: the
  key that went down waits in `PuterState::key` for the scripts' next 30 Hz step
  (`level_play::tick`). The opening key (Space, gamepad Square) is a press while the
  computer is free. Either key (also Esc, Backspace, gamepad Circle) while it is in use
  calls `request_host_use_pod(false)` on the shell world and hands the request to the
  other world (`ask_to_close`); "in use" ends with the host's answer on the next tick
  and the change-over runs back. PLACEHOLDER: no script path for closing was found;
  the pod computer's own menus, which the player leaves it through, are not modelled.
- The gamepad. FOUND (part 24): action 8 is bit 0x80 of the pad's button word; which
  button that is, is not established. No viewer code maps a physical button to a bit of
  the game's button word (`sackboy_play.rs` and `popit_play.rs` read keys and buttons
  as their own actions). So no mapping was made up: the buttons are the ones `pod_dolly`
  already had for the Puter (Square opens, Circle closes), now through the script.
- `PuterState::open` is written after each tick from the shell world's "in use",
  however that came about, so the state the rig holds is the session's.
- `--level-scripts-press-at N` (QA): the engage press on tick N (a tick's number, as
  the tool's `--press=N:0x80`; `--level-scripts-in-use-at N` counts the ticks that have
  run, so both give "in use" on N + 1). Without the shell layer it is not used and
  says so. `--pod-open` and `--level-scripts-in-use-at N` still set "in use" directly
  on both worlds (QA options).
- Printed: `Pod Sackboy before tick 1: Thing 19 of the shell level, on the pod
  computer's trigger at [15.8, -532.2, -193.7]`; per press `Engage press on tick N:
  pod.bin's script took the click and asked the host ...` or `... ignored, pod.bin's
  script made no request: <why>` (the first condition of the engage test that does not
  hold, as far as the host can tell: no Sackboy, in use, the flags, the pad zone,
  another); `Pod computer at tick N: in use true, by the host's answer to a request;
  sounds counted ...`. The "Level scripts" window has one more line for the shell
  (Sackboy, pad zone, prompts, clicks, requests).
- Before the planets level's intro has set the progression flags the press does
  nothing, as in the game (the script's own rule, mask 2; MEASURED on tick 452 of a
  fresh profile). `--level-scripts-flags 3` starts with them.

Side effects, all from the Sackboy being there:

- The pod's shape colour is the player's colour, here `PLACEHOLDER_PLAYER_COLOUR`
  (opaque white; part 24), where it was zero without an avatar. So the flat colour of
  the dissolve's band (part 29) is white, not black. Closed, the colour is the white
  the level file stores, so the shell's `(new scene)` list is empty and its line is
  no longer printed.
- `pod/pod_comps_startup` and `pod/pod_comps_shutdown` are counted in the shell world
  when "in use" changes by the host's answer (nothing is played).
- `BP_ENGAGE_POD` prompts: the script asks for the prompt on every tick the engage
  test gets as far as the click (once the flags are there and the computer is free).
  They are counted (`PodState::action_prompts`); nothing draws them. The viewer's
  message overlay shows the planets level's message list only, and these are not
  messages: no prompt is seen in the Pod picture.

Without `--level-scripts`, and with it but without the shell layer, nothing changes:
`pod_dolly` sets `open` as before and `tick_level` is the code of part 29.

### Measured (PC, release build)

`level_scripts`, pod.bin with `--with-level` pod_planets.bin, `--play=700
--enter-pad-zone=1 --press=600:0x80`, a fresh profile, the d133 snapshot with the
overlay (`local-analysis/d279-pod-session-700-zone1-press600.json`): in the pad zone
on round 1; flags 3 on 452; `IsJustClickedByID` asked on 453 to 600 (148 calls, 148
prompts, 1 true); `RequestHostUsePod(true)` on 600; in use and the start-up sound on
601; the change-over 0.05 on 601, 1.0 and mesh 145753 on 620; the planets camera at
rest at (0, -300, 900) on 626. Both levels: no fault, no unbound call. With
`--press=100:0x80` (`...-press100.json`): no click test before 453, 248 of them to
700, none true, no request, nothing opens.

Viewer, 1280 x 720, no focus taken (`--qa-capture`, window hidden), `--level-guid
00018fdb --pod-camera 1 --pod-shell 000027b9 --level-scripts --window-16-9
--preview-lighting --qa-hide-ui`. The frames stay in ignored
`local-analysis/rscript/stage6/shots/` (`d279-*`); the press stills were taken twice,
with equal bytes.

- Not in use, held at 600: equal byte for byte to part 29's closed capture. The
  printed lines differ by the Sackboy's line and the missing `Shell Things ... (new
  scene)` line (above), and by the load-time ticks that differ from run to run.
- `--level-scripts-press-at 600`. Printed: `Engage press on tick 600, from
  --level-scripts-press-at`, `Engage press on tick 600: pod.bin's script took the
  click and asked the host for the pod computer`, `Pod computer at tick 601: in use
  true, by the host's answer to a request; sounds counted in the shell level:
  start-up 1, shut-down 0`, `Pod change-over at tick 601: opening, value 0.05`, `Pod
  tent dissolve at tick 601: value 0.05, 3 draw entities; cut above level y 820.4
  ...; flat colour [1.000, 1.000, 1.000]`, a `Shell Things` line per tick to 620,
  `Pod change-over at tick 620: open`, the swap on 620, the script camera at rest at
  (0, -300, 900) on 628 (looked at once a frame).
- Held at 610 (value 0.5): the same bytes as `--level-scripts-in-use-at 600` held at
  610 in this build: the two ways give "in use" on the same tick. Against part 29's
  still of that tick 60,321 of 921,600 pixels differ, all but 4 of them brighter: the
  band under the edge. Looked at: the picture of part 29, with the circuit traces of
  the band white where they were black.
- Held at 611 (value 0.55): the tent in place, cut a little lower than at 610 (the
  outer card's edge at about a third of the frame's height, the wall-art panels'
  edge ragged), white circuit traces on the outer card at both sides and thinner
  ones along the panels' upper edge; the room's ceiling and wall through the cut
  part, the Earth half behind the back wall, the DualShock on the tent's floor.
- Held at 641 (and `--level-scripts-in-use-at 600` at 640, and `--pod-open` at 600):
  equal byte for byte to part 29's open capture: the platform disc under the
  DualShock, the Earth and the Moon above it, the room behind. At value 1 the tent is
  not drawn, so the player colour changes nothing there.
- `--level-scripts-press-at 100` held at 600. Printed: `Engage press on tick 100:
  ignored, pod.bin's script made no request: the progression flags are 0x0, without
  mask 0x2 ...`. Nothing opens; the frame equals the closed capture byte for byte.
- Without `--level-scripts`: equal byte for byte to part 29's. One level with scripts
  and no shell layer (the planets level held at 600; pod.bin alone, flags 3, in use
  from 600, held at 640): equal byte for byte to main's build on the same command.

Not seen: the keys themselves (no key was sent to a window: the press option runs the
same code after the key), the way back by the closing key (covered by the test), the
motion. Not compared: any frame against the game. The white band is the placeholder
colour, not a player's.

### Left out

- The viewer's own Sackboy in the Pod: a physics scene built from pod.bin (the play
  mode builds one from the level given, here the planets level), the Pod's walkable
  floor (MEASURED at y -478 in the game; the viewer's floor finder gives -160), the
  play figure under the shell root, a camera. Then he walks into the trigger and a
  pad button reaches `set_pad_buttons`. A Sackboy PR.
- The pod computer's own scripts and menus (`OnEnterPod__`, `PerFrameUpdate__b`), and
  with them the game's way out of the pod computer.
- The player's stored colour (the placeholder is white), the engage prompt on screen,
  the two sounds played.
- Which pad button bit 0x80 is; the other input devices of action 8.

### Verification

`lbp-runtime`: `d279_the_pod_trigger_spot_puts_a_sackboy_in_the_pad_zone` (no spot
for another class, without a position, in an empty world; the spot follows the
trigger's matrix and z offset; a Sackboy made there is in the pad zone after one
tick). `lbp-bevy-viewer`: `d279_a_press_is_one_tick_long` (the word on both pads, the
game's click test true on that tick alone, over session ticks),
`d279_the_key_while_in_use_asks_the_host_to_close` (the key table; the request waits
in both worlds, "in use" ends on both with the next tick, the shut-down sound where
the Sackboy is; dropped outside our Pod), `d279_the_press_option_is_a_tick_number_and_fires_once`
and `d279_a_press_leads_to_in_use_on_both_worlds_a_tick_later` (two worlds without
scripts through `tick_session`, the one step of pod.bin's script stood in for: a
press before the flags is ignored and the reason names them; with them the request on
the tick of the press and "in use" on both worlds on the next, the start-up sound
counted once). None needs game data. fmt, the CI clippy line, the CI test set
(612 pass) and the Python tests (36) pass; `cargo check` of both viewers and
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass; the viewer's
tests pass (71) but the one that fails on main
(`sackboy_play::tests::input_script_steps`). `tick_level` and `tick_session` are
unchanged (the runtime gained one read-only function), so the 378-level runs were not
repeated. Seen in the viewer window: the stills above. Nothing compared against the
game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
