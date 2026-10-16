# Level scripts, part 74: the planets' coronas, the pins' shadows, the card and the focused pill as the game draws them (2026-10-15, D323)

Part 73 left out what the game draws into the scene after the copy back besides the pills and
the text: two additive draws before the menu (a blue rim round the Earth and the Moon), the
pins' drop shadows (PS 3441) and the eight draws of PS 3455. Here they are read from the
RenderDoc captures (geometry, constants, textures, blends) and drawn into the menu layer the
same way.

### What was found

- FOUND (RenderDoc replays of lane B's six captures 6082, 7295, 7927, 8348, 8980, 9601, on a
  never-shown desktop; every draw after the copy back with its constants, post-transform
  vertices and textures, `local-analysis/d323-halo/probe.py`): the two additive draws (PS 3429,
  colour SrcAlpha, One; alpha Zero, One) are in all six, from 6082 (tick 600, the Pod closed,
  before "in use", no menu) to 9601 (tick 619). They are the planets' coronas, drawn whenever
  the planets are, not with the menu. Each is one square: corners (+-1.6, +-1.6) at z -0.5 in a
  frame whose z axis points at the eye (its dot with the direction to the eye 1.0000 in all
  twelve draws), the frame's unit 3632.6 world units for the Earth and 1733.6 for the Moon in
  every capture (within 0.1 %), its origin the planet's place (the Earth's 30 to 113 units
  across and 330 to 350 along from its fitted sphere's centre, part 67). Colour (0.5, 0.7, 1,
  0.5) in every capture (no fade), texture `gamedata/mesh_library/coder_art/earth_corona.tex`
  (00006993; the bound texture's top mip is that file's, byte for byte), a ring of light on
  black. The draws are depth tested: the planet's own disc hides the ring's inside (the hole's
  edge at the limb).
- FOUND: the shader's sum is texel x colour premultiplied by its alpha (0.5 x the texel's), and
  the blend takes the alpha again: + colour x texel x 0.25 at most (32, 45, 64 levels).
  MEASURED (`hole.py`, 9601): that model against the target before and after each draw, mean
  difference 0.47 / 0.41 levels over 23,169 / 65,020 pixels, ratio 1.02 (the 8-bit rounding).
  The corona's GUID is in the engine's static resource list (0xc2658c, its entry built at
  0x12668); which object draws it was not found (the colour's four floats show up only across
  the entries of an unrelated colour table).
- FOUND: PS 3441 is the pins' drop shadow, one before each pin: the pin texture's alpha at nine
  taps 0.0125 of the texture apart (weights 0.1592 centre, 0.0965 sides, 0.0586 corners), black,
  alpha 0.7 x the weighted sum (0.546 at most). MEASURED (the quads' texture coordinates, 8348 /
  8980 / 9601): the shadow's texture lands 1.0392 x the pin's size about a centre 0.0100 / 0.0112
  of the pin's side right of and below the pin's.
- FOUND: the card is three draws over one square 280.8 menu units a side: the bottom left cell of
  `gamedata/texture_library/ui/player_badge_overlays.tex` (00009c83, a dark card whose soft edge
  is the card's own drop shadow), the player's picture at (0.125, 0.039) of the square, 0.749 of
  its side (the same in 8980 and 9601), then the top left cell, the Polaroid frame (opaque over
  x 23 to 233, y 0 to 250 of 256). The card turns between captures: 4.3 degrees one way at 616,
  6.4 the other at 619 in the menu's frame (not followed; the viewer keeps part 72's 2.9).
- FOUND: PS 3455's eight draws are not under the pins: they are the focused pill. After the four
  unfocused pills, the card and "Me", "Story" is drawn as a pill in (0.2157, 0.5686, 0.8431) at
  alpha 0.8 (PS 3451), then eight times in (29, 220, 205) at alpha 0.25 (PS 3455, a curve test
  for its edge), each a step further back along the menu's z (and a little along its own axes),
  the last at the pill: a slab seen from the front. Its middle is 0.9 cyan and 0.08 blue over
  the scene, part 73's (33, 211, 202). The step is 62.8 / 44.5 / 17.7 menu units at 611 / 616 /
  619; the focused pill's own scale is 1.573 / 1.511 / 1.490 there (every other menu draw has the
  menu matrix's 3.038 view units a menu unit; the viewer's `FOCUS_SCALE` is 1.56).
- FOUND: at 611 (8348) the menu's draws carry their full colours and alphas (pills 82/255, the
  kill threshold 1.1 above every texel of the 512 x 512 mask texture): the game does not fade the
  menu's alphas in, which the viewer still does (part 70's 0.73 at 611 is not in these draws).
- FOUND: the 1122 x 632 viewport (x 79 to 1200, y 44 to 675) and the 24-index bar draw are in
  6082 too: the final pass shrinks the whole picture into it, with or without the menu
  (INFERRED: the game's screen area setting). The viewer keeps its full 1280 x 720 picture; the
  comparisons scale the game's viewport back (part 73's `game_final_*`).

### What the viewer does

- `menu_layer.rs`: `CoronaPrim` and `MenuCanvas::draw_corona`: per pixel the ray from the eye
  meets the square's plane; inside the square, unless a planet's sphere is nearer along the ray,
  colour x texel x (0.5 x texel alpha)^2 is added to the layer's colour (alpha and alpha sum
  unchanged), drawn before the menu's shapes, which then cover it as they cover the scene. The
  final pass (part 73's `MENU_LAYER`) gives `S' = layer + (1 - a) scene`: the corona added to the
  scene before the 1.2998 scale, as the game's. `MenuImage::shadow` (the nine taps) and
  `MenuImage::crop` (an atlas cell).
- `pod_menu.rs`: `PodCoronas::of` (each tick, `level_play::follow_menu`): the Things whose script
  class is `Earth` or `OrbitingMoon`, at their world matrix's place moved with the world Thing,
  with the measured unit (`CORONA_SIZES`); `paint_coronas` makes the squares facing the eye
  (pulled 0.5 units back) and takes each planet's sphere from its largest draw entity's bounds.
  The coronas are drawn with the final pass whether or not the menu is up. The focused pill on
  the layer is the blue base and the eight layers (`focus_layer_step`: the measured steps by the
  frame's age, linear between, held outside, INFERRED); each pin has its shadow first; with the
  atlas loaded the card is the back, the picture and the frame over its 280.8-unit square (the
  frame's opaque box centred on the card). `--no-final-pass` keeps part 72's overlay.

### Measured (viewer stills, hidden window, level lighting, in use at 600)

`local-analysis/d321-font/shot.ps1 -Tag d323` with `--pod-card-pins 1e755,1e75e,1e753` (stills
`local-analysis/rscript/stage6/shots/d321-d323-tick{590,600,616,619}.png`), against the game's
final output at scene scale (`game_final_{9601,8980}.png`, and `d323-halo/game_final_6082.png`
for 600), `local-analysis/d323-halo/measure.py` with masks from the game's own draws: the ring
(the coronas add 4 or more, 20 or more blue levels, outside the menu), the card's shadow (pixels
only the card's back changed), the pins' shadow fringe (changed by a shadow, not covered by its
pin), the focused pill's face (the blue base) and the slab's side (layers outside the base).
Mean absolute difference from the game (levels, RGB):

| region | tick 619 D322 / D323 | tick 616 D322 / D323 |
|---|---|---|
| corona ring, +4 (72,519 / 61,378 px) | 14.40 / 3.89 | 15.67 / 4.49 |
| corona ring, +20 (17,747 / 17,740 px) | 24.31 / 4.56 | 26.77 / 5.49 |
| card's shadow (22,310 / 10,064 px) | 10.06 / 7.84 | 11.21 / 10.45 |
| pins' shadow fringe (644 / 297 px) | 57.23 / 37.84 | 59.42 / 57.62 |
| focused pill face (7,561 / 3,447 px) | 21.34 / 16.68 | 22.77 / 16.97 |
| focused pill slab side (1,465 / 1,317 px) | 31.59 / 20.78 | 41.02 / 23.83 |

- The ring's mean colour at 619: game (184.2, 151.7, 113.5), D322 (173.1, 138.4, 96.9), D323
  (181.8, 150.6, 114.5); signed -13.6 to -0.8. The pins' fringe is a few pixels wide and the
  viewer's pins sit a pixel or two off the game's: its signed mean goes from +44.8 to +23.2
  (619) and +24.7 to -1.8 (616).
- Before the menu: at 600 (the game's 6082, closed) the ring (13,662 px) is 19.04 levels off in
  the D322 build (its 590 still, the same frame but 0.1 level) and 2.67 in D323's 600 still
  (6,091 px at +20: 30.16 / 3.40). The 590 still differs from D322's only in the two rings
  (24,449 pixels, x 487 to 772, y 244 to 446): the coronas are there in the game from 600 on,
  INFERRED at 590 too (no capture). With `--no-final-pass` the 619 still differs from D322's
  only in Sackboy's box at the pod computer (3 pixels).

### Left out

- What draws the coronas and what sets their unit (MEASURED per class, 1.12 of the Earth's
  sphere); the depth test against other surfaces than the planets' spheres.
- The rule of the focused pill's layer step and of its scale (1.573 / 1.511 / 1.490 at 611 / 616
  / 619 against the viewer's 1.56), the card's turn between captures, the shaders' edge rules.
- The fade: the game's menu draws at 611 have their full alphas; the viewer's linear fade-in of
  the alphas stays (part 70) until the game's way in is read.

### Verification

Viewer: `d323_corona_adds_and_hides_behind_spheres`, `d323_shadow_and_atlas_cell`,
`d323_focus_layer_step_by_age`. `cargo fmt --all --check`; `cargo clippy -p lbp-bevy-viewer
--all-targets -- -D warnings`; the viewer's tests pass (170) but
`sackboy_play::tests::input_script_steps`, which fails on main locally too. RenderDoc only on a
never-shown desktop (0 foreground samples in every replay); nothing in RPCS3 was started.
Research files stay in ignored `local-analysis/d323-halo/`.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
