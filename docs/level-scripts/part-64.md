# Level scripts, part 64: the depth of field's ring against the game's own frame (2026-10-15, D313)

Viewer, and a pass-by-pass replay of the game's depth of field; no runtime change. Part 63 left
the room sharper than the game's (0.57 against 0.37). This part runs the viewer's CPU mirror of
the passes on the game's own input buffers and compares each pass with the game's output.

### What was found

- MEASURED (`local-analysis/d313-range/`): the range is the same in both. The game's depth range,
  fitted from the clip positions of 8348's and 8980's draws, gives near 10 and far 100,000
  (within 0.2 %), and the gate's range equals the viewer's within 0.02 %. 8348 is viewer tick
  611 and 8980 is tick 616.
- FOUND (RenderDoc 1.46, 8980, `local-analysis/d313-blur/`; each buffer read with
  `GetTextureData` in its stored row order): the main target is 1280 x 720 R8G8B8A8_UNORM, the
  depth 1280 x 720 D24S8 (depth in the top 24 bits), and every half-size buffer 640 x 360
  R8G8B8A8_UNORM. The viewer's stills are 1280 x 720. The stored first row is the top of the
  picture. RenderDoc's saved images are the same rows bottom first (MEASURED: equal to the
  flipped buffer), so they are upside down.
- FOUND (fc of the ring programs: DoF 1's 3469 in 8348 and 8980, DoF 2's 3537 / 3475 in 8980 and
  14810 / 14815 in 14790): every ring holds the same five directions, (0, 1), (+-0.951, 0.309)
  and (+-0.588, -0.809). Only the step and the offsets differ between them.
- MEASURED (the Python port, `port.py`, matches the Rust mirror on random inputs (bit for bit
  before this part, within 0.3 of a level after); 8980 DoF 1, the game's own input to each
  pass; mean |d| in levels of 255):

  | pass | main (b1f8e81) | this part |
  | --- | --- | --- |
  | source (2706): 2x2 mean, gate from the block's top-left pixel | 0.11 | 0.11 |
  | ring 1 (2724) | 0.27 | 0.015 |
  | ring 2 (2741, twice the step) | 0.26 | 0.018 |
  | composite (2769, from the game's ring 2; clean pixels) | 0.46 | 0.46 |
  | all four, the game's input (clean blurred pixels) | 0.67 | 0.57 |
  | room gradient, 80 x 80 at (638, 505); the game 1.09 | 1.08 | 1.09 |

- MEASURED (all eight ring passes dumped: DoF 1 2797 / 2814 in 8348 and 2724 / 2741 in 8980,
  DoF 2 3045 / 3063 in 8980 and 3002 / 3020 in 14790): with +y down the picture the mirror
  matches within 0.003 to 0.022 of a level. With +y up it misses by 0.046 to 0.27, in 14790's
  DoF 2 too (0.020 against 0.054). Part 49's "+y up" was the same buffers read bottom row
  first (INFERRED from the saved images above). It is one direction for every pass, not one
  per pass.
- MEASURED (8980 DoF 2): its offset texture (unit 3) is not mid-grey here. 21 % of its texels
  differ, in a rectangle in the middle of the picture (x 195 to 460, y 125 to 320 of 640 x 360).
  With part 63's offsets (`(t - 0.498) x 0.04`, x +-1.25 / +-0.5 / 0 on the first ring and
  +-2.5 / +-1 / 0 on the second) the first ring fits within 0.014. Without them it misses by
  0.28. In 14790 it is mid-grey everywhere. The second ring's alpha adds
  `saturate(4 |offset| - 0.2)` to the radius (the alpha is exact in 14790; in 8980 it is 6.5
  off, not resolved).
- FOUND (replay artefact): the replayed 2769 output has flat (164, 104, 58) tiles over 24.5 %
  of the blurred pixels (178,563). The composite's own input does not hold them (it equals ring
  2's output), and lane B's screen at that frame has none. 14790's replayed source shows such
  tiles as well. The composite was compared on the other pixels.
- INFERRED: on the game's own input the viewer's passes already give the game's room gradient
  (1.08 against 1.09; main's doubled second ring was part 63). The stills' remaining gap (0.54
  to 0.57 against 0.37) is upstream of the depth of field: the viewer's scene going in, or the
  game's post chain after it. The tap direction does not change it.

### What the viewer does

`lbp-bevy-viewer`, `dof_post.rs`: the rings' taps take +y down the picture. The shader's
`dirs[k] * DOF_TAP_STEP * (r * scale)` drops the y flip, and the CPU mirror reads `v + dy`.
The CPU ring is now its own function (`dof_ring_cpu`, with `bilinear_cpu`), which
`dof_cpu_form` calls. No other change.

### Measured

Release build, hidden window, level lighting (`local-analysis/d313/shot.ps1`, prefix d313;
`measure3.py`). Main is `target-d312` (b1f8e81: its 613 still equals `d312-v2-tick613` byte for
byte).

- Changed against main: 44,041 pixels at 611, 128,041 at 613, 192,601 at 614, 267,693 at 615 and
  308,468 at 616. Of those, 108 / 198 / 173 / 170 / 261 change by more than 8 (at most 50).
- MEASURED, mean gradient, game / main / this build: the open room 0.37 / 0.57 / 0.57 at 615
  and 0.37 / 0.54 / 0.54 at 616 (d0600); 0.21 / 1.01 / 1.01 at 613 (d0550); 0.65 / 2.11 / 2.11
  at 611 (d0500, ~612.3). The room behind the window 0.68 / 1.05 / 1.04 (`open-t1`, 614); the
  left glass 1.50 / 1.58 / 1.58. Viewport mean |d| is unchanged to 0.01 at every tick.
  [Part 66: the sweep frames' ticks here are the Earth-radius clock's; on the dissolve's clock
  `d0550` is about 615.5 and `d0600` about 617.5, so these 613 / 615 / 616 pairs compare
  mismatched moments.]


### The room's sharpness, stage by stage

Capture 8980 against the viewer at tick 616 (the same focus range), each game stage read in a
fresh replay (a replay is clean only at the first event it is set to; later jumps leave the flat
(164, 104, 58) tiles of above), each viewer stage isolated with its `--no-*` flags, the viewer's
box registered by the warp and a cross-correlation (0.93 to 0.97). Gradient over the open room
(scene x 47-207, y 442-559), game / viewer, all MEASURED (`local-analysis/d314-chain/`):

- Scene before DoF 1: 1.008 / 1.136, mean colour (151, 135, 110) / (159, 142, 114).
- After DoF 1: 0.315 / 0.368. The translucent loop, DoF 2 and the anti-aliasing job leave the
  box unchanged on both sides.
- After the final pass, in presented pixels: 0.482 / 0.544 (x 1.53 / x 1.48).

So the gap is in the scene going in: the viewer's open room is about 13 % sharper (21 % over the
mural), 4.5 % brighter at every percentile and has about 10 % more contrast; the extra detail is
largest at 1 px (+13 %) and smaller at 4 px (+9 %), as a sharper mip would give (INFERRED). On the
desk box (80 x 80 at (638, 505)) the viewer ends slightly softer than the game (1.48 / 1.54).
The 0.37 of the sweep frame `d0600` is softer than the capture's 0.48 at the same range, so most
of the 0.37 / 0.57 gap above is the two references' timing (INFERRED).

### Verification

`lbp-bevy-viewer`, no game data:

- `d313_ring_taps_point_down_the_picture`: on a 640 x 360 buffer at radius 1, one bright texel
  reaches the centres above it at (+-2, -1) with 11/255 and those below with nothing.
- `d313_shader_taps_without_a_y_flip`.
- `d298_dof_shader_validates`, updated: naga validates every entry.

fmt and `cargo clippy --all-targets -- -D warnings` pass. The viewer's tests pass (151), except
the one that fails on main (`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
