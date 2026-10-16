# Level scripts, part 29: the Pod tent's top-down dissolve (2026-10-13, D278)

Fifth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change, no
new native. Until now the tent left the Pod picture by sliding out of the frame as a
whole on the viewer's own exit path and popped at the mesh swap (part 28). Now it stays
where the closed picture has it and is cut away from the top down, by the rule the
game's own pod materials hold.

### What the game does

Read for this PR's scoping in the fragment and vertex programs of the three materials of
`pod.mol` (`pod_card`, `pod_card_edge`, `window_glassy`; 30 fragment programs, one
vertex program) and in an offline replay of two frames captured earlier (Pod closed,
Puter open). The notes and every listing stay in ignored `local-analysis/rscript/stage6/`.
With v the change-over value of part 28 (`PodTransition`):

- FOUND: the materials take the height of the pixel over the camera: the world position
  minus the camera position, not turned and not scaled; only its y is used. MEASURED in
  the two frames: the camera position handed to the pod's draws is (0, -250, 1130) with
  the Pod closed and (0, -299.5, 902.4) with the Puter open. Those are the eyes of the
  planets level's script camera at rest in the two states (part 28: (0, -250, 1130) and
  (0, -300, 900)). This answers part of an open point of parts 26 and 28: at the two end
  states the shell is drawn from the script camera's eye. Between them it was not
  captured.
- MEASURED in the two frames: the colour handed to the pod's draws is the pod Thing's
  shape colour, a player colour with alpha 1 closed and 0 open: alpha = 1 - v, as the
  script sets it (part 28). INFERRED: the engine code that hands it over was not read.
- FOUND, the cut: while the alpha is under 1, a pixel is discarded when its height
  over the camera is above 1150 - 1550 v, plus 100 times the green of the material's
  own diffuse colour (a ragged edge; the glass material has a constant 11.8 in its
  place), plus 400 where a second mask of the two card materials (`pod_mask.tex`, GUID
  0x1fbc7, sampled at the plain UV) is white. That mask is white on 15.6% of its area
  (MEASURED), a few islands, which so stand 400 units longer. With alpha 1 the rule is
  off.
- FOUND, the band under the line: over the 400 units below it the pixel goes from the
  lit colour toward the flat, unlit shape colour, by an edge value (1 on the line, 0 at
  400 below) and the red of the transition mask of part 28 (GUID 132243 or one of six
  per backdrop), sampled at 40 times the UV: kept share = 1 - edge + mask, between 0
  and 1, and 0 where the edge value is above 0.99023. So the mask moves no edge. It
  draws its pattern, in the player's colour, in a band under the edge, strongest at
  the edge. The default mask is a circuit-board drawing, mostly white (82% of texels
  over 0.5, MEASURED).
- The line over v, camera at y -250: world y 822 at 0.05 (the tent's top is at 1278, so
  the top 16% of its height goes on the first tick), 125 at 0.5, -572 at 0.95, just
  under the platform's edge (world -560 to -539, MEASURED). The tent reaches down to
  -1516: what is below the floor is still there at 0.95 and goes with the mesh swap at 1.
- INFERRED, not drawn here: at v = 1 the same rule runs on the platform's
  `pod_card_edge` with alpha 0 and a line 800 under the camera. By the formula the
  platform's edge is then drawn only where the second mask is white, tinted by the
  transition mask. The frame's pixels were not inspected.
- Not established: the camera's height between the two end states; the engine code that
  hands over the colour and the camera position; whether the flat colour is meant as a
  display value (the lit colour passes a square root just before the mix).

### What the viewer does

`lbp-bevy-viewer` (`pod_dissolve.rs`, `dual.rs`, `thing_view.rs`, `level_play.rs`,
`main.rs`). Only under `--level-scripts --pod-camera N --pod-shell HEX`
(`LevelMode::shell_scripted`).

- `pod_dissolve.rs` holds the numbers in one place (`LINE_AT_ZERO` 1150, `LINE_FALL`
  1550, `GREEN_RISE` 100, `BAND` 400, `FLAT_ABOVE` 0.99023, `MASK_TILES` 40, the mask's
  GUID) and the arithmetic: `value` (the alpha as a change-over value, only strictly
  between 0 and 1), `line_y` (camera y + 1150 - 1550 v) and `world_plane`.
- The viewer's material (`dual.rs`) has a dissolve block: a plane, a flat colour, the
  constants, a mask texture and two more planes for mask coordinates. A plane of zero is
  off and the shader then does what it did. With a plane: the height above the line
  minus 100 times the green of the surface colour, over 400, plus 1, is the edge value;
  above 1 the fragment is discarded, in the opaque pass (as the alpha test's discard:
  the alpha mode is not changed, nothing is blended); under it the finished colour is
  mixed with the flat colour by the kept share above. The mask is sampled with explicit
  gradients.
- The line is in the shell level's own y: `camera y + 1150 - 1550 v`, with the camera
  the planets level's script camera as the viewer shows it (its eye's y in level units,
  each frame). INFERRED: that the shell level's frame and the script camera's frame are
  one frame in the game. The two end states fit (the tent's heights over the camera in
  the closed frame are its level heights plus 250), and the camera between them is the
  script's own motion (about -250 to -300 over 25 ticks), which was not captured in
  the game.
- The plane goes to the shader in Bevy world space, with the height in level units as
  its value, so the 400-unit band and the ragged edge keep their size under the tent
  root's scale. The tent's entities hang under a root that is placed each frame:
  `level_play::follow_dissolve` runs each frame after `script_camera` and before a
  capture, takes the root's own `Transform` times the shell's display matrix as the
  level-to-world matrix, and writes a material only when its values change. A held
  tick (`--level-scripts-hold-at`) draws from the value kept on the entity.
- `thing_view::apply`: in a map with a `dissolve_mesh` (set on the shell layer's map
  only: 0x2f6c), the entities of that mesh of the level's pod Thing
  (`LevelWorld::pod_thing(true)`, no Thing number in the viewer) get the value and the
  shape colour's red, green and blue on their tag, and a material of their own on the
  first value (the clone of part 19's tint). The pod's decorations and the platform's
  entities get nothing. At alpha 1 nothing is cloned and no uniform is set: the closed
  picture goes the old path. At alpha 0 the script's mesh swap hides the tent (part 27)
  and the dissolve is taken off; on the way back it is set again with the first value
  under 1. A scene spawned in mid change-over gets the value through the first list.
  Counted as `dissolve set` in the `Shell Things` line (one line per tick of the
  change-over). The pod's shape colour is no longer counted as "editor or shape colour,
  not drawn" in that map: the dissolve is all the game draws of it. A tint is now
  counted as taken off only where one was on.
- The tent root is placed at opening progress 0 always (`shell_layer_root`): the tent
  stays where the closed picture has it. The slide out of the frame was the viewer's
  placeholder (P2g) and is gone under scripts. The shell root (DualShock, strings, the
  platform) still follows the change-over value as in part 28 (PLACEHOLDER), so between
  the ends the DualShock moves against the tent's floor. Without `--level-scripts`
  `pod_dolly` and its tent path are untouched.
- The mask: GUID 132243 is decoded with the level (the load worker, the recipe of the
  runtime textures) when the shell has a script world, and kept as a linear, repeating
  image with mips in a resource that goes when the shell goes. The script names its
  mask only once it ticks, and it is the same GUID for every backdrop but six; one line
  says, when the dissolve is first drawn, whether the script's GUID is the preloaded
  one. The six other masks are not loaded. Without game data for the mask the band is
  not tinted and the cut stays.
- Mask coordinates. The game samples by the mesh's UV. The viewer's `in.uv` is not that:
  it holds the coordinates of the bucket's diffuse sampler after the material's own
  scale and offset, and zeros on a bucket drawn in a solid colour. So: a bucket with a
  diffuse texture samples the mask at `in.uv` times 40 (right where the material samples
  its diffuse at the plain UV, off by the material's factor elsewhere: the glass panels
  with the wall art are sampled at twice the UV, so their pattern is half the size); a
  bucket without one takes coordinates from the position, a tile of 70 level units over
  (x + z, y). PLACEHOLDER: that tile size and that fallback.
- Flat colour: the shape colour's red, green and blue as the script set them, shown as
  they are (INFERRED). Without an avatar the script makes them zero: the band's pattern
  is black here, where the game with a player shows his colour.

What is not the game's rule here, in one list:

- the green of the viewer's surface colour before lighting stands in for the green of
  the material's diffuse product (for the glass too, where the game has a constant);
- the second mask (`pod_mask.tex`) is left out: no islands that stand 400 units longer,
  and nothing of the tent stays above the line;
- the flat colour is not fogged; under `--preview-lighting` the lit colour has no
  square root, so the mix is in the viewer's own colour space;
- the platform at v = 1 is drawn solid, as before;
- the per-backdrop masks, the pod's decorations (pod.bin stores none), the input.

### Measured (PC, release build)

Viewer, 1280 x 720, no focus taken (`--qa-capture`, window hidden), the planets level
(`--level-guid 00018fdb`) with `--pod-camera 1 --pod-shell 000027b9 --window-16-9
--preview-lighting --qa-hide-ui`, the d133 snapshot with the overlay. The frames stay in
ignored `local-analysis/rscript/stage6/shots/` (`d278-*`); each was taken twice, with
equal bytes.

- Without `--level-scripts`: equal byte for byte to part 28's capture.
- With `--level-scripts`, not in use, held at 600: equal byte for byte to part 28's
  (and part 26's) closed capture. Printed: `Pod tent dissolve mask 00020493: loaded,
  512 x 512`; the `Shell Things ... (new scene)` line with `dissolve set 0`.
- With `--level-scripts-in-use-at 600`. Printed: `Pod change-over at tick 601: opening,
  value 0.05`, `Shell Things at tick 601: ... dissolve set 1 (5 entities) ...`, `Pod
  tent dissolve at tick 601: value 0.05, 3 draw entities; cut above level y 820.4
  (camera y -252.1 + 1150 - 1550 x value), flat colour [0.000, 0.000, 0.000]; mask
  132243 loaded, 512 x 512; the script's mask GUID: the same`, a `Shell Things` line
  per tick to 620, `Pod change-over at tick 620: open`, `Shell Things at tick 620: ...
  mesh swapped 1, dissolve set 1 ... [(2, "00023959")]`.
- Looked at, held at 602 (v 0.10, line near level y 740): to the eye the closed
  picture (the bytes differ). The cut is above the frame.
- Held at 605 (v 0.25): the tent is in place. Its roof is gone: the room's ceiling shows
  through an opening with an arched front edge, the roof's ring hangs in it cut off
  above, the inner walls end in clean slanted edges; a band of circuit pattern lies
  under the edge on the back wall and the outer card.
- Held at 610 (v 0.5): the tent is cut at about a third of the frame's height. Above it
  the room's ceiling and wall; the Earth has sunk behind the back wall (the planets
  camera is on its way). The edge is ragged on the wall-art panels (by their colours)
  and nearly straight on the card, with dark circuit traces in the band under it.
- Held at 615 (v 0.75): the walls stand a little over the floor, the panels' edges
  ragged, the outer card at the bottom corners full of circuit traces; the room is
  open all round. The DualShock is smaller and lower on the floor than at 610 (the
  shell root follows the change-over; the tent does not).
- Held at 619 (v 0.95): the floor and the walls are gone. Left are parts of the tent
  below the floor line, as dark slabs with circuit traces at the bottom of the frame,
  around the DualShock, which is at its open place before the Earth and the Moon. That
  is the rule's own remainder until the swap.
- Held at 640: the platform, the tent gone: equal byte for byte to part 28's open
  capture; so is `--pod-open` held at 600.
- In every still between, the planets and the room show through the cut part; nothing
  of the tent is blended.

Not seen: the motion, the way back (covered by the test), a player colour (no avatar:
black), the dissolve under the default lighting. Not compared: any frame against the
game. All of the picture is unverified against the game; only the rule was read.

### Left out

- The second mask of the card materials and its islands; the glass's constant.
- The platform's edge at v = 1 (INFERRED above; drawn solid).
- The six per-backdrop masks; the mesh's plain UV for the mask where the diffuse
  sampler has a factor.
- The pod's decorations and their fade (part 28; none is loaded).
- The shell root's opening and the shell level's camera between the end states: the rig
  and the shell's placement are as in part 28. The input (PR 4b).
- The preview light of the shell turned with its root (part 26).

### Verification

`lbp-bevy-viewer`, none needs game data: `d278_the_line_falls_with_the_value` and
`d278_the_world_plane_measures_in_level_units` (`pod_dissolve.rs`: the value's range,
the line at three values and two cameras; under a turned, scaled and moved root and a
display matrix the plane gives the height over the line in level units, the mask planes
their tiles; a matrix without an inverse gives nothing),
`d278_the_dissolve_setter_writes_and_clears` (`dual.rs`),
`authored_shader_validates_with_every_branch_enabled` extended (two discards, the mask
sample with gradients, the mix), `d278_the_tent_entities_dissolve_by_the_shape_colours_alpha`
(`thing_view.rs`: nothing cloned at alpha 1; the tent's two entities and not the
platform's or the decoration; a new value without a new material; taken off at the
swap and at alpha 1 on the way back; a map without a dissolve mesh counts the colour as
before) and `d278_the_tent_root_stays_closed_under_the_script_camera`
(`level_play.rs`). fmt, the CI clippy line, the CI test set (611 pass) and the Python
tests (36) pass; `cargo check` of both viewers and `cargo clippy -p lbp-bevy-viewer
--all-targets -- -D warnings` pass; the viewer's tests pass (67) but the one that fails
on main (`sackboy_play::tests::input_script_steps`). Seen in the viewer window: the
stills above. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
