# Level scripts, part 68: the Pod Earth drawn per bone under the scripts (2026-10-15, D317)

Part 67 put the viewer's Earth about 5 % smaller than the game's before rest. That was a model
curve, not a measurement. Fitting an ellipse to the Earth's limb on the GPU-captured frames and on
viewer stills of the same ticks shows the widths agree and the viewer's Earth is 10 to 17 % too
tall.

### What was found

- MEASURED (limb ellipse fits at 1280 x 720; game frames saved from lane B's captures 6082, 8348,
  8980 and 9601, `local-analysis/d316-zoom/rt20_*`): the game's Earth has height over width 0.91
  to 0.96 and the viewer's 1.06 to 1.12, with equal widths and centres within 2 px.
- MEASURED (`pod_globe_extent`, pod_planets Thing 1, mesh 000027bd, boneThings [1, 2, 3]): the
  bone-0 rule the scripted Pod drew gives half-extents (1635, 1829, 1610); placing each vertex by
  its own bone gives (1637, 1577, 1612), the proportions of the game's Earth vertices
  (3286 : 3151 : 3219 at the world's x 2).
- FOUND: per-bone placement was off in the scripted Pod (D275) because the per-bone copy
  (`MeshSource::BoneSkin`) was left off the Thing map, so the scripts never moved it and it stayed
  at its stored place (up and to the left, by more as the planets zoom grows).
- MEASURED (new probe `pod_bone_things`, shell and planets session, in use at 600): Things 2 and
  3 (sea, land) are children of Thing 1; the scripts move Things 1 to 3 by one translation, and
  the places of 2 and 3 relative to 1 equal the stored ones at ticks 5, 600, 611, 616 and 625.

### What the viewer does

- The per-bone copy, baked in the frame of its Thing's item, is on the Thing map like the file's
  mesh, so it follows Thing 1. Per-bone placement is on by default in the scripted Pod
  (`--no-per-bone-placement` turns it off). Without scripts nothing changes.

### Verification

Viewer stills (hidden window, level lighting, in use at 600; centre, half-width x half-height):

| tick | game | before | after |
|---|---|---|---|
| 600 | (588.7, 345.1) 67.3 x 64.9 | (588.9, 344.8) 67.4 x 75.5 | (588.8, 344.9) 67.5 x 65.1 |
| 611 | (530.6, 376.1) 107.5 x 101.6 | (531.7, 374.7) 108.2 x 119.5 | (531.6, 374.7) 108.4 x 103.0 |
| 616 | (456.0, 319.4) 163.1 x 153.1 | (456.2, 318.7) 162.8 x 176.8 | (456.1, 319.3) 163.0 x 152.6 |
| 625 (game 619) | (416.2, 296.8) 195.0 x 177.9 | (411.3, 294.0) 196.3 x 210.9 | (411.4, 295.0) 196.3 x 181.9 |

Scripts and stills stay in ignored `local-analysis/d320-earth/` and `rscript/stage6/shots/d320-*`.

Counts: +0 newly viewable / +0 newly Complete.
