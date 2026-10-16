# Level scripts, part 43: the glow on the dissolve's line (2026-10-14, D292)

Nineteenth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
Part 42 found that the green halo around the tent's line in the game's frame is not in the
tent's shader but a post pass reading the scene's alpha. This part copies that pass.

### What was found

- FOUND (`pod_card_edge`, frame 5247, part 42): the tent writes alpha = t x texalpha, 0 on
  the line and rising through the band; opaque surfaces write 1.
- FOUND (frame 5247's post chain): pass 3256 at half size, one bilinear tap per texel,
  `glow_src = 0.7 x (1 - a) x rgb` on the encoded values (fc4 = 0.7); passes 3281 and 3298, a
  separable 9-tap blur at a quarter of the size, weights 0.3 at the centre and 0.225 / 0.15 /
  0.105 / 0.06 at +-2 / +-6 / +-10 / +-14 full-size pixels, 1.38 per pass (not normalised);
  final pass 3454 `c = scene x 1.2998 + glow`.
- FOUND (Bevy 0.19.1 source): the opaque mesh pipeline has no blend and writes all four
  channels, so an opaque fragment's alpha is stored as written; the blend and premultiplied
  pipelines blend alpha as One + OneMinusSrcAlpha, which keeps a destination alpha of 1; the
  glass's dual-source pipeline changes only the colour blend. The viewer's main camera is
  LDR with tonemapping off (main texture `Rgba8UnormSrgb`) and Bevy's default 4x MSAA; post
  passes read the resolved texture. egui and Bevy UI draw after `Core3dSystems::PostProcess`.

### What changed

`lbp-bevy-viewer`:

- `dual.rs` shader: while the dissolve is on, a fragment of an opaque material (Bevy's
  alpha-mode bits) writes alpha = the kept share (`dissolve_keep`); blended draws keep 1,
  the glass keeps its own alpha (parts 37, 38, 41).
- New `glow_post.rs`: on the main 3D camera, in `Core3dSystems::PostProcess` after
  tonemapping (so before egui/UI), four full-screen passes: the source at half size (the
  2x2 mean of the encoded colour and alpha, then 0.7 x (1 - a) x rgb), the horizontal and the
  vertical blur at a quarter size with the game's offsets and weights (bilinear taps,
  `Rgba16Float` textures), and the add: `out = encode(scene) + glow`, saturated, alpha 1. The
  constants are `GLOW_STRENGTH`, `BLUR_OFFSETS` and `BLUR_WEIGHTS`. On an sRGB main texture
  the pass encodes what it reads and decodes what it writes, so the sums are the game's 8-bit
  numbers; a plain 8-bit texture is used as is; other formats (HDR) skip the pass.
- The chain runs only while some viewer material has a dissolve on (the camera's
  `EdgeGlow` marker reaches the render world only then) and not under the new
  `--no-edge-glow` (`--no-glow` is P3h's material glow). Without a dissolve nothing runs.
- The final pass's x1.2998 over the whole frame is not copied (part 42 left it open).

### Measured

Pod with the scripts, in use at 600, held at 615 / 617, both lighting routes; D291's
stills (`d291-new-*`) against this build. Frames in ignored
`local-analysis/rscript/stage6/shots/` (`d292-new-*`, capture script `scratch/d287-shot.ps1`,
hidden window).

- Equal byte for byte: Da Vinci's rookie test (0001e786) and the planets level at 600
  with `--pod-camera 1` and the scripts (no dissolve, the pass does not run).
- Changed: 83,209 pixels at 615 and 57,389 at 617 (57,141 under the default lighting), all
  in the tent's region (y 555 and below at 615): a green halo on both sides of every line.
- Green excess (G - (R + B) / 2) added above the line at 1 / 2 / 3 / 5 / 9 / 14 px: x 360
  +29 / +28 / +21 / +16 / +9 / +3 (615), +26 / +21 / +19 / +14 / +7 / +2 (617); x 935 +29 /
  +24 / +20 / +12 / +6 / +2 (615), +30 / +26 / +20 / +13 / +7 / +2 (617). The game's
  `open-t1` (x 362, over its baseline of about 3): 19 / 12 / 9 / 5 / 0. Same reach, but
  about 1.5 to 2 times the game's strength, as part 42's notes predicted for 0.7 (t1 fits
  about 0.4 to 0.45, INFERRED); the FOUND 0.7 is kept.
- The line: (88, 192, 14) becomes (116..127, 240..255, 23..32), against the game's peak
  (113..122, 227..255, 25..35), which also carries the x1.3.
- Under the line the band (alpha between 0 and 1) glows over its whole height: +30 to +50
  of excess within 16 px below. The game's frame was not measured there.

### Left out

- The final pass's x1.2998, and the strength 0.7 against t1's apparent 0.4 to 0.45 (the
  menu frame's own settings were not captured).
- The texalpha factor of the game's alpha (the viewer writes the kept share alone).
- The pass's other terms (the volumetric light, the blue side streak) and the depth of
  field before it.
- HDR cameras (the pass skips them).

### Verification

`lbp-bevy-viewer`, no game data: `d292_blur_weights_sum_to_1_38`,
`d292_line_glow_follows_the_model` (a CPU mirror of the chain, `glow_cpu`, on a 3 px alpha-0
line of COL0: averaged over the four phases against the 4 px grid, within 0.02 of the
notes' 0.23 / 0.17 / 0.13 / 0.10 / 0.07 at 1 / 2 / 3 / 6 / 9 px, falling off on both sides,
0 at 24 px; on the line 0.24 to 0.30 where the notes' model said 0.35, INFERRED a
difference of sampling in that session model), `d292_opaque_scene_has_no_glow`,
`d292_glow_shader_validates` (both target kinds parse and validate with naga, the FOUND
constants are in the text, the add has no scale and writes alpha 1),
`d292_target_formats`, `d292_extracted_only_while_active`; the dual shader text test checks
the alpha write before the glass's. fmt, `cargo clippy -p lbp-bevy-viewer --all-targets --
-D warnings` and `cargo check --workspace` pass; the viewer's tests pass (113) but the one
that fails on main (`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
