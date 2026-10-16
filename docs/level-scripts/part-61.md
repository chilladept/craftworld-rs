# Level scripts, part 61: steps per frame at the Pod push and a game sweep of the change-over (2026-10-15, D310)

Research only, no code change. Part 59 left the planets zoom's last tick to a guess: a frame
running no simulation step at the push. RPCS3 (01.33, Vulkan, the owner's save) was run in the
background and read in memory, and a sweep of game frames through the change-over was taken
and matched against the viewer tick by tick.

### What was measured (game memory, three pushes)

- The game object is at guest 0x3611a200 this boot (found by a memory diff of "in use" over
  two pushes and a leave, and two static pointers to it at 0xdc4344 and 0xdc7b00); its +0x1fc
  is the steps run this frame (reset at the frame's start) and +0x408 "in use". The planets
  object is the pod computer's +4 (pod computer `**0xda1870`), its +0xb4 the count of its
  per-frame call. (The TOC-relative pointers part 56's listings use did not resolve to it from
  outside; the diff did.)
- Every frame ran exactly one step: 121 of 121 frames in each of the three runs, the frame
  where "in use" turns on included. "In use" turns on at the frame-start pump (seen after the
  step count's reset and before the step), so that frame's shell step and pod computer step
  both read it. The no-step frame of part 59 is ruled out, and the VM's order
  (`LevelWorld::begin_session_tick`) is the game's.

### The change-over sweep

Ten F12 frames at 1280x720 were taken at 0.2 to 1.1 s after the press, one per push, each with
the planets' call count around it (ignored `local-analysis/d309-push/sweep/`). Against the
viewer's stills (main 6515fc4, every tick 600 to 640, hidden window, warped into the game's
framing; scripts in ignored `local-analysis/d310-changeover/`):

| Game frame | Earth radius | Zoom matches tick | Tent / dissolve matches tick |
|---|---|---|---|
| d0200 | 63.0 | 603.3 | about 603.5 to 604 |
| d0300 | 72.3 | 606.7 | about 606.7 |
| d0400 | 81.2 | 608.7 | about 609 |
| d0500 | 109.4 | 612.8 | about 613 |
| d0600 | 128.4 | 614.75 | about 615.2 |
| d0700 | 150.1 | 617.0 | about 617 to 617.3 |
| d0800 to d1100 | 166.7 to 167.1 | at rest | gone |

- MEASURED: in every game frame the zoom and the dissolve sit on the same VM tick, within about
  half a tick, at every offset. Part 52's one-tick lead of the game's zoom over the dissolve is
  not reproduced; it rested on `open-t1` alone, whose framing differs (below). The Earth's
  centres follow the viewer's path too (d0300 at (564, 387), tick 608 at (567, 386)).
- The sweep runs a steady 3 to 4 frames behind the call counts read beside it, which is the
  screenshot's own delay plus the poller's lag; the end of the zoom gives the same figure.
- These frames have no depth of field, vignette or final pass at all, even at rest (open room
  gradient 0.83, the viewer without depth of field 0.95, with it 0.50; `open-t1`/`open-t2` 0.36
  and 0.31 on the same patch, with the vignette and the smaller screen area), and the menu's
  items are drawn struck through in magenta where `open-t2` has them green. INFERRED: this
  session ran without the post chain (a setting or the offline state; not found).
- The dissolving glass (d0600 against tick 615, level lighting): the room through the glass
  band has gradient 2.12 in the game, 1.08 in the viewer with depth of field and 1.98 without;
  the band's circuit pattern is crisp in the game and absent in the viewer (its glass still uses
  part 35's position tile); the colour through it is (124, 114, 99) in the game against (85,
  79, 64), partly because the viewer's tent wall sits higher from tick 615 on. The viewer's
  depth of field changes nothing before about tick 613, so the earlier frames cannot separate
  part 57's rules, and with no post chain in these frames neither can the later ones.

### Left out

- Part 57's question (the dissolving glass drawn with the stencil rule or after the depth of
  field) needs a sweep with the post chain on, as in the `open-t1` session; what turned it off
  here.
- The band's circuit pattern on the dissolving glass and the tent wall's height from 615 on.

### Verification

Memory reads (ReadProcessMemory, no writes), F12 screenshots and background key presses only;
RPCS3's start-in-fullscreen setting was turned off for the run and restored. Frames, logs and
scripts stay in ignored `local-analysis/` (`d309-push/`, `d310-changeover/`). No code change.

Counts: +0 newly viewable / +0 newly Complete (research).
