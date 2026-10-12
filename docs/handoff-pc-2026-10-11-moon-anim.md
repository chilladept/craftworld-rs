# Handoff: Moon movement and Sackboy animation (2026-10-11, PC session)

Read CLAUDE.md, then this file. Thread: "Moon movement 1:1" in the Claude Project.

## Done (all merged to main)
- #124 gravity/air drag/landing dip, #125 vertical drag + gravity 2.698 (no terminal clamp, 173.7/tick
  reached), #126 own mesh 0x43f + 21-clip bank, #127 EBOOT layer rules + weighted blending + facing fix
  (root turn held to bind, mirror applied), #128 Ghidra pass-2 values (idle odds 0..249, layer order,
  300-tick idle gate, seed 42, blink at 0). Tables: docs/d230-moon-movement.md, docs/d232-sackboy-anim-rules.md.
- Moon measurements (per tick, RPCS3): local-analysis/d230-moon/*.csv, scripts d211-moon/scripts/run2.py
  (POS env = Sackboy row; this boot 0x34188580). Clips: local-analysis/sackboy-clips-2026-10-11b/out/*.mp4,
  published in the screenshot artifact (https://claude.ai/artifact/UfbA32sCfkeRtnUDCJiy9u).
- Ghidra reports: local-analysis/results/2026-10-11-ghidra-sackboy-{movement,anim-rules,anim-open-items}.md.

## State
- RPCS3 (pid from `Get-Process rpcs3`) is running LBP2 on the owner's Moon in create mode, Sackboy
  standing near x -7700. Owner granted focus use for RPCS3 input on these jobs (2026-10-11). Never save.
  Window capture (ImageGrab/PrintWindow) freezes; use F12 screenshots (RPCS3\screenshots\BCUS98245,
  about 1 per second) via `scratchpad f12.py` pattern: pad.hold('f12') then crop.
- Hover: D-pad Down toggles; W flies up (reached y 7572 in ~5 s).

## Open (priority order)
1. Emotes / mood stands and head look: traced in the EBOOT (0x203cdc, 0x2022b8, 0x201fb8, 0x2007e4),
   not implemented. Arms pose mapping and face-morph readers not traced.
2. Slopes and walls: Ghidra slope limit ~55 deg (cos 0.5735), blank Moon has none; needs a level with
   geometry (tutorial/story level data) before measuring. Controller moves-and-stops, no slide.
3. Camera lead in walk direction (BiasVector/TransientBiasX strings, no readers found).
4. Crouch spring input (overlap depth/350, ground path), mirror maths (SPU job), GroundTurnSpeed readers.
5. Blank-level spawn is 136 units inside the floor in the Rookie offset path (fixed in #127 for checkpoint
   spawns); backdrop in blank level is black in our viewer.
6. No RPCS3 side-by-side beyond one full-speed run pose (matched; game leans a bit more).
