# Level scripts, part 48: the glow's strength against the change-over (2026-10-14, D297)

Viewer only, no runtime change. Parts 43 and 45 left the glow's strength open: 0.7 is FOUND
in frame 5247 (the Pod closed), and `open-t1` seemed to want about 0.4 to 0.45. With the final
pass (part 45), the culled tent (part 47) and the pod computer's step (part 46) together, this
part measures it again and looks for a reason the change-over would differ.

### What was found

- FOUND (RenderDoc replays of frames 5247 and 14790, the Puter open, hidden desktop): the same
  four post passes in both (source at 640 x 360, the two blurs at 320 x 180, the final pass at
  1280 x 720), and every fragment constant of each pass equal in both frames: fc4 = 0.7, the
  blur's steps 1/640 and 1/360 (one half-size texel, 2 full-size pixels), its weights, 1.2998
  and the vignette's. The strength is not a per-frame value between the closed Pod and the
  Puter open.
- FOUND (frame 14790's own buffers): the game's horizontal and vertical blur outputs are
  rebuilt from its source buffer by the viewer's taps, offsets and weights to within 0.25 of a
  level (mean, 8-bit). The game's glow buffers are 8-bit (`R8G8B8A8_UNORM`); the viewer's are
  16-bit floats, a difference below a level here.
- FOUND (frame 14790): the final pass draws into the 1120 x 630 viewport (x 79 to 1200, y 49
  to 669) from the full 1280 x 720 scene and glow: the blur works in scene pixels, which are
  the viewer's pixels (the change-over registers at 0.877). So no resolution factor is missing.
- MEASURED (tick 614, `--final-pass`, the stills `d297-before-tick614` and `-noglow`): the
  viewer's own glow (on minus `--no-edge-glow`, median of 28 post columns) is 19.8 / 18.8 /
  15.3 / 10.3 / 5.8 at 1 / 2 / 3 / 5 / 9 game px, the shape of a 2 px line through the chain
  (model 21.4 / 16.9 / 13.0 / 8.8 / 5.4): no band term, the falloff is the blur's own.
- MEASURED: the halo over the 20-30 px baseline, game / this build: all columns 47.4 / 14.7 /
  11.2 / 5.0 / -0.7 against 46.1 / 10.7 / 7.7 / 4.1 / -3.1 (D294 alone gave 45.5 / 23.0 / 18.0 /
  12.6 / 6.8). The right post's background differs (the viewer's scene without glow falls 16
  to 20 levels toward the line there: the room behind it is not the game's yet). On the left
  post, where the viewer's background is flat (scene alone 2.4 / 0.5 / 0.8 / -1.2 at 2 to 9 px):
  game 47.4 / 18.8 / 12.3 / 6.6 / 3.2, viewer 80.0 / 18.6 / 12.0 / 9.4 / 3.9. Within 0.7 of a
  level at 2, 3 and 9 px; +2.8 at 5 px; the 1 px value is the line's own edge.
- The line's peak (median over the columns): game (120, 228, 30), viewer (115, 239, 21).
- So the excess part 43 saw was the doubled line (part 47) and the missing final pass, not
  the strength: 0.7 stays, FOUND, and part 43's 0.4 to 0.45 for t1 is withdrawn.

### What changed

`lbp-bevy-viewer`, `glow_post.rs`: no change to what runs. `GLOW_STRENGTH`'s comment carries
the second frame and the measurement; a new test checks the chain's halo on the game's line.

### Measured

Stills in ignored `local-analysis/rscript/stage6/shots/` (`d297-*`, capture script
`scratch/d287-shot.ps1`, hidden window). The build after this part draws the same frames as
the build before it (the change is comments and a test): Da Vinci's rookie test (0001e786)
and the closed Pod at 600 with `--final-pass` equal byte for byte, and the change-over at 614.

### Left out

- The game's halo at 5 px (+2.8 here) and the right post's background (the room's view).
- The source pass's other terms (the volumetric light, the side streak); the 8-bit glow buffers.

### Verification

`lbp-bevy-viewer`, no game data: `d297_line_halo_matches_t1_at_0_7` (an alpha-0 line of COL0
about 2.3 viewer px thick, the mean of a 2 and a 3 px line, through the CPU mirror at 0.7,
times the post tops' vignette 0.83, as green excess: within 20% of t1's left post at 2 and
3 px, above 0 at 9 px); `d292_blur_weights_sum_to_1_38`
still checks 0.7. fmt, `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` and the
viewer's tests pass but the one that fails on main (`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
