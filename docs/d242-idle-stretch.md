# D242: idle breathing amplitude (2026-10-12)

Owner (2026-10-12): the body stretches vertically a little too much during the idle animation.

## Measured (game `idle.bin`, 20 ticks vs our stand clip, per joint, y relative to joint 3)
| joint (body part) | game range | ours range | game mean | ours mean | bind |
|---|---|---|---|---|---|
| 12 chest/neck | 2.2 | 10.5 | 109.2 | 103.2 | 111.8 |
| 23, 24 head top | 2.9 | 11.3 | 117.4 | 114.0 | 123.2 |
| 22 head | 2.3 | 7.4 | 79.6 | 77.8 | 82.8 |
| 8, 10 hands | 3.2, 3.5 | 6.4, 6.7 | -38.3 | -34.5 | -34.6 |
| 25, 26, 30, 31 legs | 0.8-2.2 | 0.8-1.9 | 13.5 / 2.9 | 16.2 / 5.5 | 14.3 / 2.5 |

Same 20-tick period. The game moves the upper body about a quarter as far and holds it nearer the
bind pose (head 3-6 higher). Fit of `lerp(bind, stand frame, w)` to the 20 ticks: w 0.3-0.8 (about
0.5), RMS 0.9-1.4 against 1.3-1.6 for the full-weight stand clip.

## Change
`STAND_BASE_WEIGHT` = 0.5: the stand clip is lerped over the bind pose at half weight (only when the
base is Stand; walks/jumps lerp over it as before). Weight INFERRED from the fit, amplitude MEASURED.

## Open
- Why the game's weight is below 1 (mood-stand layers? 0x294 clips at neutral?) and whether the idle
  chain clips need the same; the jump layer's "10-40% stand under it" (D234) may be the same effect.
- Verify in RPCS3 with a tick-aligned idle capture right after spawn (stand, not idle chain).

## Follow-up (2026-10-12, RPCS3 play mode on the ground, 98 vertex samples after a stop)
- Fresh capture agrees with `idle.bin`: top vertex 1852.7-1855.2 (range 2.5), feet fixed at 1654.4.
- ICP fit with the stand clip at bind-blend weights 0.1-1.0 is flat (RMS 5.3 for every weight; the Pod costume
  adds geometry, so the fit cannot pick a weight). Per-joint amplitude gives 0.2-0.3 (chest 2.2/10.5, head top
  2.9/11.3, head 2.3/7.4), the means 0.3-0.6. `STAND_BASE_WEIGHT` lowered to 0.3 (amplitude-led; INFERRED).
- Capture notes: pools are 2920 records big-endian at 0x329a8f00 / 0x396a8f00 / 0x39fa8f00 on this boot (they were
  16-byte BE floats all along; world x is about -19800, so a +-3000 filter hides them).

## D244 (2026-10-12): the weight never reached the viewer
- Owner: the stretch is still there. FOUND: `blend_frame` in the viewer lerped layer 0 at weight 1 whatever the
  picker said, so 0.5 and 0.3 changed nothing on screen. It now uses every layer's own weight.
- MEASURED (viewer `LBP_DUMP_VERTS`, standing): top vertex travel per breath 18.5 at full weight, 6.1 at 0.3,
  about 4.6 at 0.25; game 2.6 (`d243-idle.bin`, height 198.3-200.9, feet fixed).
- `STAND_BASE_WEIGHT` = 0.25: joint travel chest 2.6 / head top 2.8 (game 2.2 / 2.9). INFERRED weight. The top
  vertex alone would ask for 0.14, which a single weight over the bind pose cannot give together with the joints.

## D246 (2026-10-12): the stretch was inherited scale
- FOUND (clip and rig data, `pose_fit` `SCALES=1`): the stand clip scales the two spine bones (first: 0.91..1.02
  along the spine, 1.02..1.10 across; second: 0.94..1.01 and 1.02..1.06) and the zip pull under the chest by
  the inverse of the chest's scale. Blink holds the same spine values constant; walk and both jump clips have
  none. The thighs and the chest hang under the first spine bone, the neck (head, arms) under the second.
- Our skinning multiplied plain matrices, so every bone below inherited the product: rigid vertex groups of
  the thighs, feet, head and hands measured 0.96..1.03 over a breath at weight 0.25 (0.87..1.12 at weight 1).
- The skeleton is a 3ds Max Biped (bones named `Bip...`) with a few plain bones. Rule now used for the
  Sackboy play figure (`ScaleRule::Biped`, INFERRED from the data below; the game's composition is not
  traced): a Biped bone is placed in its parent's unscaled frame, its offset multiplied by the parent's
  scale; its own scale applies to its own vertices; plain bones inherit as before.
- MEASURED, clean D234 captures (`idle.bin`, `jump.bin`; three rotating buffers, the changed one is the tick):

| | game | ours now | ours before (inherited, weight 0.25) |
|---|---|---|---|
| standing height per tick | 201.1..206.4 | 200.8..206.1, RMS 0.34 | 198.3..204.4, RMS 2.5 |
| order-free vertex fit of the stand clip, weight 1 | floor 0.7 | 0.73 | 1.62 (inherited, weight 1) |
| the same at weight 1 / 0.75 / 0.5 / 0.25, every bone compensating | | 0.78 / 0.90 / 1.18 / 1.54 | |
| jump (6-tick press) height, ticks 1..20 | | max 2.3, RMS 1.4 | max 2.4, RMS 1.5 |
| jump feet apex | 242.9 | 241.9 | 241.7 |
| eyelid scale in a blink | 0.08 | 0.080 | 0.078 |

- `STAND_BASE_WEIGHT` = 1. The 0.5 / 0.3 / 0.25 of D242-D244 compensated for the inherited scale.
- The D243 idle capture those weights were tuned on (height 198.3..200.9, "top vertex travel 2.5") is not the
  neutral stand: the right hand is 51 units above the pelvis, the body leans, and no stand or mood-stand clip
  fits it better than 14 units per vertex. Its rigid groups still agree with the rule (thighs, head, hands and
  zip pull at scale 1.000, chest at 1.02 / 0.94 / 0.94 = the clip's rest scale at full weight). What state it
  shows is OPEN; do not tune against it.
- Our idle now, per region relative to the feet (viewer dump): thighs 2.4, pelvis 2.9, chest 5.2, head 5.1, top
  vertex 5.2, height range 5.3. The clean capture has a different vertex order, so only its height (range 5.3)
  and the vertex fit are compared.
- Open: after touchdown the game's height dips to 195 for three ticks and ours stays near 200 (as before).
