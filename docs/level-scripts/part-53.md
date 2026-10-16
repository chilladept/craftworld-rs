# Level scripts, part 53: the game's anti-aliasing job in the Pod picture (2026-10-14, D302)

Viewer only, no runtime change. Part 51 found the Pod's post chain handing the scene to the
EBOOT's "EDGE MLAA" SPU job and getting it back with alpha 0 and changed edges, and left the
filtering itself out. This part reads what the job does from its parameters, its program's
edge test and the captures, and adds it to the viewer behind `--mlaa`.

### What was found

- FOUND (EBOOT 01.33): 0x31dfd4 kicks the job through 0xc0bca8 with the buffer as both input
  and output, 1280 x 720, pitch 0x1400, two byte values 10 and 0x59, a third byte 1 and the
  label value. 0xc0bca8 writes them into each SPU task's 64-byte record (width +0x1c, height
  +0x1e, pitch +0x20, the 1 at +0x22, 10 at +0x24, 0x59 at +0x26). 0xc0beec, which creates the
  task set, takes the SPU count, the SPURS instance, the priorities, the label index and a
  work buffer.
- FOUND (the job's SPU program, its edge test only): before the test each pixel's threshold is
  computed from the largest of its R, G and B, multiplied by a 16-bit parameter with the high
  byte kept, floored by a second parameter, and written into the pixel's alpha byte. The test
  takes the larger byte of two neighbours lane by lane (so the larger threshold), copies that
  alpha over the colour lanes and compares each channel's absolute difference with it: an
  edge where any channel's difference reaches the threshold. The alpha byte is the job's own
  scratch (the copy back's alpha 0, part 51). INFERRED which kick value is which: 10 the
  absolute floor and 0x59 the factor (89 / 256 = 0.348); the other way round the floor would
  be 89 and the blended steps of contrast 50 to 55 in 5247 (below) would not be edges.
- MEASURED (frame 5247's capture, replayed on a hidden desktop, 0 foreground samples): every
  texture of 640 x 360 or more (38) has one sample, the scene target (1280 x 720, R8G8B8A8)
  and its depth buffer (D24S8) among them: the game draws the Pod without MSAA.
- MEASURED (5247, the scene before the copy out, eid 3195, against the copy back, 3364): the
  copy back is stale (part 51) and differs on 185,654 pixels, most of them flat (whole tiles of
  the window paintings, an earlier frame's content). Kept for the comparison: the 644,697
  pixels whose flat neighbours within 6 px agree and where no change exceeds 0.6 of the local
  contrast. 14790's copy back is stale everywhere and gives nothing.
  - Along runs of edges that end in a step at both ends (Z and U shapes), the change is
    Reshetov's (2009) morphological blend: from the middle of each step a line runs to the
    middle of the run, and each pixel beside the run takes the colour across by the area the
    line cuts from it. Fit scale 0.97 (median, 74 runs of 8 pixels or more); a 16-pixel run
    gives 0.41, 0.35, 0.29, 0.22, 0.16, 0.09, 0.03 from its end against the areas 0.41, 0.34,
    0.28, 0.22, 0.16, 0.09, 0.03, the second half the same on the other side.
  - A pixel beside edges on two sides takes only its larger weight: on 45-degree stairs the
    change is 0.48 of the summed areas (1/8 of the contrast, not 1/4). Residual of the copy
    back against the model (as a share of its difference from the scene): the larger of the
    four weights 0.640, their sum 0.792, the mean of the two directions 0.688, the rows' blend
    and then the columns' 0.712.
  - With the FOUND rule and the larger weight: thresholds (10, 0.348) 0.618, against (5,
    0.348) 0.635, (20, 0.348) 0.634, (10, 0.25) 0.679 and (10, 0.45) 0.684. Runs with one
    step (L shapes) blended: 0.637 against 0.650 without.
  - Runs longer than some length are not blended: residual 0.637 with no limit, 0.629 at 64,
    0.626 at 44, 0.618 at 36 to 40, 0.621 at 32. A 36-pixel Z run is blended as the areas say
    (fit 1.04 and 0.95 for its halves), a 42 and a 101 do not follow them (one 76-pixel U of
    weak contrast does). INFERRED limit 40.

### What changed

`lbp-bevy-viewer`, new `mlaa_post.rs`, and `main.rs`, `glow_post.rs`, `dof_post.rs`:

- `--mlaa`: in the Pod picture (`--pod-camera`) on the level-lighting route, two full-screen
  passes after the depth of field. The first marks the edges below and right of each pixel
  with the FOUND test in 8-bit encoded values (`MLAA_ABS_THRESHOLD` 10, `MLAA_REL_THRESHOLD`
  0x59); the second finds each run through a pixel's four sides (up to
  `MLAA_MAX_EDGE_LENGTH` 40 each way; longer runs are left alone), the steps at its ends and
  the area for the pixel, and mixes in the neighbour across its largest weight. The blend is
  INFERRED as the published algorithm with the MEASURED weights; the SPU program beyond its
  edge test was not followed. Elsewhere the flag prints that it is ignored. Off by default.
- The camera renders without MSAA under the flag (the game's single sample).
- Order, as the game's (frame 5247: copy out 3194/3195, glow source 3256, copy back
  3363/3364, final pass 3454): the depth of field, then the job; when the glow chain's last
  pass runs, the job writes a texture of its own that this pass reads as the scene, so the
  glow's source still reads the scene before the job; otherwise the job replaces the main
  texture. The scene's alpha is kept; the final pass's alpha 0 in the Pod is part 51's rule.

### Measured

d302 release build, hidden window (`scratch/d287-shot.ps1 -Prefix d302`, list in
`scratch/d302-all.ps1`), stills in ignored `local-analysis/rscript/stage6/shots/` (`d302-off-*`
default, `d302-on-*` with `--mlaa`); edges by the new `scratch/cmp293/d302edge.py` (each image
in its own pixels: the sub-pixel position where luma crosses the middle of the step along a
line, and its RMS about a quadratic fit, about 0.29 px for a hard staircase).

- Two runs of each are byte-equal. The default closed Pod is byte-equal to D300's
  (`d300-on-closed-tick600`); Da Vinci's rookie test with `--mlaa` prints that it is ignored
  and is byte-equal to the default.
- The closed Pod at 600: `--mlaa` changes 43,179 pixels (15,117 by more than 8 levels). Staircase
  RMS, the game's `d156-rpcs3-pod-default` / the default (MSAA) / `--mlaa`: the floor line on the
  left 0.136 / 0.349 / 0.252; the right post's left outline 0.196 / 0.264 / 0.250; the left
  post's window edge 0.116 / 0.179 / 0.151; the DualShock's shoulder (short, curved) 0.875 /
  1.061 / 0.978. The profiles across the edges are within 0.05 of each other. Mean |d| over
  the viewport against the game (warped s 0.875, dx +1, dy +1) 6.21 both.
- The change-over at 614 against `open-t1` (warped s 0.877, dx +2), region means as part 51:
  within 0.3 of the default (DualShock body 0.1 -> 0.4, the post cards 2.9 / 2.4 -> 2.9 / 2.5,
  the right glass 3.6 -> 3.7, the room 3.0 -> 3.2), viewport 12.72 -> 12.71. (The default at
  614 is not D300's still: part 52 moved the zoom a tick.)

### Left out

- The job's program beyond its edge test: how it finds runs, the pattern rules where a run
  meets steps on both sides of one end, its real length limit (here 40, INFERRED) and why
  its byte value 1 is passed.
- A frame where the copy back is current (RenderDoc does not keep the CPU's writes to it).
- `--mlaa` by default in the Pod, and MSAA in the Pod's other views.

### Verification

`lbp-bevy-viewer`, no game data: `d302_constants_and_threshold` (10 below a largest channel
of 29, 0.348 above, any channel against the larger threshold), `d302_straight_edge_and_low_contrast_untouched`,
`d302_staircase_follows_the_areas_and_the_game` (a 16-pixel run against the areas and the
MEASURED weights within 0.02), `d302_diagonal_takes_one_neighbour` (1/8 on 45-degree stairs),
`d302_long_runs_untouched` (40 blended, 41 not), `d302_mlaa_shader_validates` (naga, both
target kinds) and `d302_route_and_extract`; the CPU mirror runs the shader's two passes. fmt,
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` and `cargo check --workspace`
pass; the viewer's tests pass (136) but the one that fails on main
(`sackboy_play::tests::input_script_steps`). RenderDoc only on a never-shown desktop (0
foreground samples); nothing in RPCS3 was started or captured.
