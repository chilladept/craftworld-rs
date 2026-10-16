# Level scripts, part 57: the depth of field's stencil pass and gate constants (2026-10-15, D306)

Research only, no code change. Part 49 left two things of the game's depth of field open: which
surfaces carry the stencil bit 0x80 that its first full-size composite is drawn over, and the
code that turns the two stored depths into the gate's constants (INFERRED there from a fit).
Both are found here. Copying the first into the viewer was tried and is not kept (below).

### What was found

- FOUND (01.33 executable): the gate's constants come from 0x32820c (`lbp_DofGateConstants`),
  called by the depth-of-field function 0x32897c (`lbp_DepthOfFieldPasses`) before its first
  pass and handed to that pass's program as a four-float parameter (hash 0xa1caa5d8, through
  0x16683c; fc1 of PS 14739 in the replays). It reads the two depths `d0`, `d1` at +0x350 /
  +0x354 of the render settings and returns `A = 1.0 x 0.125 / (d1 - d0)`, `B = 0.875 - A d0`
  (the three constants at TOC +0x3e74 / +0x3e78 / +0x3e7c, 0xd9df6c..0xd9df74, read from the
  executable: 1.0, 0.125, 0.875). So `g = 7/8 + (d - d0) / (8 (d1 - d0))`: 7/8 at the first
  depth, 1 at the second, as part 49's fit said; the fit is now FOUND. It is the viewer's
  `gate(ramp)` exactly.
- FOUND: the settings it reads are those behind the pointer at 0xe3dd30 (TOC +0x3e70). 0x2e852c
  writes the two depths into the static settings at 0xe3dd40 (+0x350 / +0x354); 0x3205a0 copies
  the whole settings block into a block on its stack (the "packet" of part 49, same layout, the
  depths again at +0x350 / +0x354) and points 0xe3dd30 at it while it renders. So the packet is
  read as it is; nothing converts it. (INFERRED: outside 0x3205a0 the pointer holds the static
  block.)
- FOUND: 0x32897c has two forms. With argument 1 (the post chain, 0x31ef24; the one in both
  replays): the half-size source, two rings, then the composite twice: with the stencil test
  EQUAL 0x80 (mask 0x80) and the depth test off, then NOTEQUAL 0x80 with the depth test on
  (function 0x204) against a full-screen quad at clip z `2 d0 - 1` (read back in 14790's
  replay: 0.998533, the depth 0.999267). With argument 0 (the scene list 0x349b64 with flag
  0x200, between its opaque and translucent loops): no stencil, one composite. The one caller
  that could pass 0x200 (0x31bd28) never does on the path that reaches it (listing; the
  decompile reads it wrong).
- FOUND (0x349b64, `lbp_DrawSceneLists`, list 0): the scene is drawn in two loops over the same
  list: front to back for the opaque primitives (flag 0x40000), then back to front for the
  translucent ones (flag 0x80000). Before each draw 0x3498d0 (`lbp_SetDrawStencil`) sets the
  stencil function (ALWAYS, or GEQUAL for some materials), the reference and the write mask; in
  the second loop the reference has 0x80 added. So every draw of the translucent loop sets bit
  0x80 on the pixels it passes the depth test on, whatever their alpha.
- MEASURED (RenderDoc replays of frames 5247 and 14790 on a never-shown desktop, 0 foreground
  samples; per draw the GL stencil and depth state, and the depth-stencil buffer after each
  marking draw): the marking draws are exactly the blended draws at the end of the scene.
  - 5247 (the closed Pod): draws 2910, 2928, 2946, 2963 and 2982 (the planets in front of the
    window, Earth and the moon, and the line under the moon: 15,597 pixels), 3015 (a thin
    strip along the window frames' edges: 17,495 more) and 3033, the glass (One / InvSrcAlpha,
    the viewer's glass bucket, D286: the left and right panels, 395,246 more): 428,338 pixels marked. The glass
    writes its depth on every one of its pixels.
  - 14790 (the main menu): 2821 to 2925, the menu's sketched planets (140,877 pixels), each
    pixel with its depth written; 754 of them beyond the start of the range.
  - The source pass's depth (14736 / 14808, RPCS3's copies) equals the buffer after the glass
    on 93.6 % of the pixels (50.7 % for the one before it): the gate is taken from the
    translucent surfaces' own depth. In 5247 its alpha is 0 on all the glass's half-size
    texels.
- So the rule (FOUND): translucent surfaces write their depth, so the gate under them is theirs
  (sharp when they are nearer than the start, whatever lies behind); the first composite then
  blends the blur over them by the read-back radius without the plane test, which reaches them
  only where the radius is above 0 (their outline next to a blurred background, through the
  bilinear read-back); every other pixel is composited where the scene lies beyond the plane.
- Not reconciled: the second composite's depth function is GREATER (0x204), which would pass
  nearer than the plane, while 14790's replay changes only pixels beyond it (every unmarked
  pixel there is beyond; 49,685 change). The viewer keeps part 49's reading (beyond).
  [Corrected in part 63: RenderDoc's depth-test overlay shows GREATER passing the pixels nearer
  than the plane; the replay diffs this reading came from do not reproduce.]

### What the viewer does

Unchanged. Its glass is drawn in Bevy's transparent pass without a depth write, and the depth
of field composites it where the room behind it is beyond the start; the planets are drawn
opaque (depth written, not marked).

Tried and not kept (the diff is kept locally, `scratch/d306-glass-depth-experiment.diff`): the
glass writing its depth while the depth of field runs, a copy of the depth taken between the
opaque and the transparent pass, and the composite run without the plane test where the two
differ (the stencil pass), with a CPU mirror and tests. d306 release builds, hidden window
(`scratch/d287-shot.ps1 -Prefix d306`, list in `scratch/d306-all.ps1`, comparison
`scratch/cmp293/d306cmp.py`), main 2e209d5 (`d306-main-*`) against the experiment
(`d306-on-*`):

- The closed Pod at 600 and Da Vinci's rookie test (0001e786) byte-equal to main.
- The change-over at 614: 44,545 pixels change (2,279 by more than 8 levels), all in the lower
  part of the picture (y 492-719), most in the left glass panel (21,687 of its 29,900 in the game's frame); the
  other regions of part 51 unchanged. Against `open-t1` (warped s 0.877, dx +2): the left glass
  panel's mean colour |d| 0.30 both, its gradient game 1.50 / main 1.21 / experiment 1.79;
  viewport mean |d| 11.72 -> 11.74. In the game the room behind that panel is soft and the
  panel's circuit pattern sharp; main blurs both; the experiment keeps both sharp. So at the
  change-over the game does not take the gate from the glass there (INFERRED: the glass is
  dissolving there, part 29, and is drawn otherwise than in 5247's closed Pod; no capture of
  the change-over exists). Not kept.

### Left out

- A capture of the change-over: how the dissolving glass is drawn there (depth write, list,
  mark), and what draws the circuit pattern over the soft room.
- Which of the viewer's surfaces the game draws in its translucent list beyond the glass (the
  planets, 5247 and 14790: their outline next to a blurred background would take some blur).
- The second composite's GREATER against the replay (above); the GEQUAL materials of
  0x3498d0 (its 0x206 branch) and the stencil's low bits.

### Verification

Listings and decompiles only (local, not committed); names and plate comments recorded in the
Ghidra project (`lbp_DepthOfFieldPasses` 0x32897c, `lbp_DofGateConstants` 0x32820c,
`lbp_SetDrawStencil` 0x3498d0, `lbp_DrawSceneLists` 0x349b64; a plate on 0x3205a0). Replays
only on a never-shown desktop (0 foreground samples); nothing in RPCS3 was started or
captured. The experiment's `d306_*` tests passed before it was reverted. No code change;
`cargo fmt --all --check` passes.

Counts: +0 newly viewable / +0 newly Complete (research).
