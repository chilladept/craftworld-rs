# Current state (overwrite each milestone) — after D347

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D348 the goodies disc's pie (2026-10-17)
- Part 95 (`docs/level-scripts/part-95.md`): a part-filled goodies disc is two segments in one
  draw (FOUND 0x4f5abc / 0x4a3824): the collected share in the profile's colour, a 0.1 radian
  gap, the rest in the shade (0, 0, 0, 0.3). The viewer's `MenuShape::Pie`; the shader's
  zero axis and turn (straight up, clockwise) are INFERRED.

## D347 the info bubble's size and goodies row (2026-10-17)
- Part 94 (`docs/level-scripts/part-94.md`): the goodies row is the game's play stat widget
  (FOUND 0x496700 / 0x4f5400: a disc in the profile's second colour, the percent at S x 1.9,
  the label at S x 0.9 below); the bubble's texts at the Gooey scale 1.42 (MEASURED), its
  rows by a fitted line height, and the half-white line under the author. 345 x 148 pixels
  against the game's 348 x 144.

## D346 the Story badge's medals and bubble (2026-10-17)
- Part 93 (`docs/level-scripts/part-93.md`): the badge's overlay slots 1..12 (FOUND: GUIDs,
  offsets on the card, SearchResultSlot flags); `--profile-records` `overlays` lines (written
  by `scripts/rpcs3_profile_records.py`) pick them, so The Cosmos's card shows the gold ace,
  finish and collect medals. The info bubble's tail is pinned to the card's right rim.

## D345 the Story badge's frame colour (2026-10-17)
- Part 92 (`docs/level-scripts/part-92.md`): the badge's frame group (a solid grey; neither
  of the card's gmats has a ThingColor box) is multiplied by the profile's first colour from
  `--profile-records`, so the frame is gold as in the game. Left out: the light mat inside
  the rim, the goodies icons, the bubble overlap.

## D344 the Story badge's size (2026-10-17)
- Part 91 (`docs/level-scripts/part-91.md`): the game's group badge angle is 0.27 (RPCS3
  memory, planet results +0x210), not 0.18; at the Earth Thing's scale the card is 157 px
  tall against the game's ~161 (was 194). FOUND for next: the frame tint is the profile's
  first colour (0xffceb424 in the badge's instance).

## D343 the Story view's badge card (2026-10-17)
- Part 90 (`docs/level-scripts/part-90.md`): the default group's badge, `cosmos_polaroid.mol`
  with The Cosmos's icon on its picture, drawn on Earth while Story is up at the game's
  matrix (FOUND 0x6ffff4) in the Earth Thing's frame; its centre matches the owner's frame,
  its size about 1.18x. Left out: the yellow frame tint, goodies icons, the bubble overlap.

## D342 the profile's colour (2026-10-17)
- Part 89 (`docs/level-scripts/part-89.md`): the Story view's Gooey takes the local profile's
  colours each frame (FOUND 0x4381d8, `*(profile+0x514)` +0x4c..); `--profile-records` now
  carries them (`colours` line) and the viewer's bubble is the owner's yellow. The green was a
  new profile's default.
- FOUND for next: the planet badge is a mesh (`cosmos_polaroid.mol` 0x6a1a etc., result+0x220)
  with the slot icon in mesh 0's texture slot (0x6fe974).

## D341 the profile's per-slot records (2026-10-17)
- Part 88 (`docs/level-scripts/part-88.md`): the save is encrypted, so
  `scripts/rpcs3_profile_records.py` reads the local profile's per-slot records from LBP2 in
  RPCS3 (FOUND chain 0xE4F800 / 0xE50A04, records profile+0x280, owned list +0x78) and the
  viewer's `--profile-records PATH` shows the goodies row from them.

## D340 the info bubble's goodies row (2026-10-16)
- Part 87 (`docs/level-scripts/part-87.md`): a story level or group's bubble adds "N%" and
  "Goodies & prizes won" (key 0xd9b1b864) 15 frames after it is up, when its total is not
  0; FOUND: the counts come from the profile's per-slot records (0x199c9c, 0x43982c).
- Left out: reading the profile (row hidden, as for a fresh profile), the stat's icon and
  divider line, linked-level recursion.

## D339 the Story view's info bubble (2026-10-16)
- Part 86 (`docs/level-scripts/part-86.md`): while Story is up a filled speech bubble in the
  player's colour (style 13, FOUND 0x7006c0) shows the default result's name and author,
  its tail at the badge's measured screen place.
- Left out: text wrapping, the badge card.

## D338 the Story globe's AO overlays blended (2026-10-16)
- Part 85 (`docs/level-scripts/part-85.md`): a solid material with its alpha port wired
  (the AO overlays `cloth_earth_land_ao` / `cloth_earth_sea_ao`) blends by that port, so
  the Story globe's continents show copper.

## D337 the Story globe's jelly on the glass route (2026-10-16)
- Part 84 (`docs/level-scripts/part-84.md`): `proc_victoria_jelly_glassy` (0x1a6f5) takes
  the glass route (opacity 1, rim in full); a static gloss colour above 1 no longer drops a
  material's whole surface in lbp-preview.
- FOUND: the globe's black continents are the AO overlays `cloth_earth_land_ao` /
  `cloth_earth_sea_ao` (black on port 0, texture x 0.8 alpha on port 1), drawn opaque.

## D336 the Story backdrop and its lighting (2026-10-16)
- Part 83 (`docs/level-scripts/part-83.md`): while Story is up the viewer swaps the Pod's
  backdrop for The Cosmos's own (0xdb2c4, FOUND) and lights the Pod world with that
  backdrop plan's preset under The Cosmos's PWorld factors (0xda074, FOUND). With
  `--lbp-lighting` the room fades to dark with coloured dots as in the game.
- D331 (part 82): Story turns Earth to The Cosmos; Story never zooms (view 6).
- D330 (part 81): the Story title's speech bubble.

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
- Git: remote origin github.com/chilladept/craftworld-rs (private), default-deny
  `.gitignore`, CI `.github/workflows/ci.yml`. Standing rules: CLAUDE.md.
- Native QA windows hidden or on the non-primary monitor; never steal focus.

## Where detail lives
Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`. Earlier milestone
docs under `docs/` (grep headings); format evidence `docs/formats.md`; rules `AGENTS.md`.
