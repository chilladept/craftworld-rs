# Level scripts, part 41: the glass's blend per channel (2026-10-14, D290)

Seventeenth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
Part 38 left two things about the Pod's glass: the left panel's share looked about twice
the game's over a darker room, and the blend used one transmittance for three channels.
This part finds that the first was a measuring error and fixes the second.

### What was found

- MEASURED (a probe build that drew the glass fully clear, not kept; closed Pod, tick 600,
  the level's own lighting, the game's panel mask): the room behind the viewer's left panel
  is (142.7, 142.2, 108.2) against the game's (145.1, 144.5, 110.7), and behind the right
  (72.6, 85.9, 86.9) against (73.2, 86.0, 86.9). The room is not darker.
- MEASURED (the same run): the viewer's glass share, out - 0.8 x the room, is (15.0, 19.7,
  22.1) on the left against the game's (17.3, 21.9, 22.9), and (2.4, 14.0, 24.0) on the
  right against (7.3, 14.7, 20.8). The left share is within error of the game's.
- INFERRED, the cause of part 38's numbers: D287 took the room from the planets-only still
  (`d287-defaultlight-planets-podcam-tick600`, without `--pod-shell`). Behind the closed
  Pod's left panel that still shows the right cloth's dark edge, the wall and the table,
  where the closed run and the game show the pale left cloth with the yellow ring. Out minus
  0.8 x that wrong room gave (39, 47, 43). The viewer's output was right all along.
- MEASURED (the game's glass draw re-done in a script from its post-transform vertices and
  its pixel program, no bump; it fits the game's share to 1 level): the sun is on +x and
  lights the left panel (N.L 0.52) and not the right, so the game's own left share is about
  twice its right one by design. The viewer lights the panels the same way (which panel the
  sun hits, the normal's sign, the rim); no change is needed there.
- MEASURED (the game's share per pixel blended over the viewer's room three ways): the
  viewer's output equals "game shading plus the one-T secant" to 0.3 levels. The right
  panel's (-2.4, +1.2, +5.1) against the exact blend is all the shared T (a blue share over
  a dark room); a T per channel leaves at most 1.1 on both panels.

### What the viewer does

`lbp-bevy-viewer` (`dual.rs`), the glass bucket (material GUID 0x1d14c) only.

- The secant of part 38 is now taken per channel: t_c = clamp((y2_c - y1_c) / (l2 - l1), 0,
  1) and S_c = max(0.5 ((y1_c - t_c l1) + (y2_c - t_c l2)), 0), through the same encoded
  destinations 0.35 and 0.75 (`glass_secant`, mirrored by `glass_blend_per_channel`).
- The glass writes two blend sources at location 0: the colour (S, 1) and the cover (1 - t,
  1). Its main-pass pipeline blends the colour as One + OneMinusSrc1, so out = S + t D per
  channel; the alpha blend is unchanged. The material's pipeline key now says whether it is
  glass; only that pipeline takes a second copy of the shader that starts with `enable
  dual_source_blending;` and the define `GLASS_DUAL_SOURCE`. Every other material keeps the
  pipeline and shader it had.
- FOUND (Cargo.lock): Bevy 0.19.1 on wgpu 29.0.4 and naga 29.0.4, naga_oil 0.22.0. naga_oil
  takes the `enable` directive out of the source and puts it back first, and Bevy's shader
  cache sets the composer's capabilities from the device's features. Bevy's default
  `WgpuSettings` (priority Functionality) asks for every feature the adapter has, so
  `DUAL_SOURCE_BLENDING` is on wherever the adapter has it; no setting is changed. wgpu
  allows it with one colour target only; the glass's pass has one.
- A render start-up system notes whether the device has the feature. Without it, or with
  more than one colour target, the glass keeps part 38's one-T blend.

### Measured (PC, release build)

Closed Pod, `--pod-camera 1 --pod-shell 000027b9 --level-scripts --level-scripts-hold-at
600`, the level's own lighting, D289's build against this one. Frames in ignored
`local-analysis/rscript/stage6/shots/` (`d290-*`, capture script `scratch/glass2-shot.ps1`).
The log says the dual-source blend is on.

- Equal byte for byte: the planets level at 600 with `--pod-camera` (no shell) and Da
  Vinci's rookie test (0001e786). D289's closed still equals D288's.
- Changed: the closed Pod, 377,445 pixels, every one of them inside the viewer's glass.
- Panel means on the game's mask (left; right): before (129.2, 133.5, 108.6); (60.5, 82.8,
  93.6), now (129.6, 133.4, 107.8); (64.0, 82.3, 89.4), the game (133.4, 137.6, 111.5); (65.9,
  83.5, 90.3). The right panel moves from (-5.4, -0.7, +3.3) to (-1.9, -1.2, -0.9) of the
  game's, as section 3 of the notes predicted ((64.0, 82.4, 89.3)).
- INFERRED: the left panel's remaining (-3.8, -4.2, -3.7) is the room behind it (0.8 x
  about -2.4) plus a share 1 to 2 under the game's (its bump, not in the model); within
  error, not chased.

### Left out

- A device without dual-source blending still gets one T (a dedicated pass would be exact
  there).
- The left panel's few levels (above).

### Verification

`lbp-bevy-viewer`, no game data: `d290_per_channel_secant_is_exact_at_both_points` (exact at
0.35 and 0.75 for tinted colours, where one T misses by over 3 / 255),
`d290_grey_glass_matches_the_mean_t_blend`, `d290_pod_panels_follow_the_game_per_channel`
(the measured shares and rooms: within 1.5 / 255 per channel; one T 4 or more off in red and
blue on the right), `d290_per_channel_blend_beats_one_t_over_the_room` (mean under 1.5 /
255, one T over 2.5; worst under 9), `d290_per_channel_output_is_valid`,
`d290_glass_keys_the_dual_source_pipeline` (the key and One + OneMinusSrc1) and
`d290_dual_source_variant_validates` (the dual-source shader with `enable
dual_source_blending;` parses and validates). The D287 tests stay on the one-T fallback;
the shader validation now resolves the `#ifdef`s (the fallback variant). fmt, the CI clippy line, the CI
test set, `cargo check` of both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets --
-D warnings` pass; the viewer's tests pass (104) but the one that fails on main
(`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
