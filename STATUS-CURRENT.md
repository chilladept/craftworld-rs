# Current state (overwrite each milestone) — after D247, 2026-10-12

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D247 landing timing and dip; stop pose measured (2026-10-12)
- Stop pose measured tick-aligned: fit mean 1.56 against the steady-walk floor 1.51, matches the game, no
  change (the old "fit 3-5" was the fit tool not converging at 72 deg yaw). Turn: no game capture yet.
- Landing: the tail now starts on the tick the feet reach the floor (`SackboyBody::grounded` is the contact
  after the move; controller step unchanged) and `LANDING_DIP` = -25.9, -15.9, 0. Touchdown +1/+2 against
  d234 `jump.bin`: height game 206.0/202.9, ours 206.3/202.6; lowest point game -29.0/-16.2, ours
  -29.0/-16.1. Flight ticks unchanged (height max 2.3, lowest max 1.1). `docs/d241-landing-dip.md`,
  tools `local-analysis/d249-landing/` (`run_ours.ps1`, `land.py`).

## Before: D246 idle stretch fixed at the cause (2026-10-12)
- The stretch was scale inheritance, not the clip weight: the stand clip breathes by scaling the spine bones
  and our skinning passed that scale to every bone below (legs, head, arms). `lbp-preview` skinning has
  `ScaleRule::Biped` (a `Bip...` bone keeps its parent's scale out of its own shape; its offset still moves),
  used by the Sackboy play figure only; `STAND_BASE_WEIGHT` is 1 again. Clean D234 captures: standing height
  per tick 201.1..206.4, ours 200.8..206.1 (RMS 0.34, was 2.5); 6-tick jump heights within 2.3 for ticks
  1..20, feet within 1. The D243 idle capture used for D242-D244 is not the neutral stand (do not tune
  against it). Open: landing dip after touchdown (game 195, ours 200), ledge-fall layer weight, stop/turn
  poses, lean. `docs/d242-idle-stretch.md` (D246).

## Before: D245 jump pose (2026-10-12)
- Take-off tucked the legs early. From the EBOOT jump layer: the frame rule was right; the pose is
  `lerp(jump_small_*, jump_*, held ticks / 10)` (count frozen on release) at full layer weight after a
  take-off. `sackboy_anim.rs`: clips jump_small_stand / jump_small_walk (bank is 56), `JUMP_HOLD_FULL`.
  D243 capture: mesh height per tick within 4.6, rms 2.3 (was 20 low) with the capture's vertical speed; in
  the viewer (our 6-tick arc) max 7.0 / rms 4.8. That capture is not a clean 6-tick jump (feet apex +277,
  106 units sideways); against the clean D234 capture the viewer is within 2.4 (feet rise within 1.8), so
  the jump arc is unchanged. Not yet seen in the viewer next to the game. Open: ledge-fall layer weight
  (still the D234 ramp), a fresh capture with a known hold length, then stop/turn poses and lean/stretch.

## Before: D244 idle weight reaches the viewer (2026-10-12, #5)
- The viewer's pose blender forced the base layer to weight 1, so D242's weight never showed. Honoured now;
  `STAND_BASE_WEIGHT` = 0.25 (head-top travel per breath 18.5 -> about 4.6, game 2.6; `docs/d242-idle-stretch.md`).

## Before: D243 Popit leftovers (2026-10-12)
- Pod card uses the same head anchor as the Moon (no change); UV Tool / Slice icon guids 42930-42931 are absent from disc (`docs/d235-popit.md` pass 8). RPCS3 can be driven without focus: `scripts/rpcs3_bgpad.py`, `scripts/rpcs3_bgcap.py`.

## Before: D242 idle stretch (2026-10-12)
- Owner: stretch is in the idle animation. Game idle moves head/chest 2-3 units, ours 7-11; stand clip now at 0.5 over
  bind (`docs/d242-idle-stretch.md`, weight inferred). Needs a tick-aligned idle capture to confirm.

## Before: D241 landing dip (2026-10-12)
- Jump arc matches the game (centroid within 3). Landing dip was 12 units too deep and 10+ ticks too long;
  `LANDING_DIP` now follows the game's lowest vertex (`docs/d241-landing-dip.md`). D240 (#140) Pod/play-mode Popit merged.
  Open: take-off crouch vs game's straight stretch, landing/stop/turn pose fit 3.5+, Popit Pod card placement.

## Before: D235 Popit overlay (2026-10-11)
- D234 merged (#132). `--play-sackboy` now draws the Popit (`popit_play.rs`, `--popit-mode edit|play`,
  `--popit-open`, Z toggles): item order/icons/labels from poppet.ff, layout/colours measured on RPCS3 frames.
  `docs/d235-popit.md` (FOUND / MEASURED / OPEN). Open: gradient/camera constants, cord shape, Sackboy turn/tint,
  real D-pad Down binding, pixel compare against more frames.

## Before: D234 animation fitted to game vertices (2026-10-11)
- D233 moods/emotes/head look merged (#130, compiled, tests pass).
- D234: RPCS3's skinned Sackboy vertices read from memory and fitted to our poses
  (`crates/lbp-preview/examples/pose_fit.rs`, `scripts/rpcs3_vdump.py`, viewer env
  `LBP_DUMP_VERTS`). Walk right 0.8, walk left 1.5 (mirror OK), jump now 1.3-1.5 after the
  air weight fix (0.7 -> 0.9, was 1.0). `docs/d234-anim-vertex-fit.md`.
- Emote inputs confirmed in play mode (memory reads): Up/Down/Right/Left = happy/sad/angry/scared,
  levels 1-3-off, replace and opposite-cancel rules, no timeout (docs/d234-anim-vertex-fit.md).

## Before that (2026-10-11)
D231 play mode draws Sackboy (0000043f) with his own clips; D232 blends them by the EBOOT
layer rules with eased facing, mirror and the QA fixes; D232 pass 2: idle roll 0..249, lane
after jump, idle after 300 quiet ticks, seed 42 (`docs/d232-sackboy-anim-rules.md`).
Movement: D230 fall drag / gravity; Moon handoff `docs/handoff-pc-2026-10-11*.md`.

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- Mesh census 01.33 (4,261 GUIDs, after D188-D190): 4,259 viewable, 4,239 all-diffuse by
  default, 0 Complete. Files `local-analysis/b177-{default,legacy}.json`.
- Texture census 01.33: all 23,247 hashes decode (top mip, decodability only).
- Levels 01.33: 398 resources; lbp-level walks 236 in-band, Toolkit parity 236/236.
- D233: +0 newly viewable / +0 newly Complete (runtime feature).

## Open blockers
- Stop pose: done (matches the game, D247). Turn pose needs a game capture with a direction reversal.
- Post-landing leg squash: game mesh height about 195 at touchdown +3..+6, ours 203..199 (8 units over
  about 5 ticks), unattributed. Jump air lean: ours +4 deg forward of the game, unattributed.
- Walk start: our cycle is 2 frames ahead of the game on one capture. Head-top bones 22/23/24/27/28.
- After a landing the body rests 1.6 above the spawn height (floor cast skin); not looked into.
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; mirror flip types; arms mapping; face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Turn pose (game capture with a direction reversal); post-landing squash; air lean; arms; emote
   clip frames vs game.
2. Popit: compare more frames, cursor mode, inventory grid pages (submode 11).
3. Camera lead, slopes (paused by the owner).

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu). Build trees are
  ~11 GB: reuse `target-pr*` through `CARGO_TARGET_DIR`.
- Git: remote origin github.com/chilladept/lbp2-asset-explorer (private), default-deny
  `.gitignore`, CI `.github/workflows/ci.yml`. Standing rules: CLAUDE.md.
- Native QA windows hidden or on the non-primary monitor; never steal focus.

## Where detail lives
Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`. Earlier milestone
docs under `docs/` (grep headings); format evidence `docs/formats.md`; rules `AGENTS.md`.
