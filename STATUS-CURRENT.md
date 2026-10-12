# Current state (overwrite each milestone) — after D248, 2026-10-12

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D249 short legs after a landing (2026-10-12)
- The pose blender has a two-bone leg solver (`BlendPose::move_chain_end`, `lbp-preview`); the play figure
  raises both ankles by `LANDING_SQUASH` (7.6 .. 0.5 on touchdown +3..+8) and `LANDING_DIP` runs twelve ticks
  (two under the floor, then the body's settle). MEASURED against d234 `jump.bin`: height, pelvis height and
  lowest vertex within 0.4 on touchdown +3..+8 (height was 8 off). Both tables are one standing jump's
  series; the game's foot pass (0x210dd8) inputs are untraced. `docs/d241-landing-dip.md` (D249), tools
  `local-analysis/d250-crouch/` (`run.ps1`, `final2.py`).

## Before: D248 crouch spring found, not the landing squash (2026-10-12)
- `stretch_crouch` spring (EBOOT 0x2011d4) is in `sackboy_anim.rs` (`CrouchSpring`, tested) but unfed; as a
  landing kick it does not fit. Idle, steady walk and stop match the game within 0.4 deg lean / 0.4 height.
  No extra in-air body layer in the EBOOT. Vertex dumps log `sackboy pose N`; the stand phase at a script
  tick varies between viewer runs (+-1 height).

## Before: D247 landing timing and dip; stop pose measured (2026-10-12)
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

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- Mesh census 01.33 (4,261 GUIDs, after D188-D190): 4,259 viewable, 4,239 all-diffuse by
  default, 0 Complete. Files `local-analysis/b177-{default,legacy}.json`.
- Texture census 01.33: all 23,247 hashes decode (top mip, decodability only).
- Levels 01.33: 398 resources; lbp-level walks 236 in-band, Toolkit parity 236/236.
- D233: +0 newly viewable / +0 newly Complete (runtime feature).

## Open blockers
- Stop pose: done (matches the game, D247). Turn pose needs a game capture with a direction reversal.
- Landing: only a standing jump is captured (ledge fall, running jump: same tables, unchecked).
  Jump air lean: ours +4 deg forward of the game; candidates are the head-look tilt from the physical
  body's rotation (0x200c6c) and the 0x20a5f4 peak follower.
- Walk start: our cycle is 2 frames ahead of the game on one capture. Head-top bones 22/23/24/27/28.
- After a landing the body rests 1.6 above the spawn height (floor cast skin); not looked into.
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; mirror flip types; arms mapping; face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Turn pose (game capture with a direction reversal); air lean; arms; emote clip frames vs game.
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
