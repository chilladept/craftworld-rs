# Level scripts, part 27: the pod's mesh swap in the scene (2026-10-13, D276)

Third PR of the "Pod picture" follow-up to stage 6: viewer and scene builder, no runtime
change, nothing new read in the executable. The runtime has reported the mesh a script
puts on a Thing's PRenderMesh since part 21 (`ThingDisplay::mesh`,
`DisplayChange::mesh`); the viewer built its scene once from the level file and counted
such a Thing as "another mesh, not drawn" (part 23). Now the scene of a played level
holds both meshes of the one pair the scripts swap, and shows the one the Thing has.

### What the game does

- FOUND (parts 21 and 24): `Thing.SetRenderMeshMesh` has one call site in the 426
  scripts, `Pod.SetPodTransition__b`, on the pod Thing of `gamedata/levels/pod/pod.bin`
  (Thing 2): mesh 0x2f6c (`pod.mol`) while the change-over value is under 1, 0x23959
  (`pod_platform.mol`) at 1. `Thing.GetPodThing__b` tests for the same two GUIDs. The
  same function sets the pod's shape colour with w = 1 - value and the fade of the pod
  decorations.
- MEASURED before (scoping, `bind.exe`): both meshes have one bone with the same bind
  pose, so the Thing's stored matrix places either.
- Not established: what the game draws between 0 and 1 (the fade), and what happens to
  the Things that hang on the pod (below).

### What the viewer does

- `lbp-preview` (`level_scene.rs`): `LevelSceneOptions::mesh_alternates`, a list of
  mesh GUID pairs, empty by default. `add_alternate_meshes` runs after the RENDER_MESH
  items are built: a POS Thing drawn with one mesh of a pair gets a second item for the
  other mesh, at its POS under that mesh's own bone-0 inverse bind, marked
  `SceneItem::alternate`. Such an item is not counted as drawn and is left out of the
  scene bounds (the framing does not move); one `Alternate meshes` line says how many
  there are. With an empty list nothing is loaded and no line is printed.
- `lbp-bevy-viewer`: `thing_view::SWAPPED_MESHES` is the pair table, one entry (0x2f6c,
  0x23959). The list is handed to the scene of the level a script world plays
  (`--level-scripts`), never to the shell layer and never without scripts. An alternate
  item is spawned hidden; `thing_view::Item` has the mesh GUID its entities draw and
  the `alternate` mark.
- `thing_view::apply`, on a Thing with a second set: the entities of the mesh the
  PRenderMesh has now are shown and the others hidden. The play-mode visibility rule of
  part 19 goes on top: a mesh the visibility byte hides stays hidden through a swap,
  and when it is shown again the set of the mesh now is the one shown, whichever change
  came first. Counted as `mesh swapped` (not as hidden or shown); a mesh that was not
  loaded with the scene is still counted as `another mesh` and leaves the file's set in
  the scene. A scene spawned after the swap gets it through the first list. The `Things
  at tick N` line names the Things with another mesh than the file's.
- QA option `--level-scripts-in-use-at N`: `LevelWorld::pod_computer_in_use` is set
  when N ticks have run, before the next (the twin of `level_scripts --puter-in-use=N`).
  It is a stand-in for the path of part 24 (our Pod, pad zone, press, the host's
  answer), which needs a Sackboy and a pad the viewer does not feed in. MEASURED
  (`level_scripts`, pod.bin alone, 200 ticks, in use from 100): "in use" alone runs the
  change-over, with or without progression flags 3 (one mesh change, GUID 145753 at the
  end, fade 1.0, no fault, no unbound call).
- Under `--level-scripts --pod-camera N` the played level is the planets level, which
  has neither mesh, and pod.bin as the shell layer has no script world: nothing changes
  there. On every other level no Thing has a mesh of the pair.

### Measured (PC, release build)

Viewer, 1280 x 720, no focus taken (`--qa-capture`), `--window-16-9 --preview-lighting
--qa-hide-ui`. The frames stay in ignored `local-analysis/rscript/stage6/shots/`
(`d276-*`).

- Without `--level-scripts`, before (main at D275, the same build tree) and after:
  pod.bin alone equal byte for byte (the printed lines too); the shell picture
  (`pod_planets.bin --pod-camera 1 --pod-shell 000027b9`) equal byte for byte, and equal
  to part 26's capture.
- pod.bin alone with `--level-scripts --level-scripts-flags 3
  --level-scripts-in-use-at 600`: `Alternate meshes ...: 1 Things with a second item`,
  `Thing map ...: 2 Things ..., 11 draw entities`, `Pod computer at tick 600: in use
  true`, and `Things at tick 620: ... mesh swapped 1 (5 entities) ... with another mesh
  than the file's [(2, "00023959")]` (the change-over takes 20 ticks).
- Hold at 610 (change-over at 0.5): equal byte for byte to the capture without scripts,
  and to one with scripts and no "in use" held at 640. Hold at 640: the swap is drawn.
  With in use from 30 and hold at 100 the swap is over before the scene is up: it
  arrives in the `(new scene)` list and the frame equals the hold-at-640 one byte for
  byte.
- Looked at, before: the cardboard pod with the DualShock in its opening, a black
  rectangle above it with two small orange pieces under it, and two more small orange
  pieces at the pod's upper sides. After: the pod is gone and a flat round cardboard
  disc lies where its floor was, the DualShock on it. The black rectangle and its two
  pieces are unchanged. The two orange pieces at the pod's sides stay where they were,
  now in mid-air.

Not seen: the swap in motion, the way back to the tent (covered by the test). Not
compared: any frame against the game.

### Not modelled

- The fade. The pod pops from one mesh to the other on the tick the value reaches 1;
  the shape colour's alpha and the pod decorations' fade are still "not followed".
- The two pieces left in mid-air. They are not entities of the pod Thing (it has the
  tent's and the platform's only; pod.bin stores no decoration and no sticker). Which
  Things they are was not looked up: the level has two models placed between JOINT end
  points, and they may be the ends of those. What the game does with them in the open
  Pod is not established.
- Any other mesh a script sets (none does): it is counted, not loaded.
- The editor colour of the Thing on the alternate mesh (it is loaded with the base
  inputs; the pod's is white), and `--runtime-texture` rules name one mesh as before.
- The Pod picture itself: with the shell layer pod.bin has no script world, so the tent
  still leaves the frame on the viewer's own dolly (part 26). That is PR 4.

### Verification

`lbp-preview`: `d276_a_thing_with_a_mesh_of_a_pair_gets_an_item_for_the_other` (the
pair lookup; the second item's Thing, matrix and mesh; counts, bounds and the item list
before it unchanged; a mesh that does not load; a mesh already held is not loaded
again). `lbp-bevy-viewer`: `d276_a_swapped_mesh_shows_its_own_entities` (two entity
sets and a decoration on one Thing: swap, the same mesh again, hidden by the visibility
byte before and after a swap and on the same tick, back again, a mesh that was not
loaded, also after the other of the pair) and
`the_in_use_option_is_a_tick_count_and_fires_once`. None needs game data. fmt, the CI
clippy line, the CI test set (611 pass) and the Python tests (36) pass; `cargo check` of
both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass; the
viewer's tests pass (58) but the one that fails on main
(`sackboy_play::tests::input_script_steps`). Seen in the viewer window: the stills
above. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
