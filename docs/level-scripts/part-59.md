# Level scripts, part 59: the checkpoint's ring colour and the planets zoom's frame (2026-10-15, D308)

Viewer change for the checkpoint ring (`lbp-preview`) and research on the planets zoom's last
tick (no runtime change). Part 58 left the checkpoint update's colour vector unread and the
infinite checkpoints' ring drawn black, unverified; parts 52 and 56 left one tick between the
game's planets zoom and the VM's at the change-over.

### What was found (01.33 executable)

- FOUND: the checkpoint update 0x5aa9c (`lbp_checkpoint_update_ring_colour`; Ghidra stops at a
  Cell `lvlx` at 0x5acb0, so 0x5acb0..0x5ad2c was decoded from the bytes) writes the whole
  editor colour word (ARGB, truncated `int(255 v)` per channel) of the shape of the checkpoint
  Thing's +0x50 Thing, and sets that shape's tint to 0xffffffff. Its eased value x (+0x18, 0.1 a
  frame) targets 1 when a global flag or the checkpoint's +0x41 is set, else 0 when the
  maximum (+0x14) is at most 0, else 1 when none is left (+0x10 = 0) on the active checkpoint
  (+0x40), else left / maximum. The alpha is `1 - 0.5 x` (part 58).
- INFERRED (strong): +0x41 is `instanceInfiniteSpawns` (0x5a038 lets a checkpoint activate
  without spawns left when it is set). So an infinite checkpoint's ring is white, active or not.
- FOUND: the RGB, with no warning, is the base colour 0xfc2d0 (`lbp_checkpoint_ring_base_colour`):
  white (1, 1, 1) in normal play; the player's or team's colour only in other game modes
  (INFERRED: team play, player colours). With a warning (+0x1c, none left on the active
  checkpoint) the ring pulses red towards (0.18, 1, 1) (INFERRED: the out-of-lives pulse).
- FOUND: on activation (`lbp_checkpoint_activate` 0x599dc) a checkpoint whose counts are -1 / 0
  gets `+0x38 x (n + 2)` of both (+0x38 the life multiplier, above 1 a double checkpoint; n a
  global count, INFERRED players). So -1 / 0 is "not yet given", not "infinite".
- FOUND: a third eased value (+0x20, 0.05 a frame, flickering on the way up with the
  `checkpoint_flicker_on/off` sounds) follows "active" and goes to a float at +0x60 of an object
  through 0xf9110 (a plain store; its other caller 0xbb088 builds that object). INFERRED: a
  light's level, not the ring's colour.
- Census (all 398 levels, 236 read whole): 1,314 checkpoints in 221 levels, 458 infinite. Counts
  are (-1, 0) in 1,087 (418 of them infinite) and equal left and maximum elsewhere. Bravery
  Test: all 15 checkpoints are infinite (14 at -1 / 0, the spawn door 4805 at 3 / 3).
- Planets zoom: the render (0x31e820, through 0x17634 and 0x3200e0) runs after the frame
  function and reads the live planets object; no copy or interpolation of the transition was
  found. The main loop is lockstep, not a fixed time step: each pass pumps, checks a cap of steps
  per frame (game +0x1fc against a maximum, 3 by default), needs a step packet from the host
  connection (`lbp_NetReceiveStepPacket` 0x28cb50) and then runs one world update; the frame
  function (the pod computer's step, the zoom's +0.05) runs once per frame unless the catch-up
  flag is set. So a frame can run 0, 1 or more steps while the zoom moves once.
- FOUND: message 0x30 goes out over the real socket to the host connection
  (`lbp_NetConnectionForMode` 0x28a834, mode 0) even for our own Pod, and is read back by
  `recvfrom` before a later pump: never before the next tick, as the VM has it.
- FOUND: the fourth argument of the planets object's slot 0x1c only lets the planet take the
  pad (0x4381d8); `UpdatePlanetsView` never sees it.

### What the viewer does

`checkpoint_light_alpha` takes x = 1 for an infinite checkpoint, and `checkpoint_editor_color`
writes the ring's RGB white (normal play) as well as the alpha byte. Bravery Test's 14 other
checkpoints' rings go from black to white.

### Measured

Release builds of main df7f53f and d308 from the same target dir, hidden window
(`scratch/d287-shot.ps1 -Prefix d308 -Level 0001e795 -NoShell -DefaultLight --play-sackboy
--play-sackboy-at ...`, list in `scratch/d308-all.ps1`, compare by `scratch/d308cmp.py`), stills in
ignored `local-analysis/rscript/stage6/shots/` (`d308-main-*`, `d308-on-*`). Sackboy's pose
differs between two runs of one build (3.7k to 4.2k px in one box), left out of the counts.

- The spawn door (4805, 3 / 3): 0 px changed; the ring was already white (2,393 px, 252.7).
- Checkpoints 1000, 1018 and 1425 (each with the infinity sign on its door): 2,203, 2,523 and
  2,182 px change, all inside the ring; its mean RGB goes from about (8, 5, 3) to (249, 249, 249).
- Checkpoints 12, 89 and 3559: no ring in view from the capture point (12 is a small gear
  checkpoint that draws no ring in either build); 0 px.
- The forced white RGB changed no other Thing.

### Left out

- The tick: INFERRED, the frame in which "in use" arrives runs no simulation step (a hitch while
  the main menu is pushed, or no step packet yet), so the pod computer's push and first 0.05
  come one frame before the shell's first step that sees it. To confirm by a memory read of the
  steps this frame (game +0x1fc) around the push, or the planets object's call count (+0xb4)
  against the shell's tick. Modelling it would run the planets level's frame part once when the
  answer arrives, before the shell's tick, behind a switch since it depends on the frame timing.
- The light level (+0x20) and what reads +0x60; the ring of an unactivated checkpoint in a game
  frame; team and player-colour modes.

### Verification

Listings and decompiles only (local, not committed); names saved in the Ghidra project
(`lbp_checkpoint_update_ring_colour` 0x5aa9c, `lbp_checkpoint_ring_base_colour` 0xfc2d0,
`lbp_checkpoint_activate` 0x599dc, `lbp_NetConnectionForMode` 0x28a834,
`lbp_NetReceiveStepPacket` 0x28cb50, `lbp_FrameRenderBlock` 0x17634). Tests:
`d308_infinite_checkpoint_ring_white`, `d307_checkpoint_light_alpha` (now RGB white). The census
was a temporary example, not kept.

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
