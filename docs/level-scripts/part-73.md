# Level scripts, part 73: the Pod main menu blended as the game blends it (2026-10-15, D322)

Part 72 left the menu's pills and labels brighter than the game's: the viewer drew them as
Bevy UI after the final pass, blended in linear light. Here the game's own draws are read
from the RenderDoc captures and the viewer composes the menu the same way: into the scene,
in encoded values, before the final pass, which then mixes back by the menu's alpha.

### What was found

- FOUND (RenderDoc replays of frames 8980 and 9601, ticks 616 and 619, on a never-shown
  desktop; every action after the copy back, `local-analysis/d322-blend/list.py`): after the
  anti-aliasing job's copy back (8980 eid 3299/3300, 9601 3287/3288) the scene target
  (resource 15618, R8G8B8A8_UNORM, no sRGB conversion) receives first two additive draws
  (SrcAlpha, One; PS 3429 with a texture: a blue halo round the Earth's and the Moon's rims,
  up to +52 levels over 137,683 pixels in 9601), then the menu: pills (PS 3451, 42 / 48
  vertices, curve-edged), text (PS 3445, a distance-field shader), the card and its pins (PS
  3429 textured, 3441 a nine-tap blurred alpha: the drop shadows, 3455 round shapes), all
  with colour blend One, OneMinusSrcAlpha and alpha blend One, One. Every one of those
  shaders ends in a premultiplied output (`rgb x a`, `a`). Then the final pass (8980 3789,
  9601 3777) reads that target, and a 24-index draw (3800 / 3788) puts the letterbox bars on.
  So part 51's "additive" holds for the alpha only: colour is premultiplied over, in the
  target's 8-bit encoded values; the alphas add up.
- MEASURED (the target before and after the pills, `local-analysis/d322-blend/an2.py`): an unfocused pill's body is the player's colour (150, 221,
  60) at alpha 82/255 over the scene: solved per pixel, (149.2, 220.8, 59.1) in 9601 and
  (149.2, 220.9, 59.1) in 8980, standard deviation 0.9 (the 8-bit rounding). Its text is
  white at alpha 102/255 over the pill (solved (255.0, 255.0, 255.0)); the target's alpha is
  82 on a pill and 184 (82 + 102) under its letters, 255 on the focused pill, the card and
  the pins, and only those values in quantity (9601: 14,297 / 2,538 / 25,280 pixels). The
  focused pill's text is opaque white (255 on the cyan).
- MEASURED: `local-analysis/d316-zoom/rt20_9601_up.png` and `rt20_8980_up.png` are that
  target after the menu (mean difference 0.12 from 15618 at 3743), not the final pass's output;
  part 72's game colours (pill (68, 96, 38), text (146, 164, 128)) are values before the
  final pass, and the viewer's stills are after it. The final pass's output (resource 4494 /
  4489) is the picture's 1122 x 632 viewport (x 79-1200, y 44-675) scaled down, stored top
  row first; resized back to 1280 x 720 it lines up with the target (`gfinal.py`, Me's pill
  correlation 0.95 / 0.89 at offset 0).
- With part 51's mix the game's picture over the menu is `mix((S' x 1.2998 + glow) x
  vignette, S', A)` with `S'` the scene after the menu draws and `A` their summed alpha (0 on
  the 3-D picture): a pill keeps 0.32 of its own value and takes 0.68 of the scaled one, its
  letters (A 0.72) 0.72 and 0.28, the focused pill and the card (A 1) are returned as drawn.

### What the viewer does

- `menu_layer.rs` (new): `MenuCanvas` rasterizes the menu's primitives (`MenuPrim`: a
  rectangle or a capsule, turned, a colour in encoded values with an alpha, optionally an
  image over its rectangle, bilinear, premultiplied) on the CPU, keeping both things the
  game's two blends need: the premultiplied colour composed over (and 1 - the
  transmittance) and the sum of the alphas. It becomes two textures the size of the window,
  `Rgba8Unorm` and `R8Unorm`.
- `pod_menu.rs`: with the final pass on (the level-lighting route) `paint_script_menu` places
  the menu as before but draws it into that layer: pills as capsules in the player's colour at
  82/255 (focused: (29, 220, 205), opaque), labels as the game font's raster in white at
  102/255 (focused 1), the card's frame, picture and pins at the places its nodes gave them,
  all faded as before. The UI nodes then draw nothing but the OS-font labels used when the
  game's fonts are missing. The layer is redrawn and uploaded only when a primitive or the
  window size changes. Without the final pass (`--no-final-pass`) the menu is the Bevy UI
  overlay as in part 72.
- `glow_post.rs`: `MenuLayer` (extracted to the render world) and the shader define
  `MENU_LAYER` for the final entries: `S' = layer.rgb + (1 - layer.a) x encoded(scene)`, the
  layer's summed alpha added to the scene's (0 in the Pod), `c = (S' x 1.2998 + glow) x
  vignette`, `mix(c, S', A)`. The glow's source still reads the scene before the menu, as the
  game's does. Without a layer that is on and uploaded the entries are D300's (the pipeline
  without the define).

### Measured (viewer stills, hidden window, level lighting, in use at 600)

`local-analysis/d321-font/shot.ps1 -Tag d322` (stills `local-analysis/rscript/stage6/shots/
d321-d322-tick{616,619,625}.png`, `--pod-card-pins 1e755,1e75e,1e753`), against the game's
final pass output at scene scale (`local-analysis/d322-blend/game_final_{9601,8980}.png`),
`local-analysis/d322-blend/measure.py` with the game target's alpha as masks: fill = median
over the pill's body (alpha 82, 3 px from edges and letters), text = mean of the brightest
pixels (as many as the game's letter cores), ink = the sum of luminance above the fill over
the pill (thousands; free of the game picture's resampling blur).

| tick 619 | game fill | D321 fill | D322 fill | game text | D321 text | D322 text | ink game / D321 / D322 |
|---|---|---|---|---|---|---|---|
| Community | 134, 150, 66 | 131, 155, 68 | 133, 150, 67 | 186, 195, 147 | 220, 225, 210 | 188, 198, 150 | 105 / 190 / 119 |
| Recent Activity | 113, 141, 71 | 115, 150, 73 | 116, 143, 72 | 190, 202, 154 | 220, 227, 211 | 191, 205, 154 | 180 / 263 / 172 |
| Store | 101, 134, 69 | 101, 144, 69 | 97, 132, 69 | 165, 183, 147 | 214, 222, 210 | 165, 184, 150 | 28 / 59 / 32 |
| Me | 94, 129, 55 | 99, 143, 55 | 94, 129, 56 | 160, 180, 139 | 213, 221, 208 | 160, 180, 141 | 29 / 54 / 30 |

| tick 616 | game fill | D321 fill | D322 fill | game text | D321 text | D322 text | ink game / D321 / D322 |
|---|---|---|---|---|---|---|---|
| Community | 134, 151, 69 | 129, 153, 69 | 132, 149, 69 | 183, 193, 145 | 221, 226, 210 | 191, 202, 151 | 43 / 82 / 46 |
| Recent Activity | 142, 164, 82 | 132, 161, 83 | 134, 160, 81 | 190, 202, 152 | 224, 232, 210 | 202, 221, 160 | 42 / 96 / 53 |
| Store | 106, 140, 76 | 103, 147, 77 | 100, 137, 76 | 167, 186, 151 | 214, 222, 211 | 166, 187, 156 | 14 / 32 / 19 |
| Me | 88, 125, 54 | 94, 140, 52 | 84, 124, 53 | 156, 177, 139 | 213, 221, 209 | 161, 182, 143 | 13 / 26 / 14 |

- Part 72's whiteness rule (saturation under 45, luminance over 120) at 619: Me 253 game /
  512 D321 / 288 D322 pixels, Store 543 / 1,008 / 751, Recent Activity 1,106 / 2,769 /
  1,408, Community 202 / 1,736 / 279. The letters' extra coverage was their brightness: at
  the game's colours the ink is the game's within 4 to 16 % at 619 (8 to 29 % at 616, where
  the letters are a few pixels tall).
- Text spans along the pills at 619 (`d321-font/text_span.py`) move by at most 1 px from
  part 72's (Community 147.5, Recent Activity 199.5, Me 40.0, Store 83.5; game final 148.0,
  202.0, 40.5, 83.5). The focused Story pill is drawn opaque as before: (29, 220, 205)
  against the game's (33, 211, 202) after its final pass ((38, 203, 201) before).
- Unchanged elsewhere: the tick-590 still (before the menu) is byte-identical to main's; at
  619 with `--no-final-pass` the stills differ only in Sackboy's box at the pod computer
  (x 611-694, y 625-717), as two runs of main do (130 pixels); D322's 616 / 619 stills
  differ from D321's only in the menu's boxes and that box.

### Left out

- The planets' rim halo (the two additive draws before the menu, PS 3429): not drawn.
- The card's drop shadows (PS 3441's blurred alpha) and the round draws of PS 3455 under the
  pins; the focused pill's colour beyond the player-colour entry (its shader's gradient was not
  read); the shaders' own edge rules (the pills' curve test, the text's distance-field edge)
  in place of the viewer's one-pixel soft edges.
- The fade: the viewer still scales every alpha by the frame's transition; how the game's
  draws fade was not read.

### Verification

Viewer: `d322_pill_then_text_composes_over_and_sums_alpha` (82 and 184, the over colour),
`d322_capsule_edge_and_turn`, `d322_image_lands_texel_for_pixel`,
`d322_menu_layer_shader_validates` (the define's text, D300's text without it, naga on the four
combinations). `cargo fmt --all --check`; `cargo clippy -p lbp-bevy-viewer
--all-targets -- -D warnings`; the viewer's tests pass (167) but
`sackboy_play::tests::input_script_steps`, which fails on main locally too. RenderDoc only on
a never-shown desktop (0 foreground samples); nothing in RPCS3 was started. Research files
stay in ignored `local-analysis/d322-blend/`.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
