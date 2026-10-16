# Level scripts, part 75: the Pod main menu comes in, and the focused pill grows, as the game's (2026-10-16, D324)

Part 74 left two things of the menu's motion open: how the game brings the menu in (its draws
have their full alphas at 611, the viewer faded them) and the focused pill's changing scale and
slab step. Here the three captures taken while the menu comes in (7295, 7927, 8348: ticks 606,
609, 611) are read next to 8980 and 9601.

### What was found

- FOUND (RenderDoc replays of 7295 / 7927 / 8348 on a never-shown desktop, every draw after the
  copy back): while the menu comes in its draws go to a target of their own (15604, cleared),
  with the same blends as in part 73, and their colours and alphas are the full ones. One last
  draw (PS 3483, a full-screen square, One / OneMinusSrcAlpha on colour, One / One on alpha) then
  lays that target over the scene: the target read at the pixel and at seven taps on a ring (its
  constants 2 to 8, unit directions), the eight added and divided by 8, times the vertex colour's
  alpha. The ring's radius is 16 texels (constant 0) times the vertex colour's red. The draw is
  absent from 8980 (616) on, where the menu is drawn into the scene directly (part 73).
- FOUND: the vertex colour is (95, 0, 153, 101), (63, 0, 102, 152), (42, 0, 68, 186) of 255 at
  606 / 609 / 611: the alpha is (age + 1) / 15 within a level (age from the frame's first build at
  601, the transition time 15), the blue is 1 - the alpha and the red 0.618 of the blue. So the
  ring is 9.9 (1 - alpha) pixels of the 1280-wide picture: 6.0, 4.0 and 2.6.
- MEASURED (the target, the scene before and after the draw, `local-analysis/d324-motion/`):
  the model above reproduces the game's composite at 606 within 0.28 levels a channel (4.95 without
  the ring). At 609 and 611 the replayed composite carries square blocks over the scene that are
  not in the target (INFERRED: RPCS3 surface artifacts in the replay); there it is 7.5 / 7.4
  (12.6 / 13.6 without the ring).
- MEASURED (the draws' matrices): the focused pill's scale over the other pills' is 1.468 /
  1.571 / 1.574 / 1.511 / 1.491 at ages 5 / 8 / 10 / 15 / 18 (part 74's three plus two). A damped
  spring from 1 at age 0 to 1.5, `1.5 - 0.5 e^(-0.1869 age) cos(0.2830 age)`, fits all five within
  0.003 (INFERRED: the rule; Gooey's code was not read). The layout's 1.56 (part 70, measured on
  the drawn pill) is the rest value; the viewer scales it by spring / 1.5.
- FOUND: the eight layers of the focused pill (PS 3455) are not drawn at 606 and 609 and are at
  611. INFERRED: from the spring's peak, age 9.04. A game frame of the Moon's menu at rest
  (`pc-helpers/pod-s-sel.png`) still shows the slab: its step is held after 619 as in part 74.

### What the viewer does

- `pod_menu.rs`: the frame's fade is (age + 1) / 15. On the layer route every draw keeps its own
  alpha; while the fade is below 1 `update_layer` draws the menu into a canvas of its own and
  `MenuCanvas::compose_transition` (`menu_layer.rs`, `TRANSITION_TAPS`) lays it over the layer as
  the game's draw does, ring radius `TRANSITION_BLUR_PX` (16 x 0.618) x (1 - fade) scaled to the
  window's width. From fade 1 the menu is drawn into the layer directly, as before. The focused
  pill's size and its label grow by `focus_grow` (the spring), and its layers wait for
  `focus_slab_shown`. The UI-node route (`--no-final-pass`) keeps the per-item fade.

### Measured (viewer stills, hidden window, level lighting, in use at 600)

`local-analysis/d321-font/shot.ps1 -Tag d324` (and `-Tag d323base` with the D323 build), with
`--pod-card-pins 1e755,1e75e,1e753`, against the game's final output at scene scale
(`local-analysis/d324-motion/game_final_{7295,7927,8348}.png`; 616 / 619 as part 74), in the
menu's region (the game's menu target dilated 6 px), mean absolute difference in levels:

| tick | D323 | D324 |
|---|---|---|
| 606 (2,252 px) | 23.19 | 11.14 |
| 609 (5,901 px) | 25.07 | 15.18 |
| 611 (8,505 px) | 17.29 | 9.56 |
| 616 (29,994 px) | 17.82 | 17.88 |
| 619 (61,403 px) | 12.94 | 12.76 |

The signed mean moves from about 0 to +7 / +6 / +3 at 606 / 609 / 611: the viewer's menu now lands
brighter than the game's there (the game's pills and the viewer's sit a few pixels apart, part 70).

### Left out

- Gooey's own focus code (the spring's constants and the slab's start are fitted, not read); the
  slab step's rule (part 74's table, held after 619); the 8-bit rounding of the menu's target
  before the ring is read.
- The focus moving to another entry (the spring then starts from the new entry; no capture).

### Verification

Viewer: `d324_focus_spring_slab_and_transition`, `d324_transition_composes_the_menu_as_one`.
`cargo fmt --all --check`; `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`; the
viewer's tests pass (172) but `sackboy_play::tests::input_script_steps`, which fails on main
locally too. RenderDoc only on a never-shown desktop (0 foreground samples); nothing in RPCS3 was
started. Research files stay in ignored `local-analysis/d324-motion/`.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
