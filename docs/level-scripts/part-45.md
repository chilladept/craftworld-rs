# Level scripts, part 45: the final pass's brightness and vignette (2026-10-14, D294)

Twenty-first PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
Parts 42 and 43 left the game's final pass out but for its glow add: it also scales the whole
frame by 1.2998 and darkens it toward the edges. A comparison of the change-over at 614 against
the game's `open-t1` (main at D293) found the viewer's lit colours equal to the game's scene
before that pass, so this part copies it, behind a flag.

### What was found

- FOUND (frame 5247, final pass 3454, PS 14857, read from the RenderDoc replay's constants):
  `out = (scene x 1.2998 + glow) x (0.2 + 0.8 x smoothstep(0, 1, saturate(1.2 - r)))`, with
  `r = length(((u - 0.5) x 1.0996, (v - 0.5) x 1.7002))` over the viewport's uv (fc13 = 1.2998,
  fc15 = (1.0996, 1.7002), fc18.w = 1.2, fc26 = 3 the smoothstep's 3 - 2x, fc12 = fc2 = fc8 = 0.2
  the floor; the colour matrix fc14..25 is the identity). Encoded (8-bit) values, as the glow.
- The vignette is 1 within r 0.2 of the centre, about 0.83 at the Pod's post tops, 0.47 at the
  bottom centre and 0.27 in a 16:9 frame's corner (r 1.012); its 0.2 floor (r 1.2) is never
  reached inside the frame.
- MEASURED (main at D293, before this part): x1.3 alone takes every region of `open-t1` further
  from the game; x1.3 with the vignette brings the Pod shell and the left glass panel within 0
  to 2 levels. So the level lighting is fitted to the scene before the final pass (as D290's
  RenderDoc-based fit implied), and the line's 1.0 x COL0 against t1's 1.3 x COL0 is the same
  missing factor.

### What changed

`lbp-bevy-viewer`, `glow_post.rs`:

- New consts `FINAL_SCALE` (1.2998), `VIGNETTE_AXES` (1.0996, 1.7002), `VIGNETTE_START` (1.2)
  and `VIGNETTE_FLOOR` (0.2), written into the WGSL like part 43's.
- The add pass has two new entry points: `final_composite`, `(encode(scene) x FINAL_SCALE +
  glow) x vignette` saturated at the end (the game's 8-bit target), and `final_only`, the same
  with the glow term 0. The vignette uses each pixel's centre over the main texture's size
  (WGSL `smoothstep(0.0, 1.0, saturate(..))`, the HLSL one). Alpha 1, as part 43's.
- New flag `--final-pass`, off by default, on the level-lighting route only: a level without
  `--preview-lighting` or `--no-lbp-lighting` (that route is the one fitted to the game's
  scene target; the preview lighting is not the game's). Elsewhere it prints that it is
  ignored. With it the add pass runs on every frame (the camera's `EdgeGlow` marker reaches
  the render world while the glow or the final pass is on); with no dissolve, or under
  `--no-edge-glow`, only the add pass runs (`final_only`), the glow's three passes do not.
- Without the flag nothing changes: part 43's chain and `glow_composite` are as they were.

### Measured

d294 release build, hidden window (`scratch/d287-shot.ps1 -Prefix d294`, list in
`scratch/d294-all.ps1`), stills in ignored `local-analysis/rscript/stage6/shots/` (`d294-off-*`,
`d294-on-*`); comparisons `scratch/cmp293/d294cmp.py` (open: warped as `warp.py`, s 0.877, dx
+2) and `d294reg.py` (closed: registered again, s 0.875, dx +1, dy +1, correlation 0.73).

- Without the flag the build is byte for byte main's: the in-use-at-600 still held at 614
  equals `cmp293-defaultlight-inuse600-tick614`. Under `--preview-lighting` the flag is
  ignored (the 614 stills with and without it are equal).
- Open, 614 against `open-t1` (region mean RGB, mean |d| over the channels):

| region | game | without | with `--final-pass` |
|---|---|---|---|
| DualShock body | (14.1, 15.6, 14.3) | (17.8, 19.9, 18.3) 4.0 | (13.9, 15.5, 14.3) 0.1 |
| left post card | (29.9, 38.8, 14.8) | (34.7, 42.2, 18.0) 3.8 | (27.3, 33.3, 14.1) 2.9 |
| right post card | (25.1, 33.3, 12.7) | (29.7, 37.1, 15.6) 3.8 | (23.0, 28.8, 12.0) 2.4 |
| wall under the window | (23.1, 23.8, 12.1) | (34.5, 34.3, 18.3) 9.3 | (22.1, 22.0, 11.7) 1.1 |
| left glass panel | (97.6, 91.0, 72.9) | (133.9, 126.1, 92.4) 30.3 | (97.5, 91.4, 67.4) 2.0 |
| right glass panel | (61.1, 65.1, 54.8) | (78.8, 89.2, 74.6) 20.5 | (52.4, 58.4, 49.1) 7.0 |
| room behind the window | (178.3, 139.0, 97.8) | 11.1 | 10.6 |

- The Pod shell by 16-row band, (R + B) / 2, game / without / with: left post y 540 36.2 /
  34.7 / 32.8, y 556 32.4 / 33.1 / 29.8, y 620 15.8 / 21.8 / 14.7, y 652 10.7 / 18.2 / 10.1; right
  post y 652 10.1 / 18.3 / 10.0. The game's fall toward the bottom edge is now the viewer's.
- The line's peak at x 362: game (117, 247, 26), without (124, 254, 25), with (126, 255, 25); at
  x 895 (122, 255, 25), (130, 255, 25), (125, 255, 23). The halo above the line (green excess
  over the 20-30 px baseline, median of 28-30 post columns, 1 / 2 / 3 / 5 / 9 px): game 47.4 /
  14.7 / 11.2 / 5.0 / -0.7, without 48.4 / 27.0 / 21.3 / 14.4 / 8.0, with 45.5 / 23.0 / 18.0 /
  12.6 / 6.8: the vignette takes a little off, still 1.6 to 2.5 times the game's at 2 to 5 px
  (part 43's strength and the line's thickness, not this pass).
- Mean |d| over the game's viewport (x 80-1199, y 51-668): open 614 29.7 -> 25.2 (the room is
  in another view, not this pass's); closed Pod at 600 against `d156-rpcs3-pod-default.png`
  19.0 -> 5.8, over a 4 x 4 grid of the viewport 16.0 -> 2.4 (mean signed (-2.2, -2.8, -2.4)).
- Other levels with the flag: the planets level at 600 (`--pod-camera 1`, the scripts) and
  Da Vinci's rookie test (0001e786) draw; centre x1.30, corners x0.41. Da Vinci, already bright,
  clips: 13.9 % of its pixels reach 255 in a channel (0 % without). No game frame of it was
  compared.

### Left out

- The flag stays off by default. Both Pod frames support it (above), but the constants are
  one frame's of one level, and no non-Pod game frame was compared (Da Vinci's clipping).
- The pass's `lerp(c, scene, scene.a)` (alpha 0 in the replay's copy, part 43), its FXAA-like
  edge offset, the volumetric term and the depth of field.
- The glow's strength and the line's thickness (the halo above).

### Verification

`lbp-bevy-viewer`, no game data: `d294_final_pass_constants`,
`d294_vignette_centre_corner_and_floor` (a CPU mirror: 1 at the centre and at r 0.2, about 0.27
in the corner, 0.2 from r 1.2 out and never below it, the notes' 0.83 / 0.47 / 0.44 in the game's
viewport), `d294_final_pass_scale_and_glow` (COL0 at the centre gives the bytes (114, 250, 18);
white at the floor 0.26, not saturated first; saturated at the end) and
`d294_final_pass_shader_validates` (the consts, the smoothstep and the two entries in the text;
naga validates both target kinds); `d292_glow_shader_validates` checks that part 43's add still
has no scale, `d292_extracted_only_while_active` the marker with the final pass on. fmt,
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` and `cargo check --workspace`
pass; the viewer's tests pass (117) but the one that fails on main
(`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
