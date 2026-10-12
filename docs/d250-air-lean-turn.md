# D250 / D251: jump air lean and the turn (2026-10-12)

## D250: the upper body eases back toward stand in the air

Question: why does our Sackboy lean about 4 degrees further forward than the game's through a jump?

- MEASURED (d234 `jump.bin` against the release viewer, per-bone rigid fits relative to the pelvis): the whole
  difference is one joint, Spine1 against Spine, up to 5.5 degrees, with the head and clavicles riding on it;
  the upper arms are off by up to 15 degrees against Spine1 with the same shape in time. Pelvis, Spine, legs and
  the head against Spine1 match. Not a frame offset, not the tap/held mix, not a look or crouch layer.
- FOUND (EBOOT, read-only): the dispatcher 0x20e28c keeps a state s (+0x6ec) that eases 0.1 per tick toward 1
  while the jump layer runs (in the air and through the landing tail) and toward 0 otherwise. Each tick the arm
  pass (0x2097e0 -> 0x208dec) does a masked lerp (0x1eaccc) of the pose toward frame 0 of the stand clip (or
  the mood stand) with weight max(u, arm terms) x (1 - u), u = 1 - s taken before the tick's ease: s (1 - s)
  with no arm input, so 0 on the ground and a quarter at most. The mask (skeleton+0x3a0, built in 0x1e79d8) is
  `Bip01 Spine1` and every bone below it except `Bip01 Head` and the bones below the head. It runs after the
  jump and lane layers, before the mood face layer, the crouch spring and the head look.
- The per-index table at anim+0x4ec that 0x2072dc writes is not an animation weight: it goes to the soft-body
  job (0x347b0c). Not modelled.
- Change: `AIR_EASE`, `Blend::UpperBody` and the layer in `lbp-sim` `sackboy_anim.rs`; `BlendPose::lerp_masked`
  and `Rig::subtree_mask` in `lbp-preview`; the viewer's `blend_frame` applies it.
- Result, MEASURED on the same jump (game ticks 15..41, take-off to the last airborne tick): Spine1 pitch
  against Spine, ours minus game, was +1.7 .. +5.5 (max 5.5), now within 0.05 on every tick; upper arms
  against Spine1 were 1.7 .. 14.7 degrees off, now 0.2 .. 2.9. Heights and the landing (D249) are unchanged.
- Not modelled: the arm terms (+0xae0, +0xae4), state 0xc14 == 1, the unmasked lerp of the fourth pose stream
  in the same call. The remaining 2 to 3 degrees on the upper arms late in the flight are unattributed.

## D251: the turn

Game captures (new, RPCS3, story level "Rookie Test", flat ground, one vertex buffer per tick): three
right-to-left reversals, one left-to-right, one stop; the new direction's key goes down before the old one
comes up. Aligned on the first tick the body's speed drops (N = 0).

- MEASURED, matching already: speed (3 per tick through zero, no pause), the turn clip's frame N on tick N
  with its 5-frame ramps, the walk phase running on underneath, and the stop (yaw held at 72 degrees, then
  released; frames and speeds as ours).
- MEASURED, wrong in ours: facing. The game's yaw eases 0.4 per tick toward the plain speed angle
  (speed / 30 x 90 degrees) through the whole turn, within 0.4 degrees of that model for N = 0..15. Ours
  switched to the 72-degree hold on the stick's side as for the stop: 25, -14, -37 degrees on N = 0..2 where
  the game shows 85, 79, 72, up to 111 degrees off. Now the hold applies to the stop and the lane change only.
- MEASURED, wrong in ours: the walk after the turn. On the tick after the turn's last frame the game is on
  walk frame 1 in all three captures, ours was on 0. The reset now leaves the phase one on.
- Result (capture `turnRL2` against our run, vertex-to-vertex rms at our own yaw): N = 0..18 was 48 .. 98
  (the yaw), now 1.9 .. 3.6; N = 19..23 was 6 .. 16, now 3.5 .. 3.8. The steady walk before the turn is 2.3
  .. 2.6 on the same measure, of which 2 degrees of yaw are the level's top speed (29.33 to the right there,
  88 degrees) against our 30.
- The game's own three runs differ by 0.4 .. 1.6 on the pure turn frames and by up to 25 where the walk
  phase at the reversal differed.
- Open: the 29.33 top speed to the right in that level; the game's yaw 0.8 .. 1.5 degrees ahead of the model
  on N = 16..18; mirrored poses fit at 2.5 against 0.7 unmirrored (left-to-right turn: a front torso bone up
  to 15 units off on frames 6..12); standing foot spacing 43.4 in that level against our 51.9.

Tools (local, not in the repository): `local-analysis/d251-air-lean/` (`lean.py`), `local-analysis/d252-turn/`
(captures), `local-analysis/d253-turn-cmp/` (`runall.ps1`, `cmp.py`).
