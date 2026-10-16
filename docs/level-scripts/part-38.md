# Level scripts, part 38: the Pod's glass blended as the game does (2026-10-14, D287)

Fourteenth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
Part 37 left the glass lighter and less green than the game's and named three causes to
check: the blend's colour space, the specular's scale and the left panel's view. This part
measures what the game's glass does to the room behind it and fixes the first two.

### What was found

- MEASURED (the captured Pod frame, read with a script on a hidden desktop): the glass
  draws into a 1280 x 720 `R8G8B8A8_UNORM` target with sRGB conversion off, so the game
  blends its encoded output (the shader's square root, P2r) directly: out = s + 0.8 d.
- MEASURED (the target just before and just after the glass draw, over the 302,608 pixels
  the draw changes, the emulator's corrupt blocks left out): the room behind the left panel
  is (145, 145, 111) and becomes (133, 138, 111); behind the right panel (73, 86, 87) becomes
  (66, 84, 90). The glass's own share, s = out - 0.8 d, is (17, 22, 23) on the left and (7,
  15, 21) on the right (of 255): a faint blue-green.
- FOUND (the program, evidence only): s = 0.2 x the encoded lit colour (its hemisphere
  light is blue: (0.47, 0.47, 0.71)) plus 0.6 x 6 x light^2 x (pow(saturate(R.L), 66) +
  saturate(rim / 2)), the specular added after the alpha and not encoded. No shadow map is
  bound.
- MEASURED, the main finding: the room behind the panels is what makes the game's glass
  dark and green-blue, and part 37's stills were taken with `--preview-lighting`. Under the
  level's own lighting the viewer's room seen from the same camera (planets level, pod
  camera, tick 600) is (112, 109, 83) behind the left panel and (69, 90, 88) behind the
  right; under `--preview-lighting` it is (191, 173, 137) and (160, 163, 148).
- MEASURED (c): under the level's own lighting the two hanging cloths show their art, the
  large yellow ring on the left and the blue and purple rings on the right, as the game's
  frame does. Under `--preview-lighting` they are plain tan. So the left panel's cloth in
  part 37 is the preview lighting, not the camera or the cloth's place. INFERRED: the
  cloths' colour comes through the level lighting's route only; not traced.

### What the viewer does

`lbp-bevy-viewer` (`dual.rs`, `main.rs`), the glass bucket (material GUID 0x1d14c) only.

- The glass's encoded colour is s = alpha x the sRGB encoding of its lit colour plus the
  specular, alpha = 0.2 x the dissolve's kept share as in part 37.
- The specular (level lighting only, where the gloss is drawn): the gloss colour (0.6) x 6
  (`GLASS_SPECULAR_SCALE`, fc45) x the squared light x (pow(saturate(R.L), CosinePower) +
  saturate(rim / 2)), added after the alpha; no shadow. The glass no longer takes
  `gloss_term` in its lit colour.
- The blend (`glass_blend`): Bevy's target is sRGB and blends linear light, out = S + T D
  with one T for the three channels. The game's blend, written in linear light, is
  dec(s + (1 - alpha) enc(D)), which is not of that form. The viewer uses its secant through
  the encoded destinations 0.35 and 0.75: T the mean of the three channels' slopes, S the
  mean residual at the two points (clamped at 0); the fragment writes S and alpha 1 - T for
  `AlphaMode::Premultiplied`. No extra pass and no screen copy.
- How close (MEASURED with the captured s per pixel): over the game's own room pixels the
  mean error against the exact encoded blend is (2.4, 2.0, 3.7) of 255, over the viewer's
  room (1.7, 0.9, 1.2); part 37's linear blend was off by (10.2, 5.4, 2.7) over the viewer's
  room. On a grid of glass colours like the measured ones and destinations 0.2 to 1 the mean
  error is 1.7 (part 37's form 7.4); the worst is 12.5, a tinted colour over a dark
  destination, where one T for three channels cannot follow.
- A dedicated pass (glass into its own target, combined in encoded space) would be exact
  but adds a render graph node for two panels; left out.

### Measured (PC, release build)

Same setup as part 36. Frames in ignored `local-analysis/rscript/stage6/shots/`
(`d287-*`), compared with D286's.

- Equal byte for byte: the planets level at 600, with and without `--pod-camera`;
  `--pod-open`; in use from 600 at 640; the walk and press run at 580.
- Changed, the glass only: closed at 600 and the closed Sackboy still at 470 (411,439 pixels,
  at most 13 levels), the shell without scripts (98,973), with the game camera (238,651), at t
  0.5 (38,374); in use from 600 at 610 (238,389) and at 615 (29,141; under the level lighting
  30,861, at most 51, the specular now added after the alpha).
- Panel means against the game (left; right), the game's frame being the target after the
  glass draw: game (133, 138, 111); (66, 84, 90). With `--preview-lighting`: D286 (170, 156,
  125); (143, 147, 135), now (163, 151, 122); (133, 141, 132). Under the level's own lighting,
  new still `d287-defaultlight-closed-tick600`: (129, 133, 109); (60, 83, 94).
- The viewer's glass share under the level lighting, out - 0.8 x the room: right (5, 11, 23)
  against the game's (7, 15, 21); left (39, 47, 43) against (17, 22, 23).
  Corrected in part 41: the left share's room came from the planets-only still, which shows
  other pixels behind that panel; with the room of the same closed run the left share is
  (15, 20, 22), within error of the game's.

### Left out

- `--preview-lighting` washes out the room (the centre wall (203, 179, 150) against the
  game's (174, 127, 83)) and drops the cloths' art; Pod comparisons of colour should use the
  level's own lighting.
- The left panel adds about twice the game's share under the level lighting (its specular or
  light there); the room behind it is darker than the game's (112, 109, 83) against (145, 145,
  111).
  Corrected in part 41: neither holds; the room is (143, 142, 108) against the game's (145,
  145, 111), and the share is within error.
- One T for three channels; a dedicated pass for an exact encoded blend.

### Verification

`lbp-bevy-viewer`, none needs game data: `d287_srgb_pair_round_trips`,
`d287_glass_blend_follows_the_encoded_blend` (exact at 0.35 and 0.75 for a grey colour; mean
error under 2 / 255 over the grid where part 37's blend is over 7, worst under 13),
`d287_glass_blend_output_is_valid` (colour at least 0, alpha in 0 to 1, a dissolved glass
leaves the room as is), `d287_the_glass_carries_its_specular_scale`, and
`d286_glass_carries_its_opacity_and_green` and the shader validation updated (the blend call,
the glass specular, no `rgb *= alpha`, the secant's numbers). fmt, the CI clippy line, the CI
test set, `cargo check` of both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets --
-D warnings` pass; the viewer's tests pass (96) but the one that fails on main
(`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
