# Level scripts, part 62: the post chain's patches and the dissolving glass's depth of field (2026-10-15, D311)

Research only, no code change. Part 61's sweep had no depth of field, vignette or final pass,
and part 57's dissolving-glass question waited on a sweep with them. The cause was found in
the emulator's settings, the sweep was taken again, and the change-over was captured on the GPU.

### Why part 61's frames had no post chain

- FOUND (lane A's `config/patch_config.yml`, enabled 2026-10-09 08:57; the definitions in its
  `patches/patch.yml`): three LBP2 01.33 game patches were on. "Disable MLAA" writes 0 to the
  byte 0xdb8208, the gate of part 51's anti-aliasing job, so the scene's alpha stays 1 and the
  final pass's x1.2998 mix and vignette never show. "Disable Motion Blur" patches 0x32897c to
  return at once: that is part 57's depth-of-field function (`lbp_DepthOfFieldPasses`), so no
  depth of field either. "Disable Score Bubble FX" returns at 0x2f4fb4. The owner has turned
  all three off for good (2026-10-15).
- So every lane A frame taken after 2026-10-09 08:57 lacks the depth of field and the final
  pass because of the patches, not the game: part 61's sweep, and any of part 50's level frames
  taken on lane A then. Lane B (`rpcs3-lane2`, OpenGL, RenderDoc) has no patch file; part 50's
  blank-Moon RenderDoc frame came from it, so what keeps the job off there stays open.

### The sweep with the post chain on (lane A, patches off)

Ten F12 frames, one per push, 0.3 to 1.1 s after it (ignored `local-analysis/d311-post/sweep/`),
matched to the viewer's ticks by the Earth's radius as in part 61 (d0500 612.3, d0550 613.3,
d0600 615.6, d0650 616.7, d0700 617.8, d0800 620.0). The depth of field and vignette are back.

- MEASURED (warped crops, `sbs-glass.png`): at 613 the left glass's circuit pattern is crisp
  over a soft room; at 615.6 the room through the band is soft (gradient 0.34 in the game, 1.08
  in the viewer with depth of field, 1.98 without). At rest the open room's patch reads 0.36 in
  the game against the viewer's 0.51 with depth of field and 0.94 without.
- The game's menu items are struck through in magenta on lane A (network disconnected) and
  green on lane B; the tent's circuit colour is the profile's colour (yellow on lane A's boot).

### The change-over on the GPU (lane B under RenderDoc)

Lane B was booted under `renderdoccmd capture` and RenderDoc's target control was triggered from
a script 0.45 to 0.9 s after each push (`rdtrig_sweep.py`; frames 7295, 7927, 8348, 8980, 9601;
lane B runs slower, so 8348 is the glass mid-dissolve and 8980 the band with the glass gone).
Replays read per draw (`ana/`, per-draw pixel diffs; full pixel history hung):

- FOUND (8348, 8980 and again 5247): the depth of field runs twice a frame. DoF 1 runs between
  the opaque loop and the translucent loop with no stencil: half-size source (2779 / 2706), two
  rings, one full-screen composite into the main target (2842 / 2769: depth test LessEqual
  against its plane, no depth write, SrcAlpha blend), then a downsample. DoF 2 is part 57's
  post-chain form (the EQUAL 0x80 composite with the depth test off at 3167 / 3094, then
  NOTEQUAL 0x80 with GREATER at 3180 / 3107). So the scene-list form of 0x32897c (argument 0,
  flag 0x200) is reached; part 57's reading that its caller never passes the flag is wrong
  (INFERRED: the listing was misread; 5247 has the same sequence at 2771 to 2834).
- FOUND (8348, the left glass at (120, 450), (150, 620), (60, 300)): the glass is the last
  translucent draw (3041: 2,121 indices, One / InvSrcAlpha, LessEqual with depth write, stencil
  Always, reference and write mask 0x80). It comes after DoF 1 and before DoF 2. Its depth there
  is 0.98664 and the stencil 0x80 (the wall behind it 0.999786, stencil 0). DoF 2's EQUAL
  composite runs over it and leaves the colour as it was ((0.510, 0.412, 0.380) both); the
  NOTEQUAL one skips it. So the glass and its circuit pattern stay sharp.
- FOUND: the room's blur is DoF 1's. The open room (644, 272), depth 0.999908, goes from (1.0,
  1.0, 0.859) to (0.620, 0.475, 0.337) at 2842; the wall behind the glass (0.99977 to 0.99979) is
  in front of DoF 1's plane and is not changed there. In 8980 the room (638, 214) is blurred by
  2769 the same way; the band (364, 678) is the 4,884-index opaque Pod mesh at depth 0.992 that
  neither composite changes (only DoF 2's NOTEQUAL composite touches its outline).
- INFERRED: DoF 1's plane lies between depth 0.99979 and 0.99988 in 8348.

So the game blurs the opaque scene before the translucent surfaces are drawn, and those are
drawn sharp over it: the dissolving glass is drawn as in 5247 (translucent, marked, writing
depth), and part 57's experiment was right about the glass but kept the room sharp because the
viewer has only the one, post-chain depth of field.

### What the viewer does

Unchanged: one depth-of-field pass after the transparent pass. The game's order (a composite of
the opaque scene beyond the plane before the transparent pass, then the transparent surfaces,
then part 57's stencil form) is the change this part points to.

### Verification

Lane A: memory reads, background key presses and F12 only; its patch file is now the owner's
(all off) and start-in-fullscreen was restored. Lane B: background keys and RenderDoc target
control; its config restored. Frames, captures and scripts stay in ignored
`local-analysis/d311-post/`. No code change.

Counts: +0 newly viewable / +0 newly Complete (research).
