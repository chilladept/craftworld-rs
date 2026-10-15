# Current state (overwrite each milestone) — after D303

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D303 the alpha of glowing materials (2026-10-15)
- Viewer only (`docs/level-scripts.md` part 54). FOUND (RenderDoc frame 5247): the game's lit
  program with a constant glow g (port 4, added before the x4 and the square root) writes alpha
  tintcol.w / (4|g|^2 + 1): the lamp bulb's g = (0.66, 0.56, 0) gives 64, the stars' g = 1 gives
  20 (part 51's 319 pixels). FOUND (the Moon capture): the glow chain runs in a level too.
- Viewer: opaque glowing surfaces on the level-lighting route write that alpha; the glow passes
  run with the final pass on every frame (`glow_post::glow_runs`), not only during a dissolve.
- MEASURED (closed Pod at 600 against the game's frame): bulb rings 0-4 / 8-12 / 12-16 px mean
  |d| 31.8 / 24.4 / 18.5 -> 13.1 / 3.7 / 8.4; star pixels 14.21 -> 12.95. Da Vinci: 127
  glowing-point pixels brighter; Bravery unchanged. Tests `d303_*` (three).
- Open: the fog's mix of the alpha in the thing-colour glow listing; glow maps' alpha
  (INFERRED); blended glowing draws; the spawn door's white ring in Bravery (not drawn).

## Before: D300 the scene's alpha at the final pass (2026-10-14)
- Viewer only (`docs/level-scripts.md` part 51). FOUND (RenderDoc frames 5247 and 14790, hidden
  desktop): after the depth of field and the glow's source the frame goes to main memory
  (0x33200000) for an SPU anti-aliasing job and comes back with alpha 0 on every pixel; the final
  pass's `lerp(c, scene, a)` then outputs `c` (x1.2998, glow, vignette) on the whole 3-D picture.
  Before the round trip the shell, glass, DualShock, room, planets and clear are all alpha 255.
  The Puter menu (14790) is drawn after it with additive alpha and keeps its brightness. FOUND
  (EBOOT): 0x31dfd4 runs the round trip only while byte 0xdb8208 is set (the job is the "EDGE
  MLAA" task); 0x3205a0 and a paint UI page clear it. Levels' game frames show no pass, so the
  job is off there (what keeps it off INFERRED: the 0x3205a0 / 0x40d184 render path).
- The final pass is on by default on the level-lighting route and mixes back by alpha: 0 for the
  Pod picture (`--pod-camera`), the scene's own elsewhere. `--no-final-pass` turns it off;
  `--final-pass` is still accepted. The glow's source still reads the scene's alpha (D292).
- MEASURED (`shots/d300-*`): closed Pod at 600 and the change-over at 614 byte-equal to D294's
  `--final-pass` stills (closed vs the game 19.0 -> 5.8 viewport, grid 15.9 -> 2.5; 614 left glass
  31.9 -> 0.4, DS body 4.0 -> 0.1). Da Vinci byte-equal to `--no-final-pass`; Bravery equal outside
  Sackboy's box (his idle, as two runs without the pass).
- Tests `d300_*` (five); fmt, the viewer's clippy and `cargo check --workspace` pass; viewer tests
  129 pass, `sackboy_play::tests::input_script_steps` fails as on main.
- Next: what keeps the job off in a level's play; the job's edge filtering in the viewer; the
  alpha the viewer writes on glow materials in a level.

## Before: D301 the pod computer's step against the planets view (2026-10-14)
- Script VM (`docs/level-scripts.md` part 52). FOUND (EBOOT 01.33): the frame function 0x18558
  makes the one call of the pod computer's slot 0x14 (0x40663c) after the main level's update;
  it runs `PerFrameUpdate__b`, then (0x4064bc -> 0x43d900 -> 0x4381d8) the planets level's
  update, the planets' `Update__rb` and `UpdatePlanetsView` (0x43ac68). `tick_level_in_session`
  follows that order. MEASURED: the menu view blend steps on the tick of its push (0.05 on 601,
  1.0 on 620, with the change-over; was 0.0 on 601, 1.0 on 621); nothing else changes
  (`d301-session.json`). One of part 50's two ticks is explained; the other may be where
  message 0x30's handler (it sets "in use") runs in the frame.

## Before: D298 the game's depth of field (2026-10-14)
- Viewer only (`docs/level-scripts.md` part 49). FOUND (RenderDoc frames 5247 and 14790, hidden
  desktop): before the glow, a half-size source (scene 2x2 mean, depth gate in alpha), two
  rings of five pentagon taps at radius `saturate(8 g - 7)` x 1.6 half-size texels (a tap
  counts when its gate is within 0.01 of the centre's or above), and a bilinear composite by
  that radius beyond the range's start (scene alpha kept). FOUND (EBOOT 0x2e81ac): the
  `SetDOFAndFog` distances become depth-buffer values; the two frames' gate constants fit
  (80,000, 100,000) and (12,000, 16,000) with the Pod camera's planes within 0.13 %: the radius
  runs linearly in depth from the first value to the second.
- New `dof_post.rs` and `--depth-of-field` (off by default, level-lighting route only), fed each
  frame from the scripts' range (`level_play::follow_dof`). Consts `DOF_RAMP`, `DOF_TAP_STEP`,
  `DOF_TAP_DIRS`, `DOF_SAME_LAYER`, `DOF_RING_PASSES`.
- MEASURED (`shots/d298-*`, with `--final-pass`): without the flag every still is byte-equal to
  the build before. At 614 (range 31,159-39,667) the room's gradient against `open-t1` (1.20):
  3.76 -> 1.84; upper left 1.30 -> 1.18 (0.67), upper right 1.45 -> 1.35 (0.65), upper centre
  1.54 -> 0.92 (0.76); DualShock and posts unchanged. Closed Pod at 600 and Da Vinci unchanged
  (nothing beyond 80,000; no range), as the game's 5247.
- Tests `d298_*` (six); fmt, the viewer's clippy and `cargo check --workspace` pass; viewer tests
  124 pass, `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the flag on by default after a non-Pod game frame with depth of field; the stencil-marked
  composite; the change-over's room view against `open-t1` (`--final-pass` by default: D300).

## Before: D299 where the game's final pass applies (2026-10-14)
- Docs only (`docs/level-scripts.md` part 50). FOUND (RenderDoc, the blank Moon level): the
  final pass has frame 5247's constants (x1.2998, the vignette, glow 0.7) but ends by mixing
  back to the scene by the scene's alpha, which is 1 outside the Pod, so it returns the scene.
  The game's Moon and Bravery Test frames match the viewer without `--final-pass`; it stays off
  by default. MEASURED: with D295 the room behind the window registers against `open-t1` at
  scale 0.96 at tick 614 (was 1.62), region |d| 10.6 -> 5.0 (1.2 at 616). The game's zoom runs
  about two ticks ahead of the VM's (one is part 46's step order).

## Before: D295-D297 the planets menu zoom, the tent's line and the glow (2026-10-14)
- D297 (`docs/level-scripts.md` part 48), viewer comment and test only. FOUND (RenderDoc frames
  5247 and 14790): the four post passes and their constants are the same in both frames, so the
  glow strength 0.7 holds; the blur rebuilds the game's within 0.25 level. MEASURED (tick 614,
  `--final-pass`, after D296): halo above the line 46.1 / 10.7 / 7.7 / 4.1 / -3.1 at
  1/2/3/5/9 px against `open-t1`'s 47.4 / 14.7 / 11.2 / 5.0 / -0.7; on the left post (flat
  background) within 0.7 level at 2, 3 and 9 px. The earlier excess was the doubled line and the
  missing final pass. Test `d297_line_halo_matches_t1_at_0_7`.
- D296 (part 47), viewer: the tent's back faces are culled while the dissolve is on (FOUND: every
  captured Pod draw culls back faces). The far wall's second line no longer shows through the
  cut; the line is 1.99 game px against the game's 1.77 (was 3.56).
- D295 (part 46), runtime: the planets level's pod computer runs `PerFrameUpdate__b` each tick
  (FOUND EBOOT 0x40663c); in use it pushes the main menu, whose view starts the planets world's
  0.05-a-tick blend from view 0 (z -15000) to view 1 (z -5000). MEASURED: `SetView` on tick 601,
  blend 0.718 on 614, done on 621; every D293 path event unchanged. The session report's planet,
  video and per-frame counts were always 0 before (fixed).
- Next: make `--final-pass` the default after a non-Pod game frame (Da Vinci clips 13.9% with
  it); the viewer's change-over stills with the planets view zoomed (D295) against `open-t1`;
  depth of field (FOUND in 5247's chain); the halo's 5 px gap and the right post once the room
  is the game's view.

## Before: D294 the final pass's brightness and vignette as the game's (2026-10-14)
- "Pod picture" PR 21, viewer only (`docs/level-scripts.md` part 45). FOUND (frame 5247, final
  pass PS 14857): `out = (scene x 1.2998 + glow) x (0.2 + 0.8 smoothstep(0, 1, saturate(1.2 - r)))`,
  `r = |((u - 0.5) x 1.0996, (v - 0.5) x 1.7002)|`, encoded values.
- Viewer (`glow_post.rs`): consts `FINAL_SCALE`, `VIGNETTE_AXES`, `VIGNETTE_START`,
  `VIGNETTE_FLOOR`; the add pass's `final_composite` (with the glow) and `final_only` (no
  dissolve, glow 0). New `--final-pass`, off by default, level-lighting route only (ignored
  with a note under `--preview-lighting` / `--no-lbp-lighting`); with it the pass runs every frame.
- MEASURED (`local-analysis/rscript/stage6/shots/d294-*`, hidden window): without the flag the 614
  still equals main's byte for byte. In use at 600, held at 614, against `open-t1`, mean |d|:
  DS body 4.0 -> 0.1, left glass 30.3 -> 2.0, right glass 20.5 -> 7.0, wall 9.3 -> 1.1, post cards
  3.8 -> 2.9 / 2.4; viewport 29.7 -> 25.2. Closed Pod at 600 against `d156-rpcs3-pod-default`:
  19.0 -> 5.8. Halo at 2/3/5 px 27.0/21.3/14.4 -> 23.0/18.0/12.6 (game 14.7/11.2/5.0). Da Vinci
  clips with it (13.9 % of pixels at 255; no game frame).
- Tests: `d294_*` (four); fmt, the viewer's clippy and `cargo check --workspace` pass; viewer
  tests 117 pass, `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the flag on by default after a non-Pod game frame; the glow's strength and the line's
  thickness against t1.

## Before: D293 the 2-D and scalar smoothing steps as the game's (2026-10-14)
- "Pod picture" PR 20, script VM only (`docs/level-scripts.md` part 44). FOUND (EBOOT 01.33):
  `native_Smooth2D` 0x60195c -> 0x5ee7e4 -> core 0x38f2fc, the 2-D twin of 0x3905b8 (distance and
  projection on x, y; 1-D step; offset over distance x new speed; zero under 1e-4 or the new
  speed; velocity only, w cleared). `native_Smooth__fffff` 0x6019f8 -> 0x5ee844 -> 0x38ef7c with
  (pos, vel, target, acc, top speed), returning (new pos, new vel, 0, 0); the step lands on the
  target where its plan fits. It divides by the acceleration (zero gives non-finite values).
- VM: `smooth_nd` serves `Smooth3D` (unchanged) and `Smooth2D`; `smooth_1d` serves `Smooth`. The
  older INFERRED rule is gone.
- MEASURED (`level_scripts`, both Pod levels, in use at 600,
  `local-analysis/rscript/stage6/d293-smooth-session.json`): the same report byte for byte as
  D289's. `Smooth` is called 699 times, all (0, 0, 0, 10, 100) (the zoom stays 0); `Smooth2D`
  (`PlanetThing.Update__rb`) is not reached.
- Tests: `d293_*` (two); the part 6 scalar case now lands at rest. fmt, the VM's clippy and tests
  and `cargo check --workspace` pass.
- Next: `PlanetThing.Update__rb` in the Pod session; the pod computer state's extra zoom; the
  viewer's change-over stills against `open-t1`.

## Before: D292 the glow on the dissolve's line (2026-10-14)
- "Pod picture" PR 19, viewer only (`docs/level-scripts.md` part 43). FOUND (frame 5247): the
  tent writes alpha = t; post pass 3256 (half size) `0.7 x (1 - a) x rgb`, 3281/3298 a 9-tap
  blur at a quarter size (0.3, 0.225, 0.15, 0.105, 0.06 at 0, +-2, +-6, +-10, +-14 px; 1.38 per
  pass), 3454 adds it (and scales the frame by 1.2998, not copied).
- Viewer: opaque materials under the dissolve write alpha = the kept share (`dual.rs`); new
  `glow_post.rs`, four full-screen passes on the main camera in `Core3dSystems::PostProcess`
  (before egui/UI), in encoded values on the sRGB main texture, only while a dissolve is on;
  `--no-edge-glow` turns it off.
- MEASURED (`local-analysis/rscript/stage6/shots/d292-new-*` against `d291-new-*`): Da Vinci
  and the planets at 600 equal; in use at 600, held at 615/617, a halo on both sides of the
  line, excess above it +26..30 / +21..28 / +19..21 / +12..16 / +6..9 at 1/2/3/5/9 px, about
  1.5-2 x the game's `open-t1` (19 / 12 / 9 / 5 / 0 over its baseline); 0.7 (FOUND) kept.
- Tests: `d292_*` (six) with a CPU mirror of the chain; the shader text test checks the alpha
  write. fmt, the viewer's clippy and `cargo check --workspace` pass; viewer tests 113 pass,
  `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the glow's strength and the x1.2998 against t1's own settings; the band's glow against
  a game frame.

## Before: D291 the dissolve's band mixed in the game's output (2026-10-14)
- "Pod picture" PR 18, viewer only (`docs/level-scripts.md` part 42). FOUND (`pod_card_edge`,
  frame 5247): `out = mix((COL0, 0), (lit_encoded, texalpha), t)`, then fog; nothing adds light
  on the line. Fog is 0 on the tent in both captures (vc[16]). The glow is a post pass reading
  the scene's alpha (0.7 x (1 - a) x rgb, two 9-tap blurs, added in the final pass, which also
  scales the frame by 1.2998).
- Viewer: the dissolve mixes the encoded pair (`dissolve_mix`), then fogs on the level route.
- MEASURED (`local-analysis/rscript/stage6/shots/d291-base-*` / `d291-new-*`, in use at 600,
  held at 615/617/618): only the band changes (up to 51 levels darker, green most); the line's
  pixels at (88, 192, 14) are unchanged.
- Tests: `d291_*` (three) and the shader text test. fmt passes; viewer tests 107 pass,
  `sackboy_play::tests::input_script_steps` fails as on main.
- Next: D292 the glow post pass (alpha = keep while dissolving, a full-screen node).

## Before: D290 the glass's blend per channel (2026-10-14)
- "Pod picture" PR 17, viewer only (`docs/level-scripts.md` part 41). MEASURED (closed Pod, tick
  600, level lighting, the room from a probe with the glass clear): part 38's left-panel "twice
  the game's share over a darker room" was the planets-only still's room (other pixels behind
  that panel); the real room (142.7, 142.2, 108.2) and share (15.0, 19.7, 22.1) are within error
  of the game's (145.1, 144.5, 110.7), (17.3, 21.9, 22.9). The game's left share is about twice
  its right by design (the sun lights the left panel, N.L 0.52). Part 38's lines corrected.
- Viewer, glass GUID 0x1d14c only: the secant of the encoded blend per channel (t_c, S_c),
  written as two blend sources (S, 1) and (1 - t, 1) with the colour blend One + OneMinusSrc1;
  a pipeline key marks the glass, which alone takes a copy of the shader with `enable
  dual_source_blending;`. Bevy 0.19.1 / wgpu 29 already asks for every adapter feature; without
  `DUAL_SOURCE_BLENDING` the glass keeps the one-T blend (D287).
- MEASURED (`local-analysis/rscript/stage6/shots/d290-*`, D289 build against this one): planets
  at 600 with the pod camera and Da Vinci (0001e786) equal; the closed Pod changes 377,445
  pixels, all inside the glass. Panel means (left; right): before (129.2, 133.5, 108.6); (60.5,
  82.8, 93.6), now (129.6, 133.4, 107.8); (64.0, 82.3, 89.4), game (133.4, 137.6, 111.5); (65.9,
  83.5, 90.3).
- Tests: `d290_*` (seven); the D287 tests stay on the fallback; the shader validation resolves
  the `#ifdef`s.
  fmt, CI clippy, CI tests, `cargo check` of both viewers and the viewer's clippy pass; viewer
  tests 104 pass, `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the left panel's remaining (-4, -4, -4) (room 0.8 x -2.4 plus the bump, within error);
  `Smooth2D` and the scalar `Smooth` from the EBOOT; the change-over stills against `open-t1`.

## Before: D289 the script camera's smoothing step as the game's (2026-10-14)
- "Pod picture" PR 16, script VM only (`docs/level-scripts.md` part 40). FOUND (EBOOT 01.33,
  `native_Smooth3D` 0x5fed34 -> core 0x3905b8 -> 1-D step 0x38ef7c): the velocity is projected on
  the direction to the target, clamped to the top speed (arg 5), and one fixed step of 1.0 of a
  time-optimal move runs (speed up by arg 4, brake to stop on the target); only the new velocity
  comes back, zero under 1e-4 or when closer than the new speed, so the camera stops short. The
  game's resting eye (0, -299.4804, 902.3901, RenderDoc f14790) is that stop-short.
- VM: `Math.Smooth3D__rrrff` uses that step. `Smooth2D` (core 0x38f2fc not read) and the scalar
  `Smooth` (wrapper's argument order not read) keep the older INFERRED rule.
- MEASURED (`level_scripts`, both Pod levels, in use at 600,
  `local-analysis/rscript/stage6/d289-camera-session.json`): the planets camera leaves on 601,
  last moves on 614 and is reported at rest on 615 at (0, -299.4803, 902.3904), no overshoot
  (before: 122 past on 612, rest on 626 at (0, -300, 900)). The dissolve is unchanged (0.75 on 615,
  1.0 on 620), so the camera is at its open eye while the tent stands, as in `open-t1`.
- Tests: `d289_*` (three). fmt, CI clippy, CI tests and `cargo check` of both viewers pass.
- Next: `Smooth2D` and the scalar `Smooth` from the EBOOT; the viewer's change-over stills again
  against `open-t1`; the left panel's larger glass share.

## Before: D288 the hanging cloths' art on every lighting route (2026-10-14)
- "Pod picture" PR 15, viewer only (`docs/level-scripts.md` part 39). FOUND: each hanging cloth
  of the planets room is two buckets of the backdrop mesh on the same triangles, a plain cloth
  (0x1f3ad, earlier) and the art (0x1f3b3 left, 0x1f3b4 right, later, slot 0). MEASURED: which
  one won the equal depth followed Bevy's draw order, not the lighting (the level alone lost the
  art under its own lighting too; `--bump-gloss` brought it back under the preview). The game
  shows the art.
- Viewer: every opaque bucket gets a constant depth bias equal to its index in the mesh (capped
  at 65), so a later material wins an equal depth; bucket 0 and one-material meshes unchanged,
  decals 8, glass 0.
- The preview's washed-out room is its own design (D121 Lambert preview, "not game lighting"):
  MEASURED mostly its Bevy distance fog (the level's fog colour as linear light); unchanged.
- MEASURED (`local-analysis/rscript/stage6/shots/d288-*`): planets at 600, pod camera, preview
  331,393 pixels and level lighting 290,610 (art and the easel's canvas now drawn); closed Pod
  with the preview 290,699, under the level lighting 6; Da Vinci rookie test (0001e786) equal
  with the preview, 28 pixels (at most 8) under its own lighting.
- Tests: `d288_a_later_bucket_on_the_same_triangles_is_drawn_nearer`. fmt, CI clippy, CI tests,
  `cargo check` of both viewers and the viewer's clippy pass; viewer tests 97 pass,
  `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the left panel's larger glass share; the camera's timing; the game's depth rule for
  coplanar primitives from a capture.

## Before: D287 the Pod's glass blended as the game does (2026-10-14)
- "Pod picture" PR 14, viewer only (`docs/level-scripts.md` part 38). MEASURED (the captured
  frame, before and after the glass draw): the target is `R8G8B8A8_UNORM` without sRGB, so the
  game blends its encoded output, out = s + 0.8 d; the glass's share s is (17, 22, 23) left and
  (7, 15, 21) right. The game's dark green-blue panels are the room behind them: the viewer's
  room under the level's own lighting is (69, 90, 88) behind the right panel against the game's
  (73, 86, 87); part 37's stills used `--preview-lighting` ((160, 163, 148)), which also draws
  the hanging cloths plain tan where they carry art (the left panel's "cloth against wall art").
- Viewer, glass GUID 0x1d14c only: s = alpha x the encoded lit colour + the glass specular
  (gloss 0.6 x 6 x light^2 x (pow(R.L) + rim / 2), after the alpha, level lighting only);
  `glass_blend` writes the secant of the encoded blend through destinations 0.35 and 0.75 as a
  premultiplied colour and one alpha. Mean error (1.7, 0.9, 1.2) / 255 over the viewer's room
  pixels (D286's linear blend: (10.2, 5.4, 2.7)).
- MEASURED (`local-analysis/rscript/stage6/shots/d287-*`): planets frames, `--pod-open`, 640 and
  the walk run equal D286; the glass frames change by at most 13 (51 at 615 under the level
  lighting). Panel means (left; right): game (133, 138, 111); (66, 84, 90); now under the level
  lighting (129, 133, 109); (60, 83, 94); with `--preview-lighting` (163, 151, 122); (133, 141,
  132), D286 (170, 156, 125); (143, 147, 135).
- Tests: `d287_*` (four) and two D286 tests updated. fmt, CI clippy, CI tests, `cargo check` of
  both viewers and the viewer's clippy pass; viewer tests 96 pass,
  `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the preview lighting's washed-out room and the cloths' art; the left panel's larger
  glass share; the camera's timing.

## Before: D286 the Pod's side walls as clear glass (2026-10-14)
- "Pod picture" PR 13, viewer only (`docs/level-scripts.md` part 37), from the owner's report
  of a strange texture on the Pod's side walls. P2j's default wall art (0x1f3b9 in slot 0 of
  `window_glassy` 0x1d14c, drawn opaque) is gone; `--runtime-texture` still adds it on request,
  `--no-pod-wall` does nothing.
- MEASURED (the captured glass draw): blend One, InvSrcAlpha; only the normal map and the
  transition mask bound; FOUND: alpha 0.2 x the dissolve's kept share, colour premultiplied,
  the constant 0.11765 in place of the green in the edge; TC0 is the mesh's UV0.
- Viewer: material GUID 0x1d14c only (alpha mode 4 not used as the rule) draws
  `AlphaMode::Premultiplied`, alpha 0.2 x kept share, lit grey with the bump and gloss times
  that alpha; raw UV0 recovered from the bump's UV0 x 2 (`lbp-preview` `BumpMap::uv_transform`).
- MEASURED (`local-analysis/rscript/stage6/shots/d286-*`): the planets captures, `--pod-open`
  and 640 equal D285/D284; the closed, no-script, 610 and 615 frames show the room through the
  panels. Against `d156-rpcs3-pod-default.png`: the right panel's drawings and statue match;
  the left shows the cloth where the game shows wall art; the game's glass is darker and
  greener (right panel mean (63, 88, 92) against (145, 151, 140)).
- Tests: `p2j_d286_*`, `d286_*` (three) and the shader validation extended. fmt, CI clippy, CI
  tests (612), `cargo check` of both viewers and the viewer's clippy pass; viewer tests 92 pass,
  `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the glass's blend in the encoded output space and its specular scale; the left panel's
  view; the camera's timing.

## Before: D285 the player's default colour as the Pod's flat colour (2026-10-14)
- "Pod picture" PR 12, runtime and viewer (`docs/level-scripts.md` part 36). FOUND (01.33,
  evidence only): a new profile takes entry 0 of the script class `PlayerColour`'s palette (12
  entries; 0x1589a8 given index 0 by both callers); entry 0's primary is (150, 221, 60). The Pod
  script sets the tent's shape colour from `GetPlayerColour(avatar)` (part 21).
- Runtime: `lbp_script_vm::natives::DEFAULT_PLAYER_COLOUR = 0xff96dd3c` replaces the white
  placeholder as a player record's start colour; `level_scripts --player-colour` still overrides.
- Viewer: `pod_dissolve::uniforms` squares the stored rgb before the decode (MEASURED: the captured
  vc[4] is exactly the squares of (150, 221, 60) / 255, alpha not squared, and the shader takes no
  root of it), so the frame shows (88, 192, 14) as the game's does. The dissolve line prints the
  display bytes.
- MEASURED (`local-analysis/rscript/stage6/shots/d285-*`, base `d285-base-*` = d284 bytes): without
  scripts, closed at 600, the planets level, 619 and 640 equal main's build; 610 and 615 (both
  lightings) changed, the line and traces green. Edge pixels: viewer 615 median (88, 191, 14);
  game `open-t1` median (84, 158, 24), (93, 180, 24) at the same brightness, a brighter glow
  (121, 222, 36) the viewer does not draw.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed; `d285-play-all-*.json`): Things
  stopped by a fault 3 / 3 / 3; plain equal to D274's in every value; with the visit only pod.bin's
  shape colour now differs from the level file (one more level and Thing in `changed` and
  `shape_colour`).
- Tests: `d285_pod_flat_colour_is_the_square_of_the_stored_colour`, the start colour in
  `d273_pod_computer_natives` and the runtime's `d273_*` play test. fmt, CI clippy, CI tests (612),
  `cargo check` of both viewers and the viewer's clippy pass; viewer tests 89 pass,
  `sackboy_play::tests::input_script_steps` fails as on main.
- Next: the Pod's side walls show a texture where the game has clear glass (owner report; glass
  material 0x1d14c drawn with a placeholder); the camera's timing.

## Before: D284 the Pod's second mask, by the mesh's own UV (2026-10-14)
- "Pod picture" PR 11, viewer only (`docs/level-scripts.md` part 35). The card materials'
  `pod_mask.tex` (0x1fbc7) is sampled at the mesh's raw UV0 (FOUND: identity UV transform in
  the captured draws): where its red is over one half the line stands one band higher, and at
  value 1 the rule's last term adds a band where it is black: `edge = (above - 100 g)/400 + 1 -
  m6 + at_one (1 - m6)` (`pod_dissolve::edge`, the shader's arithmetic).
- MEASURED (throwaway probe, value for value against the mesh's channels): `pod_card` 0x2f6a's
  primary coordinates are UV0; `pod_card_edge` 0x2fda's primary are UV1 and its second
  sample's are UV0 (tent and platform); the glass is solid. Each bucket now records which input
  holds raw UV0 (`pod_dissolve::uv0_source`: `in.uv`, `in.uv_b` or none, the glass keeping the
  position tile) and its material GUID; the transition mask moves to UV0 x 40 on 0x2fda too.
- `thing_view::apply`: at alpha 0 the platform's 0x2fda entities keep the dissolve at value 1
  (`LevelThings::dissolve_edge`), 0x252f3 gets nothing; `follow_dissolve` gives `pod_mask` to
  the two card materials only. MEASURED: all 291 triangles of the platform's card edge have
  their UV0 on the white strip and patch, so at value 1 the rim is kept whole (lit), not cut.
  The raised parts of the tent are its floor, the block beside it and its rim (mesh height 204
  to 991 and 883 to 892).
- MEASURED (`local-analysis/rscript/stage6/shots/d284-*`, base `d284-base-*` = d283 bytes): the
  intro, the four without scripts, closed at 600, 640, 641, the planets captures and pod.bin alone
  equal D283's; in use at 619, 620 and 640 (both lightings) equal main's build. Changed: 610 and
  615 (the card edge's band pattern now at UV0). A check build painting `m6` = 1 shows none of the
  raised parts in view at 610 to 619 and the platform's rim in 146 pixels at 640. Also changed:
  the viewer's Sackboy in the two `--play-sackboy` closed stills (zipper and leg patches); bisected
  to the material type holding one more image handle (INFERRED: coplanar draw order), not the rule.
- Game frame `d156-rpcs3-pod-open-t1.png` (mid change-over, camera at the open eye): posts with a
  green top line and green traces. The viewer's 619 shows none (its camera is not at the open eye
  until 628); its 615 is closer (posts with a white line). INFERRED: the game's posts are plain
  tent near value 0.7, not raised parts. 640 against `open-t2`: unchanged, the camera's height.
- Tests: four viewer tests (`d284_*`) and the shader validation extended. fmt, CI clippy, CI tests
  (612), `cargo check` of both viewers and the viewer's clippy pass; viewer tests 88 pass,
  `sackboy_play::tests::input_script_steps` fails as on main. The runtime is untouched.
- Next: the player's green as the flat colour (part 30's stored colour); the camera's timing.

## Before: D283 the script camera's lens (2026-10-14)
- "Pod picture" PR 10, viewer only (`docs/level-scripts.md` part 34). FOUND (01.33, static
  reading, evidence only): the render camera's lens is ortho factor, vertical FOV (radians),
  aspect, near, far; perspective when the ortho factor is 0. `Cam.SetCameraFromToUp` writes near,
  far, ortho factor and FOV every call (the Pod: 10, 100,000, 0, 1.0 rad = 57.3 deg) and not the
  aspect (the game camera's 16:9 stays; INFERRED). Part 23's third float is the ortho factor.
  The override has no lens of its own: the game camera's near 10, far 40,000 (+ a zone's
  pull-back), 60 deg; part 20 corrected (10 and 40,000 are the game camera's constants always).
- Viewer: `level_play::script_lens` draws a script camera with the script's FOV and planes and
  the override with the game camera's; planes from level to display units by the level's
  display scale (INFERRED: the game's world units are level units). In the Pod (scale 1.5706e-4)
  Bevy gets near 0.0015706 and far 15.706 display units, FOV 1.0 rad (vertical, as Bevy's); was
  0.01 and 100 (64 and 636,705 level units). Ortho factor other than 0: the viewer's lens stays
  (PLACEHOLDER). Aspect never written. The viewer's lens is kept and put back when the camera goes
  out; a level without a script camera is never touched. Bevy's far plane only culls whole
  entities (its projection is infinite).
- MEASURED (`local-analysis/rscript/stage6/shots/d283-*`): the planets level reaches at most
  72,847 level units along the view (69,306 at rest): nothing culled by far 100,000. The FOV was
  already the script's (part 23), so the closed Pod at 470, the walk and press at 580, the intro at
  30 and 200 and nine of the eleven equality captures are equal byte for byte to D282's. Changed:
  in use from 600 held at 610, 3 pixels (a shell sliver between 10 and 64 units from the eye); the
  planets level alone at 600 (no world Thing root; the camera among its Things), 15.3 percent:
  a near object the old near plane cut is now drawn whole. Against the game's closed frame the
  470 still is as close as D282's (the same bytes).
- Cinema bars: FOUND `HUD.EnterCinemaMode`/`LeaveCinemaMode` (bars smoothstep(amount) x 0.17 +
  0.01 of the height each); the Pod's scripts never call them. INFERRED: the intro's black bar is
  the level's Thing 12. Docs only.
- Tests: three viewer tests (`d283_*`). fmt, CI clippy, CI tests (612), `cargo check` of both
  viewers and the viewer's clippy pass; viewer tests 84 pass, `sackboy_play::tests::input_script_steps`
  fails as on main. The runtime is untouched.
- Next: the orthographic branch; the camera zones' FOV and far; the pod computer's clips.

## Before: D282 his z span, his arrival and the pod computer's user (2026-10-14)
- "Pod picture" PR 9 (input option C), viewer only (`docs/level-scripts.md` part 33). Three
  placeholders of D281 replaced by FOUND rules of the 01.33 EBOOT (static reading, evidence only).
  Only with `--play-sackboy --level-scripts --pod-camera N --pod-shell HEX`.
- Z span. FOUND: absolute world z -500 to 100, set by the creature's constructor and rewritten
  every creature update (widening paths not taken in normal play: INFERRED).
  `level_play::POD_PLAYER_Z_SPAN` replaces `POD_PLAYER_HALF_DEPTH` (200). pod.bin's trigger
  (z -193.7) holds him in all three lanes; the lane decision is the pad zone's z >= -300 alone.
  The play mode's span on other levels (100, PLACEHOLDER) is unchanged.
- Arrival. FOUND: checkpoint translation, z max(checkpoint z, a box z) + 50, slot x offset (0,
  +80, -40, +40, -80; slot 0 first), no height offset. `sackboy_play::checkpoint_arrival`, box
  not identified (INFERRED: the checkpoint's z): start (-3.05, 540.76, 0), he falls. MEASURED:
  out of the pad zone on tick 1, in on tick 25 (y -331.3, still falling), on the floor on tick
  28 at y -476.4; nothing in between. The game's arrival (-5.37, -477.97): 2.3 in x unexplained.
  `data_spawn` (Moon x -1.11) stays for other levels.
- Pod computer user. FOUND: no script native moves him; the animation code switches his input
  context, faces him to pi and draws his body offset by (target - position) * blend, target
  (-12.469, -482.979, -93.170) with x from the "pod_controller" Thing. Viewer: while in use his
  movement input is dropped (`pod_input`), his facing eases to pi at 0.4 per tick
  (`pod_facing`), his figure is drawn at lerp(position, target, blend) (`engage_drawn`); blend =
  the change-over value (PLACEHOLDER for the clips' progress); his physics place stays. FOUND in
  the file: "pod_controller" (Thing 5) is at x 17.846, not -12.469 (30.3 unexplained). The
  D281 note "the game moves him to z -200" is not explained by this code.
- MEASURED (`local-analysis/rscript/stage6/shots/d282-*`): walk run as D281 (out 471, press 472
  ignored, in 514, press 535 taken, in use 536, change-over 1.0 on 555), he stays at x -11.0
  while in use with walk and lane keys given, shown facing 177.8 degrees by 545, 180 by 560.
  Lanes: out on 504 at z -305 (z rule, still in the trigger's list), in on 566, in use 595.
  Stills at 470 and 580 equal over two runs. Eleven captures without `--play-sackboy` equal byte
  for byte to D281's.
- Left: the enter/hold/leave clips and their timing; the controller x gap; the z -200 read; the
  arrival's x; the checkpoint box; the way out of the pod computer (not run in the viewer).
- Tests: three new viewer tests (`d282_*`), `d281_the_pad_zone_...` updated (back lane out by
  the z rule only). fmt, CI clippy, CI tests (612), `cargo check` of both viewers and the
  viewer's clippy pass; viewer tests 81 pass, `sackboy_play::tests::input_script_steps` fails as
  on main. The runtime is untouched.
- Next: the pod computer's clips; the Sackboy by default in the scripted Pod picture;
  `pod_mask.tex`.

## Before: D281 the viewer's own Sackboy in the Pod (2026-10-13)
- "Pod picture" PR 8 (input option C), viewer only (`docs/level-scripts.md` part 32). With
  `--play-sackboy --level-scripts --pod-camera N --pod-shell HEX` the Sackboy of the scripted Pod
  picture is the viewer's playable one, in the shell level (pod.bin). Without `--play-sackboy`
  nothing changes: part 30's undrawn record on the trigger stays (PLACEHOLDER).
- The floor. FOUND: the pod Thing's shape has depth under a matrix that lays its z along the
  level's y; in the plane it is the slab at y -160, projected through the whole matrix it is the
  pod's cross-section with a floor inside. MEASURED: that floor puts him at y -476.9 under x -4.2
  and -474.4 under x 338; the game has -477.97 and -475.53. The game's floor is pod.bin's own
  collision, 1.1 off; nothing fitted.
- Viewer: `sackboy_play::pod_scene` (the projected scene; every body held, no joints:
  PLACEHOLDER for the moving pod), the start standing at the data spawn (INFERRED rule: (-4.16,
  -476.91), lane 0; the game's measured arrival (-5.37, -477.97), lane 0), the play figure under
  the shell's frame root, the script's camera (no follow camera), no Popit.
  `level_play::stand_sackboy` hands his place to the shell world before every tick (`add_player`,
  then `move_player`), so the trigger's list and `GetIsInPadZone` follow where he stands; z span
  200 either way (PLACEHOLDER; MEASURED the game engages from lane 0 with the trigger at z
  -193.7; FOUND the span is two floats of the game, not read).
- Input, no key ever sent to a window: `--play-sackboy-input` walks him (`l r j b f`), `e` there
  is the engage button on a step's first tick; `--level-scripts-press-at N` as before; live, Z or
  gamepad Square (Space is his jump). `--level-scripts-hold-at` holds his input and pose too.
- MEASURED (`local-analysis/rscript/stage6/shots/d281-*`): in the pad zone on tick 1. Walking
  right: out on 471 at x 189.5 (the box ends at 183.1), the press on 472 ignored, back in on 514,
  the press on 535 taken, in use on 536, change-over 1.0 on 555. Lanes: in at z -200, out on 504
  at z -305 (the z rule), the press at z -400 ignored, in on 566, the press on 594 taken.
- Stills: standing (100) and sitting (600) twice with equal bytes; against D280's frames only his
  box differs (5,976 pixels closed, 6,310 open). By eye against the game's closed frame he is the
  same size at the same place within about 0.01 of the picture (0.470 to 0.523 by 0.700 to 0.881;
  game 0.47 to 0.525 by 0.706 to 0.885).
- Unchanged: ten captures without `--play-sackboy` equal byte for byte to D280's (without
  scripts, with the shell under scripts, one level with scripts); the play mode on one level
  gives the same positions.
- Left of option C: the default picture without `--play-sackboy`; the game's move to z -200 on
  engaging (he can still walk while the computer is in use); the arrival (a drop or not); the
  pod as a moving body; his z span; the floor's rise while walking (he stays at -476.9); a body
  in the lane on a lane change (he stands in the DualShock's shape in the back lane); the Popit,
  the prompt, his colour. Not seen: the keys, the motion. Not compared: the open frame.
- Tests: five viewer tests (`d281_*`). fmt, CI clippy, CI tests (612), Python tests (36),
  `cargo check` of both viewers and the viewer's clippy pass; the viewer's tests pass (78) but
  `sackboy_play::tests::input_script_steps` (fails on main). The runtime is untouched.
- Next: the Sackboy by default in the scripted Pod picture; the lane on engaging; `pod_mask.tex`.

## Before: D280 the Pod shell level in its own frame, under the script camera (2026-10-13)
- "Pod picture" PR 7, viewer only (`docs/level-scripts.md` part 31). Under `--level-scripts
  --pod-camera N --pod-shell HEX` the shell level (pod.bin) was placed by the viewer's rig: a shell
  eye fitted to the DualShock that followed the change-over value, a tent eye with a fitted tilt
  (PLACEHOLDER since parts 26 to 29). Now it stands in its own level frame and the planets level's
  script camera draws both levels. Nothing is fitted.
- MEASURED before (part 29, two captured frames): the camera the game hands the pod's draws is the
  script camera's eye, (0, -250, 1130) closed and (0, -299.5, 902.4) open. INFERRED: one camera and
  one frame for both levels, also between the end states.
- Viewer: `level_play::shell_frame_root` (the planets level's display matrix over the shell's: no
  turn, scale 0.7788 here); both shell roots are spawned with it and stay; `script_camera` places
  the camera only. Removed: `PodRig::layer_at` / `shell_root_at` / `tent_root_at`,
  `shell_layer_root` (the rig keeps `layer` for the path without scripts). The dissolve's plane
  (`follow_dissolve`) is unchanged in code: its matrix is now the planets level's display matrix,
  so the line is level y minus camera y minus `1150 - 1550 v`. The shell light's display scale is
  the planets level's; no light turns with the camera any more (part 26's shade issue is gone).
  `level_play::layer_depths` prints how the two levels lie under the camera.
- MEASURED (the viewer's line, closed at rest): shell box (-1276, -1521, -846) to (1219, 3438,
  1184), the camera inside it; along the view the shell reaches 2,161; the planets level's room is
  around both; the nearest planets-level surface to the shell's box is 14,421 away (14,476 to
  14,551 on ticks 605 to 619, 14,518 open). Near plane 64 level units, far 636,705. Preview fog
  25,805 to 78,454: none on the shell.
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d280-*`): without scripts (closed,
  `--pod-open`, `--pod-game-camera`, `--pod-t 0.5`) equal byte for byte to main's build; one level
  with scripts and no shell equal to part 30's. With the shell under scripts every frame differs,
  by design; closed at 600 twice with equal bytes; in use at 640, press at 641 and `--pod-open`
  give one frame.
- Compared by eye against two earlier RPCS3 frames (`local-analysis/d156-rpcs3-pod-default.png`,
  `d156-rpcs3-pod-open-t2.png`), fractions of the picture, about 0.01: closed, the tent's window
  0.34 to 0.68 by 0.26 to 0.72 (game 0.34 to 0.68 by 0.25 to 0.73), Earth (0.46, 0.48), Moon
  (0.56, 0.47), DualShock top 0.68 (game 0.68); open, DualShock 0.40 to 0.61 wide with its top at
  0.84 and the platform's rim at 0.97 (game 0.41 to 0.61, 0.84, 0.98). The open planets are view 0,
  not the game's menu view (not modelled).
- Eye check, 605 / 610 / 615 / 619: the DualShock keeps its place against the tent's floor and
  back wall (one fixed root for both); the camera's own overshoot takes it half below the frame at
  615. Ticks 30 and 200 now show the pod from outside, hanging in the room (INFERRED the game's
  fly-in; not compared).
- PLACEHOLDER left: the fitted Pod figure (`--select-guid`) under the scripts. Not applied: the
  script camera's near and far planes.
- Tests: four viewer tests (`d280_*`); two removed with their code (`d275_layer_at_...`,
  `d278_the_tent_root_stays_closed_...`). fmt, CI clippy, CI tests (612), Python tests (36),
  `cargo check` of both viewers and the viewer's clippy pass; the viewer's tests pass (73) but
  `sackboy_play::tests::input_script_steps` (fails on main). The runtime is untouched.
- Next: a frame of the change-over from the game; the viewer's own Sackboy in the Pod; `pod_mask.tex`.

## Before: D279 the pod computer entered through the pad zone and the press, in the viewer (2026-10-13)
- "Pod picture" PR 4b: viewer, one function moved into the runtime (`docs/level-scripts.md` part
  30). The Puter key set "in use" on both worlds itself; now pod.bin's own script does it (the path
  of D273: pad zone, click, `RequestHostUsePod(true)`, the host's answer a tick later).
- Runtime: `LevelWorld::pod_trigger_spot()` (the `Trigger_Pod` trigger's world position plus its z
  offset; the tool's spot finder, moved; the tool calls it).
- Viewer, only with the shell level in the session (`--level-scripts --pod-camera N --pod-shell
  HEX`): a Sackboy is put on that spot before the shell's first tick (`place_sackboy`); the pad's
  button word is written on both worlds before every tick, bit 0x80 on the tick of a press
  (`feed_pad`, `pad_word`); `pod_dolly` latches the key in `PuterState::key` and `tick` takes it:
  the opening key (Space, gamepad Square) is a press while the computer is free, any Puter key
  while it is in use asks the host to close (`ask_to_close`: `request_host_use_pod(false)`, handed
  to the other world); `PuterState::open` follows the session's "in use". QA option
  `--level-scripts-press-at N` (the press on tick N). `--pod-open` and `--level-scripts-in-use-at
  N` stay direct.
- PLACEHOLDER: he stands on the trigger from the start and is not drawn; closing (no script path
  was found, the pod computer's menus are not modelled); the player colour (opaque white). No
  gamepad mapping was made up: nothing in the viewer maps a button to the game's button word, the
  Puter's own buttons are kept (which button bit 0x80 is, is not established).
- A press before the intro has set the flags does nothing, as in the game; one line says why.
- Side effects: the dissolve's flat band colour is white (was black without an avatar); closed,
  the pod's shape colour is the file's white, so the shell's `(new scene)` line is gone; the two
  pod computer sounds are counted; `BP_ENGAGE_POD` prompts are counted and not drawn (the overlay
  shows the planets level's messages only: none seen).
- MEASURED, session run, `--enter-pad-zone=1 --press=600:0x80`, 700 rounds
  (`local-analysis/d279-pod-session-700-zone1-press600.json`): pad zone on 1, flags on 452, 148
  click tests from 453, request on 600, in use and the start-up sound on 601, change-over 0.05 on
  601 and 1.0 with mesh 145753 on 620, camera at rest on 626; no fault, no unbound call. Press on
  100: no request, nothing opens.
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d279-*`): closed at 600, press at
  100 held at 600, open (press at 600 held at 641, in use from 600 at 640, `--pod-open`) and
  without scripts equal D278's captures byte for byte; one level with scripts and no shell equals
  main's build. Press at 600: `Engage press on tick 600: pod.bin's script took the click ...`,
  `Pod computer at tick 601: in use true, by the host's answer`, change-over 601 to 620. Held at
  610 press and in-use-at give the same bytes; against D278's 610 still 60,321 pixels differ, all
  but 4 brighter (the band).
- Eye check (611): the tent cut at about a third of the frame, white circuit traces in the band
  on the outer card. 641: the platform, as D278. Nothing compared against the game.
- Left out: the viewer's own Sackboy in the Pod (a physics scene of pod.bin, the floor at the
  game's measured y -478, a camera); the pod computer's menus.
- Tests: one runtime test, four viewer tests (`d279_*`). fmt, CI clippy, CI tests (612), Python
  tests (36), `cargo check` of both viewers and the viewer's clippy pass; the viewer's tests pass
  (71) but `sackboy_play::tests::input_script_steps` (fails on main). The 378-level runs were not
  repeated (`tick_level` unchanged).
- Next: the viewer's own Sackboy in the Pod; `pod_mask.tex` and the plain UV; the platform's edge.

## Before: D278 the Pod tent's top-down dissolve (2026-10-13)
- "Pod picture" PR 5, viewer only (`docs/level-scripts.md` part 29). Under `--level-scripts
  --pod-camera N --pod-shell HEX` the tent no longer slides out of the frame: it stays where the
  closed picture has it and is cut from the top down while pod.bin's change-over value v is between
  0 and 1.
- FOUND (the pod's three materials, one vertex program; notes in ignored
  `local-analysis/rscript/stage6/pod-dissolve-notes.md`): a tent pixel is discarded when its height
  over the camera is above 1150 - 1550 v, plus 100 x the green of the material's diffuse, plus 400
  where a second mask (`pod_mask.tex`, 0x1fbc7) is white; the transition mask (132243, UV x 40) only
  tints the 400 units under the line toward the flat shape colour. Alpha of the pod's shape colour =
  1 - v; at alpha 1 the rule is off. MEASURED (two captured frames): the camera handed to the pod's
  draws is the planets script camera's eye in both end states; between them not captured.
- Viewer: `pod_dissolve.rs` (the numbers and the plane arithmetic), a dissolve block in the viewer
  material (`dual.rs`: discard in the opaque pass by a world-space plane, tint by the mask),
  `thing_view::apply` keeps the value and colour on the tent's entities of the shell's pod Thing
  (own material on the first value; nothing touched at alpha 1; `Followed::dissolved`),
  `level_play::follow_dissolve` writes the plane each frame from the tent root and the script
  camera's y; the tent root is placed at t = 0 (`shell_layer_root`); the mask is decoded with the
  level (`PodDissolveMask`).
- INFERRED: the shell level's frame and the script camera's frame are one frame (so camera y between
  the end states is the script's). PLACEHOLDER: mask coordinates from the position (70-unit tile)
  on a bucket without a diffuse texture; the shell root still follows v (the DualShock moves against
  the tent's floor between the ends).
- Not the game's rule: the viewer's surface colour green for the diffuse green; `in.uv` is the
  diffuse sampler's coordinates (x 2 on the glass panels); no second mask; flat colour not fogged;
  the platform at v = 1 solid; the six other backdrop masks not loaded. No avatar: the flat colour
  is black.
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d278-*`): without scripts, closed
  at 600, and open at 640 equal part 28's captures byte for byte. In use from 600: `Pod tent
  dissolve at tick 601: value 0.05, 3 draw entities; cut above level y 820.4 ...; the script's mask
  GUID: the same`, `Shell Things at tick 620: ... mesh swapped 1, dissolve set 1`.
- Eye check (602, 605, 610, 615, 619): the tent in place, cut from the top down (roof open at 605,
  a third of the frame at 610, low walls at 615, only parts under the floor at 619), circuit traces
  in the band under the edge, the room and planets through the cut part. Nothing compared against
  the game.
- Tests: six viewer tests (`d278_*`, the shader test extended). fmt, CI clippy, CI tests (611),
  Python tests (36), `cargo check` of both viewers and the viewer's clippy pass; the viewer's tests
  pass (67) but `sackboy_play::tests::input_script_steps` (fails on main).
- Next: `pod_mask.tex` and the plain UV; the platform's edge at 1; PR 4b, the Puter key through the
  pad zone and press.

## Before: D277 both Pod levels in one viewer session (2026-10-13)
- "Pod picture" PR 4a: viewer, one runtime accessor, one tool option (`docs/level-scripts.md` part
  28). With the shell layer under `--level-scripts` only the planets level was played: the pod's mesh
  swap never showed in the Pod picture and the tent left on the viewer's own dolly.
- Viewer: under `--level-scripts --pod-camera N --pod-shell HEX` pod.bin gets a script world and a VM
  of its own (`level_play::ShellLevel`); each tick is one `lbp_runtime::tick_session` round, the shell
  first; `our_pod` when the shell has a pod Thing (PLACEHOLDER, the tool's stand-in). "In use" (Puter
  key, `--pod-open`, `--level-scripts-in-use-at N`) is set on both worlds (`set_in_use`: a value set
  on one is written over by the first level's hand-over). `thing_view::ShellThings` (a second Thing
  map), the swapped pair loaded for the shell (`LevelMode::shell_scripted`), `follow_things` once per
  level (`Shell Things at tick N`). After each tick `PuterState.t` is the shell world's change-over
  value kept in 0..1 (`opening`); `pod_dolly` no longer moves `t` there. Without the shell layer the
  `tick_level` path is unchanged.
- Runtime: `LevelWorld::pod_transition()` (the last value `SetFadePodDecorations__f` was given).
  `level_scripts --with-level` honours `--puter-in-use=N` (both worlds).
- FOUND in the EBOOT and the game data (scoping; part 28 has the addresses): `SetFadePodDecorations__f`
  (0x403ca4) stores a fade byte per decoration of the pod Thing from its world height and the value
  (a wipe from the top down, each over 0.2 of the value, none drawn at 1); pod.bin has no decoration.
  `SetPodTransitionTexture__g` (0x4015e8) swaps texture slot 5 (a mask) of the three materials of
  `pod.mol`. The tent goes by a dissolve from the top down in the pixel shader, through that mask and
  the alpha of the pod's shape colour (1 - value), not by an even fade. The two orange pieces of D276
  are Things 13 and 14, string joints (mesh 0x4c7f) from the bar Thing 12 to the pod Thing 2; nothing
  read hides them (INFERRED: the game keeps them drawn), so they are left drawn.
- MEASURED, session run pod.bin + pod_planets, 700 rounds, in use from 600
  (`local-analysis/d277-pod-session-700-inuse600.json`): in use on 601, change-over 0.05 on 601 to 1.0
  on 620, mesh 145753 on 620, planets camera at rest at (0, -300, 900) on 626; no fault, no unbound
  call in either level.
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d277-*`): equal byte for byte to
  main's captures: pod.bin alone and the shell picture without scripts, pod.bin alone with scripts
  (held at 610 and 640), the planets level alone with scripts (600), and the Pod picture with scripts
  closed at 600 (D275's). In use from 600: `Pod change-over at tick 601: opening`, `at tick 620: open`,
  `Shell Things at tick 620: ... mesh swapped 1 (5 entities) ... [(2, "00023959")]`.
- Eye check: at 610 the tent is half way out (its inner walls at the frame's left and right borders);
  at 640 the tent is gone and a flat round disc lies under the DualShock at the bottom of the frame,
  the DualShock's grips in its cut-outs. The string ends are in neither frame (INFERRED: above the
  open view). Two short dark marks on the disc were not identified. Nothing compared against the game.
- PLACEHOLDER: the shell's opening `t` from `PodTransition` (nothing read ties the shell's eye or the
  tent's slide to it; the game's shell camera is not established); `our_pod`; the Puter key.
- Left out: the tent's dissolve (no blended or discard material path yet: next PR), the viewer's
  Sackboy and pad as the input (PR 4b), the player's Pod decorations, the snap for a computer open on
  arrival, one VM for both levels.
- Tests: four new viewer tests (`d277_*`), one runtime test extended. fmt, CI clippy, CI tests (611),
  Python tests (36), `cargo check` of both viewers and the viewer's clippy pass; the viewer's tests
  pass (62) but `sackboy_play::tests::input_script_steps` (fails on main). The 378-level runs were not
  repeated (`tick_level` unchanged; Things stopped by a fault 3 / 3 / 3 as in D274).
- Next: the tent's dissolve; PR 4b, the Puter key through the pad zone and press.

## Before: D276 the pod's mesh swap in the scene (2026-10-13)
- "Pod picture" PR 3, viewer and scene builder (`docs/level-scripts.md` part 27). The runtime reported
  the mesh a script puts on a PRenderMesh and the viewer counted it as not drawn. FOUND before: the one
  such call is `Pod.SetPodTransition__b` on pod.bin Thing 2, `pod.mol` (0x2f6c) under 1,
  `pod_platform.mol` (0x23959) at 1.
- `lbp-preview`: `LevelSceneOptions::mesh_alternates`, `add_alternate_meshes`, `SceneItem::alternate`:
  a Thing drawn with one mesh of a pair gets a second item for the other (not counted as drawn, out of
  the bounds). Viewer: `thing_view::SWAPPED_MESHES` (one pair), only under `--level-scripts` and never
  for the shell layer; the second set is spawned hidden; `thing_view::apply` shows the set of the mesh
  the Thing has now, under the play-mode visibility rule (`Followed::mesh_swapped`). QA option
  `--level-scripts-in-use-at N` sets `pod_computer_in_use` when N ticks have run (stand-in for D273's
  pad zone, press and host answer; MEASURED that "in use" alone runs pod.bin's change-over).
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d276-*`): without `--level-scripts`
  pod.bin alone and the shell picture are equal byte for byte before and after. pod.bin with scripts,
  in use from 600: `Things at tick 620: ... mesh swapped 1 ... [(2, "00023959")]`; the still at 610
  equals the capture without scripts byte for byte.
- Eye check: before, the cardboard pod with the DualShock in its opening; after, a flat round cardboard
  disc where the pod's floor was, the DualShock on it. Two small orange pieces at the pod's sides stay
  in mid-air (not entities of the pod Thing; not looked up). Nothing compared against the game.
- Left out: the fade (the pod pops), the swap in the Pod picture (the shell layer has no script world).
- Tests: one `lbp-preview` and two viewer tests. fmt, CI clippy, CI tests (611), Python tests, `cargo
  check` of both viewers and the viewer's clippy pass; the viewer's tests pass (58) but
  `sackboy_play::tests::input_script_steps` (fails on main).
- Next: Pod picture PR 4, the two Pod levels in one viewer session.

## Before: D275 the Pod shell layer under the script's camera (2026-10-13)
- "Pod picture" PR 2, viewer only (`docs/level-scripts.md` part 26). With `--pod-camera N --pod-shell
  HEX` under `--level-scripts` the viewer kept its own Pod camera and said the script's was "not
  applied". Now the planets level is drawn with the script's camera there too, and the shell and the
  tent are placed from it.
- Viewer: `PodRig::layer_at` / `shell_root_at` / `tent_root_at` (`pod_menu.rs`; `layer` calls
  `layer_at`, same arithmetic). `LevelMode::pod_scripted` (`--level-scripts` with `--pod-camera N`):
  the planets level under the world Thing's root, Thing map on, with or without the shell; the menu
  constants of `--pod-game-camera` (`place_menu_planets`, `planets_root(t)`, default per-bone
  placement) are left out there. The rig has the game's field of view and the camera presets.
  `level_play::script_camera` places the shell and tent roots from the script camera's eye and turn
  and the Puter state's `t`; `pod_dolly` only moves `t`; `tick` hands the Puter state to
  `pod_computer_in_use` when it changes.
- MEASURED (no focus taken; `local-analysis/rscript/stage6/shots/d275-*`): without `--level-scripts`
  the shell captures before and after are equal byte for byte, with and without `--pod-game-camera`.
  With `--level-scripts` the flag `--pod-game-camera` changes nothing (equal byte for byte).
- Eye check: at rest (tick 600) the tent's window shows the Earth near (0.46, 0.48) of the frame and
  the Moon near 0.56 of the width, the framing of the `--pod-game-camera` capture without scripts,
  and clear (fog bias). Ticks 30 and 200: the intro's ceiling and wall in the window. The shell does
  not cut into the planets and the fog does not swallow the view. `--pod-open`: the camera rests at
  the in-use preset, the tent is gone; view 0, not the game's menu view. Nothing compared against
  the game.
- Seen, not fixed: under `--preview-lighting` the tent's shade changes while the camera is pitched
  (the preview light is not turned with the shell root; the default lighting turns it).
- INFERRED: the shell's size rule. PLACEHOLDER: the Puter key as "in use"; `t` is the viewer's dolly.
- Left out: the shell level's camera in the game, the tent's fade and `PodTransition`, the menu
  views, the mesh swap (PR 3), the shared viewer session (PR 4).
- Tests: two new viewer tests (`d275_layer_at_...`, `the_puter_state_is_handed_...`). fmt, CI clippy,
  CI tests, Python tests, `cargo check` of both viewers and the viewer's clippy pass; the viewer's
  tests pass but `sackboy_play::tests::input_script_steps` (fails on main).
- Next: Pod picture PR 3, the pod's mesh swap.

## Before: D274 Earth and Moon by their own scripts, the fog bias, the video update (2026-10-13)
- "Pod picture" PR 1, after RScript plan stage 6. The planets level's Earth and Moon stayed at their
  stored places (their per-frame function was not called), `--preview-lighting` washed the played
  planets level out, and the place of the pod computer's video call was not known. Natives 255 -> 257.
- FOUND in the EBOOT (scoping for this PR; `docs/level-scripts.md` part 25): the planets object's
  per-frame function (0x4381d8) calls `Update__rb(v4, bool)` on every planet (0x42115c), then the
  function that ends in the world script's `UpdatePlanetsView`. The vector (0x421708) is a point of
  the planet to turn toward the camera; the bool is a byte a true result sets ("arrived"). The script
  puts the planet at its class's constant place (Earth (-1250, 0, 0), Moon (2000, 0, 0)) with the axis
  lengths it was created with, turned toward the point. `Math.transform` (0x5ee288): matrix times
  vector; `Math.ModAngleToWithinPI` (0x5eebb0): the angle less whole turns. The third `SetDOFAndFog`
  value is a fog bias: the Pod's measured fog start 25810 is the level's fog near 5810 plus 20000.
  `PodComputer.UpdateVideo__` is the last step of the pod computer's draw function (0x404598), called
  once a rendered frame after the scene (0x31e820).
- Runtime (`lbp-runtime/src/lib.rs`, `lbp-script-vm`): frame entries for `Earth` and `OrbitingMoon`
  (`PLANET_UPDATE`), `LevelScripts::planets` (`PlanetFrame`: point, arrived, calls),
  `set_planet_point`, the planets before the world script's entry; `LevelScripts::update_video`, once
  a tick after the global-settings pass (new, not moved: D271 had not added the call);
  `StepReport::planet_updates`, `planets_arrived`, `video_updates`; the two Math natives. The virtual
  call on a Thing (`CALLVsp`) now dispatches on the class of the Thing's own script
  (`Host::thing_script`): it was a plain call, and the planets' `GetPosition__` needs it.
- Viewer: under `--level-scripts --pod-camera N` without the shell layer the Thing map is on, so the
  planets' models follow their scripts inside the world Thing's root; `follow_fog` hands the bias to
  `LightingState::fog_bias` and `--preview-lighting` moves its fog out by it (`Fog::biased`). Without
  `--level-scripts` nothing changes (a capture is equal byte for byte to D272's).
- MEASURED, pod_planets alone, 600 ticks (`local-analysis/d274-planets-600.json`): Earth at (-1250, 0,
  0) with axes 0.9610 and Moon at (2000, 0, 0) with axes 2.1121 from tick 1 to 600; with the meshes'
  inverse bind scale (1.4052, 0.7312) that is the size read in the running game (P2k: 1.3505, 1.5444).
  1,200 planet calls, 600 video calls, 0 entries without a function (2 a tick before), 0 faults, 0
  unbound calls.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed; `d274-play-all-*.json`): Things
  stopped by a fault 3 / 3 / 3; unbound natives called 40 -> 41 / 71 -> 72 / 71 -> 72; unbound calls
  61,419 -> 61,420 / 862,919 -> 862,920 / 868,592 -> 868,593; native calls 573,134 -> 574,456 /
  5,365,247 -> 5,366,569 / 5,424,893 -> 5,426,259. Two levels differ: the planets level, and one
  cutscene level where a virtual call now reaches a subclass (`Gooey.StartFrameNamed`, unbound, once).
- Eye check (no focus taken; `local-analysis/rscript/stage6/shots/d274-*`): at rest the Earth's
  centre is near (0.46, 0.48) of the frame and the Moon near 0.56 of the width, the places P2k read
  from the closed Pod; the room is clear with a light haze under `--preview-lighting` (fog
  25804.6..78453.6 level units; 5804.6..58453.6 before). Nothing compared against the game side by
  side.
- PLACEHOLDER: the point while the host sets none, (0, 0, 1) (no turn; the game's idle vector was not
  read), so the side of the Earth that faces the camera is not the game's. INFERRED: the dispatch rule
  of the virtual call on a Thing; a tick is a frame and the video call is its last script call; the
  rounding of `ModAngleToWithinPI`; the argument registers of `Update__rb`.
- Left out: the planets' entries and turning; sticker decals of the planets (the scene has none: the
  viewer projects stickers on shapes only); depth of field; the bias in the default lighting's shader
  fog; the shell layer, the mesh swap, the shared viewer session (PR 2 to 4 of the Pod picture).
- Tests: `lbp-script-vm` `d274_planet_math_natives`, `virtual_calls_on_a_thing_dispatch_on_the_script_it_carries`;
  `lbp-runtime` 61 unit and 52 world tests (three new `d274_*`); two viewer tests. fmt, CI clippy, CI
  tests, Python tests pass; clippy and `cargo check` on the viewers pass; the viewer's tests pass but
  `sackboy_play::tests::input_script_steps` (fails on main).

## Before: D273 the pod computer is entered (2026-10-13)
- RScript plan stage 6 (the Pod), PR 4. The Pod's world script lets the leader engage the pod computer
  (flags, pad zone, button), asks the host for it, and then runs its change-over. Five natives of that
  path were unbound, the request was only recorded and the two Pod levels could not be played together.
  Natives 249 -> 255.
- FOUND in the EBOOT (read for this PR; `docs/level-scripts.md` part 24 has the table):
  `Native.GetIsInPadZone` (0x13bac0): our Pod, the primary player's head, the Thing in the list of a
  PWorld entry whose script passes a class test and a call by name, world z >= -300.
  `Input.IsJustClickedByID` (0x3890a0): an action number, not a pad mask; action 8 has context mask
  0x1001 and is bit 0x80 of the pad's button word, newly down; the string is the prompt's key only.
  `Native.GetPlayerColour` (0x139308): a stored 32-bit colour of the player as (R, G, B, A) / 255.
  `Game.CurrentLevelIsOurPod`: a byte written when a level is entered (0x1ab508), true for one slot id.
  `Video.PressedStartButton` (0x756520): bit 0x80000 of the first of seven pads, used up.
  `PodComputer.RequestHostUsePod` (0x3fa9cc): only in our own Pod; stores and sends the request. "In
  use" is set by the receiver of the host's message (0x3fa868 -> 0x18ed30), with a start-up or
  shut-down sound. `SetPodTransitionTexture` (0x4015e8) sets a texture on three engine resources.
- Runtime (`lbp-runtime/src/pod.rs`, `lbp-script-vm/src/natives.rs`): `LevelWorld::our_pod`,
  `in_pad_zone`, `request_host_use_pod`, `begin_session_tick` (the answer, first step of `tick_level`),
  `hand_session_to` and `tick_session` (several levels in one session: progression flags, "our Pod",
  "in use" and a waiting request, media requests, per player the colour and used-up buttons);
  `PadState::is_just_clicked`, `use_up`, `held_off`; `PlayerRecord::colour`, `colour_vector`.
  `RequestHostUsePod` left the recorded-only list; `SetPodTransitionTexture` joined it.
- `level_scripts`: `--our-pod` (else a level with a pod Thing is started as our Pod: the tool's
  stand-in, the game tests the slot), `--enter-pad-zone=TICK`, `--press=TICK:MASK,..`,
  `--player-colour=AARRGGBB`, `--with-level=PATH` (a second level in the session; `runs.session` has
  the path of events).
- MEASURED, pod.bin and pod_planets.bin in one session, 700 rounds, fresh profile
  (`local-analysis/d273-pod-session-700.json`): flags 3 on round 452; the Sackboy in the pad zone from
  460; the press and `RequestHostUsePod(true)` on 470; in use on 471 (start-up sound counted); the
  change-over 0.05 on 471 to 1.0 on 490, where the pod's mesh becomes GUID 145753; the planets camera
  leaves (0, -250, 1130) on 471 and rests at (0, -300, 900) on 496. No fault and no unbound call in
  either level. The other level order gives the same events on the same rounds.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed; `d273-play-all-*.json`): Things
  stopped by a fault 3 / 3 / 3; unbound natives called 41 -> 40 / 73 -> 71 / 73 -> 71; unbound calls
  61,480 -> 61,419 / 862,986 -> 862,919 / 868,663 -> 868,592; native calls the same. Only pod.bin
  differs. In the visit runs one Thing fewer has a shape colour unlike the file (the pod's, by the
  placeholder colour).
- INFERRED: the pad zone's list is the PTriggers and its class `Trigger_Pod` (the call by name is not
  made); the first player record is the primary player, the host and the Pod's owner; the host's
  answer comes at the start of the next tick; Start has the pad word's bit numbering; context bit 0x1
  is gameplay. PLACEHOLDER: a player's stored colour starts opaque white (the game's value and who
  writes it were not found).
- Left out: the viewer (`--level-scripts` still plays one level; nothing draws the change-over or feeds
  the viewer's Sackboy and pad in); the pod computer's own scripts and menus; the computer open on
  arrival and closed on leaving; the ownership test; the other 98 actions; one VM for both levels.
- Tests: `lbp-script-vm` `d273_pod_computer_natives`; `lbp-runtime` 61 unit and 49 world tests (three
  new `d273_*`). fmt, CI clippy, CI tests, Python tests pass; clippy on the Bevy viewer passes and its
  tests pass but `sackboy_play::tests::input_script_steps` (fails on main); its sources did not change.
  Nothing run in the viewer; nothing compared against the game.
- Next: the shared session and the change-over in the viewer, or the pod computer's own scripts.

## Before: D272 the Pod's camera and its moving world (2026-10-13)
- RScript plan stage 6 (the Pod), PR 3. The planets level's world script writes the camera itself each
  tick (`Cam.SetCameraFromToUp`) and moves its world Thing each tick. The runtime kept both and nothing
  drew by them. No native is new (249).
- FOUND in the EBOOT (scoping notes of stage 5 and 6): `SetCameraFromToUp` (0x2e9798) builds the camera
  matrix from eye, point and up with the look-at builder 0x2e7990 and writes it into the render camera,
  no blend; its four floats are stored as near, far, one value not read, and the field of view
  `Cam.GetFov__` returns (radians). The script passes 10, 100000, 0, 1.
- Runtime (`lbp-runtime/src/camera.rs`): `ScriptCamera` (eye, direction, up as passed, lens),
  `LevelWorld::script_camera` (stands from the call to the end of the next tick's count),
  `LevelWorld::display_camera` (which camera a frame is drawn with: a camera a script wrote, outright;
  else the follow camera under an override; else none), `DisplayCamera`, `CameraSource`,
  `CameraPose::matrix_with_up`; `GetCameraMatrix` without a view answers with the written matrix.
  `display.rs`: `LevelWorld::world_thing_moved`. `level_scripts`: five counts in the `camera` block and
  the camera after ticks 1, 226, 452 and the last for a level played alone.
- Viewer (`level_play.rs`): `script_camera` applies `display_camera` (eye, turn, field of view; near and
  far stay the viewer's); `follow_world` places the root of the planets level (`N M N^-1`) each tick.
  With `--pod-camera N` and no `--pod-shell` the level hangs under that root; with the shell layer the
  viewer keeps its own Pod camera and says so once (that layer is placed for it). Without
  `--level-scripts` no flag changed. New QA option `--level-scripts-hold-at N` (scripts stop after tick
  N and a capture waits for it). `thing_view.rs` counts a swapped mesh as not drawn.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): the `play_all` block equals D271 but
  for the new counts. Things stopped by a fault 3 / 3 / 3; unbound natives called 41 / 73 / 73; unbound
  calls 61,480 / 862,986 / 868,663. `SetCameraFromToUp` 60 / 60 / 62 calls in 1 level; levels with a
  written camera and an override in at once: 0.
- pod_planets alone: the camera after tick 1 eye (-500, -2000, 2000), after 226 (-250, -1125, 1565),
  from 452 on (0, -250, 1130) looking along (0, 0.132, -0.991), 57.3 degrees; with the pod computer in
  use (0, -300, 900). The viewer prints the same poses.
- Eye check (no focus taken, `local-analysis/rscript/stage6/shots/`): stills at ticks 30, 200, 440 and
  600: the camera comes from low, looking up at the ceiling, to a level view of the room with the Earth
  and the Moon near the middle. Under `--preview-lighting` they are washed out by the viewer's distance
  fog (the level is 31,000 units away); the default lighting shows the room clearly. With the shell
  layer the capture is equal byte for byte with and without scripts. Nothing compared against the game.
- INFERRED: a written camera stands for one tick and goes before an override; the look-at axes; the
  field of view is vertical; the whole planets level is drawn under its world Thing (measured in the
  running game before, P2k; the engine code was not read).
- Left out: the shell layer under the script's camera, Earth and Moon by their own scripts (they stay
  at their stored places, so the planets' framing is not the game's), the pod's mesh swap in the scene,
  near and far, depth of field, fog. Next: PR 4, the pod computer is entered. `docs/level-scripts.md`
  part 23.

## Before: D271 video, voice-over and progression flags (2026-10-13)
- RScript plan stage 6 (the Pod), PR 2. The planets level's intro thread (451 ticks of camera zoom,
  then a wait on "a voice-over line plays or the pod computer has a video handle") never ended: the
  unbound `Video.StartVideo` answered 0 and the script takes any handle but -1 as a playing video.
- Nothing plays. A video or voice-over request is kept with its arguments (`LevelWorld::media`: video
  GUID, line name, flags, frame) for a later video or audio system; the script gets the game's answer
  for "nothing started".
- FOUND in the EBOOT: `Video.StartVideo__Q5Thinggb` (0x67c620) answers -1 when no video starts;
  `Video.UpdateVideo__ib` (0x67c22c) false for a handle no video has; `Audio.RequestVoiceOver` (0x2de6b8)
  false when no line starts; `IsVoiceOverPlaying` (0x2de218) false while none plays; `StopVoiceOver`
  true; `SetMixOverlay` / `RemoveMixOverlay` (0x2cdb7c, 0x2cdaa0) false without the audio system object;
  `Profile.SetLBP2GameProgressionFlag__i` (0x15d22c) ORs a bit mask into one 32-bit word of the profile
  and `IsLBP2GameProgressionFlagSet__i` (0x15d2a8) wants every bit of its mask. The intro sets mask 3.
- Runtime: `LevelWorld::progression_flags` is one `u32` (was a set of numbers); `--flags` and the
  viewer's `--level-scripts-flags` take masks (`parse_progression_flags`). New
  `lbp-runtime/src/media.rs` (`MediaState`, `VideoRequest`, `VoiceOverRequest`). Natives 238 -> 249
  (`STATE_QUERIES` 26 -> 29, `RECORDED_ONLY` 23 -> 24, new `RECORDED_REQUESTS` of 5 with their answers).
  `level_scripts`: `media`, `progression_flags`, `thread_ticks`. Viewer: the option's parse only.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): Things stopped by a fault 3 / 3 / 3
  (unchanged); unbound natives called 44 -> 41 / 78 -> 73 / 78 -> 73; unbound calls 61,483 -> 61,480 /
  865,266 -> 862,986 / 872,042 -> 868,663. Video requests 1 in 1 level, voice-over requests 1 in 1
  level (all three); mix overlays removed 0 / 2,260 in 114 levels / 3,355 in 143; a script sets a
  progression flag in 0 / 3 / 3 levels. The rest equals D270.
- The Pod levels alone: pod_planets, 600 and 1,200 ticks: the thread starts on tick 1 and ends on tick
  452; the flag word goes 0 -> 3 on tick 452; no thread starts again; 0 unbound natives (was 4, 152
  calls); the camera rests on the not-in-use preset (the in-use one with `--puter-in-use`). pod.bin is
  as in D270; with `--flags=3` and a visiting player its update reaches `Native.GetIsInPadZone`.
- INFERRED: `HasPlayedLBP1AndUnlockedPod` / `Moon` false (no save data of the first game); the debug
  and build overrides of the flag test are off; `RequestVoiceOver` true when a line starts.
- Not added: the engine's per-tick `PodComputer.UpdateVideo__` call. FOUND who calls it (0x50373c, at
  the end of 0x404598, a function reached only through a table slot); who calls that slot and when was
  not found. Not bound: `Video.PressedStartButton__`. The flag word is per level world: one session
  for both Pod levels is PR 4. Next: PR 3, the viewer's Pod camera. `docs/level-scripts.md` part 22.

## Before: D270 script names, the pod Thing, the avatar (2026-10-13)
- RScript plan stage 6 (the Pod), PR 1. The Pod is two levels, `gamedata/levels/pod/pod.bin` (the shell)
  and `gamedata/lbp2/pod/levels/pod_planets.bin` (the planets view); the runtime plays each alone. The
  shell's world script was the one Thing outside TriggerAirStone that stopped on a fault: two unbound
  natives answered null and it read the PShape of the null Thing.
- FOUND in the EBOOT: `Thing.FindThing__Q7StringA` (0xd3360) compares a PWorld list's byte strings with
  the argument by the C string compare (whole string, case sensitive), first match wins, null with none.
  `Thing.GetPodThing__b` (0xd32dc) gives the first entry of a PWorld list whose resource GUID is 0x2f6c;
  one with 0x23959 only when the bool is true (with false the search ends there with null).
  `Game.GetAvatarFromPlayerID__i` shares the core of `GetYellowheadFromPlayerNumber` (0x18ec14).
  `Thing.SetRenderMeshMesh` (0x755580) sets a PRenderMesh's mesh from a resource whose type is a mesh.
  `FileList.AsGUID__i` / `AsInt__g` are the identity; `Game.IsKioskBuild__` is the constant false.
- Reader: `lbp-level` keeps the PScriptName's name (`PartData::script_name`: `ScriptName { stored, name }`,
  bytes up to the first NUL); it kept the length. Same bytes read; Toolkit parity 236 / 236.
- Runtime: `lbp-runtime/src/pod.rs` (`LevelWorld::find_thing`, `pod_thing`, `script_names`,
  `planets_background`, `PodState`); `ThingState::script_name`; `mesh_guid` follows `SetRenderMeshMesh`;
  a mesh change reaches the display list (`DisplayChange::mesh`, `ThingDisplay::mesh`). Natives
  225 -> 238 (2 recorded only: the pod decorations' fade and the request to use the pod computer).
  `level_scripts` has a `pod` block. The viewer is unchanged (it compiles and its tests are as on main).
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): Things stopped by a fault 4 -> 3 in all
  three (fault kinds 2 -> 1, levels with a fault 2 -> 1); left: 3 x `Thing.GetCOM__` (TriggerAirStone, 1
  level). Unbound natives called 48 -> 44 / 81 -> 78 / 81 -> 78; unbound calls 62,000 -> 61,483 /
  865,777 -> 865,266 / 872,549 -> 872,042. 173 Things in 66 levels have a script name (141 not empty);
  `FindThing` 1 call, 1 found; `GetPodThing` 61 / 64 / 66 calls, all found.
- The Pod levels alone, 600 ticks: pod.bin has no fault, its script is resumed on all 600 ticks, the
  checkpoint is set active, the pod keeps its mesh (transition 0). pod_planets is unchanged: the intro
  thread runs its 451 ticks of zoom and then waits on the video handle (PR 2).
- INFERRED: the two PWorld lists are the PScriptName and the PRenderMesh parts in Thing order; a null
  string finds nothing; a null resource clears a mesh; no start menu and no photo.
- Not bound (effect or answer not established): `PodComputer.SetPodTransitionTexture__g` (61 calls in
  pod.bin), `Game.CurrentLevelIsOurPod__`, `Native.GetPlayerColour__Q5Thing`, `Native.GetIsInPadZone`,
  `Input.IsJustClickedByID`. Next: PR 2 (video, voice-over, progression flags), PR 3 (the viewer's Pod
  camera). `docs/level-scripts.md` part 21.

## Before: D269 the script camera and the screen (2026-10-13)
- RScript plan stage 5, PR 5 (the last of stage 5). FOUND in the EBOOT: the level's camera keeps one
  override: a position and a direction (`Cam.SetOverrideCamera__rr`, 0x3760e4), a blend target and a
  blend rate (`Cam.SetOverrideCameraActivation__bf`, 0x374cec). Each camera update (0x6eb650) the blend
  steps linearly by the rate, is eased with 0.5 - 0.5 cos(pi x blend), and the eye is mixed linearly and
  the view direction on the arc from the follow camera; above a blend of 0.0001 the lens is 60 degrees,
  near 10, far 40000. Only a script ends it. `Cam.GetCameraMatrix__` (0x7347e0) is camera to world;
  `HUD.GetScreenSpacePosition__r` (0x2e7bdc, 0x2e74a0) gives pixels from the top left, y down, on the
  1280 x 720 of `HUD.GetXRes__` / `GetYRes__`, with no test for a point behind the camera.
- Runtime: `lbp-runtime/src/camera.rs`. `CameraState` has the `OverrideCamera`, the `CameraView` a
  display writes (camera matrix, view-projection) and counters; `step`, `ease`, `slerp`, `blend_pose`,
  `screen_position`, `view_projection`; `LevelWorld::step_camera` at the end of `tick_level`,
  `camera_pose(follow_eye, follow_dir)` for a display. Natives 219 -> 225 (the six above).
- Viewer: `--level-scripts` takes its own camera placement as the follow camera each frame, draws with
  the mixed camera and the 60 degree lens while the override is in, and writes the camera it drew with
  for the scripts (`level_play.rs`, `script_camera`). Orbit, pan and zoom go on underneath. It prints
  `Camera at tick N`.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): unbound natives called 51 -> 48 /
  87 -> 81 / 87 -> 81; unbound calls 73,325 -> 62,000 / 1,002,445 -> 865,777 / 1,010,529 -> 872,549; the
  override comes in on 0 / 101 / 101 levels (fully in on 98; 39,386 / 39,588 set calls); screen
  positions asked in 28 / 61 / 70 levels (3,775 / 32,360 / 32,730 calls). Things stopped by a fault
  4 -> 4; messages, global settings and display counts equal to D268.
- INFERRED: world +Y as up under the override; the step at the end of the tick; both directions at
  length 1 in the mix; without a display `GetCameraMatrix` is the default follow pose (or the identity)
  and a screen position is the middle of the screen, so that a run without a display counts as before.
- Not bound: `Native.SetSpeechBubbleActive__Q5Thingb` (which part it finds and what the call does are
  not established). `Cam.SetCameraFromToUp` is the Pod's: stage 6.
- Checked in the window without taking focus (main_04_bravery_test, the Sackboy started in the
  scoreboard's trigger): with the scripts the camera is centred on the scoreboard, at the override's
  position; without them the follow camera. Not seen: the blend in motion (the scripts tick while the
  scene loads, so it was fully in at the first frame), a bubble closed for being off the screen.
  Open: with `--play-sackboy` some levels start the Sackboy at the scoreboard's own checkpoint, and the
  camera then goes to the scoreboard at once and stays. Next: stage 6, the Pod.
  `docs/level-scripts.md` part 20.

## Before: D268 hidden, tinted and moved Things (2026-10-13)
- RScript plan stage 5, PR 4. FOUND in the EBOOT: `Thing.HideRenderMesh` / `ShowRenderMesh` clear and
  set two bits (mask 3) of the PRenderMesh's visibility byte, the stored `visibilityFlags`, and go down
  the Thing's children that have a mesh and no shape (0xebf98); bit 0 is "visible in play mode"
  (`PostItNote.IsVisibleInPlayMode__`, 0x5e9260); the mesh colour, mesh tint, shape colour and shape tint
  setters store packed colours and do nothing for a null Thing or a missing part.
- Runtime: `lbp-runtime/src/display.rs`. `LevelWorld::take_display_dirty` lists the Things whose
  visibility byte, colours or world matrix changed (`DisplayChange`); `thing_display` and
  `display_at_load` give the values (`ThingDisplay`, with `mesh_hidden` and `moved_from`);
  `display_changed`, `display_summary`. Natives 216 -> 219 (`ShowRenderMesh`, `SetShapeEditorColourTint`,
  `IsVisibleInPlayMode`); hide and the mesh colour are real bindings (`RECORDED_ONLY` 23 -> 21).
- Viewer: `--level-scripts` tags the draw entities of each Thing's own model (render mesh, decorations)
  and keeps a Thing-to-entity map (`thing_view.rs`). Each tick it hides a mesh whose play-mode bit is
  clear, gives a tinted mesh a material of its own (red, green and blue multiply the surface colour) and
  moves a Thing's entities by the change of its world matrix. A new scene starts with every mesh hidden
  that the level file stores as not visible in play mode. It prints `Things at tick N`.
- Not followed: shapes (merged into shared meshes per material and colour: a recoloured or moved shape
  keeps the file's look and place), an editor colour set by a script (built into the material at load),
  the tint's alpha, the Pod, per-item skinned meshes.
- MEASURED (378 levels, 60 ticks, the three runs alike): 68,618 render meshes, 12,106 in 218 levels
  stored as not visible in play mode (byte 2: 11,573, byte 0: 533); visibility bytes changed by scripts
  0 (the 113 hide calls are on meshes already stored hidden); mesh colour unlike the file 10 Things / 1
  level; mesh tint 102 / 101 (101 white with an alpha, 1 with a colour); shape colour 102 / 101; world
  matrix unlike in any bit 4,545 / 32, beyond rounding 1 Thing (no mesh). Things stopped by a fault
  4 -> 4; unbound natives unchanged.
- INFERRED: a mesh without bit 0 is not drawn in play and its decorations stay; the tint as a multiply
  before lighting; the two colour words are the stored `editorColor`; the packed channel order.
- Checked in the window without taking focus (side_rts_tutorial, unlit): the level link's padlock and
  the scoreboard's two corner markers are gone with the scripts. On ghost_train the one tint with a colour (0.70, 1.0, 0.15 on the scoreboard's mesh) turns the scoreboard's frame yellow-green. Not seen: a change while the
  scene is up (covered by the `thing_view` test). Next: PR 5, camera and screen natives.
  `docs/level-scripts.md` part 19.

## Before: D267 script global settings take effect (2026-10-13)
- RScript plan stage 5, PR 3. FOUND in the EBOOT: the PWorld's per-frame function (0xd5e84, after the
  switch pass in the world update) calls `ApplyGlobalSettings__` on the scripted Thing of each
  global-settings slot and clears a slot whose Thing is gone. The script steps its blend factor there,
  which is why no setting moved before.
- Runtime: `LevelScripts::apply_global_settings` in `tick_level`; `LevelWorld::global_settings_holder`;
  `lbp-runtime/src/global_settings.rs` with `WorldState::settings_in_force` (`SettingsInForce`),
  `LevelWorld::settings_at_load` / `settings_unlike_load`, `GlobalSettingsReport`
  (`StepReport::global_settings`) and `LightFollower`.
- Viewer: `--level-scripts` builds the level's light again when lighting or darkness in force changes,
  by the rule it already uses for the file's values (`LevelSettings::at_lighting`). Not applied: fog,
  fog tint, colour correction (mapping not read), gravity, water. It prints `Lighting at tick N`.
- MEASURED (378 levels, 60 ticks; plain / trigger visit / pulsed): calls 4,562 / 80,255 / 80,631; a
  setting in force unlike the level file in 48 / 81 / 96 levels (0 before); lighting group 47 / 76 /
  89, gravity 3 / 11 / 15, water level 0 / 1 / 2. Things stopped by a fault 4 -> 4; unbound natives
  unchanged.
- INFERRED: the call's place after `Update__` (the engine's per-Thing `Update__` call was not found).
- Checked in the window without taking focus: cutscene_outro_theme1, lighting 0.4 -> 1.0, darkness
  0 -> 0.6. Open: a new light in the first frame after the scene is spawned lost the GPU device (a
  60-frame wait avoids it, cause unknown); the viewer's fog covers most of those frames.
  Next: PR 4, hidden, moved and tinted Things. `docs/level-scripts.md` part 18.

## Before: D266 the viewer draws the messages (2026-10-13)
- RScript plan stage 5, PR 2. `lbp-bevy-viewer --level-scripts` loads the English base and patch
  translation tables for its level world (as `level_scripts`) and draws `LevelWorld::messages`: a plain
  box per open bubble above its anchor Thing (speaker name, text, close prompt string of
  `CLOSE_PROMPT_KEY`) and the subtitle at the bottom centre. Without tables it still runs: no prompt line.
- `lbp-runtime/src/message_view.rs` (tested in CI): `LevelWorld::message_overlay` (what is drawn),
  `display_text` (markup out), `close_prompt`, `place_bubbles`, `place_subtitle`. The viewer's
  `speech_overlay.rs` measures and paints with egui. The viewer prints a `Messages at tick N` line on
  each change; the window has a messages line.
- MEASURED: the tables' markup is seven tags in angle brackets (emphasis 1,838 places, line break 280,
  icon 299, font 68, player and slot icons 46 of 29,477 strings). Line break -> new line, icon -> its
  name in square brackets, the rest left out.
- INFERRED: the box above the Thing, pushed back inside the screen at an edge; bubbles of one Thing
  stacked; every open bubble drawn; captions and think bubbles like speech; empty bubbles and the
  level-link and wormhole prompts not drawn; the look. The prompt is text only (event 2 not modelled).
- Checked: pulsed run equal to D265 (Things stopped by a fault 4); one level seen in the window without
  taking focus (one bubble at its Thing; clamped at the upper edge from the follow camera). Not seen: a
  subtitle, a name, a prompt line, a stack. Next: PR 3, global settings applied.
  `docs/level-scripts.md` part 17.

## Before: D265 the message list (2026-10-13)
- RScript plan stage 5, PR 1. `LevelWorld::messages` (`lbp-runtime/src/messages.rs`) is the game's list
  of open speech bubbles, captions, subtitles and level-link and wormhole prompts, run by the messaging
  natives. FOUND in the EBOOT (0x3e8dc0, 0x3e4124, 0x3e8aa0, 0x3e3248, 0x3e528c, the cores 0x7382e0 ..
  0x737a00, 0x737910): manager 4 holds at most 8; a request of a type about a Thing that has one is
  dropped; a bubble without a prompt or a subtitle is written over the one of its type; a ninth clears
  the eighth; event 3 closes the messages of its Thing; no time limit; the actor name is a translation
  tag in slot types 0 and 8. The four caption and think natives are bound (212 -> 216); the ten
  messaging natives are `natives::MESSAGING`, no longer recorded only (29 -> 23).
- Read API for the viewer: `messages.open` (`Message`: kind, style, text, actor, thing, anchor, bubble
  position, flags, frame), `Message::age`, `has_close_prompt`, `LevelWorld::message_position`.
- INFERRED: a closed message leaves at once; the list keeps the order of adding; the anchor is the
  Thing's microchip, else the Thing; age restarts when written over. Not modelled: the player's close
  button (event 2), which queued bubble is shown, placement.
- MEASURED, 378 levels, 60 ticks (plain / visit / pulsed): requests 92 / 1,690 / 2,381; opened 57 /
  1,036 / 1,187; written over 35 / 621 / 1,161; dropped 0 / 33 / 33; closed 0 / 1,000 / 1,152; levels
  with a message 47 / 192 / 216. Text from a loaded table in all but 1 non-empty case; every message has
  an anchor. Things stopped by a fault stay at 4. Next: PR 2, the viewer draws the list.
  `docs/level-scripts.md` part 16.

## Before: D264 tag sensors and player sensors (2026-10-13)
- The switch pass computes the tag sensor (type 4) and the player sensor (type 3) each tick; the other
  sensors keep the stored signal. FOUND in the EBOOT (0x118d5c, 0x119910, 0x11873c, 0xd5c1c, 0x12f6f4,
  0xf9a64): the tag map by (colour index, name), rebuilt when the pass starts; a tag's signal from its
  Thing's input port 0, on when unwired; range by Radius and MinRadius in x and y; closeness; on at 0.85 of
  the Radius, off at 0.95, hold between; the BulletsRequired-th strongest tag; KeySensorMode 1 hands on the
  tag's signal; RequireAll and PlayerMode for players; inversion afterwards. `lbp-level` keeps the sensor
  fields of PSwitch and the PSwitchKey colour and name. `lbp-runtime/src/sensors.rs`.
- INFERRED: no cone test (AngleRange below 180: 1,811 player and 2,975 tag sensors) and no layer test
  (IncludeTouching 1: 212 and 489); positions are the Things' positions; the leader is the first record.
- MEASURED, 378 levels, 60 ticks: 5,627 player and 16,953 tag sensors, 19,629 tags. Sensors that differ
  from the stored signal on some tick: 184 / 5,446 (plain), 5,204 / 6,272 with a player visiting every
  trigger and player sensor. Reaction calls 262 -> 323, 283 -> 8,970 with the visit, 5,704 -> 14,346
  pulsed. Things stopped by a fault stay at 4. Next: stage 5. `docs/level-scripts.md` part 15.

## Before: D263 the natives the stage 4 runs reached (2026-10-13)
- 48 natives bound (164 -> 212), 16 of them recorded only. FOUND in the EBOOT: a pad per player record
  with the pressed / just pressed / just released rules (0x388e2c, 0x388358, 0x389400); `LAMS.IsInLams`
  as a search of the loaded translation tables (0x159c1c); the global-settings blends, start + (target -
  start) * t per setting, with the setters, the blend factor and the water getters (0xce608, 0xd9dac,
  0xd623c, 0x5ec6f8); `GetGameFrame`, `GetNumYellowHeadAvatars`, the score data type, the scoring slot
  test; `Slot.GetSlotGroup` (0x7477d8: a SlotID (0, 0) for a slot the database lacks, never null).
  Speech bubbles, prompts, game events and audio settings are kept with their arguments
  (`LevelWorld::requests`) for stage 5.
- Bug fix: a script cast (`CASTsp`) tests the Thing's script class chain; every cast passed before.
- INFERRED: both pad mode masks are 1 and the old buttons are copied at the end of a tick; the English
  base and patch tables are the loaded ones (`level_scripts` only; the viewer loads none); every setting
  starts at rest on its stored value; no off-screen indicator, no Sackbot names, no modal overlay, a
  fresh profile.
- MEASURED, 378 levels, 60 ticks, every output pulsed: unbound natives called 110 -> 88, unbound calls
  952,819 -> 766,940, recorded-only calls 17,206 -> 83,820. Things stopped by a fault stay at 4 in all
  three runs (fault kinds 2, levels with a fault 2). No input is fed to the pads yet. Next: stage 5.
  `docs/level-scripts.md` part 14.

## Before: D262 world, position, player and game state (2026-10-13)
- RScript plan stage 4. `LevelWorld` keeps what the scripts ask the engine about the level. FOUND in the
  EBOOT: PWorld `GameMode`, `Frame`, `WaterLevel` and the two global-setting arrays (serializer 0xb3f3c0);
  PPos local and world matrices, where a write moves the descendants (0xf2e80, 0xf26bc); the microchip proxy
  (0x754498); player records with score, multiplier counter, timer, source, visible flag, number and team
  (0x156b18); the global-setting priority rule (0xce554); SlotID objects (0x16cb18). 41 natives bound
  (123 -> 164), 9 of them recorded only. Fixes: part names that start with P (`Pos`), the VM's m44 element
  order (column * 4 + row), `GetWorldPosition__` returns the matrix.
- INFERRED: one local player numbered 0, there from the start; `IsVcrPaused` false in play (the files store
  IsPaused true in 220 of 236 levels); the frame steps at the end of a tick; the proxy is the outermost chip;
  the level's own slot is 0 / 0.
- MEASURED, 378 levels, 60 ticks: Things stopped by a fault 597 -> 4 (plain), 598 -> 4 (trigger visit),
  1,794 -> 4 (every output pulsed); fault kinds 11 -> 2; levels with a fault 221 -> 2; reaction calls
  240 -> 262 (4,397 -> 5,704 pulsed). Left: `Thing.GetCOM__` (3 Things) and one null Thing in the Pod.
  Next: stage 5, show what the scripts do. `docs/level-scripts.md` part 13.

## Before: D261 switch network and reactions (2026-10-13)
- RScript plan stage 3b. `lbp-runtime/src/switches.rs`: each tick `LevelWorld::update_switches` computes
  every switch output in wiring order and applies it to its targets (FOUND in the EBOOT: 0x1244a4, 0x11ddf0,
  0x11bd58, 0x3ed2c, 0x409dc). Computed: microchip, circuit board, NOT, AND, OR, XOR, battery, direction,
  sign split, toggle, selector, counter, timer, inverted forms, microchip off. Stored signal kept: sensors,
  randomiser, sequencer boards. `SwitchPerformReaction__b` / `__bf` are called on the 16 script GUIDs the
  game lists.
- MEASURED, 378 levels, 60 ticks: 24,612 of 144,608 outputs differ from the stored value; 204 `__b` and
  36 `__bf` calls (3,052 and 1,345 with `--pulse-switches`); stopped Things 582 -> 597; 19 more unbound
  natives reached. Next: stage 4, player and game state. `docs/level-scripts.md` part 12.

## Before: D260 switch inputs (2026-10-13)
- RScript plan stage 3a. PSwitch is kept by `lbp-level`; `LevelWorld::wire_inputs` builds each Thing's
  input table from the switch outputs' targets (INFERRED build; table and readers FOUND in the EBOOT:
  0x7541e0, 0x754308, 0x7542a4, 0x75437c, 0x754ccc, 0x754244, 0x754af8). `IsSwitchTriggered__i` is true
  when an output is wired to the port, whatever its signal. 5 natives bound (123): the ternary form,
  `GetInput`, `GetOutput`, `Native.GetInThing`, `BaseWorld.IsScoreBoard`.
- MEASURED, 378 levels: 141,572 PSwitch Things, 177,800 targets, no port collision, 3,732 scripted Things
  with a wired port. Things stopped by a fault 1,722 -> 582; with the trigger visit unbound natives
  reached 94 -> 91. `docs/level-scripts.md` part 11.

## Before: D259 trigger events (2026-10-12)
- RScript plan stage 2. PTrigger is kept by `lbp-level`; `LevelWorld::update_triggers` tests the players
  against every trigger and `LevelScripts::tick_level` calls `Enter__Q5Thing`, `Inside__Q5Thing`,
  `Leave__Q5Thing`. FOUND in the EBOOT (0xe3bc0, 0x1364f0, 0x135fe0, 0x135a18, 0x1346fc): position in the
  trigger Thing's space; circle, box or sphere of the radius multiplier, times the hysteresis multiplier
  while something is inside; layer rule by the z span of Sackboy's body; Enter/Leave to the Thing's script
  or its parent's, Inside to the parent's. 2 natives bound (118). Viewer: `--level-scripts` with
  `--play-sackboy` feeds the Sackboy to the triggers (his z span is a PLACEHOLDER).
- MEASURED, 378 levels: 23,990 triggers, 4,662 scripted. `level_scripts --all --play=60 --visit-triggers`:
  11,430 Enter, 895 Inside, 7,357 Leave calls run; stopped Things stay 1,722; unbound natives reached
  65 -> 94. Next: stage 3, switch events. `docs/level-scripts.md` part 10.

## Before: D258 scripts run past unbound natives (2026-10-12)
- RScript plan stage 1 (`docs/rscript-plan.md`). An unbound native returns its type's default and is counted
  (`Bindings::default_unbound`; viewer and `level_scripts --play`; off in tests). `level_scripts --all
  --play=N` reports bound and unbound calls and the faults left over every level. 19 natives bound (116).
- MEASURED, 378 levels, 60 ticks, 13,547 scripted Things: stopped by a fault 4,940 -> 1,722; 76,269 calls
  to 64 unbound natives defaulted. Left: switch inputs 1,143, `PWorld.GameMode` 220, local transforms 199,
  TriggerLevelLink 113. Next: stage 2, trigger events. `docs/level-scripts.md` part 9.

## Before: D257 ledge-fall jump layer weight (2026-10-12)
- MEASURED on three RPCS3 ledge falls (lower-body vertex fits): the jump layer eases in with the 0x20a5f4
  follower from 0, peak += 0.125 (|vy| - peak), weight clamp(peak/10) times the blend-in; full weight about
  10 ticks after the edge (ours stopped near 0.77). w error 0.064 (D234 ramp 0.276). Landing tail at full
  weight. INFERRED: the follower holds under the landing tail. `docs/d250-air-lean-turn.md` (D257).

## Before: D256 standing foot placement (2026-10-12)
- FOUND in the EBOOT (foot pass 0x210dd8): standing (ground-relative speed under 1), each Foot joint is put
  on the floor point nearest its leg anchor, 25 to either side of the body, by a two-bone solve; in over 3
  ticks, out over 5. `FOOT_PLANT_*`, `Sackboy::foot_plant`, applied in `blend_frame`. MEASURED: foot spacing
  43.6 (game 43.4, was 51.9), standing rms 0.79 (was 1.30). `docs/d250-air-lean-turn.md` (D256).

## Before: D255 stick response, shortest jump, mirrored morphs (2026-10-12)
- FOUND in the EBOOT (0x38a578, 0x38aa10): stick x = (byte - 128) / 128 unclamped, response past the dead
  zone rescaled and squared, times both speed caps. Full right is 0.9778: MEASURED 29.34 / 22.00 to the
  right against 30.00 / 22.50 (Moon and Rookie Test). `stick_response`, `STICK_FULL_RIGHT`.
- MEASURED (d211): one- and two-tick presses peak at 33.1 and 90.4; `JUMP_MIN_TICKS` 1.
- Mirrored morphs applied (FOUND 0x1e42b4). Arm terms read: 0 without the arm buttons, no change.
  `docs/d250-air-lean-turn.md` (D255).

## Before: D254 mirrored poses (2026-10-12)
- FOUND in the EBOOT (pose mirror 0x1e42b4): a mirrored layer takes each bone's partner's local transform
  through one of six rules picked by the skeleton's mirror flip type; the zip-pull bones are copied, not
  reflected. `Rig::mirrored_trs` does the same. MEASURED on the left-to-right turn: vertex rms 2.3..4.2 ->
  0.5..1.4 on the turn frames, mirrored walk 0.7..1.6. `docs/d250-air-lean-turn.md` (D254).
- Walk start: MEASURED, no change needed. The walk phase is a free-running counter in the game too (phase
  at the first moving tick 0, 0, 3, 4, 4 over five captures); at equal phase our start fits at 0.9..1.7.

## Before: D250 jump air lean, D251 turn (2026-10-12)
- Air lean: FOUND in the EBOOT and applied. In the air and through the landing tail the upper body (Spine1
  and below, without the head) is lerped back toward stand frame 0 by s (1 - s), s easing 0.1 per tick
  (`AIR_EASE`, `Blend::UpperBody`). MEASURED on d234 `jump.bin`: Spine1 pitch within 0.05 degrees of the game
  on all flight ticks (was up to 5.5 off).
- Turn: first game captures of a direction reversal (`local-analysis/d252-turn/`, story level, flat). Pose,
  speed and clip timing matched already. Fixed: facing through the turn follows the speed angle (the
  72-degree hold is for the stop and lane change only), and the walk restarts on frame 1. Vertex rms through
  the turn 1.9..3.8 (was 48..98). `docs/d250-air-lean-turn.md`.

## Before: D249 short legs after a landing (2026-10-12)
- Two-bone leg solver (`BlendPose::move_chain_end`); `LANDING_SQUASH` and a twelve-tick `LANDING_DIP`,
  within 0.4 of the game on touchdown +3..+8 (one standing jump). `docs/d241-landing-dip.md` (D249).

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- Mesh census 01.33 (4,261 GUIDs, after D188-D190): 4,259 viewable, 4,239 all-diffuse by
  default, 0 Complete. Files `local-analysis/b177-{default,legacy}.json`.
- Texture census 01.33: all 23,247 hashes decode (top mip, decodability only).
- Levels 01.33: 398 resources; lbp-level walks 236 in-band, Toolkit parity 236/236.
- D233: +0 newly viewable / +0 newly Complete (runtime feature).

## Open blockers
- Foot placement on slopes and steps (the standing spot is fixed in model space, not the contact point).
- Landing: only a standing jump is captured (ledge fall, running jump: same tables, unchecked). Upper arms
  2 to 3 degrees off late in the flight, unattributed (the arm terms are 0 there).
- Head-top bones 22/23/24/27/28.
- After a landing the body rests 0.5 to 1.5 above the spawn height (shape cast distances).
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; arm buttons (the arm clips); face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Emote clip frames vs game.
2. Popit: compare more frames, cursor mode, inventory grid pages (submode 11).
3. Camera lead, slopes (paused by the owner).

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu). Build trees are
  ~11 GB: reuse `target-pr*` through `CARGO_TARGET_DIR`.
- Git: remote origin github.com/chilladept/lbp2-asset-explorer (private), default-deny
  `.gitignore`, CI `.github/workflows/ci.yml`. Standing rules: CLAUDE.md.
- Native QA windows hidden or on the non-primary monitor; never steal focus.

## Where detail lives
Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`. Earlier milestone
docs under `docs/` (grep headings); format evidence `docs/formats.md`; rules `AGENTS.md`.
