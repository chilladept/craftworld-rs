# Level scripts, part 58: the glow's alpha under fog, glow maps, blended draws and the spawn ring (2026-10-15, D307)

Viewer only, no runtime change. Part 54 copied the alpha a glowing material writes (`1 / (1 + 4
|g|^2)`, frame 5247's bulb and stars) and left four things open: the fog's mix of that alpha in
one listing, glow maps' alpha (INFERRED), what blended draws leave in the alpha, and Bravery
Test's spawn door, whose white ring the viewer did not draw. All four are read here; the first
three from the decoded programs and the recorded blend states, the last from the material graph,
the level data and the executable.

### What was found

- FOUND (the 7,980 decoded gmat programs, variant 8, the last write of the alpha): 524 programs
  write `tintcol.w x (a + f (1 - a))`, `a = 1 / (1 + 4 |g|^2)` (RCP of the sum) and `f =
  sat(TC3.z)`, the fog's share that also lerps the colour; so the fog lifts the glow alpha
  towards 1 as it fogs the colour. 22 programs write `tintcol.w / (1 + 4 |g|^2)` (DIV) and one
  (`burn`) the RCP alone: the fog leaves their alpha. Among those 23 are the lamp bulb
  (`light_bulb`) and the stars (`cosmos_starc`), which is why part 54's replay saw no fog in the
  alpha, and the checkpoint ring (`neon_lifebar`, below). The thing-colour listing part 54 named
  (`proc_da_vinci_glow`) is of the common form. Nothing in the stored material tells the forms
  apart: `light_bulb` and `ape_escape_hat_red_he` have the same graph (a colour box on port 0, a
  colour box on port 4), the same scalars and flags, and fall in different forms; the gmat
  revision does not split them either.
- FOUND (same programs): the glow in the dot is the whole port-4 value, a sampled map included:
  `neon_lifebar` multiplies its texture by the thing colour (COL0) and feeds the product to both
  port 0 and port 4, and its alpha dots that product (`DP3 |R1|, R1` at x4, R1 = COL0 x tex); 401
  of the common-form programs have a texture sample in the alpha's chain. So part 54's INFERRED
  glow-map rule is FOUND. The listings also show COL0 feeding the glow as it feeds the diffuse
  (`proc_da_vinci_glow`: `MOV H1.xyz, COL0`), which P3h had left out of the glow.
- FOUND (the action lists part 51 recorded from the replays of 5247, 14790 and the Moon's frame
  40, every draw with blending enabled): the glass (5247's 3033 and 2732) blends its alpha as Zero
  + One, so the alpha beneath it is kept; 2834, 3159 and 3172 (the last two the depth of
  field's composites, part 57) likewise. The
  translucent list's draws (the planets, 2910 to 3015; the menu's planets in 14790, 2821 to 2925;
  the Moon's 3903 to 4068) blend alpha as InvDstAlpha + One, `a_d + (1 - a_d) a_s`, which over an
  opaque pixel is 1 and over a glowing one raises it by the draw's own alpha. That is Bevy's One +
  OneMinusSrcAlpha for its blended pipelines, the same sum. The UI after the copy back (14790's
  pills) blends alpha One + One (part 51).
- FOUND (Bravery Test, 0001e795, the spawn door): the ring is primitive 4 of mesh 00007a06 (Thing
  4801), gmat 0000be15 `neon_lifebar`: texture 0000be14 (64 x 32, the left half black, the right
  half white) sampled at UV1 minus the thing colour's alpha (the graph's wire into the sampler's
  port 2, which the instance route already subtracts, D96), times the thing colour, on port 0 and
  port 4. The ring's UV1 u lies in 0.0017 to 0.4921, so at a thing-colour alpha of 1 (the stored
  white editorColor) every pixel samples the black half: the viewer drew the ring black over the
  door's rim, not white.
- FOUND (01.33, the checkpoint's per-frame update 0x5aa9c, listing and its constants): it eases a
  value `x` (+0x18, at 0.1 a frame) towards `+0x10 / +0x14` when `+0x14 > 0` and `+0x10 != 0`,
  else to 0 (or to 1 when the count is 0 and a flag at +0x40 is set, or a global +0x180 / a flag
  at +0x41 is), and builds a colour whose alpha is `1 - 0.5 x` (the constants -0.5 and 255 at
  0xd932b4 / 0xd932bc), stored as a byte. The level file's PCheckpoint holds spawnsLeft and
  maxSpawnsLeft (INFERRED: those are +0x10 and +0x14, the stored order). Bravery's spawn door
  checkpoint is Thing 4805 with 3 of 3 spawns, a child of the mesh Thing 4801 (the update checks
  the render mesh of its Thing's +0x50, INFERRED the parent); so `x = 1`, alpha 0.5 (byte 127),
  and UV1 minus 0.498 lands every ring pixel in the white half. The other 14 checkpoints in
  Bravery store -1 of 0 (infinite): `x = 0`, alpha 1, the black half.
- FOUND (P3a's own evidence, the 885 programs that name AlphaTestLevel and write alpha 0 below
  it): the stored AlphaTestLevel cuts out only where the program names it. `neon_lifebar` stores
  0.5 and names none; with its colour alpha at 0.498 the viewer's cut-out discarded the whole
  ring. The others write 1 or a glow alpha, which the game draws (the bulb at 64 / 255).

So the rules: a glowing opaque surface writes `a + f (1 - a)` (the 23 listed programs `a`); a
glow map is in the dot; the glow takes the thing colour; blended draws keep the glass's
destination alpha and the translucent list's sum; a checkpoint lights its parent mesh's colour
alpha at `1 - 0.5 x`; a material cuts out by AlphaTestLevel only when its program names it.

### What changed

`lbp-bevy-viewer`, `lbp-preview`, `lbp-level`:

- `dual.rs` shader: where the glow is added on the level-lighting route, the glow alpha is mixed
  towards 1 by the fog's share (`lbp_fog_share`, the same share as the colour's fog) unless the
  material is one of `dual::GLOW_ALPHA_UNFOGGED` (the 23 programs, by GUID; an unknown material
  takes the common form). The flag rides graph_info.w's bit 16 above the glow mode
  (`enable_glow(.., fogged)`); CPU mirror `dual::glow_alpha_under_fog`.
- `dual.rs` pipeline: the glass's alpha blends Zero + One (keeps the destination's), with and
  without dual sources (`glass_alpha_blend`).
- `lbp-preview` (`linked.rs`): the glow route takes the same instance colour as the diffuse (P3h
  passed none), so a glow fed by the thing colour, or by a product with it, is drawn; the stored
  AlphaTestLevel is dropped when the shader blob names no `AlphaTestLevel`
  (`surface::alpha_test_in_program`; an unread blob keeps it).
- `lbp-level` keeps PCheckpoint's spawnsLeft, maxSpawnsLeft and instanceInfiniteSpawns
  (`parts::Checkpoint`); `lbp-preview` (`level_scene.rs`) draws a Thing's render mesh with the
  alpha byte `checkpoint_light_alpha` of the checkpoint whose parent it is
  (`checkpoint_lights`, `checkpoint_editor_color`; RGB stays the stored editorColor).
- `glow_post.rs`: unchanged code; a test of the fogged alpha through the chain.

### Measured

d307 release build and main 709679f built from the same tree (`target-d307`), hidden window
(`scratch/d287-shot.ps1 -Prefix d307`, list in `scratch/d307-all.ps1`; A/B builds with one rule
off each in `scratch/d307-ab.ps1`, a local build with temporary switches, not kept), stills in
ignored `local-analysis/rscript/stage6/shots/` (`d307-main-*`, `d307-on-*`, `d307-no*-*`);
changes by `scratch/cmp293/d307cmp.py` and `d307chg.py`, the bulb by `scratch/d303/bulb.py`.

- The closed Pod at 600: 11,726 pixels change (548 by more than 8 levels), all from the glass's
  alpha (with it off the still is byte-equal to main); the fog's rule, the glow's instance
  colour, the cut-out and the checkpoint change nothing there. The bulb's rings against
  `d156-rpcs3-pod-default` (warped s 0.875, dx +1, dy +1) are unchanged (0-4 px 14.3, 8-12 px
  4.0, 32-48 px 1.7 mean |d|): it is not behind the glass. The changed pixels are the stars and
  the window behind the glass, now feeding the glow (part 51 saw the stars at alpha 20 after the
  glass): against the game, those changed by more than 1 level 7.48 -> 5.63 mean |d| (3,404 px),
  by more than 3 10.90 -> 6.97 (1,450 px), by more than 8 21.77 -> 12.72 (423 px), about +7
  levels brighter on average; viewport 4.869 -> 4.862.
- The change-over at 614: 5,603 pixels change (625 by more than 8), all in y 518-629 (the
  dissolve's line and band behind the glass panels, their alpha now kept under the glass), all
  from the glass's alpha. Against `open-t1` (warped s 0.877, dx +2): changed by more than 1 15.78
  -> 14.84 (2,388 px), by more than 8 20.80 -> 18.13 (486 px); part 51's regions unchanged
  (DualShock 0.1, post cards 2.9 / 2.5, glass 0.3 / 3.7, room 3.2), viewport 9.542 -> 9.540.
- Da Vinci's rookie test (0001e786): 482 pixels change (246 by more than 8, at most 45 levels):
  the fog's rule 289 (its glowing points darker by about 30 / 15 / 12 where fogged), the glow's
  thing colour 255 (newly drawn glows, +13 / +18 / +18), the cut-out 13, the checkpoint's light
  114 (at most 10 levels). Nothing clips (0 % at 255 before and after).
- Bravery Test (the spawn view, `--play-sackboy-at -10765,2773,-200`): the ring is drawn white
  with its gap at the bottom, as in `live-bravery-a-20261010`: 2,621 ring pixels go from (13.6,
  7.6, 3.0) to (245.7, 244.6, 244.2), 2,161 of them at 250 or more in every channel; the game's
  ring 1,176 pixels at (254.4, 254.2, 254.1) (the viewer's view is closer: the ring 140 px wide
  against the game's 102, 1.37 x, the pixel count 2.04 x). With the checkpoint's light, the
  glow's thing colour or the cut-out rule off, the ring is again black or absent. The rest of
  the still equals main outside Sackboy's own box (x 461-519, y 377-483, his idle; 3,765 pixels
  there between two runs).

### Left out

- The checkpoint's colour vector (the update lerps a fixed (0.18, 1, 1) by +0x24 and converts it;
  the conversion and the store were not read past Ghidra's undecoded VMX block at 0x5acb0): the
  viewer keeps the stored RGB. The flags at +0x40 / +0x41 and the global +0x180, and the
  flicker (+0x1c); the field mapping of +0x10 / +0x14 (INFERRED from the stored order); the
  infinite checkpoints' black ring against a game frame.
- Blended glowing draws' colour: the game's SrcAlpha is the program's alpha, which for the
  blended DIV-form programs (e.g. `proc_base_hologram`) includes the glow's division; the viewer
  blends by the material's alpha. Their alpha over an opaque pixel is 1 either way.
- The per-program form is a GUID list (decoding the program in the viewer is not done); the
  glow under a per-fragment diffuse graph (P3h, as before).
- No new RenderDoc replay was run: the blend states are the recorded action lists.

### Verification

`lbp-bevy-viewer`, no game data: `d307_glow_alpha_under_fog` (the mix, its bounds and monotony,
the bulb, stars and ring listed, `proc_da_vinci_glow` and an unknown GUID fogged, no duplicates),
`d307_glow_mode_keeps_the_unfogged_bit_apart` (the bit above the mode; the glass's Zero + One
alpha), `d307_fogged_glow_alpha_through_the_chain` (`glow_cpu`: the halo falls with the fog to
none at alpha 1, the listed form keeps it); the dual shader text test checks the fog share, the
mode mask and the mix before the alpha write. `lbp-preview`: `d307_checkpoint_light_alpha` (full
0.5 / byte 127, half 0.75, empty and countless 1, RGB kept, the parent link, the ring's UV range
in the white half at 127 and the black half at 255), `d307_alpha_test_needs_the_program_to_name_it`.
fmt, `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` (and on `lbp-preview`,
`lbp-level`) and `cargo check --workspace` pass; the viewer's tests pass (144) but the one that
fails on main (`sackboy_play::tests::input_script_steps`); `lbp-preview` and `lbp-level` pass.
Program listings, decompiles and the Ghidra reads stay local; nothing in RPCS3 was started or
captured.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
