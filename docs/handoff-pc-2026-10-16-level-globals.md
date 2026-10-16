# Handoff: level globals, part 1 (2026-10-16, PC)

Read first (max 5):
- `docs/level-globals.md` (whole file: D330 gravity and the research for the next PRs)
- `docs/level-scripts.md` part 18 (global settings blended; lines around "### Not modelled")
- `crates/lbp-runtime/src/global_settings.rs` (`SettingsInForce`), `crates/lbp-sim/src/lib.rs` (`WorldGravity`)
- `crates/lbp-bevy-viewer/src/glow_post.rs` (final pass: the colour-correction target; part 22 also edits it)

## State

- D330 gravity: branch `globals/D330-gravity`, PR opened from this session (see the thread's merge
  reply). Worktree `E:\Projects\wt-globals`, target dir `E:\Projects\target-globals`.
- Nothing running; RPCS3 closed. All LBP2 patches stay OFF.

## Facts

- Global lighting group as stored and passed: lighting, colour correction, fog, fog tint, darkness
  (PWorld target +0x420 lighting, +0x424 cc, +0x430 cc record, +0x4a0 fog, +0x4a4 fog tint,
  +0x4a8 darkness; start at +0x4b0.. +0x538; blend factor +0x5cc).
- Colour correction: 8 records of 0x70 bytes (28 floats), blended floor/fraction (0x316a7c), tables
  in BSS at 0xe31f80 / 0xe32300 (TOC 0xd9a0f8 +0x3708 / +0x3704), filled at boot. Values read from
  RPCS3 are in the scratch file `cc-records.txt` (copy kept in `local-analysis/level-globals/`).
  Per record: tint v4, (saturation, hue, contrast, brightness), a second such pair, a vec4, two
  vignette colours (record 0: 0.2 grey = part 45's floor). Final pass 0x323354 sets them by hash
  (matrices via 0xe6e04: 0x1fa21dfe, 0x98a99db9; vignette 0x78be96ff, 0xe1e37a11).
- Darkness / fog tint: 0xac6c0 returns one colour of the blended preset block (offset 0x10):
  x (1 - darkness) on masked lanes, then fog tint > 0 blends a 15- (or 7-) colour palette.
  Which field offset 0x10 is: not settled (PLevelSettings in-memory layout unread; the Pod boot
  takes over 100 s, so search memory later in a level, or read the serializer).
- Gravity: world gravity y = setting x -2.3 - 0.4 (0xcef3c, 0xd623c); normal 1.0 -> -2.7.
- Helpers in local-analysis/level-globals/: ELF reader with TOC, capstone PPC disassembler
  (`python -I`, file must not be named dis.py), RPCS3 boot-and-read script (minimized, moved to
  the 2nd monitor, never activated).

## Next

1. Colour correction in the final pass (coordinate with whoever owns `glow_post.rs`).
2. Name the darkness / fog-tint colour and apply both (relight path in `level_play.rs`).
3. Fog amount in the renderer (part P2t's `fogccparams`, `fogcol` source UNKNOWN).
4. Water (the viewer draws none).
