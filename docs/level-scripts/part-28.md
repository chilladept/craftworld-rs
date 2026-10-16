# Level scripts, part 28: both Pod levels in one viewer session (2026-10-13, D277)

Fourth PR of the "Pod picture" follow-up to stage 6, first half (PR 4a): the viewer, one
accessor in the runtime and one option of the `level_scripts` example. No new native.
With the shell layer (`--pod-camera N --pod-shell HEX`) under `--level-scripts` the
viewer played the planets level only: pod.bin was drawn from its file, so the pod's mesh
swap of part 27 never showed in the Pod picture and the tent left the frame on the
viewer's own one-second dolly. Now pod.bin has a script world of its own, the two levels
tick in one session (part 24), and the shell layer follows its world.

### What the game does

Read for this PR's scoping in the executable (01.33), the script listing of class `Pod`
and the game data; the notes stay in ignored `local-analysis/rscript/stage6/`.

- FOUND (class `Pod`, the world script of pod.bin, on Thing 0): `PodTransition` is a
  private float field. `SetPodTransition__b`, called with false at the end of every
  `Update__`, steps it by 0.05 toward 1 while `GetPodComputerInUse__` is true and toward
  0 while it is not. On every path it then sets the pod Thing's shape colour to the
  party leader's colour (zero without an avatar) with alpha 1 - value, the PShape's
  `EditorColourBrightness` to 1, hands the value to
  `PodComputer.SetFadePodDecorations__f`, sets the pod's mesh (0x2f6c under 1, 0x23959
  at 1) and sends a mask texture GUID to `PodComputer.SetPodTransitionTexture__g`. The
  form with true (set at once) has no caller in the scripts.
- FOUND, `SetFadePodDecorations__f` (0x5fb36c -> 0x5033cc -> 0x403ca4): it takes the
  pod Thing of the current level (`GetPodThing(world, true)`, 0xd32dc) and walks the
  records of its decorations part (0x70 bytes each). For each it takes the world height
  y of the record's place under the pod Thing's world matrix, h = (y + 500) / 1000
  kept in 0..1, and f = (h - 1) * 4 + value * 5 kept in 0..1; the byte f * 255 is
  stored on the mesh instance of the record's render object. FOUND what the byte does:
  an instance with byte 0xff is not submitted (0xed358), and one with another byte than
  0 goes to the translucent pass (0x348fe0, 0x349b64). So the decorations are wiped
  from the top down by their height, each over 0.2 of the value, and at 1 none is
  drawn. INFERRED: the byte is a fade amount; the code that turns it into transparency
  was not found. MEASURED: no Thing of pod.bin has a decoration or a sticker. INFERRED:
  the records are the player's own Pod decorations, which 0x676c18 copies onto the pod
  Thing; with a fresh profile the native has nothing to work on.
- FOUND, `SetPodTransitionTexture__g` (0x617428 -> 0x4015e8, part 24): once the
  texture is loaded it replaces one counted pointer (+0x58) of three resources, GUIDs
  12138, 12250 and 119116. MEASURED (snapshot, the material reader): these are the
  graphics materials `pod_card`, `pod_card_edge` and `window_glassy`, the three
  materials of `pod.mol`; each stores `transition_mask.tex` in texture slot 5, with no
  wire in the material graph. INFERRED: +0x58 is slot 5, so the native swaps the mask
  of the pod's own materials. The script sends GUID 132243 unless the planets backdrop
  is one of six plans, each with a mask of its own; it is the same GUID every tick: a
  pattern per backdrop, not a value per tick.
- FOUND, how the tent goes: the mesh instance's colour is the PShape's colour when the
  Thing has a PShape (0xed02c, 0xfc4f4), with the alpha unchanged, so the pod's
  instance alpha is 1 - value. All fragment programs of the three pod materials fetch
  texture unit 5 and hold one discard; those of the platform's own card material do
  neither. INFERRED from one decoded program: a dissolve from the top down in the pixel
  shader. Pixels above a line that falls as the alpha falls are discarded and the mask,
  tiled over the UV, breaks up the edge; it is not an even alpha blend, and at alpha 1
  it is off. Not established: the frame of the line (the constants as read do not fit
  the pod's height), the flow control of the programs, the platform's edge at value 1.
- MEASURED (the level reader, 19 Things): the two small orange pieces of part 27 are
  the pod ends of Things 13 and 14, two string joints (mesh 0x4c7f `joint_string.mol`)
  from the bar above the pod (Thing 12) to the pod (Thing 2); the two pieces under the
  black rectangle are their other ends. Their materials are none of the three with the
  mask, and neither is stored as hidden in play mode. FOUND: nothing read hides them in
  the change-over. They are not decorations, the script touches Thing 2 only, the
  engine looks the pod Thing up in three places (the native of part 21, the
  decorations' fade, the copy of stickers and decorations), and the colour of a mesh
  without a PShape (0xece28) does not look at the Things a joint connects. INFERRED:
  the game keeps them drawn in the open Pod.
- Not established: which camera draws the shell level and how it moves when the pod
  computer opens (parts 23 and 26). Nothing read ties it to `PodTransition`.

### What was added

`lbp-runtime`:

- `LevelWorld::pod_transition()` (`pod.rs`): the change-over value, the last value
  `SetFadePodDecorations__f` was given (`PodState::fade`), none before the first
  update. A host reads it without knowing the class.
- `level_scripts`: in a `--with-level` session `--puter-in-use=N` (N > 0) sets "in
  use" on both worlds when N rounds have run; `runs.session.puter_in_use_from`. The
  viewer's stand-in is measurable without a window.

`lbp-bevy-viewer` (`level_play.rs`, `thing_view.rs`, `main.rs`):

- `Play::shell` (`ShellLevel`): with `--level-scripts`, `--pod-camera N` and
  `--pod-shell HEX` the shell level is read and gets its own VM (the 426 scripts are
  loaded twice), world, engine objects and `LevelScripts`. The session is our Pod when
  the shell has a pod Thing (PLACEHOLDER, the `level_scripts` stand-in: the game tests
  the slot). The worlds are made to agree before the first tick; create runs on the
  shell, then on the planets level, each handing its session state on.
- Each tick is one `lbp_runtime::tick_session` round, the shell first, then the
  planets level. INFERRED: the order in the game's frame was not read; MEASURED (part
  24) that the other order gives the same events on the same ticks. Without the shell
  layer the `tick_level` path is the code of part 27.
- "Pod computer in use" from the Puter key, `--pod-open` and
  `--level-scripts-in-use-at N` goes through `set_in_use`, which writes both worlds.
  The trap: the first level of a session tick hands its own value to the others
  afterwards, so a value set on the planets world alone is written over before that
  world ticks.
- `thing_view::ShellThings`, a second Thing map as a resource type of its own.
  `thing_view::apply` reaches entities through the map it is given, so Thing 2 of
  pod.bin and Thing 2 of the planets level do not collide. `spawn_level` returns its
  map and the caller keeps it as the level's or the shell's; the shell's map goes when
  a new load drops the shell roots. `follow_things` runs once per level (`follow_level`)
  and prints the shell's line as `Shell Things at tick N`.
- The shell's scene loads the swapped pair (`LevelMode::shell_scripted`), the
  platform hidden. Its entities hang under the shell root (their mesh is not the
  tent's), the tent's under the tent root as before.
- The opening progress: after each session tick `opening(pod_transition())` (the
  value kept in 0..1; 0 for none and for a value that is not a number) is written to
  `PuterState.t`, also while the ticks are held. `pod_dolly` no longer moves `t` in
  this mode; the keys only set `open`. Everything that reads `t` follows: the shell and
  tent roots in `script_camera`, the Pod fog, the figure's turn, the overlay. One line
  is printed when the value leaves an end and when it reaches one (`Pod change-over at
  tick N`). Under scripts the layer starts closed: `--pod-open` is "in use" from the
  first tick the rig is up and opens over 20 ticks; a `--pod-t` between 0 and 1 is not
  used and says so (mid stills: `--level-scripts-in-use-at N --level-scripts-hold-at
  N+k`, value 0.05 k).
- PLACEHOLDER: `t` from `PodTransition`. The FOUND uses of the value are the pod's
  alpha, the decorations' fade, the mesh at 1 and the mask. That the shell's eye moves
  with it, and the tent's slide out of the frame as a whole, are the viewer's (the
  fitted shell eyes of P2f, the made-up exit of P2g), now on the script's 20 ticks
  (0.667 s) in place of the one-second dolly. The planets camera needs about 25 ticks
  with an overshoot, so the two layers do not arrive together.
- The two string ends are left drawn, as INFERRED for the game.
- Without `--level-scripts`, and with it on one level, nothing changes.

### Measured (PC, release build)

`level_scripts`, pod.bin with `--with-level` pod_planets.bin, `--play=700
--puter-in-use=600`, the d133 snapshot with the overlay
(`local-analysis/d277-pod-session-700-inuse600.json`): flags 3 on round 452; in use
on 601; the change-over 0.05 on 601, 0.10 on 602, 0.95 on 619, 1.0 on 620, where the
pod's mesh becomes GUID 145753; the planets camera leaves (0, -250, 1130) on 601 and
rests at (0, -300, 900) on 626. Both levels: 700 ticks, no Thing stopped, no fault, no
unbound call; our Pod true in both.

Viewer, 1280 x 720, no focus taken (`--qa-capture`, window hidden), `--window-16-9
--preview-lighting --qa-hide-ui`. The frames stay in ignored
`local-analysis/rscript/stage6/shots/` (`d277-*`).

- Equal byte for byte to main's captures of the same commands: pod.bin alone without
  scripts and the shell picture without scripts (part 27's); pod.bin alone with
  scripts, flags 3, in use from 600, held at 610 and at 640 (part 27's); the planets
  level alone with scripts held at 600 (part 25's).
- The Pod picture with scripts, not in use, held at 600: equal byte for byte to part
  26's capture. The session leaves the closed picture as it was.
- The Pod picture with scripts, `--level-scripts-in-use-at 600 --level-scripts-hold-at
  640`. Printed: `Level scripts, shell level: gamedata/levels/pod/pod.bin; scripted
  Things 2; ... our Pod true`, `Alternate meshes ...: 1 Things with a second item`,
  `Thing map ...: 2 Things ..., 11 draw entities`, `Shell Things at tick 166 (new
  scene)`, `Pod computer at tick 600: in use true`, `Pod change-over at tick 601:
  opening, value 0.05`, `Pod change-over at tick 620: open, value 1.00`, `Shell Things
  at tick 620: ... mesh swapped 1 (5 entities) ... with another mesh than the file's
  [(2, "00023959")]`, and the script camera at eye (0, -300, 900).
- Held at 600 with in use from 600: the closed picture (the same bytes).
- Looked at, held at 610 (value 0.5): the tent is half way out. Its dark inner walls
  with the corrugated card edge stand at the left and right borders of the frame, a
  strip of its floor at the bottom edge; the DualShock is at the middle, in front of
  the Earth, the Moon behind its right grip; the room's walls, table and figure behind.
- Looked at, held at 640: the tent is gone. A flat round disc fills the bottom of the
  frame, its far rim an arc from the lower left to the lower right, and the DualShock
  stands at the lower middle with its grips in two cut-outs of the disc: the platform
  is under the shell root where the tent's floor was. Two short dark marks lie on the
  disc near (0.30, 0.87) and (0.71, 0.87) of the frame (not identified). The Earth is
  just above the DualShock, the Moon to its right. The disc is shaded olive green by
  the preview light (the shade issue of part 26). The string ends are not in either
  frame; INFERRED: their places at the tent's upper sides are above the open view.
  They are drawn in the capture of pod.bin alone (part 27's picture, the same bytes).
- With `--pod-open` held at 600: in use from tick 136, the change-over open on tick
  156, the swap in the `(new scene)` list; the frame equals the hold-at-640 one byte
  for byte.

Not seen: the motion (stills only), the way back. Not compared: any frame against the
game.

### Left out

- The tent's dissolve. FOUND to be a dissolve from the top down by a mask texture,
  driven by the alpha of the pod's shape colour; the viewer has no blended or discard
  material path for it yet, so the tent slides as a whole and pops at 1. The next PR.
- The viewer's Sackboy and pad as the input (the path of part 24). The Puter key
  stays a stand-in; the Pod picture has no Sackboy in pod.bin's space.
- The player's Pod decorations and their fade (none is loaded; pod.bin stores none).
- The shell level's camera in the game; the shell and planets layers arriving
  together; a mix of `t` between two ticks (20 steps are seen).
- The snap (`SetPodTransition(true)`) for a computer that is open on arrival.
- One VM for both levels (script statics and the PodComputer object are per level).
- The preview light of the shell turned with its root.

### Verification

`lbp-runtime`: `d270_the_pod_thing_follows_the_mesh_and_the_avatar_is_the_sackboy` now
also checks `pod_transition` (none, then each value given). `lbp-bevy-viewer`:
`d277_two_maps_keep_two_levels_apart` (`apply` with two maps over one query, Thing 1
in both), `d277_the_opening_is_the_change_over_value_clamped`,
`d277_in_use_is_set_on_both_worlds_of_the_session` (two worlds through `tick_session`
in the viewer's order; set on the second alone, the value is lost),
`d277_the_shell_layer_loads_the_swapped_meshes_only_with_a_script_world`. None needs
game data. fmt, the CI clippy line, the CI test set (611 pass) and the Python tests
(36) pass; `cargo check` of both viewers and `cargo clippy -p lbp-bevy-viewer
--all-targets -- -D warnings` pass; the viewer's tests pass (62) but the one that fails
on main (`sackboy_play::tests::input_script_steps`). `tick_level` is unchanged, so the
378-level runs were not repeated (Things stopped by a fault 3 / 3 / 3 as in part 25).
Seen in the viewer window: the stills above. Nothing compared against the game in
RPCS3.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
