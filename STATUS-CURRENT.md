# Current state (overwrite each milestone) — after D259

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D259 trigger events (2026-10-12)
- RScript plan stage 2. PTrigger is kept by `lbp-level`; `LevelWorld::update_triggers` tests the players
  against every trigger and `LevelScripts::tick_level` calls `Enter__Q5Thing`, `Inside__Q5Thing`,
  `Leave__Q5Thing`. FOUND in the EBOOT (0xe3bc0, 0x1364f0, 0x135fe0, 0x135a18, 0x1346fc): position in the
  trigger Thing's space; circle, box or sphere of the radius multiplier, times the hysteresis multiplier
  while something is inside; layer rule by the z span of Sackboy's body; Enter/Leave to the Thing's script
  or its parent's, Inside to the parent's. 2 natives bound (118). Viewer: `--level-scripts` with
  `--play-sackboy` feeds the Sackboy to the triggers (his z span is a PLACEHOLDER).
- MEASURED, 378 levels: 23,990 triggers, 4,662 scripted. `level_scripts --all --play=60 --visit-triggers`:
  11,430 Enter, 895 Inside, 7,357 Leave calls run; stopped Things stay 1,722; unbound natives reached
  65 -> 94. Next: stage 3, switch events. `docs/level-scripts.md` part 10.

## Before: D258 scripts run past unbound natives (2026-10-12)
- RScript plan stage 1 (`docs/rscript-plan.md`). An unbound native returns its type's default and is counted
  (`Bindings::default_unbound`; viewer and `level_scripts --play`; off in tests). `level_scripts --all
  --play=N` reports bound and unbound calls and the faults left over every level. 19 natives bound (116).
- MEASURED, 378 levels, 60 ticks, 13,547 scripted Things: stopped by a fault 4,940 -> 1,722; 76,269 calls
  to 64 unbound natives defaulted. Left: switch inputs 1,143, `PWorld.GameMode` 220, local transforms 199,
  TriggerLevelLink 113. Next: stage 2, trigger events. `docs/level-scripts.md` part 9.

## Before: D257 ledge-fall jump layer weight (2026-10-12)
- MEASURED on three RPCS3 ledge falls (lower-body vertex fits): the jump layer eases in with the 0x20a5f4
  follower from 0, peak += 0.125 (|vy| - peak), weight clamp(peak/10) times the blend-in; full weight about
  10 ticks after the edge (ours stopped near 0.77). w error 0.064 (D234 ramp 0.276). Landing tail at full
  weight. INFERRED: the follower holds under the landing tail. `docs/d250-air-lean-turn.md` (D257).

## Before: D256 standing foot placement (2026-10-12)
- FOUND in the EBOOT (foot pass 0x210dd8): standing (ground-relative speed under 1), each Foot joint is put
  on the floor point nearest its leg anchor, 25 to either side of the body, by a two-bone solve; in over 3
  ticks, out over 5. `FOOT_PLANT_*`, `Sackboy::foot_plant`, applied in `blend_frame`. MEASURED: foot spacing
  43.6 (game 43.4, was 51.9), standing rms 0.79 (was 1.30). `docs/d250-air-lean-turn.md` (D256).

## Before: D255 stick response, shortest jump, mirrored morphs (2026-10-12)
- FOUND in the EBOOT (0x38a578, 0x38aa10): stick x = (byte - 128) / 128 unclamped, response past the dead
  zone rescaled and squared, times both speed caps. Full right is 0.9778: MEASURED 29.34 / 22.00 to the
  right against 30.00 / 22.50 (Moon and Rookie Test). `stick_response`, `STICK_FULL_RIGHT`.
- MEASURED (d211): one- and two-tick presses peak at 33.1 and 90.4; `JUMP_MIN_TICKS` 1.
- Mirrored morphs applied (FOUND 0x1e42b4). Arm terms read: 0 without the arm buttons, no change.
  `docs/d250-air-lean-turn.md` (D255).

## Before: D254 mirrored poses (2026-10-12)
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
  against it). Open: landing dip after touchdown (game 195, ours 200), stop/turn
  poses, lean. `docs/d242-idle-stretch.md` (D246).

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- Mesh census 01.33 (4,261 GUIDs, after D188-D190): 4,259 viewable, 4,239 all-diffuse by
  default, 0 Complete. Files `local-analysis/b177-{default,legacy}.json`.
- Texture census 01.33: all 23,247 hashes decode (top mip, decodability only).
- Levels 01.33: 398 resources; lbp-level walks 236 in-band, Toolkit parity 236/236.
- D233: +0 newly viewable / +0 newly Complete (runtime feature).

## Open blockers
- Foot placement on slopes and steps (the standing spot is fixed in model space, not the contact point).
- Landing: only a standing jump is captured (ledge fall, running jump: same tables, unchecked). Upper arms
  2 to 3 degrees off late in the flight, unattributed (the arm terms are 0 there).
- Head-top bones 22/23/24/27/28.
- After a landing the body rests 0.5 to 1.5 above the spawn height (shape cast distances).
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; arm buttons (the arm clips); face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Emote clip frames vs game.
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
