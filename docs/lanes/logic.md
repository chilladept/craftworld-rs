# Lane state: logic (the game's gadgets and circuits) (2026-10-21)

## Read first
1. docs/logic.md: Part 1 (census of the Rookie Test's logic and the fix order), Part 2
   (anti-gravity tweakers, what the EBOOT does)
2. crates/lbp-sim/src/scene.rs (`AntiGravity`, the tweak pass in `build_scene`) and
   lib.rs `spawn_scene`, `follow_tweaks`
3. crates/lbp-runtime/src/switches.rs module doc (what the switch pass does not model)
4. docs/logic.md Parts 4-49 and docs/logic/part-50.md to part-82.md (touching layers; pistons and their mode rate; layer switch; recorded moves; attractors; aim stick and lead; winches; motors; gyroscopes and rotators;
   movers and followers; tags; every switch type; the random, impact, camera finished,
   magic mouth, projectile, paint and velocity switches)

## State
- Owner (2026-10-19): "i think we might need to work on getting the games logic and
  circuits accurate in order to go further". The level1 lane is paused for this one.
- Owner (2026-10-19, later): "would it not be better to just work on logic on its own as
  a whole and then come back to trying to play levels". So the lane works on logic as a
  whole: every switch, gate and output kind, checked against the EBOOT's rules and a
  census across all levels, each with its own unit tests. The Rookie Test route is only a
  check, rerun and recorded at the end of each PR, never a goal; no level-specific gaps
  are chased. Playing levels comes back once the logic set is complete.
- Merged: D1100 (PR 241) to D1109 (PR 250, b351ff1); D1110 (PR 251, da39db8: movers and
  followers); PR 252 (849bf6a: follower rows in game units per tick).
- Merged: D1111 (PR 253, e164663): keyed followers follow tags.
- Merged: D1112 (PR 254, 5c1ef1c): the switch census (Part 14) and the random switch.
- Merged: D1113 (PR 256, 1acd570) impact, D1114 (PR 257, e44851a) camera finished,
  D1115 (PR 258, 9c12aea) magic mouth, D1116 (PR 259, a2ad940) projectile and paint.
- Merged: D1117 (PR 260, d94b874) the velocity switches.
- Merged: D1118 (PR 261, 4c34a07) the grab switch.
- Merged: D1119 (PR 262, 1aa3098) the control pad switch.
- Merged: D1120 (PR 263, 4242f1f) the key remote switch.
- Merged: D1121 (PR 264, 5f2ac7c) the button switch.
- Merged: D1122 (PR 265, e504be1) the motion recorder switch.
- Merged: D1123 (PR 266, 8a1e600) the angle sensor switch.
- Merged: D1124 (PR 267, bdd1329) the wormhole switch.
- Merged: D1125 (PR 268, 0f11073) the pocket item pedestal switch.
- Merged: D1126 (PR 269, 1f4c887) the lever switch.
- Merged: D1127 (PR 270, 5a9b6d8) the three-way switch.
- Merged: D1128 (PR 271, 3fdf7e2) the sticker switch.
- Merged: D1129 (PR 272, a2b3b59) the score switch.
- Merged: D1130 (PR 273, e480618) the movinator pad.
- Merged: D1131 (PR 274, 3fbcd7f) switch type 55.
- Merged: D1132 (PR 275, ae24c2d) switch inputs.
- Merged: D1133 (PR 276, 91e4066) the material tweaker.
- Merged: D1134 (PR 277, bd6deb2) the emitter.
- Merged: D1135 (PR 278, 0d8f6d4) the sequencer.
- Merged: D1136 (PR 279, af19bb7) the controlinator.
- Merged: D1137 (PR 280, 922909f) the creature.
- Merged: D1138 (PR 281, 9fb266e) getting in and out of a controlinator.
- Merged: D1139 (PR 282, 5593712) player death and checkpoint lives.
- Merged: D1140 (PR 283, 4ee668f) a destroyed controlinator kills its driver.
- Merged: D1141 (PR 284, 4976a39) the tag and player sensors' cone and layer tests.
- Merged: D1142 (PR 285, 3e5b6eb) the grab and impact sensors have no cone or layer test.
- Merged: D1143 (PR 286, f32c181) the chip-table angle of a Thing without a PPos.
- Merged: D1144 (PR 287, 78d3e4e) turning tweaks 3, 4 and 5.
- Merged: D1145 (PR 288, f34c212) the layer switch, configuration 12.
- Merged: D1146 (PR 289, c240604) the recorded move player, configuration 13.
- Merged: D1147 (PR 290, 5407aee) the attractor tweak, configuration 14.
- Merged: D1148 (PR 291, ee35293) the aim tweaks' stick, flip and lead.
- Merged: D1149 (PR 292, c3eecb5) the exact turn speed of 0x1c21f4.
- Merged: D1150 (PR 293, 4d67b8a) the followers' speed through 0x1c21f4.
- Merged: D1151 (PR 294, 269a784) the followers' up axis, localSpace and allowInOut.
- Merged: D1152 (PR 295, 82767b3) the followers' tag key colour is an s32.
- Merged: D1153 (PR 296, 3e16c77) touching layers do not collide.
- Merged: D1154 (PR 297, baf0614) followers measure from their parent's point.
- Merged: D1155 (PR 298, 5b298d1) a tag switched off by its input cannot be followed.
- Merged: D1156 (PR 299, f0bacdd) aiming spins measure from their parent's point.
- Merged: D1157 (PR 300, 69f4ca5) tag synchronisers send a tag's offset and receivers
  place a tag (Part 59; corrects Parts 57-58: list +0x2b8 is the tag synchroniser).
- Merged: D1158 (PR 301, 97fc7d2) aiming spins (configuration 4) and stick movers
  (6, 7) read their stick from input ports 1 and 2 (Part 60).
- Merged: D1159 (PR 302, e512f30) spin and move tweaks on a microchip take their turn
  on the board, +0x220 and +0x224 from 0xcd2e0 (Part 61; corrects Part 50).
- Merged: D1160 (PR 303, 2abbacc) a mirrored body negates its movers' localSpace
  angle and, XOR +0x224, their stick x (Part 62).
- Merged: D1161 (PR 304, 48a8f07) the input scale 0xcbed4 scales movers and caps
  followers and spins (Part 63).
- Merged: D1162 (PR 305, 80d23f1) a deactivated tweak reads activation 0 (0xcaf9c,
  Part 63 addendum).
- Merged: D1163 (PR 306, 3d20237) a follower packs its cross row even when decel ×
  velRange is 0 (0x1c81a4; Part 64, corrects Part 12).
- Merged: D1164 (PR 307, cfd4edc) configuration 5 leads a tag on a moving body and
  passes its lead rate to the shaper (Part 65).
- Merged: D1165 (PR 308, 0560b46) configuration 5 leads the player by Sackboy's
  velocity (Part 66).
- Merged: D1166 (PR 309, 9b89c24) allowInOut followers move their group to the target's
  layer (Part 67).
- Merged: D1167 (PR 310, 44c20da) a receiver's second source 0x2a8538 is online-only
  (Part 68).
- Merged: D1168 (PR 311, 64173b1) useLayers puts a received tag on the sent tag's z
  (Part 69).
- Merged: D1169 (PR 312, 423bbb6) a piston holds its start angle; the solver's type
  table confirms the type names (Part 70).
- Merged: D1170 (PR 313, d24f248) a direction piston stops when a signed source is off,
  by the port test 0x900730 (Part 71).
- Merged: D1171 (PR 314, dc057a1) a motor off its rate path never runs past its
  target, the position term of row 0x95f494 (Part 72).
- Merged: D1172 (PR 315, f88998e) a speed motor turns at its speed in every mode, and
  wired in mode 2 stops after one turn (0xc7370, Part 73).
- Merged: D1173 (PR 316, e7ad70e) a motor pushes by its strength clamped to [0, 1]
  (Part 74).
- Merged: D1174 (PR 317, 6904f3b) the solver's motor case 0x1c9eac gives k = 5 ×
  strength for pattern 2 only and angle limits (Part 75).
- Merged: D1175 (PR 318, dd6d102) a pattern-0 motor is held to its sweep, the game's
  centred motor rule 0xc95ec (Part 76).
- Merged: D1176 (PR 319, f5bedbe) a piston or motor wired in mode 2 runs its pattern
  once with no phase (Part 77).
- Merged: D1177 (PR 320, 1d53363) a pattern-2 motor is held to its sweep (Part 78).
- Merged: D1178 (PR 321, 2027284) the motor row's second edge, 1.5 × err capped by
  k × max(|err|, 0.1) (Part 79).
- Merged: D1179 (PR 322, 474113e) a motor follows the row's band, braked past it only
  for pattern 2 (Part 80).
- Merged: D1180 (PR 323, e176509) the piston's length row 0x96178c has the motor's
  band, k cap floor 10 (Part 81).
- Merged: D1181 (PR 324, 1d4dbdf) a piston slides by its band, limits from its rule's
  range (Part 82).
- Merged: D1182 (PR 325, 9963f58) the stiff flag (+0x28) picks the band row 0x962590
  (Part 83).
- Merged: D1183 (PR 326, 9055249) the non-stiff row 0x960988 has the same band
  (Part 84).
- Merged: D1184 (PR 327, aad28ee) a non-stiff piston's extra rows hold its angle
  (Part 85).
- Merged: D1185 (PR 328, d93c4eb) the byte at +0x29 is editor-only (Part 86).
- Merged: D1186 (PR 329, 37e3ddd) a non-stiff piston's angle rows are hard (Part 87).
- Merged: D1187 (PR 330, 1021384) a piston's rule and limits start from its saved
  length (Part 88).
- D1188 (branch `logic-d1188`, worktree `E:\Projects\wt-d324`): a piston with a
  strength slides with or without a k (docs/logic/part-89.md). Route and parity PENDING
  (PC locked, viewer DeviceLost at about tick 810, control on main too).
- Decision numbers: D1100-D1199, next free D1189.

## Facts
- Route run: as docs/lanes/level1.md Facts, with `--native-scripts`; captures under
  local-analysis/logic/ (a new name each run). Before D1100 Sackboy stopped at x -385;
  after it he stands at x 22,785 on tick 2012 (x 22,770 on tick 2005 after D1102;
  after D1103 x 22,059: he waits about 25 ticks longer near x -3,000, and the inputs end
  at tick 1970; after D1104 x 25,657 on tick 2006; after D1105 x 25,645;
  after D1108 x 24,635 on
  tick 2008: the wobble bolt on microchip 57 swings at once, Part 10; after PR 252
  x 25,657 on tick 2003; after D1111 x 24,464 on tick 2005; after D1112
  x 24,464 on tick 2005, unchanged; after D1113 x 24,464 on tick 2005,
  unchanged: nothing fills the contacts yet; after D1114 x 24,464 on tick 2005,
  unchanged: no cutscene camera on the route;
  after D1115 x 24,464 on tick 2004, unchanged;
  after D1116 x 24,464 on tick 2004, unchanged;
  after D1117 x 24,464 on tick 2004, unchanged: nothing fills the bodies yet;
  after D1118 x 24,464 on tick 2005, unchanged: nothing fills the grabs yet;
  after D1119 x 24,464 on tick 2005, unchanged: nothing fills the pads yet;
  after D1120 x 24,464 on tick 2004, unchanged;
  after D1121 x 24,464 on tick 2004, unchanged: nothing fills impulses yet;
  after D1122 x 24,464 on tick 2004, unchanged: nothing writes the word yet;
  after D1123 x 24,464 on tick 2004, unchanged;
  after D1124 x 24,464 on tick 2004, unchanged;
  after D1125 x 24,464 on tick 2005, unchanged; three earlier runs of the
  same build lost the GPU device near tick 675, cause not found, and no
  type 65 switch is reached on the route;
  after D1126 x 24,464 on tick 2004, unchanged;
  after D1127 x 24,464 on tick 2005, unchanged;
  after D1128 x 24,464 on tick 2005, unchanged;
  after D1129 x 24,464 on tick 2004, unchanged;
  after D1130 x 24,464 on tick 2004, unchanged;
  after D1131 x 24,464 on tick 2005, unchanged;
  after D1132 x 24,464 on tick 2005, unchanged;
  after D1133 x 24,464 on tick 2004, unchanged; a GPU device-lost crash near
  tick 680 hit 5 of 7 runs, the D1132 build too;
  after D1134 x 24,464 on tick 2004, unchanged, no crash;
  after D1135 a GPU device-lost crash near tick 685 in 4 of 4 runs, main
  without D1135 too; after D1136 the same crash near tick 682 in 3 of 3 runs,
  main without D1136 too; after D1143 to D1148 x 24,464 on tick 2005, unchanged;
  after D1149 the GPU crash near tick 685 in 4 of 4 runs, with Sackboy at
  x -11,704.3 on tick 680 as on the D1148 build; after D1150 the same crash, tick 680
  unchanged; after D1151 the crash near tick 688 in 1 of 2 runs, and the run that
  finished stopped at x 910.8 from tick 1111 (r held, Sackboy standing) to tick 2005
  instead of x 24,464, the same with or without allowInOut followers, so from D1149 or
  D1150; run with `$env:WGPU_BACKEND="dx12"` to avoid the device-lost crash; after
  D1152 follower 2499 finds its tag and Sackboy stops at x -2,467.6 from about tick
  1,000, ending at x -2,053.9 on tick 2,007, blocked by lift 2643, follower 2652 key 35,
  jammed against gyroscope 1320, Part 54); after D1153 the lift leaves and Sackboy
  stands at x -1,092.7, y 5,238.4 on ticks 2,006 to 2,008, inside the xy range of
  body 1951 (3 movers, 3 spins), Part 55); after D1154 body 1951 holds still, but
  lift 2643 rises to its y-5,539 tag at level start and Sackboy falls at x -2,870.3 on
  tick 1,000, ending at x -2,450.6, y 1,725.8 on tick 2,007 (Part 56); after D1155
  lift 2643 no longer sees its switched-off tags and Sackboy no longer falls: x -2,870.3,
  y 5,188.6 to about tick 1,400, then x -2,440.6, y 5,400.9 to tick 2,007 (Part 57).
  After D1156 to D1187 unchanged (Parts 58-88; D1188 PENDING; D1163 changes 2 Rookie Test
  followers, none on the route; D1164 and D1165 change no Rookie Test aimer; D1166 is not hosted).
- Switch census (Part 14): `level_scripts` example with `--all` over the 01.33 catalog
  into local-analysis/logic/census-all-1.json (378 levels, 358 read, 141,572 switches);
  census-all-2.json adds the grab counts (Part 20), census-all-4.json the pads
  (Part 21), census-all-5.json the key remotes (Part 22),
  census-all-6.json the buttons (Part 23),
  census-all-7.json the motion recorders (Part 24),
  census-all-8.json the angle sensors (Part 25),
  census-all-9.json the wormhole switches (Part 26),
  census-all-10.json the pocket item pedestals (Part 27),
  census-all-16.json the levers and three-way switches (Part 28),
  census-all-17.json the three-way switches (Part 29),
  census-all-18.json the sticker switches (Part 30),
  census-all-19.json the score switches (Part 31),
  census-all-20.json the movinator pads (Part 32),
  census-all-21.json switch type 55 (Part 33),
  census-all-22.json the switch inputs (Part 34),
  census-all-24.json the material tweaks (Part 35),
  census-all-25.json the emitters (Part 36),
  census-all-26.json the sequencers (Part 37),
  census-all-27.json the controlinators (Part 38),
  census-all-28.json the creatures and enemies (Part 39),
  census-all-29.json the controlinator seats (Part 40),
  census-all-30.json the checkpoint lives (Part 41),
  census-all-31.json the chip-table cone walks and cursor chips (Part 45),
  census-all-32.json the PPhysicsTweak configurations (Part 46),
  census-all-33.json the recorded move players (Part 48),
  census-all-34.json the attractor blocks (Part 49),
  census-all-36.json the mirrored aim tweaks (Part 50),
  census-all-38.json the turning tweaks' ticks to full speed (Part 51),
  census-all-39.json the followers' ticks to full speed (Part 52),
  census-all-40.json the followers by allowInOut, localSpace and allowUpDown (Part 53),
  census-all-42.json the followKey followers with a matching tag (Part 54);
  local-analysis/logic/d1153/census-all.json (`level_physics --all`) the touching
  layer pairs (Part 55); local-analysis/logic/d1154/census-all.json the followers
  and their offset search points (Part 56); local-analysis/logic/d1159/census1.json
  the tweaks turned or flipped on their chips (Part 61);
  local-analysis/logic/d1160/census1.json the movers mirrored or flipped (Part 62);
  local-analysis/logic/d1161/census1.json the input-scaled tweaks (Part 63);
  local-analysis/logic/d1163/census1.json the followers with decel × velRange 0 (Part 64).
  local-analysis/logic/d1164/census1.json the aimers that lead a tag on a body (Part 65).
  local-analysis/logic/d1165/census1.json `chip.aim_player`: 67 keyless aimers, which
  lead the player (Part 66).
  local-analysis/logic/d1166/census1.json `chip.follow_in_out`: 149 allowInOut
  followers in 29 levels (Part 67).
  local-analysis/logic/d1167/census1.json `receivers_unsent`: 79 of 82 receivers in 7
  levels have no sender of their key in their level (Part 68).
  local-analysis/logic/d1168/census2.json `receivers_use_layers` 20 in 4 levels,
  `receivers_layered` 0 (useLayers with a sender in the level) (Part 69).
  local-analysis/logic/d1170/census1.json `joint type T mode M input signed S`: 3 mode 1
  pistons and 15 mode 1 motors have a signed source, none in the Rookie Test (Part 71).
- Census: a scratch example (not committed) prints counts (`WIRED=1` lists switch
  targets by gadget); the PPhysicsTweak histogram by configuration is in docs/logic.md
  Part 2, the wiring in Part 3.
- Piston length 0xc95ec (types 3, 4, 9 at 0xc9790), wave 0xc79c8; PJoint motion at
  +0xc8..+0xd8 (Part 4). A piston scratch dump prints each piston's motion and input.
- Tweak activation getter 0xcaf9c (Part 3): unwired 1, digital = ternary != 0, analog =
  |activation|.
- EBOOT 01.33: PPhysicsTweak part bit 30; activation getter 0xcaf9c; body update 0xc5f1c
  (gravity scale clamp(1 - Σ gravity × activation, 0, 1) at 0xc6308); SPU packer 0x1c7ed0
  (configurations 1-5 and 9 via 0x95fb20; 6-8 and 11 via 0x957df4).
- Row packer 0x95fb20 (Part 11): configuration 1 is the gyroscope (aims at world up),
  2 and 9 rotators, 3-5 aim along velocity / stick / target (dead zone x² + y² < 0.1,
  Part 46). Row: target turn speed
  +0x20, limits +0x24/+0x28 = ±accel × velRange, × activation × 1/mass (0x95fdc8).
  Speed shaper 0x1c21f4 with root helper 0x1c2128 (Part 51). Followers (11) and 6-8
  via 0x957df4 (Part 12); followers call 0x1c21f4 at 0x9586cc (Part 52); +0x5c localSpace,
  +0xe4 allowInOut (layer move 0x1c8250, then packed; Part 67), +0xe5 allowUpDown (0x9587ac) (Part 53); tag
  search 0x1c3e04 and map 0xceb8c (Part 13); followKey match 0xceac0, colour reader
  0xc1dc90 -> 0x8a9240 (zigzag LEB128 when compressed) for both tag and tweak (Part 54).
  Tag search 0x1c3e04 has no tag activation test; 0x12f6f4 rejects a tag whose Thing
  +0x7f is set, 0xf9a64 is a mode/player check, not a layer test (D1153). PShape
  thickness +0x7c, massDepth +0x70, serializer 0xb41534; the pairwise depth test is NOT
  FOUND (SPU ELFs at file offsets 0xcb1080-0xd43780) (Part 55). Followers search
  from their Thing's parent (+0x5c, else the Thing), its translation at
  `*(*(thing+0x2c)+0xc)+0x70` (0x957df4; spins 0x95fdc8 the same); 0x88f7dc ->
  0x550e70 is a player check on a global's +0x180 (Part 56).

## Subagent models
Owner rule (2026-10-21): subagents use haiku for browsing and narrow jobs, sonnet for
contained edits, Opus only when needed. One line per PR: each subagent's model, task,
tokens, tool uses, wall-clock minutes and whether its result was wrong or redone (also
when a later PR finds it), then the thread's own context at hand-off (ctx_audit).
- D1165: none (all steps ran in the thread); thread context 104k at hand-off.
- D1166: haiku general-purpose (disassembly of 0x1c8250), 39,849 tokens, 7 tool uses,
  0.7 min, used as is, not redone so far; thread context 99k at hand-off (after a
  compaction).
- D1167: haiku general-purpose (decompile 0x29ff3c, 0x2a8538), 40,481 tokens, 9 tool
  uses, 0.7 min, used as is, not redone so far; D1166's result still not redone; thread
  context 89k at hand-off.
- D1168: haiku general-purpose (decompile sender 0x13187c and 0x131638, the offset's
  lanes), 90,238 tokens, 6 tool uses, 3.5 min, used as is; D1166's and D1167's results
  not redone; thread context 132k at hand-off.
- D1169: haiku general-purpose (resolve 0x962590, 0x960988 and the type 4 case),
  43,286 tokens, 12 tool uses, 1.1 min, FAILED: resolved none of the questions (the
  Ghidra bridge cannot decompile or disassemble mid-function addresses), redone by hand
  with read_bytes and capstone; earlier results not redone; thread context 98k at
  hand-off.
- D1170: none (all steps ran in the thread); earlier results not redone; thread context
  104k at hand-off.
- D1171: none (all steps ran in the thread); earlier results not redone; thread context
  115k at hand-off.
- D1172: none (all steps ran in the thread); earlier results not redone; thread context
  at hand-off in the PR reply.
- D1173: none (all steps ran in the thread); earlier results not redone; thread context
  at hand-off in the PR reply.
- D1174: none (all steps ran in the thread); earlier results not redone; thread context
  at hand-off in the PR reply.
- D1175: sonnet, read 0xc95ec's pattern-0 branches (capstone over read_bytes), 49k
  tokens, 15 tools, 1.5 minutes, not redone; thread context 92k at hand-off.
- D1176: none (read the D1175 disassembly in the thread); thread context 77k after PR 319.
- D1177: none (read 0xc98d0 in the D1175 disassembly); thread context 101k after PR 320.
- D1178: general-purpose (thread model), decoded the motor row 0x95f494's k edge
  (capstone over read_bytes), 58k tokens, 14 tools, 3.2 minutes, not redone; thread
  context 125k after PR 321.
- D1179: none (avian's motor source and the D1178 decode); thread context 108k after
  PR 322.
- D1180: general-purpose (thread model), decoded the piston row 0x96178c (capstone over
  read_bytes), 68k tokens, 17 tools, 3.7 minutes, one follow-up that corrected the
  `+0xc` gate; thread context 127k after PR 323.
- D1181: none (avian's prismatic source and a temporary piston trace); thread context
  133k after PR 324.
- D1182: general-purpose (sonnet), found the stiff flag's offset and the piston case
  0x1c9988 (capstone over read_bytes), 68k tokens, 31 tools, 3.8 minutes; thread
  context 103k after PR 325.
- D1183: general-purpose (sonnet), decoded row 0x960988 and block 0x1c90b8, 77k tokens,
  14 tools, 3.0 minutes; thread context 119k after PR 326.
- D1184: general-purpose (sonnet), decoded block 0x1c90b8's calls and 0x1c58d0, 84k
  tokens, 21 tools, 6.2 minutes.
- D1185: general-purpose (sonnet), found the writers and readers of PJoint +0x29, 119k
  tokens, 61 tools, 9.0 minutes; thread context 92k after PR 328.
- D1186: general-purpose (sonnet), decoded the piston block's row pool and filler
  0x95ba80, 86k tokens, 20 tools, 4.6 minutes; thread context 106k after PR 329.
- D1187: general-purpose (sonnet), decoded PJoint +0xf0/+0xc8 and row 0x960988's k=0
  edges, 99k tokens, 47 tools, 6.8 minutes; thread context 136k after PR 330.
- D1188: no subagent.

## Next
Logic as a whole, one component kind per PR, each with a census across all levels, the
EBOOT rule and unit tests:
1. Every placed switch type has a rule since D1131 (Part 14), and switch
   inputs since D1132 (Part 34), material tweaks since D1133 (Part 35),
   emitters since D1134 (Part 36), sequencers since D1135 (Part 37), controlinators
   since D1136 (Part 38), creature deaths since D1137 (Part 39). The host
   must call `CreatureState::touch`, `crush_frame` and `tick` with config values (NOT
   FOUND), and seat players with `controlinator_in_reach`, `controlinator_enter` and
   `controlinator_seat_tick` (Part 40), and respawn dead players with
   `CheckpointLives::respawn` after `activate` (Part 41), call `seat_destroyed` when
   it destroys a Thing and `seated_tick` each frame (Part 42). The host
   must carry out `LevelWorld.destroy_requests`, apply `lethal_type` and
   `body_material`, call `emitter_tick` each frame and spawn the plan when it
   says so, and call `sequencer_tick` each frame and `sequencer_input` when a
   switch reaches a sequencer. The host must
   fill `PlayerState.grabbing` for the grab switch
   (Part 20), `PlayerState.piloting` (the controlinator he sits in) and `pad` for the
   control pad (Parts 21 and 38) and
   `ThingState.body` from avian2d for the velocity switches (Part 19). The host must fill
   `LevelWorld.contacts` from avian2d (Things by `lbp_sim::ThingIndex`) for the impact
   switch (Part 15) to see anything, with `impulse` for the button (Part 23), and call
   `projectile_hit` for projectile and paint sensors (Part 18).
2. Sensors: the cone of a sensor under a cursor chip (runtime cursor record, Part 45).
   The cone direction's callers 0x114de4 and 0x115088 are editor drawing only (D1144
   read). Grab and impact have no cone or layer test (Part 44), tag and player do
   (Part 43), chip tables since D1143.
3. Outputs not yet modelled: the host must call `LayerTweak::tick` each frame and move
   the body's glued group and colliders by the step it returns, with a `fits` test for
   overlap (Part 47); for configuration 13, load the recording resource (type NOT
   FOUND), call `PlaybackTweak::tick` each frame and drive the body towards
   `sample` at `frame` from its start pose (Part 48); for configuration 14, the
   gather around the camera and the SPU job's force (NOT FOUND, needs SPU code,
   Part 49);
   for followers, the 1/mass rate (Part 53), and a host calling
   `MoveTweak::in_out_step` and moving the group (Part 67);
   the sackbot input, the
   stickiness-3 tweak, the emitter's lifetime and recycling, the
   sequencer's output, music and bit-4 boards (Part 37).
4. Motors' leftovers: the turn sign (0xc9250 is an inlined atan2, matrix layout
   UNVERIFIED, Part 74); the band's cap scale (INFERRED in Part 80, the row is solved off
   the PPU); the
   wired ramp's clock start against the centre sync (Part 77); and, for pistons, the
   the impulse cap at row+0x4c (Part 87),
   the stiff pose re-base 0xcab7c when SetStiff turns stiffness on (Part 86);
   rerun the route and parity for D1188 (Part 89) once the PC is unlocked, and recheck
   the D1182 route fall with it.
5. The creature brain (PEnemy reader NOT FOUND; leads: the TOC entry for string
   "PEnemy" 0xc79058, bit 0x1c tests); who puts a player in state 5 on contact (Part
   42 has the seat path) and sends the respawn (0x7feb8, Part 41); the controlinator's
   recorded inputs (Part 38) and what sets its +0x4c (Part 40).
