# D233: Sackboy moods, emotes and head look in play mode

Follows D232 (`docs/d232-sackboy-anim-rules.md`). Source of the rules: the read-only Ghidra
pass 3 `local-analysis/results/2026-10-11-ghidra-sackboy-moods-headlook.md` and section 4
of pass 2 `local-analysis/results/2026-10-11-ghidra-sackboy-anim-open-items.md` (01.33
EBOOT, not committed; addresses below). Labels: FOUND = read from code or constants there;
INFERRED = our choice or the pass's interpretation; OPEN = not traced, not used or 0 here.

**Not compiled or run in the authoring session** (2026-10-11): its permission gate refused
every `cargo` and `python` command, so fmt, clippy, the tests and the play run below are
still to do. See "Verification".

## What changed

- `crates/lbp-sim/src/sackboy.rs`: `Mood` (happy, sad, angry, scared; `opposite` = slot
  `i ^ 1`) and `Input::mood: Option<Mood>`, the mood d-pad direction held this tick. The
  movement rules ignore it.
- `crates/lbp-sim/src/sackboy_anim.rs`: 33 new clips (54 in `ClipId::ALL` then; 56 since D245): the anticipation,
  12 emotes (sad 2/3 as `_2_loop`, `_3_into`, `_3_loop`), 12 to_neutral, 4 mood stands, 3
  look clips. `SackboyAnim` keeps `levels[4]`, `weights[4]`, the emote sequence, the mood
  stand and the attention weight; `tick` takes the press edge of `Input::mood`
  (`press_mood`), layers the mood between the base and the walk (`mood_layers`) and ends
  the stack with the three additive look layers. Pure helpers `emote_clip`,
  `to_neutral_clip`, `look_axes`, `look_frames`; accessors `mood`, `mood_weights`, `emote`,
  `emote_held`, `attention`.
- Viewer (`sackboy_play.rs`, `main.rs`): keys I / K / L / J = mood d-pad up / down / right /
  left (the arrows and the pad's d-pad already walk and change lanes, so the pad has no mood
  buttons yet). `--play-sackboy-input` takes the capitals `U D R L` as that direction held
  for the step (`U:1,-:40,U:1` = two presses). The scripted log line gains
  `mood <name> <level> <anticipation|emote|held>`, `<name> off to_neutral` or `-`, and no
  longer lists additive layers that sit on their reference frame. The clip bank binds all
  clips (54 then, 56 since D245 added jump_small_stand / jump_small_walk) through the unchanged
  `ClipId::ALL` loop.

## Rules used

| Item | Rule | EBOOT | Label |
|---|---|---|---|
| Slots | i = 0..3 = happy, sad, angry, scared; level 0x894[i], weight 0x8a4[i] | 0x1f6434, 0x203cdc | FOUND |
| Button per mood | masks 0x100000 / 0x400000 / 0x200000 / 0x800000 = d-pad up / down / right / left | 0x1f6450, 0x42e0e0 | INFERRED (strong) |
| Press | edge: down now, not down last tick; one level step per press | 0x388358 | FOUND |
| Same mood | level = (level + 1) mod 4: 1, 2, 3, off; a level above 0 zeroes every other mood | 0x1f7844 | FOUND |
| Opposite mood (i ^ 1) on | it is ended (to_neutral); the pressed mood does not start | 0x1ffe58 | FOUND |
| Other mood | replaces at level 1 | 0x1f7844 | FOUND |
| Levels | 3 for plain Sackboy (flag 0x1124 = 0); the one-level model variants are not modelled | 0x1f64f4 | FOUND |
| Timeout / movement | none: no writer besides the four above; walking and jumping keep the mood | text scan | FOUND |
| Sequence | anticipation (0x418, 6 frames) -> emote `0x8c4[i] + 2 level - 2`, frame +1 per tick, last frame holds -> on end: to_neutral `0x8d4[i] - 2 + 2 oldLevel`, then reset | 0x203cdc | FOUND |
| Anticipation on every step | 0x1f7844 stores state 1 / frame 0 while playback state 0 is the anticipation; pass 3 reads "the anticipation restarts", used here for every step to a level above 0 | 0x1f7844 | INFERRED |
| Sad loops | sad level >= 2 chains into a loop after its first clip. Here: level 2 repeats `emote_sad_2_loop`, level 3 `emote_sad_3_into` -> `emote_sad_3_loop` | 0x203cdc (0x47c) | FOUND chain, INFERRED clips |
| Weights | toward 1 (level > 0) else 0 at 0.1 per tick; taken as `w += 0.1 (target - w)` | 0x203cdc | FOUND rate, INFERRED form |
| Emote layer | the emote clip is the base under the walk and jump blends; here a weight-1 lerp right after stand/idle | 0x20e28c order | FOUND / INFERRED slot |
| Mood stand | `stand_<mood>` (0x294 + i) lerped onto the base by 0x8a4[i], weights below 0.01 skipped; shown once the emote is on its last frame (`0x764 >= len - 1`) | 0x201fb8, 0x2022b8 | FOUND lerp; INFERRED gate (the test is FOUND under flag 0x1124 only) |
| Mood stand while ending | kept under the decaying weight during to_neutral, dropped below 0.01; phase = tick mod 17 | | INFERRED |
| Idle chain | blocked while any level > 0 | 0x1fb4a8 | FOUND |
| Look axes | additive, weight 1, frame `(0.5 v + 0.5) n` minus frame `0.5 n`, v clamped to [-1, 1]; order up/down (0x340), left/right (0x33c), roll (0x344) | 0x2007e4 | FOUND |
| Look n | last frame index (8, 8, 16), so v = 1 is the last frame and neutral the middle key | | INFERRED |
| Look values | up/down `0.5 sgn sqrt(wf) f25 w + p`; left/right `y` after yaw compensation `y - yaw / (pi/2)` on the side faced; roll `f25 w sqrt(1 - wf) + r` | 0x200aa0 | FOUND |
| Walk factor | wf = 0.2 w_slow + 0.8 w_fast, sgn = sign of the anim speed (vx) | 0x200aa0 | FOUND |
| Attention w | `w += 0.1 ((1 - S) - w)`, then `w *= clamp(1 - \|0x6d4\| / scale)` in place | 0x200f28 | FOUND |
| S | `P + \|v - cfg[0xc]\| / scale`; here `\|vx\| / 30`: v / scale is the anim-speed pair (FOUND), cfg[0xc] = 0, P = 0 and 0x6d4 = vx | 0x1fb6dc | INFERRED surrogate |
| Stack slot | look layers last (after the blink): the dispatcher runs head look after the arms | D232 order | FOUND order |

Frame counts (d116 census, 01.33): anticipation 6; emote happy 16 / 18 / 31, sad 14,
sad_2_loop 19, sad_3_into 14, sad_3_loop 51, angry 19 / 16 / 23, scared 18 / 18 / 26; all
twelve to_neutral 16; the four mood stands 17; look_leftright 9, look_updown 9, look_roll 17.

## Not modelled (0 or absent here)

- Look inputs: the right stick (A, 0x868..0x870), the target blend B / 0x880 with the
  creature+0xc20 object and the 0x2e806c vector, the relative tilt term f25 and the roll's
  `2 [0x808] cfg[0xa8] ([0x810] - 0.5)` term (OPEN). With these at 0 every look axis sits
  on its neutral frame, so the three layers add nothing visible yet; `look_axes` is tested
  with non-zero values and w is computed each tick.
- The L3 free-look toggle (0x964), the head-look skip states (c14 0xc; 2 with +0xf20 = 2).
- Mood gates: 0x965, the creature-state list, sad skipped while the Popit global is set.
- Script / Sackbot mood source (0x1f76d0), event 0x20, the one-level variants (0x1124), the
  post-emote blink 0x6b4 on layer 7, face morphs, arms.
- Simultaneous d-pad directions (one `Option<Mood>` per tick).

## Tests written (lbp-sim, `sackboy_anim.rs`; 10 new, not yet run)

Level cycle 1, 2, 3, off, held = one press, no timeout, movement keeps the mood; opposite
cancels through the level's to_neutral and another mood replaces at level 1; anticipation 6
frames -> emote 16 -> held frame 15 with stand_happy from tick 21 at weight 1 - 0.9^(t+1);
to_neutral 16 frames then reset to stand only; weights 0.1, 0.19, 1 - 0.9^10 and decay;
sad 2 / 3 loops; idle blocked for 5,000 ticks; look layers last on reference frames 4, 4, 8;
`look_axes` numbers; attention 0.1 after one tick, 0.475 at half speed, 0 at full speed.
`clip_table` expected 54 distinct GUIDs at D233 (56 since D245).

## Verification

- Not run: `cargo fmt --all --check`, `cargo test -q -p lbp-sim -p lbp-preview
  -p lbp-bevy-viewer`, `cargo clippy -q -p lbp-sim -p lbp-preview -p lbp-bevy-viewer
  --all-targets -- -D warnings` (permission gate, see top). The code was formatted and
  checked by reading only; expect fmt touch-ups.
- Play run (PC, hidden window, d68 0133 snapshot, blank_level_large): "clip bank 54/54 bound to one rig" (56/56 since D245), mood column steps sad 1 -> 2 -> off (to_neutral) and angry 1 -> held; no panics. Original note: play run. Suggested script (hidden window, d68 0133 snapshot):
  `-:30,U:1,-:40,U:1,-:40,D:1,-:40,D:1,-:30,D:1,-:60,R:1,r:30,-:30,L:1,-:40`; expect
  "clip bank 56/56 bound to one rig" (54/54 at D233) and the mood column stepping through the states.
- Not verified against RPCS3: the d-pad mapping, the anticipation on every level step, the
  mood-stand gate, the weight easing form and the look frame scale.
