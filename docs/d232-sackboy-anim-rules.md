# D232: Sackboy's animation from the EBOOT rules (plan slices 3-6)

Follows D231 (`docs/d231-sackboy-animations.md`). Source of the rules: the read-only
Ghidra pass `local-analysis/results/2026-10-11-ghidra-sackboy-anim-rules.md` (01.33 EBOOT,
not committed; addresses below). Labels: FOUND = read from code or constants there;
INFERRED = our choice; OPEN = not traced, not used or stubbed.

## What changed

- `crates/lbp-sim/src/sackboy_anim.rs`: `pick_clip` (D231 hard cuts, all INFERRED) is
  replaced by `SackboyAnim::tick(&Sackboy, grounded) -> AnimFrame { layers, yaw }`. Each
  `Layer` is `{ clip, frame, weight, mirror, blend: Lerp | Additive { reference } }`,
  applied in order onto one pose as the game does onto its layer 4. 21 clips (12 new:
  stop, turn, blink and the 9 idle-chain clips).
- `crates/lbp-preview/src/skinned.rs`: `Trs` (with matrix decomposition), `BlendPose`
  (per mesh bone, from bind) with `lerp` (translation/scale lerp, rotation nlerp along the
  short arc, morph lerp) and `add` (additive difference), `sample_pose` (sub-frame:
  neighbouring frames blended), `Rig::pose_trs`, `Rig::skin_matrices_from_locals`,
  `SkinnedClip::blended_positions` / `morphed_bind_weights`.
- Viewer (`sackboy_play.rs`, `main.rs`): the clip bank no longer swaps clips. Stand stays
  the playing clip; each picked tick is blended into one `BlendPose` (`blend_frame`) and
  `animate` poses the GPU joints (or CPU skin) from it once (`Playing.layered/blended`).
  Facing comes from the picker's eased yaw, interpolated between the last two ticks by the
  fixed-step overstep (no more snap to +-90). `--play-sackboy-input` logs end with
  `yaw D layers clip@frame*share ...` (`+clip` for additive layers).

## Rules used (FOUND unless marked)

| Item | Rule | EBOOT |
|---|---|---|
| Walk | r = clamp(\|vx\|/30); w_slow = clamp(r/0.1), w_med = clamp((r-0.1)/0.3), w_fast = clamp((r-0.4)/0.6), lerped in order over the base. Anim speed s taken as vx (INFERRED simplification of the 0x6d8 lerp) | 0x201680 |
| Walk phase | all walk layers at frame `tick mod 13`, not speed-scaled | 0x20178c |
| Backwards flag | s < 0 for 5 ticks sets the walk layers' mirror flag (counting INFERRED) | 0x20e3e0 |
| Facing | target `clamp(vx/30) * 90 deg`; `yaw += 0.4 * (target - yaw)` per tick; lane change +-72 deg (sign = side already faced, INFERRED; 0x1128 scale taken as 1) | 0x2002ac, 0x201cc0 |
| Stop | stick \|x\| < 0.1 with w_fast > 0.5: walk_fast_stop 21 frames, weight min(i/5, (n-i)/5); mirrored if vx < 0 | 0x20afc0 |
| Turn | stick opposite to vx, \|x\| > 0.9, w_fast > 0.5: walk_fast_turn 19 frames, same ramps; end resets walk phase and idle frame. Input-mask test (0x1018) OPEN, treated as passing | 0x20ad14 |
| Lane | \|lane - z\| >= 100: walk_z_forwards if dz > 0 else backwards (sign FOUND 0x21685c..0x216b74; game +z toward the camera INFERRED); frame i+3, weight clamp((i+1)/3) * clamp((n-3-i)/3), 10 ticks. One clip per slide (INFERRED); the dot test is OPEN | 0x216b0c, 0x201418 |
| Jump weight | clamp(peak/10), peak = \|vy\| follower easing 0.125 per tick (rises at once: INFERRED), times clamp(airTicks / (1 + 4\|vx\|/20)); also used for an off-ledge fall (INFERRED) | 0x20a5f4 |
| Jump clip | pose = lerp(jump_small_*, jump_*, held ticks / 10) for each of stand and walk (hold count frozen on release; D245), then lerp(stand pair, walk pair, clamp(\|vx\|*0.045)), as sequential lerps; the walk clips mirrored if vx < 0 | 0x209fc8 |
| Jump frame | take-off ticks 1..5: frame = tick; rising 8(1 - clamp(vy/73)); falling 8 + 6 clamp(-vy/30.26) | 0x20a5f4 |
| Landing | jump clip plays on from int((airFrame + 15) / 2), 1 per tick to 30, weight x clamp(1 - (f-15)/16)^k, k = 1 (FOUND pass 2: bias(t, 0.5) = t, 0x20a7fc..0x20a888). No separate land clip; stretch_crouch is no longer a hard-cut squash | 0x20a5f4 |
| Idle chain | at each stand end, if idle allowed: LCG `seed*0x2e90edd + 0xe908fc1 & 0x3fffffff`, roll 0..249 (`mulli 0xfa`, pass 2, 0x20e0ac..0x20e288): 0-2 sit, 3-4 yawn, 5-6 bored, 7-9 fidget, 10 shift_feet, else stand (odds 3/2/2/3/1 in 250). Sit stands up when a 0..999 roll > 950 (0x20e044) or when idle stops, and resets the no-input counter (0x20e05c); bored after > 10 loops or when idle stops. Seed 42 (ctor 0x1ff5d4 stores 0x2a at 0x1ff64c) | 0x20dee0 |
| Idle allowed | creature state +0xc14 in {0, 1, 7, 10}, no mood level 0x894[0..3] > 0, and the no-input counter 0x75c > 300 (about 10 s); forced idle flag 0x1704 short-cuts it. The counter (0x20d3a0..0x20d3d8) +1 per tick with no input (0x1fd3dc, mask 0x7feff) in those states, else 0. Here: state = grounded, no input = stick 0 and no jump/lane button (both INFERRED); moods and the forced flag are not modelled | 0x1fb4a8 |
| Blink | additive (frame i minus frame 0), weight 1, 13 frames, next at +30 + rand(70) ticks; first at tick 0 (ctor zeroes 0x750/0x754 at 0x1f9984/0x1f9988) | 0x2040a0 |

Order of the stack (FOUND pass 2, dispatcher 0x20e28c via 0x20f140 -> 0x20eee0 ->
0x20f078/0x20f084 -> 0x20eef4/0x20ef00): base (stand/idle) -> walk slow/medium/fast ->
stop -> turn -> jump and landing tail -> lane -> blink (blink slot INFERRED: 0x2040a0 is
reached through a pointer). The game then runs the grab/override blend, the crouch spring,
arms and head look, none of which are layered here. Additive rotation is applied as `bone * (ref^-1 * frame)^w`,
scale as a ratio (INFERRED; 0x1eafc0 not traced). Clips a layer does not drive keep the
pose built so far; bones no clip drives stay at bind.

## Open items pass 2 (2026-10-11)

Source: read-only Ghidra pass `local-analysis/results/2026-10-11-ghidra-sackboy-anim-open-items.md`
(not committed). Moved to FOUND and applied in `sackboy_anim.rs` (rows above): idle roll
0..249, the layer order (lane after jump), idle allowed after 300 quiet ticks, seed 42 and
the first blink at tick 0, landing k = 1, the walk_z test sign (dz > 0 = forwards).
New tests: `idle_waits_for_300_quiet_ticks`, `layers_run_walk_stop_jump_then_lane`, the
roll rate over 100,000 LCG steps (11/250 +- 0.005).

Traced but not implemented (FOUND in the pass, out of scope here):

- Crouch spring input 0x6c56c: E = max over contacts of min(overlap depth / 350, 1), on the
  ground path only (0 when phys+0x41 picks the air path); `this+0x960 = sqrt(E)` feeds the
  spring at 0x2011d4 (s850 = 0.75v, s858 = -0.25v, bias(x, 0.2)). It is a squash from
  overlapping bodies, not a landing squash; there is no other landing squash. We model no
  body overlap, so stretch_crouch stays unused.
- Emotes and mood stands: mood slots happy/sad/angry/scared (0x894 level, 0x8a4 weight
  easing 0.1 per tick), the anticipation -> emote -> to_neutral sequence (0x203cdc..0x204094),
  `stand_<mood>` picks at 0x2022b8 and per-mood lerps at 0x201fb8.
- Head look 0x2007e4: three additive axes (updown 0x340, leftright 0x33c, roll 0x344),
  input = wrapped angle / (pi/4) x 1.3, smoothed 0.1 per tick (0x200ecc..0x20104c); its
  target 0x1fb6dc is untraced.
- Mirror: `0x1ebb48(engine, 5, src)` queues command type 3 (mirror-copy into scratch
  layer 5), 37 call sites.

## OPEN / not done

- Mirror / flip maths: command type 3 runs in the anim job dispatched by 0x1e9b78
  (probably SPU), not traced; flip types and mirror morphs unused (the viewer's mirror
  table reflection from the QA fix pass stays).
- Camera lead: camera-box field names (BoxSmoothVelX/Y, BiasVector, BiasPersistentXTarget,
  TransientBiasX/Y) suggest a velocity-driven x bias, but no reader or field offsets were
  found. A live memory diff of the follow camera against vx is the cheaper route.
- GroundTurnSpeed / FootAnimationSpeed: no reader found after `lwz ..,0xbf4`; the walk
  phase stays the fixed 13 ticks.
- Arms: arms_* clips 0x348..0x368 are used in the grab paths 0x208488 / 0x2086c0 /
  0x2072dc; the mapping is not traced.
- Face morph readers: not found. `this+0x960 > 0.01` calls 0x1f8d9c (untraced).
- The game's "input" mask 0x7feff and states {0, 1, 7, 10} are mapped to our controller
  as above (INFERRED); the stand-path counter reset at 0x20e100 is not modelled.
- Not modelled: grab_jump_*, (jump_small_* is modelled since D245), bouncepad, jump_jet, wall idle,
  the riding/holding walk-weight gate (0x64dc4), plus the traced items above.

## Verification

- `cargo test -q -p lbp-sim -p lbp-preview -p lbp-bevy-viewer`: pass. 13 picker tests
  (walk ramps at 3/12/21/30 u/tick, shared 13-tick phase at 3/15/30, 5-tick backwards
  flag, facing 0.4 ease and 45/90 deg targets, jump frames 0/4/8/11/14, stop 21 and turn 19
  frames with 0/0.2/../1 ramps and phase reset, lane frames 3..12 with 1/3 ramps and 72 deg,
  LCG and roll table, idle chain order and blink spacing over 20,000 ticks, standing and
  running jumps on the real controller); 4 new blend tests in lbp-preview.
- Clippy -D warnings (three crates, all targets), fmt clean.
- Rookie Test play run (d68 0133 snapshot, hidden window, script
  `-:1300,r:30,rj:10,r:25,-:20,b:1,-:15,j:3,-:40,l:4,-:30,l:30,r:3,-:30`): "clip bank
  21/21 bound to one rig", no frame-count mismatch. Log: stand + blink -> medium/fast mix
  with yaw rising 24 -> 59 -> 85 -> 90 deg -> jump_walk fading in over 7 ticks, frames
  2 -> 5.3 -> 8 (apex) -> 13.1 -> landing tail 12, 16, 20, 24, 28 fading while
  walk_fast_stop ramps in -> lane walk_z_backwards with yaw 57 -> 70 deg -> stand, yaw
  back to 0 -> jump_stand 3 -> 7.8 -> 11.9.
- No QA capture this time (the pose blend is visible only in motion).

## Not verified

No RPCS3 comparison of the blended poses, facing timing or idle odds. The peak
follower's rise, the additive composition and the lane yaw sign are guesses (above);
the landing exponent and the idle-allowed condition are FOUND since pass 2.

## QA fix pass (2026-10-11, PR 127 review)

A QA pass (`local-analysis/sackboy-clips-2026-10-11/`) found four defects. Causes and fixes:

1. **Facing ~90 deg off in every non-stand clip.** Clip root data (bone 0, the only
   parentless bone of 0000043f; probe over all 21 clips): the root carries a pure turn
   about the model up axis: 0 in stand, jump_stand, walk_z_* and blink; +90 deg (walking
   right) in walk_slow/medium/fast, walk_fast_stop, jump_walk and all 9 idle-chain clips;
   a 90 -> 0 -> -90 sweep in walk_fast_turn. Without the root the bodies are close to the
   camera (walk_fast 2 deg, idle_sit 4, yawn 1, jump_walk 1; walk_slow ~20). The viewer
   added the picker's 0..+-90 yaw on top, so walking right showed his back (90 + 90) and
   walking left faced the camera (90 - 90). The root rotation is the facing channel the
   game overwrites (INFERRED from that data: kept, the idle chain's FOUND hard switch
   stand -> stand_i_idle_sit would itself snap 90 deg; 0x201b18 builds a sin/cos rotation
   from the eased yaw right after smoothing). Fix: `BlendPose::hold_root_rotation` puts
   the root's rotation back to bind after blending (translation kept); the yaw turns the
   figure. Moving left, the mirror flag (FOUND, already on the layers) is now applied:
   `Rig::mirrored_trs` / `BlendPose::lerp_mirrored` reflect a layer across x = 0 through
   the skeleton's mirror table (30 of 43 bones paired), skin matrix `X S_j X` for bone i
   with partner j (symmetric bind INFERRED; flip types and mirror morphs unused).
2. **One-tick 90 deg snaps** (walk start at vx 3, stand <-> idle chain, stop): the same
   root mismatch; gone with (1). The stop clip turns its body to the camera by frame 20
   (-90 relative to its root), so the yaw now follows 0x2002ac's FOUND branch: +-72 deg
   while a stop/turn clip plays (0x77c >= 0), sign = the side faced (stick, else travel,
   else current yaw; INFERRED, phys+0x490 is the +-1 the grab branch scales by 90 deg).
   The lane change uses the same sign rule.
3. **Standing jump lands on his bottom.** The contact tick reports vy 0 one tick before
   `grounded` (our controller), which mapped back to the apex frame 8 and started the
   tail at int((8 + 15) / 2) = 11 (air pose on the floor for 4 ticks). Past the apex the
   frame now only moves on, so the tail starts at 13-14 (FOUND formula). The legs-forward
   air pose is the clip itself at the FOUND frames 6..13 (one contact tick still shows it).
   D247: the contact tick now counts as grounded, so the tail starts on it
   (`docs/d241-landing-dip.md`).
   The running landing still covers the walk layers for ~2 ticks at weight 1, then fades
   1/16 per frame (FOUND rule; k = 1 and the jump-over-walk order FOUND in pass 2, so
   this is the game's own behaviour).
4. **Back lane did nothing in the blank level.** Not a controller bug and not a physics
   vs visual floor mismatch (both have the floor top at 1655): the Rookie Test spawn
   offset (-208 in y) put his feet at 1520, 136 inside the floor slab, so the lane probe
   found no floor. The checkpoint spawn now stands him on the floor below the checkpoint
   when the offset lands inside it; `b` then moves z -200 -> -400.

Also: `--qa-record DIR,START,END[,SPEED]` (QA only) saves one PNG per scripted tick of
`--play-sackboy-input` into a new `local-analysis` directory, then exits.

Re-capture `local-analysis/sackboy-clips-2026-10-11b/` (same five scripts, hidden window,
`--no-activate`, no focus changes; `pass1-no-mirror/` is the run before the mirror):
profile toward travel both ways, stand and idle chain face the camera with no snaps,
landing on the feet. Walking left shows more of his back than walking right shows of his
front: the follow camera's +296 x offset (13.7 deg, held constant, INFERRED) views him
from the right; the game's camera lead is not modelled (OPEN).
