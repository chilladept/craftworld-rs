# Level scripts, part 39: the hanging cloths' art on every lighting route (2026-10-14, D288)

Fifteenth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
Part 38 left two things open about `--preview-lighting` in the planets room: the room is
washed out, and the two hanging cloths are plain tan where the game's frame shows their art.
This part finds that the second was not the preview lighting at all and fixes it, and says
why the first is the preview route's own design.

### What was found

- FOUND (the viewer's buckets for the planets room's backdrop mesh, 56 materials): each
  hanging cloth is drawn twice on the same triangles. The plain cloth is gfx material
  0x1f3ad (one bucket for both cloths, 3,380 triangles; an opaque tan texture times a second
  sample), early in the mesh. The art is two later buckets, 0x1f3b3 (the left cloth, 1,720
  triangles, the yellow ring) and 0x1f3b4 (the right cloth, 1,658 triangles, the blue
  rings), each a 256 x 512 texture in its own slot 0; the left one's alpha holds the
  dripping lower edge. No decal and no second texture layer: the art is its own primitive.
  The easel's canvas on the table is a pair of the same kind.
- MEASURED (single materials painted red in probe builds, not kept): with
  `--preview-lighting` and in the planets level alone under its own lighting, painting
  0x1f3b3 or 0x1f3b4 red changes nothing on screen and painting 0x1f3ad red turns both
  cloths red: the plain cloth wins. In the closed Pod under the level's own lighting the art
  wins.
- MEASURED: which one wins is not tied to the lighting. Under `--preview-lighting`, adding
  `--bump-gloss` (the bump map and the other surface extras, which the preview shading does
  not use) brings the art back; under the level lighting, leaving out one unrelated draw
  (a joint of the shell level) hides it. INFERRED: the two layers are at exactly the same
  depth and Bevy's opaque pass draws material buckets in an order of its own (bins by
  pipeline and material), so the winner followed the draw list: the level lighting gives
  the painted cloths a bump map and with it a second UV set, so another pipeline and
  another place in the order.
- MEASURED (the game's frame `d156-rpcs3-pod-default.png`): the game shows the art, with the
  plain cloth showing between the drips. INFERRED: the game draws a mesh's primitives in
  order and keeps the later one at equal depth.
- MEASURED, the washed-out room: with the preview's distance fog taken off (a probe build),
  the planets room's back right wall from the pod camera goes from (178, 169, 151) to
  (103, 162, 173), the left wall from (201, 172, 144) to (182, 164, 156); under the level's
  own lighting they are (122, 107, 91) and (168, 121, 81). So most of the wash is the preview
  fog: Bevy's linear `DistanceFog` with the level's fog colour (0.646, 0.409, 0.228) taken
  as linear light, from 25,805 to 78,454 level units with the script's bias, over a room
  that starts about 31,000 units out (INFERRED from the world Thing's offset). The rest is the Lambert preview fit (ambient plus key scaled to a lit
  peak of 1, no exposure). INFERRED: the game applies that colour in its encoded output
  space with a luma term (P2t), so the same numbers read darker there.
- FOUND (docs): `--preview-lighting` is D121's opt-in Lambert preview, labelled "not game
  lighting"; since P2w levels are lit by the level's own model by default and the flag
  keeps the older preview. It is meant to differ, so the wash is by design and not changed.
  Part 38 already says Pod colour comparisons should use the level's own lighting.

### What the viewer does

`lbp-bevy-viewer` (`main.rs`, `depth_bias`), every opaque bucket.

- A bucket is uploaded with a constant depth bias equal to its index in the mesh's bucket
  list (first-appearance order of the primitives, capped at 65), so a later material wins an
  equal depth whatever order Bevy draws them in. Bucket 0, and so every one-material mesh,
  keeps no bias. Decals keep their bias of 8 and the glass none.
- One bias step is the depth buffer's smallest step at that depth, so the most a bucket
  moves is 65 steps: surfaces that close but not equal can now draw the other way.

### Measured (PC, release build)

Frames in ignored `local-analysis/rscript/stage6/shots/` (`d288-*`, scripts
`scratch/d288-shot.ps1` and `d288-all.ps1`), compared with D287's.

- Planets level, pod camera, tick 600, `--preview-lighting`: 331,393 pixels change (at most
  81 levels): the cloths show their art and the canvas its colour patches. Left cloth mean
  (189, 170, 132) is now (192, 193, 173), the right (154, 158, 137) now (142, 137, 130).
- The same under the level's own lighting: 290,610 pixels (at most 127); this frame also
  showed the plain cloths before. Left cloth (113, 106, 77) now (113, 120, 103), right (48,
  78, 75) now (38, 60, 70).
- Closed Pod at 600 with `--preview-lighting`: 290,699 pixels (at most 65), the art through
  both panels. Under the level lighting: 6 pixels (at most 25); it already showed the art.
  The shell without scripts: 78,145 pixels.
- Another level, Da Vinci's rookie test (0001e786, used in D149): with `--preview-lighting`
  equal byte for byte; under its own lighting 28 pixels change, at most 8 levels.

### Left out

- The preview's fog and its Lambert fit stay as they are (the route's meaning).
- Under `--preview-lighting` the cloths' dripping edge is not cut (the alpha test is drawn
  with the level lighting only, P3a), so the art runs to the cloth's bottom there.
- Whether the game's depth test is "less or equal" (the INFERRED order rule) is not read
  from a capture.

### Verification

`lbp-bevy-viewer`, no game data: `d288_a_later_bucket_on_the_same_triangles_is_drawn_nearer`
(the bucket list keeps first-appearance order; bucket 0 has no bias, a later bucket more, the
cap holds, a decal keeps 8, the glass 0). fmt, the CI clippy line, the CI test set, `cargo
check` of both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass;
the viewer's tests pass (97) but the one that fails on main
(`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
