# Level scripts, part 49: the depth of field (2026-10-14, D298)

Viewer only, no runtime change. The comparison of the change-over against `open-t1` (part 45's
notes, item 5) found the game blurring the room behind the Pod and the viewer not: the scripts'
`Cam.SetDOFAndFog` passes a range each tick, and the viewer used only its third value (the fog
bias, part 25). This part copies the game's depth of field, behind a flag.

### What was found

- FOUND (RenderDoc replays of frames 5247 and 14790 on a hidden desktop: each pass's shader,
  constants, bound buffers, blend, depth and stencil state, and its input and output buffers):
  five draws before the glow's source pass, all on 8-bit buffers.
  1. Half size (640 x 360): the scene's 2x2 mean (one bilinear tap; MEASURED within 0.5 of a
     level) and, in the alpha, a gate `g = saturate(A x depth + B)` from the depth buffer.
  2. and 3. Half size, twice: the centre and five taps on the corners of a regular pentagon
     (72 degrees apart, +y up the image), at a radius `r = saturate(8 g - 7)` times (0.0025,
     0.00444) of the uv (1.6 half-size texels). A tap counts only when its own gate is above the
     centre's minus 0.01, so nearer surfaces do not bleed into the blur; the colour is the mean
     of the counted taps and the centre. The first pass keeps `g` in the alpha, the second
     writes `r`. Each tap also moves by an offset read from another buffer; that buffer is
     mid-grey (127 or 128) everywhere in both frames, so the offset is under a tenth of a texel
     and its alpha term 0 (not copied).
  4. and 5. Full size: the second pass's result, read back bilinear, blended over the scene by
     its alpha (source alpha / one minus source alpha on the colour; the destination alpha is
     kept), twice: once over the pixels whose stencil has bit 0x80 set (no depth test), once over
     the others where the scene lies beyond a plane drawn at the depth where `g` = 7/8.
- MEASURED: a mirror of passes 2 and 3 rebuilds the game's buffers from their inputs within
  0.6 and 0.9 of a level (with +y down it misses by up to 13); the composite with a bilinear
  read-back fits the replayed frame within 0.4 of a level on the changed pixels (3.5 without).
- FOUND (EBOOT 01.33): the native `SetDOFAndFog__fff` (0x5fb534) hands its three floats on
  (0x89bd9c -> 0x734638 -> 0x8a23ec -> 0x2e852c with mode 2). There the third goes to the fog
  (0x34fc48), and 0x2e81ac takes the first two, puts a point at each distance along the
  camera's axis, projects both through the view-projection and keeps `0.5 x z / w + 0.5` of
  each, the depth-buffer value at that distance; they land at +0x350 / +0x354 of the render
  settings and are copied into the frame's packet (0x3205a0).
- FOUND (fit of the two frames' gate constants, 5247 (4993.22, -4992.22), 14790 (599.87,
  -598.56), to the Pod camera's planes 10 and 100,000, part 34): the gate is 7/8 at the depth
  of the first value and 1 at the depth of the second, with (80,000, 100,000) in 5247 (the idle
  view, twice (40,000, 50,000)) and (12,000, 16,000) in 14790 (the main menu's view, twice
  (6,000, 8,000)), within 0.13 % (the depths near 1 are 32-bit floats). So the blur's radius
  is linear in the depth buffer from 0 at the first value to 1 at the second, and the
  composite's plane is the first value. The code that turns the two depths into A and B was
  not read (INFERRED from the fit).
- MEASURED: in 5247 (the Pod closed) no gate reaches 7/8: the passes run and change nothing;
  in 14790 the room and the planets world beyond 12,000 are blurred (85 % of the pixels at
  radius 1). Which surfaces carry stencil bit 0x80 was not found (in 14790 they are beyond
  the plane as well).

### What changed

`lbp-bevy-viewer`:

- New `dof_post.rs`: on the main 3D camera, in `Core3dSystems::PostProcess` after tonemapping
  and before the glow and the final pass (`glow_post.rs`), the game's four steps: the half-size
  source, two rings, the composite, on `Rgba8Unorm` buffers in the game's encoded space. The
  constants are `DOF_RAMP` (8), `DOF_TAP_STEP` (0.0025, 1.6 / 360), `DOF_TAP_DIRS`,
  `DOF_SAME_LAYER` (0.01) and `DOF_RING_PASSES` (2). The ramp is computed from Bevy's
  reverse-z depth buffer as `(1/start - 1/z) / (1/start - 1/end)`, the same function of
  distance as the game's (both buffers are affine in 1 / z), with the depth's first sample
  under 4x MSAA. The composite runs where the ramp is above 0 and keeps the scene's alpha (the
  glow's source reads it next). The stencil-marked pass and the offset buffer are not copied.
- `level_play.rs`: `follow_dof` puts the scripts' range (the first two values of the last
  `SetDOFAndFog`, level units, times the display scale) on the camera's `DepthOfField` each
  frame, and prints one line when it changes.
- New flag `--depth-of-field`, off by default, on the level-lighting route only (ignored with a
  note under `--preview-lighting` or `--no-lbp-lighting`, as `--final-pass`). With it the main
  camera's depth buffer is made readable; the passes run while the scripts have passed a range
  (0 < start < end). Without the flag nothing changes.

### Measured

d298 release build, hidden window (`scratch/d287-shot.ps1 -Prefix d298`, list in
`scratch/d298-all.ps1`), stills in ignored `local-analysis/rscript/stage6/shots/` (`d298-off-*`,
`d298-on-*`, all with `--final-pass`); gradients by `scratch/cmp293/d298grad.py` (grey,
warped as `warp.py`, s 0.877, dx +2), regions by `d294cmp.py`.

- Without the flag the build is byte for byte the one before it: the change-over at 614 equals
  `d297-before-tick614`, the closed Pod at 600 and Da Vinci's rookie test (0001e786) equal
  `d294-on-*`.
- In use at 600, held at 614: the scripts' range is (31,159, 39,667) level units (part 46's
  blend at 0.718, twice). 319,437 pixels change (of 921,600), all in the room and the planets
  world; the Pod, the DualShock and the planets in front stay equal.
- Mean gradient magnitude, game `open-t1` / without / with: room behind the window 1.20 /
  3.76 / 1.84, room upper left 0.67 / 1.30 / 1.18, upper right 0.65 / 1.45 / 1.35, upper centre
  0.76 / 1.54 / 0.92; the sharp references unchanged, DualShock 2.48 / 2.48 / 2.48 and the left
  post's card 7.89 / 7.14 / 7.14. (Main at D293 gave 1.71 / 0.76 / 1.48 for the first three, the
  notes' 1.72 / 0.77 / 1.49; the room is now in part 46's zoomed view, hence 3.76.)
- Region means against the game: the room behind the window |d| 5.0 -> 4.8; every Pod region
  unchanged. The halo over the line (median of 29 post columns, 1 / 2 / 3 / 5 / 9 px): game 47.4
  / 14.7 / 11.2 / 5.0 / -0.7, without 46.1 / 10.7 / 7.7 / 4.1 / -3.1, with 44.8 / 13.8 / 9.4 /
  4.5 / -3.0 (the blurred room behind the posts). Viewport mean |d| 14.95 -> 14.56.
- The closed Pod at 600 (range (80,000, 100,000); the planets level reaches 69,306 along the
  view) is unchanged, as the game's 5247 is. Da Vinci's level passes no range: unchanged.
- The game's room still reads softer: its view is further into the zoom (`open-t1` is about
  tick 614.2 and the camera differs, part 45's notes item 1), and its upper corners are blurred
  where the viewer's are not yet beyond the start of the range.

### Left out

- The stencil-marked composite (what sets bit 0x80) and the offset buffer (mid-grey in both
  frames).
- The code that builds the gate's two constants from the stored depths (the fit above).
- A range from anything but the scripts (a level's own settings, other `0x2e81ac` modes); the
  flag stays off by default until a non-Pod game frame with depth of field is compared.

### Verification

`lbp-bevy-viewer`, no game data: `d298_constants` (the ramp, the step of 1.6 half-size
texels, five unit directions 72 degrees apart, two rings), `d298_gate_constants_from_the_range`
(both frames' gate constants from the two ranges and the planes 10 / 100,000 within 0.2 %; the
game's radius against the viewer's ramp at five distances; the reverse-z ramp equals the one
over distance), `d298_far_blurs_near_stays` (a CPU mirror of the four steps, `dof_cpu`: a far
pattern loses over half its gradient, a near one is equal pixel for pixel, a near square does
not bleed into the far grey beyond the bilinear read-back's two pixels),
`d298_radius_follows_the_ramp`, `d298_dof_shader_validates` (the consts in the text; naga
validates both target kinds with and without a multisampled depth buffer) and
`d298_extracted_only_with_flag_and_range`. fmt, `cargo clippy -p lbp-bevy-viewer --all-targets
-- -D warnings` and `cargo check --workspace` pass; the viewer's tests pass (124) but the one
that fails on main (`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
