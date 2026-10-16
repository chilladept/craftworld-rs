# Level scripts, part 63: the depth of field in two passes (2026-10-15, D312)

Viewer, and a read of the two passes' constants and depth tests in the replays; no runtime
change. Part 62 found the game running its depth of field twice a frame, around its translucent
draws; the viewer had one pass in the post chain. This part copies the game's order.

### What the game does

- FOUND (part 62, frames 8348, 8980 and 5247): DoF 1 runs between the opaque loop and the
  translucent loop, with no stencil: the half-size source, two rings and one full-screen
  composite into the main target, no depth write. The translucent surfaces are drawn after it;
  the glass is the last of them, LessEqual with depth write, and every translucent draw sets
  the stencil bit 0x80. DoF 2 is part 57's post-chain form: the EQUAL 0x80 composite with the
  depth test off, then the NOTEQUAL one against the plane.
- MEASURED (RenderDoc replays of 8348, 8980, 5247 and 14790, `local-analysis/d312-planes/`): DoF
  1 and DoF 2 take the same gate constants (fc1 of their source programs: 8348 (2449.944,
  -2448.715), 8980 (1041.287, -1039.970)) and the same plane (its window depth equals the gate's
  `d0` to 7 digits, e.g. 0.9998557 in 8348). One range serves both.
- MEASURED (the same replays; RenderDoc's depth-test overlay on 3180 / 3107 / 3064 / 3172, clip
  conventions checked): DoF 2's NOTEQUAL composite (`glDepthFunc` GL_GREATER) passes exactly
  the pixels nearer than the plane; every sample beyond fails. DoF 1's composite (LessEqual)
  passes those beyond. Part 57's "beyond" reading came from diffing separate replays of the
  same frame, which do not reproduce (two forced replays of 3180 differ on 170k pixels); it was
  INFERRED, and is corrected here. So the room beyond the plane is blurred once, by DoF 1;
  DoF 2 changes the marked pixels (no plane test) and the unmarked pixels nearer than the plane,
  whose own radius is 0, so only their outline next to a blurred background through the
  bilinear read-back.
- MEASURED (fc1 of the ring programs): DoF 1's first ring takes taps at (0.0025005, 0.0044441),
  its second at twice that; its second ring keeps the gate, and its composite blends by
  `8 a - 7` of the gate read back. DoF 2's rings both take the step, and add an offset from the
  texture on unit 3, `(t - 0.498) x 0.04`, scaled by plus or minus 1.25 on the first ring and
  2.5 on the second (INFERRED part 49's offset buffer; its content here is not known). The tap threshold is
  -0.01 in all four rings.
- FOUND (part 62, 8348): on the glass DoF 2's composite leaves the colour as it was (its own
  depth gives a radius of 0).

### What the viewer does

`lbp-bevy-viewer`:

- `dof_post.rs`: DoF 1 is a new pass in `Core3dSystems::MainPass`, after `main_opaque_pass_3d`
  and before `main_transparent_pass_3d`, preceded there by a copy of the depth buffer as the
  opaque pass left it: the source, a ring at the step, a ring at twice the step
  (`DOF_1_SECOND_RING_SCALE`) keeping the gate, and a composite beyond the plane by `8 a - 7`.
  DoF 2 is the existing post-chain pass, both rings at the step, the second writing the radius,
  with a new composite (`dof_composite_marked`): a pixel whose final depth differs from the copy
  (a translucent draw wrote it, the game's bit 0x80) is composited without the plane test and
  left as it is where the read-back radius is 0; any other pixel only where it is nearer than
  the plane. Both take the one range. DoF 2's offset texture is not copied. There is no other
  route: whenever the depth of field runs, both passes run.
- Where DoF 1 runs, INFERRED as the closest match to what DoF 2 sees: the main camera has no HDR
  and `Tonemapping::None`, so after the opaque pass its main texture already holds the colours
  the post chain reads (the same 8-bit target; only the transparent pass lies between). DoF 1
  reads it through `post_process_write` and mixes in the encoded space as DoF 2 does, rather
  than the game's hardware SrcAlpha blend (on the viewer's sRGB target that would mix in
  linear light). Under MSAA (`--no-mlaa`, or a level outside the Pod under `--depth-of-field`)
  its composite is written into every sample of the main pass's multisampled texture and
  resolved, so the transparent pass, which loads the samples, draws over the blurred scene.
- `dual.rs`: the glass (D286's bucket) writes its depth while the depth of field runs
  (`glass_writes_depth`); otherwise it keeps Bevy's transparent default. Only the glass writes
  depth among the viewer's translucent draws, so only its pixels are marked (the game marks
  every translucent draw that passes the depth test; the others are left out, below).
- `main.rs`: the depth buffer is also a copy source under the depth of field.
- Everything stays behind `dof_route` / `--depth-of-field`: without it no pass runs, the depth
  buffer's usage and the glass's pipeline are as before.

### Measured

Release builds, hidden window, level lighting (`local-analysis/d312/shot.ps1`, prefix d312;
`measure.py`, `measure2.py`; viewer stills warped s 0.877, dx +2 as part 57). Main (`target-d310`,
the same depth of field), the first two-pass build (e56f291: the second rings at the step, DoF
2's unmarked pixels beyond the plane) and this one.

- Byte for byte as main: the closed Pod at 600 and Da Vinci's rookie test (0001e786), with and
  without the depth of field; the change-over at 614 under `--no-depth-of-field`; in use at
  600. So the closed Pod (nothing beyond the start, part 49) is unchanged.
- The change-over against main: 269,288 pixels change at 613, 412,875 at 614, 532,574 at 615
  (7,850 / 11,100 / 14,362 by more than 8).
- MEASURED, mean gradient, game / main / e56f291 / this build:
  - The left glass panel against `open-t1` (tick 614): 1.50 / 1.21 / 1.60 / 1.58; at 613 1.39 /
    1.43 / 1.42, at 615 0.88 / 1.41 / 1.39. The circuit pattern stays sharp over the room as in
    the game.
  - The open room against the sweep's d0600 (~615.6): 0.37 / 0.64 / 0.57 / 0.57 at tick 615,
    0.62 / 0.56 / 0.54 at 616; against d0550 (~613.3) 0.21 / 0.98 / 0.97 / 1.01. The room behind
    the window against `open-t1`: 0.68 / 1.17 / 1.05 / 1.05.
  - The room through the band (part 62's 0.34 / 1.08): 1.78 / 1.76 at tick 615, 0.91 / 0.86 /
    0.85 at 616. At 615 the viewer's glass still covers that strip (its dissolve trails the
    sweep frame, about tick 615.6, where the glass there is gone); its circuit pattern, which
    main blurred, is now drawn sharp, so the gradient rises. At 613 through the glass 1.94 /
    2.40 / 2.40 / 2.40.
    [Part 66: on the dissolve's clock `d0600` is about tick 617.5, not 615.6; the viewer's 615
    was compared with a later moment, and its dissolve is not late.]
- Viewport mean |d|, main / e56f291 / this build: against `open-t1` at 614 11.72 / 11.55 /
  11.52; against d0600 at 615 21.90 / 21.70 / 21.63, at 616 18.66 / 18.43 / 18.35; against
  d0550 at 613 21.06 / 20.95 / 20.90.
- Under `--no-mlaa` (MSAA) at 614: left glass 1.21 -> 1.57, room behind the window 1.21 ->
  1.10, viewport |d| 11.72 -> 11.54.
- So the two passes move the picture towards the game, but by little; the room blurred once
  at twice the second ring's step reads as the room blurred twice did. The room stays sharper
  than the game's (0.57 against 0.37); not resolved here (part 49 noted the zoom's progress and
  camera differ from the game's frames).

### Left out

- DoF 2's offset from texture unit 3 (its content is not known; part 49's offset buffer, if
  it is that one, was mid-grey in 5247 and 14790).
- The other translucent draws' marks: the viewer's planets are drawn opaque, and its other
  blended draws write no depth, so they are not marked (the game marks them; their outline
  next to a blurred background would take some blur).
- The downsample after DoF 1; the game's SrcAlpha blend in DoF 1 (the viewer mixes in the
  encoded space).
- The glass's dissolve against the sweep at 615 (the band's strip above).

### Verification

`lbp-bevy-viewer`, no game data: `d312_glass_sharp_over_a_blurred_room` (the CPU mirror of the
two passes, `dof_two_pass_cpu`: a near patterned panel over a far room keeps its pattern exactly
over the DoF 1 room, the room around it is blurred; the one post-chain pass blurs the pattern
with the room; DoF 2 alone keeps the room under it sharp), `d312_nothing_in_range_unchanged`
(and DoF 2 leaves the room beyond the plane as DoF 1 left it), `d312_dof1_wide_ring_and_dof2_nearer`
(the doubled ring blurs more; DoF 2 changes nothing beyond the plane and, nearer, only the
outline), `d312_composite_shaders` (naga validates every entry with and without a multisampled
depth buffer through `d298_dof_shader_validates`) and `d312_glass_writes_depth_with_the_dof`.
fmt and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass; the viewer's tests
pass (149) but the one that fails on main (`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
