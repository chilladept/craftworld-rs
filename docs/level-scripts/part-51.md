# Level scripts, part 51: the scene's alpha at the final pass (2026-10-14, D300)

Viewer only, no runtime change. Part 50 found that the final pass ends by mixing back to the
scene by the scene's alpha, that this alpha is 1 in the blank Moon level's frame and 0 in the
Pod's frame 5247, and left open what clears it. This part finds it and copies the rule.

### What was found

- FOUND (RenderDoc replays of frames 5247 and 14790 on a hidden desktop, every action between
  the depth of field and the final pass, alpha read per pixel): the scene surface (local
  memory 0xc1f50000) has alpha 255 on all but 319 (5247) and 330 (14790) of its 921,600 pixels
  after the depth of field, the Pod shell, the glass, the DualShock, the room, the planets and
  the clear colour alike (the rest are the lamp bulb at 64 and stars down to 20). The glow's
  source pass reads it so. Then the frame is blitted (A8R8G8B8 to A8R8G8B8, 1280 x 720, pitch
  0x1400) to a buffer in main memory (0x33200000), read back by the CPU side, and later blitted
  back over the scene surface from that buffer; between the two the glow's blurs run. What comes
  back has alpha 0 on every pixel in both frames, and its colours differ from the scene's along
  every edge of the picture (an anti-aliasing filter's mark). The final pass then reads alpha 0
  everywhere: `c` (x1.2998, the glow, the vignette) is the output.
- FOUND (14790, the Puter's menu): after the copy back the menu's pills and text are drawn over
  the scene with an additive alpha (source and destination factors one), so 55,080 pixels in the
  menu's box (x 105-805, y 104-407) reach the final pass with alpha above 0 (33,983 at 128 or
  more): the menu is mixed back towards itself and keeps its own brightness. The 3-D picture is
  alpha 0 everywhere. In 5247 the two draws after the copy back keep the destination alpha.
- FOUND (EBOOT 01.33): 0x31dfd4, called from the frame's post chain (0x31e820, at 0x31f698, after
  the glow's source and before the HUD), returns at once unless the byte 0xdb8208 is set; when
  it is set it emits exactly those blits (to the buffer at the render settings' +0x718, back from
  it), starts an SPU job on the buffer (1280, 720, pitch 0x1400) and makes the GPU wait on its
  label. The job's task set is the one 0xc0beec creates for the EBOOT's "EDGE MLAA" SPU task
  (the name string 0xcab4f0, the embedded SPU program at 0xceaf80). So the scene's alpha 0 is
  the anti-aliasing job's output (INFERRED that the job clears it: the replay's copy back carries
  the job's earlier output, see below; it is 0 in both frames).
- FOUND: the byte 0xdb8208 is 1 in the EBOOT's data and is written in only two places: the
  render call 0x3205a0 saves it, clears it for its own frame and restores it afterwards, and a UI
  page (method 0x5b54e4, page type 0x31, loading the paint shaders and frames, `paint.ff`)
  clears it while open and restores it on close (0x5b46bc). 0x3205a0 is the render used by the
  photo path (0x2ea1d4) and by 0x40d184 (a state value 2 to 4); the main loop's own render
  (0x17634) calls the post chain without touching the byte.
- MEASURED (part 50, the game's own frames): the blank Moon level paused (the RenderDoc frame:
  no copy to main memory, alpha 255, output equal to input), the Moon in play mode and Bravery
  Test at the spawn door show no x1.2998 and no vignette. With the mix FOUND, that means the
  scene's alpha was 1 at their final pass, so the anti-aliasing job did not run in those frames.
  What keeps it off in a level's play is not found (INFERRED: those frames are drawn through
  0x3205a0, the 0x40d184 path, which clears the byte; the Pod and its menu through the main
  loop's render, which keeps it).
- The replay's copy back is stale (RenderDoc does not record the CPU's writes to RPCS3's upload
  buffer in the frame): its colours are an earlier frame's filtered picture, so the replay
  cannot show the job's colour change on this frame; its alpha is the job's output either way.

So the rule: the final pass mixes back by the scene's alpha as it reaches the pass. In the
Pod's frames that is 0 on the whole 3-D picture (the anti-aliasing job ran); in a level's frames
it is the materials' own alpha, 1 on opaque surfaces, and the pass returns the scene there.

### What changed

`lbp-bevy-viewer`, `glow_post.rs` and `main.rs`:

- The final entries (`final_composite`, `final_only`) end in the game's last step, a mix from
  `c` back to the scene by an alpha `a`, saturated after the mix; where `a` is 1 the scene's
  texel is returned as it is. `a` is the scene's alpha (clamped), or 0 under the new shader
  define `SCENE_ALPHA_CLEARED` (the Pod: the anti-aliasing job's alpha). The pipeline cache keys
  on it; `EdgeGlow` carries `alpha_cleared`.
- The final pass is now on by default on the level-lighting route (a level without
  `--preview-lighting` or `--no-lbp-lighting`); `--no-final-pass` turns it off and
  `--final-pass` is still accepted (and changes nothing there; elsewhere it prints that it is
  ignored, as before). The Pod picture (`--pod-camera`) mixes back by alpha 0; every other level
  by the scene's alpha. The rule is `glow_post::final_pass_route`.
- The glow's source pass still reads the scene's own alpha (`1 - a`, part 43), as the game's
  does before the copy back: in the Pod the dissolving surfaces' alpha (D292, `dual.rs`) still
  feeds the glow while the final pass uses 0. D298's depth of field keeps the scene's alpha.
  The viewer's other draws leave alpha 1 (opaque surfaces, the clear colour, blended draws
  whose alpha factors keep the destination's 1), so a level comes back unchanged.

### Measured

d300 release build, hidden window (`scratch/d287-shot.ps1 -Prefix d300`, list in
`scratch/d300-all.ps1`), stills in ignored `local-analysis/rscript/stage6/shots/` (`d300-on-*`
default, `d300-off-*` with `--no-final-pass`); regions by `scratch/cmp293/d294cmp.py`.

- The Pod: the closed Pod at 600 and the change-over at 614 are byte-equal to the D294 / D298
  `--final-pass` stills (`d294-on-closed-tick600`, `d298-off-tick614`), and `--final-pass` gives
  the default's bytes. Against the game: the closed Pod against `d156-rpcs3-pod-default` (warped
  s 0.875, dx +1, dy +1), viewport mean |d| 19.0 without the pass -> 5.8 with it, the 4 x 4 grid
  15.9 -> 2.5. At 614 against `open-t1` (mean |d| over RGB, without -> with): DualShock body
  4.0 -> 0.1, left post card 3.8 -> 2.9, right post card 3.8 -> 2.4, wall below the window 9.3
  -> 1.1, left glass 31.9 -> 0.4, right glass 26.0 -> 3.6, room behind the window 21.1 -> 5.0.
- Da Vinci's rookie test (0001e786): the default still is byte-equal to `--no-final-pass` and to
  `d294-off-davinci-1e786` (D294's clipping, 13.9 % of pixels at 255, is gone with the game's
  rule).
- Bravery Test (0001e795, `--play-sackboy --play-sackboy-at -10765,2773,-200`, part 50): the
  default and `--no-final-pass` stills are equal outside Sackboy's own box (x 460-519, y
  376-483; 4,256 pixels differ there, as two runs without the pass differ by 4,741 pixels in the
  same box: his idle). Centre (112.8, 60.2, 20.7) and bottom-left corner (111.5, 59.1, 19.1) in
  both; D299's unconditional `--final-pass` gave (146.7, 78.3, 26.9) and (55.8, 29.6, 9.6).

### Left out

- What keeps the anti-aliasing job off in a level's play (the 0x3205a0 / 0x40d184 path is
  INFERRED); the paint page's own frames.
- The job's colour change itself (edges filtered on the SPU): the viewer has no
  anti-aliasing pass of that kind; the Pod's numbers above are without it.
- Materials that write alpha below 1 in a level (the lamp bulb's 64, glow materials): the viewer
  writes 1 on them (part 43's "texalpha in the alpha the viewer writes"), so in a level the pass
  never shows on them yet.
- The menu's own alpha in 14790 (the viewer's UI is drawn after the pass, not into the scene).

### Verification

`lbp-bevy-viewer`, no game data: `d300_final_pass_mixes_back_by_alpha` (alpha 1 the scene,
0 the D294 pass, the mix before the saturate, clamped alphas),
`d300_level_frame_unchanged_pod_frame_scaled` (an alpha-1 frame unchanged at the centre and
corners; alpha 0 gives x1.2998 at the centre and about 0.27 of it in the corner),
`d300_final_pass_default_route` (on by default on the level-lighting route, off under
`--no-final-pass` and elsewhere, alpha 0 only for the Pod), `d300_final_mix_shader_validates`
(the mix in the text with and without the Pod's define, the glow's source still reading `1 - a`,
D292's add unchanged; naga validates all four combinations) and
`d300_extracted_with_the_alpha_choice`; the D294 tests updated for the mix. fmt,
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` and `cargo check --workspace`
pass; the viewer's tests pass (129) but the one that fails on main
(`sackboy_play::tests::input_script_steps`). RenderDoc only on a never-shown desktop (0
foreground samples); nothing in RPCS3 was started or captured.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
