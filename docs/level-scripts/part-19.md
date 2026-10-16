# Level scripts, part 19: hidden, tinted and moved Things (2026-10-13, D268)

Stage 5 of `docs/rscript-plan.md`, fourth PR. Scripts hide a render mesh, set its colour
or tint and move Things. The world kept some of that and nothing drew by it. Now the
world lists the Things whose display values changed, and the Bevy viewer sets the draw
entities of a played level from that list: hidden, tinted, placed.

### What the game does (FOUND, 01.33 executable and script listings)

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Thing.HideRenderMesh__Q5Thing` | static `void (Thing)` | 0x642cd4 -> 0x754aac -> 0xebf98 | Nothing for a null Thing or one without a PRenderMesh. Else the visibility call with "off" and the mask 3 |
| `Thing.ShowRenderMesh__Q5Thing` | static `void (Thing)` | 0x642c94 -> 0x753e3c -> 0xebf98 | The same call with "on" |
| `PostItNote.IsVisibleInPlayMode__` | `bool ()` | 0x644d7c -> 0x5e9260 | Bit 0 of the visibility byte of the Thing's PRenderMesh; false for a null Thing or no PRenderMesh |
| `Thing.SetRenderMeshEditorColour__Q5Thingr` | static `void (Thing, v4)` | 0x642924 -> 0x753ed4 -> 0xecac4 | Nothing for a null Thing or no PRenderMesh. Else the colour, packed to 32 bits, into two words of the PRenderMesh |
| `Thing.SetRenderMeshEditorColourTint__Q5Thingr` | static `void (Thing, v4)` | 0x642978 -> 0x7549c8 | As above, into a third word, after the two |
| `Thing.SetShapeEditorColour__Q5Thingr` | static `void (Thing, v4)` | 0x642a20 -> 0x75495c | Nothing for a null Thing or no PShape. Else the packed colour into a word of the PShape |
| `Thing.SetShapeEditorColourTint__Q5Thingr` | static `void (Thing, v4)` | 0x6429cc -> 0x753a9c | As above, into the word before it |

- The visibility call (0xebf98) takes a PRenderMesh, on or off, and a mask. It clears the
  mask's bits in the PRenderMesh's visibility byte and, for "on", sets them again. Then,
  unless the Thing has a PShape or its mesh is one of two globe meshes (GUID 0x27bd,
  0x3e6e), it does the same for each child of the Thing that has a PRenderMesh and no
  PShape, and so on down from that child. A child without a PRenderMesh ends its branch.
- A new PRenderMesh starts with the byte 3 (its constructor), and with one start value in
  the two colour words and the tint word.
- The byte is the `visibilityFlags` field a level file stores for a RENDER_MESH part.
  Bit 0 is "visible in play mode": the game's own native of that name reads it. So the
  hide native leaves a mesh that is not visible in play mode, and a level file can store
  one that way.
- Use in the scripts. Hide and show: the level link, on its lock mesh. The editor colour:
  the global-settings tweaker, on its own Thing when it is created. The mesh tint and the
  shape colour: the scoreboard, the two race gates and the burning object; the
  scoreboard, when created, gives its shape a fixed colour and its mesh the shape's old
  colour as tint. The shape tint: the same four classes. The play-mode question: the
  note object, once.
- Read and not bound: `TriggerLevelLink.SetLockMeshVisible__b` (0x641520) makes the same
  visibility call on the link's lock mesh; no script calls it. `Thing.IsVisible__Q5Thing`
  (0x64b4b0) compares a frame number kept with the Thing's mesh against a global: whether
  the mesh was drawn, not a display setting.

INFERRED: the two colour words are the `editorColor` a level file stores, and the tint
has no stored form (a file has one colour per mesh); bit 1 of the byte is the editor's
(the community reader names the two bits play mode and edit mode); the order of the
channels in a packed colour (as the shape colour of part 2) and its rounding (the
conversion is vector code that the disassembly did not show in full). Not read: how the
renderer uses the byte, the editor colour and the tint.

### What the runtime does

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| Visibility byte | `ThingState::render_mesh_visibility`: the stored `visibilityFlags`; none without a PRenderMesh | FOUND |
| Hide, show | `LevelWorld::show_render_mesh`: the mask 3 on the Thing and down its children, by the rule above | FOUND |
| Hidden | `ThingDisplay::mesh_hidden`: bit 0 is clear. The level world runs in play mode | FOUND bit; that the game then draws nothing INFERRED |
| Editor colour | `ThingState::render_mesh_colour`: the stored colour until a script sets one; none without a PRenderMesh | FOUND target; INFERRED stored field |
| Mesh tint | `ThingState::render_mesh_tint`, only on a Thing with a PRenderMesh (before: on any Thing) | FOUND |
| Shape colour, shape tint | `ThingState::shape_colour`, `shape_tint`, only on a Thing with a PShape; a Thing without one is left alone (before: an error) | FOUND |
| Null Thing | the six setters do nothing (before: the shape colour and the mesh tint faulted) | FOUND |
| World matrix | `SetWorldPosition__m`, `SetLocalPosition__m`, the Things moved along with a parent, and a write of `PPos.WorldPosition` | unchanged rules (part 13) |

The dirty list (`lbp-runtime/src/display.rs`):

- `LevelWorld::take_display_dirty()` gives the Things whose display values changed since
  the last call, in Thing order, each with a `DisplayChange` (visibility, colour,
  transform), and empties the list. A Thing is listed when a write left a value that is
  not the value before it: the visibility byte, a colour as the packed colour the game
  keeps (a tint never set counts as white), the world matrix by its bits.
- `LevelWorld::thing_display(thing)` gives the values now (`ThingDisplay`),
  `display_at_load(thing)` the values the level file had. `ThingDisplay::moved_from` is
  the matrix that takes a Thing from its stored place to its place now: what a display
  puts in front of the matrix it placed a model with.
- `display_changed()` lists every Thing unlike the level file, for a display that starts
  after scripts already ran. `display_summary()` counts them (`DisplaySummary`).
- Not on the list: a player's Sackboy that the caller moves (`move_player`), the
  animation position and speed, the mesh texture.

Natives: 216 -> 219 in the table. New: `Thing.ShowRenderMesh__Q5Thing`,
`Thing.SetShapeEditorColourTint__Q5Thingr`, `PostItNote.IsVisibleInPlayMode__`. No
longer recorded only (`RECORDED_ONLY` 23 -> 21): `Thing.HideRenderMesh__Q5Thing` and
`Thing.SetRenderMeshEditorColour__Q5Thingr`.

`level_scripts`: `play_all` has a `display` block (and so has a level played alone): the
Things unlike the level file after the last tick and the levels with one, by kind; the
changes marked; the render meshes by stored visibility byte; the tints by kind.

### What the viewer does

`lbp-bevy-viewer --level-scripts`, for a level other than the Pod.

- The map. When the level scene is spawned, every draw entity of a Thing's own model
  gets a `LevelThing` component, and `thing_view::LevelThings` maps the Thing to its
  entities. A Thing's own model is its render mesh or one of its decorations, placed by
  its own matrix. The report lines say how many Things and entities are mapped.
- Each tick `level_play.rs` takes the dirty list and `thing_view::apply` sets the
  entities. For a scene that was just spawned it takes every mapped Thing that is hidden
  or unlike the level file instead, because the scripts tick while the scene loads. Like
  the light of part 18 it waits until the scene has been drawn for 60 frames.

| Change | In the viewer | Label |
|---|---|---|
| Render mesh hidden (bit 0 clear), by a script or in the level file | the mesh's entities are not drawn; the Thing's decorations stay | FOUND bit; INFERRED that nothing is drawn, and that decorations stay |
| Mesh tint | the mesh's entities get a material of their own; red, green and blue multiply the surface colour before lighting | INFERRED |
| Mesh tint's alpha | not used: a white tint with an alpha draws as no tint | open |
| World matrix | every entity of the Thing is moved by `ThingDisplay::moved_from` | the scene's own placing rule, moved along |
| Mesh editor colour set by a script | not drawn, counted | the scene builds the colour into the material when the level is loaded |
| Shape colour, shape tint | not drawn, counted | below |
| A moved Thing with a shape | not moved, counted | below |

- Shapes are not followed. The scene merges the shapes of one material and colour into
  shared meshes in world space, so one shape has no entity to hide, colour or move. A
  shape that a script recolours or moves keeps the look and place the level file gives
  it, and the viewer counts it ("no draw entity", "editor or shape colour"). Taking
  such shapes out of their batch is left for a later step.
- Also not followed: the Pod (`--pod-camera`: it places its planets by its own rules;
  since part 23 the planets level follows its world Thing as a whole),
  meshes skinned per item (a mesh on a joint span, `--per-bone-placement`), the
  backdrop, sticker decals.
- Without `--level-scripts` the viewer draws a level as before, with every mesh.
- The viewer prints one line per tick on which the scene changed (`Things at tick N:
  ...`), with the first Things hidden and tinted, and the "Level scripts" window has a
  Things line: the check that needs no screenshot.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path gamedata/lbp2/dlc/move/levels/side_rts/side_rts_tutorial.bin
--level-scripts --level-focus-thing 108`. Thing 108 is a level link's lock mesh. Add
`--no-lbp-lighting` to look at it without the viewer's fog.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Units: Thing; level. "Unlike" = the value after the last tick is not the level file's.
The three runs (plain, `--visit-triggers`, with `--pulse-switches`) give the same counts
except where two numbers are given.

| What | Things / levels |
|---|---|
| Render meshes | 68,618 |
| Stored visibility byte 3 / 2 / 0 | 56,512 / 11,573 / 533 |
| Not visible in play mode in the level file (bit 0 clear) | 12,106 / 218 |
| Visibility byte unlike the file | 0 / 0 |
| Mesh editor colour unlike | 10 / 1 |
| Mesh tint unlike (white when never set) | 102 / 101 |
| of them white with an alpha / with a colour | 101 / 100 and 1 / 1 |
| Shape colour unlike | 102 / 101 |
| Shape tint unlike | 0 / 0 |
| World matrix unlike in any bit | 4,545 / 32 |
| World matrix unlike by more than the counting threshold | 1 / 1 (no mesh, no shape) |
| Changes marked: visibility / colour / transform | 0 / 214 / 4,604 (pulsed 4,606) |

- So in these runs the scripts change no visibility byte: the 113 hide calls (103
  levels) are on meshes the level file already stores as not visible in play mode. What
  the viewer hides under `--level-scripts` is the 12,106 meshes of the file.
- The 1,180 editor-colour calls (135 levels) set the colour the file stores, but for 10
  Things of one level.
- The tints are the scoreboards': 101 are white with the alpha 0.498, and the viewer
  draws no tint for them. One, in one level, has a colour (0.70, 1.0, 0.15). 46 more
  Things in 21 levels are given a white tint, which is no change. That level is also
  the one with the 10 new mesh colours.
- The 4,545 moved Things are the ones whose script writes its stored place back; one
  Thing ends elsewhere, in the Pod, and it has no mesh. So no drawn Thing moves here.
  The counting threshold is 0.01, or 0.00001 of the matrix's largest element for a Thing
  far from the origin (`MOVED_FAR`, `MOVED_FAR_RELATIVE`; not game values).
- Things stopped by a fault: 4 -> 4 in all three runs; fault kinds 2. The Pod's fault
  moved by one instruction: the shape-colour call on a null Thing is now nothing, and
  the script then reads a part of that null Thing.
- Unbound natives called 51 / 87 / 87 with the same calls as before; native calls
  571,776, 5,363,790 and 5,423,370, unchanged. Calls to recorded-only natives fall by
  1,293 in each run (the two natives that left the list). Messages, global settings,
  trigger events and switch reactions are equal to part 18.
- Viewer, release build, side_rts_tutorial, 1280 x 720, no focus taken
  (`--qa-capture`): `Thing map (level scripts): 200 Things with a model of their own,
  759 draw entities`; `Things at tick N (new scene): listed 24, with a draw entity 24;
  hidden 23, ... tinted 0 ... not followed: no draw entity 0, editor or shape colour 1,
  tint of alpha only 1`. Two pairs of frames looked at, without the viewer's fog
  (`--no-lbp-lighting`), without and with the scripts: framed on Thing 108 the padlock
  of the level link is gone and the link's disc stays; framed on the scoreboard (Thing
  975) two small white markers at its lower corners are gone and its colours are the
  same. On ghost_train (869 Things mapped, 232 hidden, 1 tinted), framed on the
  scoreboard (Thing 1151): its frame turns from tan to yellow-green, and the small
  markers around it are gone. The frames stay in ignored `local-analysis/`.

### Not modelled

- Shapes: a recoloured, tinted or moved shape in the picture (102 recoloured shapes in
  101 levels).
- The mesh editor colour a script sets, in the picture; the tint's alpha; how the
  game's shader uses the editor colour and the tint.
- What the game draws for a mesh with bit 0 clear, and for the decorations of a hidden
  mesh; bit 1 of the byte.
- A Thing flag that makes a local position the world position (part 13), and the
  refresh of bodies and shapes when a Thing moves.
- The animation position and speed scripts set (537 and 560 Things in the earlier
  count): a mesh's animation is not played per Thing.
- The mesh texture a script sets (kept, not drawn); `SetCollidable`,
  `SetBonesToSkinPoseLocal` (unbound).
- The Pod's planets under `--level-scripts`.

### Verification

`cargo test -p lbp-runtime`: 52 unit and 38 world tests. New: 3 unit tests in
`display.rs` (packed colours; what counts as unlike, hidden and moved far, and the
matrix from the stored place; the sums) and the world test
`d268_hidden_coloured_and_moved_things_are_listed_for_a_display` (the seven natives on
a made-up Thing tree: the hide rule down the children, a shape, a child without a mesh
and a globe mesh; the null Thing; colours with and without the part; a move with its
descendants; the list taken once; the summary). `lbp-script-vm`: the display natives in
a world without the state. `lbp-bevy-viewer`: `thing_view` test (a new scene, then a
hide, a tint and a move after it, on three entities with a decoration; a material of
its own in the scene list; a tint of alpha only). fmt, the CI clippy line, the CI test
set, the Python tests and `cargo check` of both viewers pass; `cargo clippy -p
lbp-bevy-viewer --all-targets -- -D warnings` passes; the viewer's tests pass but the
one that fails on main (`sackboy_play::tests::input_script_steps`). Seen in the viewer
window on two levels: meshes hidden and one mesh tinted, from the first set of the
scene. Not seen by eye: a mesh hidden, tinted or moved while the scene is up (no level
of these runs does it after its first ticks; the test does), a moved Thing. Nothing
compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer feature).
