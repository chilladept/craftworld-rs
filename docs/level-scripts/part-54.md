# Level scripts, part 54: the alpha of glowing materials (2026-10-14, D303)

Viewer only, no runtime change. Part 51 found the scene's alpha 255 at the final pass on all
but 319 pixels of frame 5247 (the lamp bulb at 64, stars at 20) and left the viewer writing 1
on those materials. This part reads what the game's material program writes there and copies
it; the glow chain now runs on every frame the final pass does.

### What was found

- FOUND (RenderDoc replay of frame 5247 on a hidden desktop, the scene surface read after each
  draw): the bulb's pixels take alpha 64 at eid 1560, the stars' 20 at the 180-index draws
  2545 to 2689; the blend is off in both, so the alpha is the program's output. Both draws use
  the same fragment program, a lit material with a constant glow: the glow `g` (fc8, again at
  fc20) is added to the lit colour before the x4 (`tintcol`) and the square root, as the
  viewer's P3h glow is, and the alpha is `tintcol.w / (4 dot(g, g) + 1)` (fc23.w = 1, fc21.w
  = 1). The fog lerp touches the colour only.
- FOUND (the constants of those draws): the bulb's `g` = (0.6627, 0.5608, 0), so 1 / 4.015 =
  0.249, byte 64; the stars' `g` = (1, 1, 1), 1 / 13, byte 20. They are the only two alpha
  values below 255 in the frame (part 51's 319 pixels), so no other material writes alpha.
  This is the form the offline listings of the glow programs had (`DP3` of the glow at x4,
  `ADD 1`, `RCP`); the one listing that also lerps the alpha towards 1 by the fog (a
  thing-colour glow, port 4 fed by COL0) was not seen in a capture.
- FOUND (part 50's Moon replay, frame 40, list of actions): the same glow chain runs in a
  level: the source pass (eid 4192, 640 x 360), the two blurs (4237, 4269, 320 x 180) and
  the final pass (5403). So the game runs the glow on every frame, the Pod's and a level's;
  in the Moon's frame the scene's alpha is 255 everywhere, so its source is 0 there.
- So the rule (FOUND): an opaque surface with glow `g` (port 4: the stored colour, times its
  map when there is one, INFERRED for maps) writes alpha `1 / (1 + 4 |g|^2)`; the rest 1. The
  glow's source pass reads it (`0.7 x (1 - a) x rgb`, part 43), and the final pass mixes back
  by it (part 51): in the Pod (alpha 0 at the pass) the halo shows around the bulb and stars;
  in a level the halo reaches only the glowing pixels themselves, `lerp(c, scene, a)` with
  `c` the scaled scene plus the glow, and every alpha-1 pixel around them is returned as is.

### What changed

`lbp-bevy-viewer`:

- `dual.rs` shader: on the level-lighting route, where the P3h glow is added, the fragment
  keeps `glow_alpha = 1 / (1 + 4 dot(glow, glow))` (`lbp_glow_alpha`); an opaque draw writes it
  as its alpha, times the dissolve's kept share while a dissolve is on (D292). Blended draws
  keep 1, the glass its own. Without glow (or under `--no-glow`) the alpha is 1 as before.
  CPU mirror `dual::glow_alpha`.
- `glow_post.rs`: the glow's three passes now run whenever the final pass does (on by default
  on the level-lighting route, part 51), not only while a dissolve is on (`glow_runs`);
  `--no-edge-glow` still turns them off, and with `--no-final-pass` they run only with a
  dissolve, as before.

### Measured

d303 release build, hidden window (`scratch/d287-shot.ps1 -Prefix d303`, list in
`scratch/d303-all.ps1`), stills in ignored `local-analysis/rscript/stage6/shots/` (`d303-on-*`
default, `d303-noedge-*` with `--no-edge-glow`), against D300's `d300-on-*`; ring and region
numbers by `scratch/d303/bulb.py` and `stars.py` (the viewer warped s 0.875, dx +1, dy +1 onto
`d156-rpcs3-pod-default`, as part 51).

- The closed Pod at 600: 14,135 pixels change, all in x 474-864, y 217-517; the most (70
  levels) at the bulb. With `--no-edge-glow` the still is byte-equal to D300's (the alpha
  alone changes nothing in the Pod, where the pass mixes by 0). The bulb against the game,
  mean RGB in rings about its centre (game / D300 / now, mean |d| D300 -> now): 0-4 px (255,
  255, 135) / (234, 219, 97) / (248, 235, 123), 31.8 -> 13.1; 4-8 px 34.5 -> 14.8; 8-12 px
  (198, 161, 101) / (171, 129, 87) / (198, 154, 97), 24.4 -> 3.7; 12-16 px 18.5 -> 8.4; 16-24
  px 10.9 -> 8.9; 24-32 px 3.0 and 32-48 px 1.4 unchanged. The stars and the rest: 2,456
  pixels change by more than 1 level outside 48 px of the bulb (box x 507-806, y 253-447,
  +2.6 / +2.1 / +1.5 on average); their mean |d| to the game 14.21 -> 12.95. Whole viewport
  4.89 -> 4.87.
- Da Vinci's rookie test (0001e786, level lighting, final pass on): 127 pixels change (126
  with `--no-edge-glow`), at most 45 levels, all brighter: small glowing points (e.g. (169,
  233, 239) -> (191, 255, 255)); every other pixel equal to D300's.
- Bravery Test (0001e795, part 50's spawn view): equal to D300's outside Sackboy's own box
  (4,235 pixels there, his idle, as between two D300 runs), so no glowing draw changed in
  the viewer's view. The game's frame shows a white ring around the spawn door's face that
  the viewer does not draw at all (not a glow alpha matter; not followed here).
- The change-over at 614: the stills differ from D300's over the whole frame with and without
  the glow (818,508 / 874,454 pixels, mean about -2 in red): INFERRED part 52's planets zoom a
  tick earlier (main moved under D300's stills), since the glow alpha alone leaves the closed
  Pod byte-equal.

### Left out

- The fog's lerp of the alpha that the COL0-glow listing has (not in a capture); glow maps'
  alpha (the textured glow in the dot, INFERRED).
- Blended glowing draws (the viewer keeps 1 there; the game's blend factors for them were not
  read), and the glow skipped under a per-fragment diffuse graph (P3h, as before).
- Part 51's "Next" item on the glow alpha is done here; the spawn door's white ring in
  Bravery.

### Verification

`lbp-bevy-viewer`, no game data: `d303_glow_alpha_matches_the_game_bytes` (the bulb's glow gives
64, the stars' 20, none 255; monotonic), `d303_glow_runs_with_the_final_pass` (the glow with
the final pass or a dissolve, never under `--no-edge-glow`; the composite is then
`final_composite`), `d303_glowing_surface_through_the_chain` (`glow_cpu` on a 4 px glowing
square of alpha 1/13 in an opaque frame: a halo 2 px outside it, none 30 px away; the final pass
returns an alpha-1 neighbour as it is, brightens the square, and shows the halo with the
Pod's alpha 0); the dual shader text test checks the glow alpha's line, its write before the
dissolve's, and the dissolve's `dissolve_keep * glow_alpha`. fmt, `cargo clippy -p
lbp-bevy-viewer --all-targets -- -D warnings` and `cargo check --workspace` pass; the viewer's
tests pass (132) but the one that fails on main (`sackboy_play::tests::input_script_steps`).
RenderDoc only on a never-shown desktop (0 foreground samples); nothing in RPCS3 was started
or captured.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
