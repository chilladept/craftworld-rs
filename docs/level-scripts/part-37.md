# Level scripts, part 37: the Pod's side walls as clear glass (2026-10-14, D286)

Thirteenth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
The owner reported that the Pod's side walls show a strange texture where they should be
clear. This part fixes that.

### What was found

- FOUND (P2j, D178): the two side panels are `pod.mol` (mesh 0x2f6c) primitive 2, gfx
  material `window_glassy` 0x1d14c: slot 0 `tissue_paper_bump` on the bump port only, a
  constant 0.118 grey diffuse, specular 0.6, alpha mode 4. Since P2j (D181, D184) the viewer
  filled slot 0 with the planets room's own wall art (0x1f3b9) by default and drew the
  panels opaque. That was a PLACEHOLDER matched by eye before the room was drawn behind the
  shell (parts 26 to 31): it matched the room seen through the glass, not a texture on it.
- MEASURED (the captured Pod frame, the glass draw; read with a script on a hidden
  desktop): blend One, InvSrcAlpha (premultiplied alpha), destination alpha kept; depth
  test LessEqual and depth write on. Bound: only the tissue-paper normal map (unit 0) and
  the transition mask (unit 5). No colour texture, no screen copy, no reflection map.
- FOUND (the program, evidence only): output alpha = 0.2 times the dissolve's kept share;
  colour = the lit grey times that alpha plus a specular term. The dissolve takes the
  constant 0.11765 (30 / 255) in place of the surface green.
- MEASURED (the captured draw): the glass's first coordinates are the mesh's UV0 (u 0.83
  to 0.95, v 0.007 to 0.90), so the mesh has UV0 even though the viewer's bucket is solid.
- INFERRED: a panel pixel is 0.8 times the room behind it plus a faint lit grey and a
  glossy sheen: clear, slightly darkened glass.

### What the viewer does

`lbp-bevy-viewer` (`main.rs`, `dual.rs`, `pod_dissolve.rs`) and one field in
`lbp-preview` (`surface::BumpMap::uv_transform`, the bump sampler's transform, kept so the
viewer can recover raw UV0).

- The default wall art is gone: `--pod-shell` adds no runtime-texture rule. An explicit
  `--runtime-texture 00002f6c:0:HEX` is still taken; `--no-pod-wall` is accepted and does
  nothing.
- The glass rule is the material GUID 0x1d14c only. Alpha mode 4's meaning was measured
  on this one draw, so it is not used as the rule; no other material changes.
- A glass bucket is drawn with Bevy's `AlphaMode::Premultiplied`, whose blend is One,
  OneMinusSrcAlpha, the captured blend. The shader writes alpha = 0.2 x the kept share and
  multiplies its colour by that alpha. The colour is the existing lit route: the 0.118
  grey with the tissue-paper bump and the 0.6 gloss (P2q). Bevy's transparent pass writes
  no depth (the game does); nothing behind the panels is lost by that.
- The dissolve's edge takes 0.11765 in place of the surface green on the glass.
- Raw UV0: the glass's bucket has no diffuse coordinates, so its UV0 is recovered from the
  bump map's coordinates (UV0 x 2, the inverse of the sampler's scale and offset) and
  given as `in.uv`; the dissolve's mask lies on the mesh's own UV, not on the 70-unit
  position tile.

### Measured (PC, release build)

Same setup as part 36. Frames in ignored `local-analysis/rscript/stage6/shots/`
(`d286-*`), compared with D285's frames (D284's for the Sackboy and pod camera stills).

- Equal byte for byte: the planets level at 600, with and without `--pod-camera`; the
  shell without scripts opened (`--pod-open`, no panel in view); in use from 600 at 640.
- Changed, the panels now clear: closed at 600 (411,472 pixels, y 0 to 635), the
  closed `--play-sackboy` still at 470, the shell without scripts (98,998), with the game
  camera (238,666) and at t 0.5 (38,379); in use from 600 at 610 (258,755) and at 615
  (55,658 under both lightings, the rows under the line).
- The walk and press run at 580 shows no panel; it equals main's build before D284 byte
  for byte, so the viewer's Sackboy's coplanar parts draw in the old order again (part
  35). In the 470 still, around him (x 590 to 690, y 500 to 640), 191 pixels differ from
  that build and 665 from D284's. INFERRED: the material layout again, not the glass.

Against the game (ignored `local-analysis/d156-rpcs3-pod-default.png`), by eye: through the
right panel the viewer now shows what the game does: the wall drawings with the dome
sketch and the ringed art, and the Sackboy statue on its table. Through the left panel
both show the room behind glass, but the viewer's view is mostly the tan hanging cloth,
where the game's shows painted wall art (a large yellow ring) with the cloth only at the
top. The game's panels are darker and tinted green to blue with a crinkled sheen. MEASURED
mean colour over the right panel: (63, 88, 92) in the game, (145, 151, 140) in the viewer
(it was (94, 103, 125) with the wall art); left (145, 150, 115) against (173, 156, 124).

### Left out

- Where the blend happens: the game blends in its encoded output (a plain RGBA8 target),
  the viewer in linear light, so the room behind keeps more than 0.8 of its display value.
  INFERRED from the shader's output gamma (P2r); not compensated.
- The specular term's own scale (x 0.6 x 6, added after the alpha in the game); the viewer
  multiplies its gloss by the alpha with the rest, so the sheen is weaker.
- Why the left panel shows the cloth where the game shows the wall art (the cloth's place
  or the room's camera).

### Verification

`lbp-bevy-viewer`, none needs game data:
`p2j_d286_the_shell_adds_no_wall_texture_but_keeps_an_explicit_rule` (the options with
`--pod-shell`, with and without `--no-pod-wall`, have no rule; an explicit one is kept),
`d286_the_glass_bucket_blends_premultiplied_at_0_2` (the glass bucket and its uploaded
material are premultiplied with opacity 0.2, a plain bucket opaque and a decal blended),
`d286_the_glass_takes_raw_uv0` (UV0 back from UV0 x 2; none for another channel, a matrix
or a zero scale; the mesh's `in.uv` is UV0), `d286_glass_carries_its_opacity_and_green`,
and `authored_shader_validates_with_every_branch_enabled` extended (the green, the alpha
and the premultiplied colour). fmt, the CI clippy line, the CI test set (612 pass), `cargo
check` of both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`
pass; the viewer's tests pass (92) but the one that fails on main
(`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
