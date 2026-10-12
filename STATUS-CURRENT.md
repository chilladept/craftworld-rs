# Current state (overwrite each milestone) — after D254

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D254 mirrored poses (2026-10-12)
- FOUND in the EBOOT (pose mirror 0x1e42b4): a mirrored layer takes each bone's partner's local transform
  through one of six rules picked by the skeleton's mirror flip type; the zip-pull bones are copied, not
  reflected. `Rig::mirrored_trs` does the same. MEASURED on the left-to-right turn: vertex rms 2.3..4.2 ->
  0.5..1.4 on the turn frames, mirrored walk 0.7..1.6. `docs/d250-air-lean-turn.md` (D254).
- Walk start: MEASURED, no change needed. The walk phase is a free-running counter in the game too (phase
  at the first moving tick 0, 0, 3, 4, 4 over five captures); at equal phase our start fits at 0.9..1.7.

## Before: D250 jump air lean, D251 turn (2026-10-12)
- Air lean: FOUND in the EBOOT and applied. In the air and through the landing tail the upper body (Spine1
  and below, without the head) is lerped back toward stand frame 0 by s (1 - s), s easing 0.1 per tick
  (`AIR_EASE`, `Blend::UpperBody`). MEASURED on d234 `jump.bin`: Spine1 pitch within 0.05 degrees of the game
  on all flight ticks (was up to 5.5 off).
- Turn: first game captures of a direction reversal (`local-analysis/d252-turn/`, story level, flat). Pose,
  speed and clip timing matched already. Fixed: facing through the turn follows the speed angle (the
  72-degree hold is for the stop and lane change only), and the walk restarts on frame 1. Vertex rms through
  the turn 1.9..3.8 (was 48..98). `docs/d250-air-lean-turn.md`.

## Before: D249 short legs after a landing (2026-10-12)
- Two-bone leg solver (`BlendPose::move_chain_end`); `LANDING_SQUASH` and a twelve-tick `LANDING_DIP`,
  within 0.4 of the game on touchdown +3..+8 (one standing jump). `docs/d241-landing-dip.md` (D249).

## Before: D248 crouch spring found, not the landing squash (2026-10-12)
- `stretch_crouch` spring (EBOOT 0x2011d4) is in `sackboy_anim.rs` (`CrouchSpring`, tested) but unfed; as a
  landing kick it does not fit. Idle, steady walk and stop match the game within 0.4 deg lean / 0.4 height.
  No extra in-air body layer in the EBOOT. Vertex dumps log `sackboy pose N`; the stand phase at a script
  tick varies between viewer runs (+-1 height).

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
- Turn: the game's top speed to the right was 29.33 in the captured level.
- Landing: only a standing jump is captured (ledge fall, running jump: same tables, unchecked). Air ease:
  arm terms (+0xae0/+0xae4) not modelled; upper arms 2 to 3 degrees off late in the flight.
- Head-top bones 22/23/24/27/28.
- After a landing the body rests 1.6 above the spawn height (floor cast skin); not looked into.
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; mirror flip types; arms mapping; face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Air ease arm terms; arms; emote clip frames vs game.
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
