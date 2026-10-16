# Level scripts, part 35: the Pod's second mask, by the mesh's own UV (2026-10-14, D284)

Eleventh PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
Three leftovers of part 29 together: the card materials' second mask (`pod_mask.tex`),
the platform's edge at value 1, and the coordinates both masks are sampled at.

### What was found

- FOUND (part 29, the shader bytes): the two card materials (`pod_card` 0x2f6a and
  `pod_card_edge` 0x2fda) sample texture unit 6, `pod_mask.tex` (0x1fbc7), at TC0, and
  where its red is over one half (`m6` = 1) the edge value drops by one band. At value 1
  the rule adds one band where `m6` is 0. In the viewer's terms, with `above` the height
  over the line: `edge = (above - 100 g) / 400 + 1 - m6 + at_one (1 - m6)`, cut above 1.
  `window_glassy` has no unit 6.
- FOUND (the four captured draws): TC0 is UV0 times (1, 1) plus (0, 0): both masks are
  sampled at the mesh's stored UV0, with no transform.
- MEASURED (decoded, 512 x 512): `pod_mask` is white on 14.9 percent of its texels, in
  four shapes: two lobes and a rectangle in the upper left half of the UV square, a thin
  strip at u 0.93 from v 0.25 down to 1, and a small patch at u 0.84 to 0.86.
- MEASURED (a throwaway probe over pod.mol and the platform mesh, comparing the
  coordinate sets each draw bucket gets with the mesh's own UV channels, value for
  value): `pod_card`'s primary coordinates are UV0 unchanged; `pod_card_edge`'s primary
  coordinates are UV1 and its second sample's are UV0 unchanged, on the tent and on the
  platform alike (the platform's: u 0.848 to 0.942, v 0.268 to 0.984, as in the captured
  draw); `window_glassy` is a solid bucket with no coordinates. D278 had used the primary
  coordinates for the transition mask, so on `pod_card_edge` the circuit pattern was laid
  at UV1 x 40.
- MEASURED (the same probe, `pod_mask` at the UV0 of each vertex): on `pod_card` 637 of
  3,798 triangles are white at all three corners, all in the lower part of the tent (mesh
  height 204 to 991 of 0 to 2,573: the floor the DualShock stands on and the block beside
  it, INFERRED from the shapes); on the tent's `pod_card_edge` 570 of 8,490, all at mesh
  height 883 to 892 (the rim); on the platform's `pod_card_edge` all 291 triangles. So the
  platform's rim lies on the strip and the patch: at value 1 it is kept whole, not cut to a
  strip. Part 29's notes expected "a strip, not a solid edge"; the strip is in UV space.

### What the viewer does

`lbp-bevy-viewer` only (`pod_dissolve.rs`, `dual.rs`, `main.rs`, `thing_view.rs`,
`level_play.rs`).

- Each draw bucket records the input that holds raw UV0 (`pod_dissolve::uv0_source`):
  the primary coordinates when they are channel 0 unchanged (`in.uv`), else the second
  sample's (`in.uv_b`), else none. `pod_card` takes `in.uv`, `pod_card_edge` `in.uv_b`, the
  glass none (the position tile of D278 stays, PLACEHOLDER). Each bucket also keeps its
  material GUID, and the shell's draw entities carry it on their tag.
- The load worker decodes `pod_mask` next to the transition mask (linear, clamped, with
  mips). The shader samples it at raw UV0 with explicit gradients, `m6 = step(0.5, red)`,
  and computes the edge as above; the transition mask is sampled at raw UV0 x 40 too. A
  material without the dissolve is drawn as before.
- `thing_view::apply`: at alpha 0 the entities of the platform mesh with `pod_card_edge`
  (`LevelThings::dissolve_edge`) get value 1 and a material of their own; the platform's
  other material (0x252f3) and the tent's entities get nothing. Any other alpha takes it
  off. `level_play::follow_dissolve` gives `pod_mask` to the two card materials only and
  turns the value-1 term on at value 1.
- Printed: the mask line gives `pod_mask 0001fbc7: loaded, 512 x 512`; the first dissolve
  line the counts per input (`in.uv 1, in.uv_b 1, none (position) 1; pod_mask on 2`); a new
  line `Pod platform edge at tick 620: value 1, 1 draw entities; cut above level y -1092.5
  where pod_mask is black, 2 bands higher where it is white`.

Not done: the flat colour is still white with a player (black without), not the player's
green of part 30's stored colour; that is no constant the viewer already has.

### Measured (PC, release build)

Same setup as part 34 (1280 x 720, hidden window, `--preview-lighting` but where named).
Frames in ignored `local-analysis/rscript/stage6/shots/` (`d284-*`); the same commands with
main's build first (`d284-base-*`), which gave the d283 frames byte for byte.

- Equal byte for byte to D283's: the intro at 30 and 200; the four captures without
  scripts; the shell under scripts closed at 600; in use from 600 at 640 and the press on
  600 at 641; the planets level with and without `--pod-camera` at 600; pod.bin alone at
  640. Also equal to main's build: in use from 600 at 619 and 620, and 640 under the
  default lighting.
- Changed, in use from 600: at 610, 70,172 pixels (y 69 to 551, both sides) and at 615,
  45,856 (y 580 to 719, both sides), 46,058 at 615 under the default lighting: the band of
  the tent's card edge, whose circuit pattern now lies at UV0 x 40.
- A check build that painted every kept fragment with `m6` = 1 (not committed) gave the
  same frames at 610, 615 and 619: none of the parts `pod_mask` raises is in view there.
  At 640 it painted 146 pixels at the bottom edge (y 710 to 719): the platform's rim. It is
  kept and its colour is the lit one (the tint `1 - E + mask` with E about 0.25 is 1 or
  more wherever the transition mask is over 0.25), so the frame equals D283's.
- Changed, not by the dissolve: the closed Pod with `--play-sackboy` at 470 (856 pixels)
  and the walk and press run at 580 (1,229 pixels), on the viewer's Sackboy only (his
  zipper and two leg patches show now). MEASURED by bisecting builds: main with only a
  plain `Option<Handle<Image>>` field added to the material gives the new frame byte for
  byte; main with a plain field of the same size and no handle gives the old one; the D284
  build with main's shader and bind layout still gives the new one. INFERRED: coplanar
  parts of his model draw in another order once the material type holds one more image
  handle; nothing of the rule is involved.

Against the game (ignored `local-analysis/d156-rpcs3-pod-open-t1.png`, taken 3 seconds
before `open-t2` and not cited before): the main menu is up, the DualShock is where
`open-t2` has it, and the tent still stands in part, two posts beside the window and a
lower middle behind the DualShock, every top edge a solid green line and the walls below
it green circuit traces on dark card: the player's colour (88, 192, 14) as the flat colour.

- 619 against `open-t1`: the viewer's 619 shows no tent at all above the DualShock, whose
  top is at y 655 (the game's 572): the viewer's script camera is not yet at its open eye
  (part 31: it rests at 628). Not closer than D283's (the same bytes). The viewer's 615
  (value 0.75) is the closer frame: two posts beside the window with a lit top edge and
  circuit traces under it, but at x 0 to 360 and 940 to 1280 (the game's 350 to 420 and
  880 to 940), tops at y 580 to 650 (the game's 515), the line white, not green. INFERRED:
  the game's posts are plain tent under the line of part 29 (value about 0.7), not parts
  `pod_mask` raises, which are the floor, the block and the rims.
- 640 against `open-t2`: the viewer shows the platform's card as a band at the frame's
  bottom (y 690 to 720) and its rim in the last rows; the game a thin pale arc at y 655 to
  669. Unchanged from D283: the difference is the camera's height, not the rule.

### Left out

- The player's colour as the flat colour (part 30): the band and the line are white with a
  player, where the game has its green.
- The glass's constant in place of its colour's green (part 29 notes, at most 10 units of
  a 400-unit band); the glass keeps the position tile for the transition mask.
- The six per-backdrop masks (the script always sends 132243).
- The order the Sackboy's coplanar parts draw in.

### Verification

`lbp-bevy-viewer`, none needs game data: `d284_edge_matches_the_shader_rule` (the edge
against the game's arithmetic at values 0.05, 0.5, 0.95 and 1, `m6` 0 and 1; the cut at
value 1 at camera y - 800 + 100 g + 800 m6; the platform's measured heights cut where
black and kept where white), `d284_the_uv_source_follows_the_route` (UV0 primary to
`in.uv`, UV1 primary with a UV0 second sample to `in.uv_b`, a solid or transformed set to
none), `d284_the_dissolve_carries_the_island_mask_and_the_uv_source` (the shader's
parameters, read back and cleared), `d284_the_platform_edge_keeps_the_dissolve_at_one`
(under 1 the tent's three buckets; at alpha 0 the platform's card edge only, with a
material of its own; back under 1 it loses it in tag and material), and
`authored_shader_validates_with_every_branch_enabled` extended (the selector, the
`pod_mask` sample and the edge). fmt, the CI clippy line, the CI test set (612 pass),
`cargo check` of both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D
warnings` pass; the viewer's tests pass (88) but the one that fails on main
(`sackboy_play::tests::input_script_steps`). The runtime is untouched.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
