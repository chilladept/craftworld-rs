# Handoff: Story zoom groundwork, part 28 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-81.md`, this note.

## Facts

- Open: PR 101 (D329, `d329-story-frame`) and PR 102 (D330, `d330-earth-badges`, stacked), unmerged.
  The PC classifier refuses every `gh pr` command (view/edit/merge) and refused the settings
  edit even after the owner's in-thread yes. The owner was asked to add the allow block
  (`gh pr merge/view/edit/ready/checks`) to this folder's `.claude/settings.json` by hand.
  Once it is in: retarget 101 to main, merge on green; rebase 102 onto main
  (`git rebase --onto origin/main d329-story-frame`), ready, merge; then this branch.
- New branch `d331-story-zoom` (stacked on `d330-earth-badges`), pushed, no PR yet, in worktree
  `E:\Projects\wt-d324`. It keeps PMaterialOverride (`PartData.material_override`) and applies
  it in level scenes as a per-Thing mesh variant (region -> gmat swap; uvScale not applied).
  Tests `d331_*` pass; clippy clean. No part-82 doc yet.
- FOUND: The Cosmos is group (4, 27); its planet decorations are a LEVEL (g102473 = 0x19049,
  `earth_decorations_cosmos.bin`), not a plan: Thing 1 Earth (mesh g10173, MATERIAL_OVERRIDE
  body -> gmat 105200 blue rock, continent regions -> gmat 108277), child Thing 3 decoration
  mesh g144897, Thing 5 moon g15982 + child g145741, LEVEL_SETTINGS backdrop plan 00019c73
  (the game's Story background). The Pod level 00018fdb has the same Earth (Thing 1, g10173)
  and moon (Thing 4). The game swap (0x42324c) clears Earth's world and pastes the level.
- Open issue: gmat 108277 (Herbert's copper continents) fails ("sampler must be static,
  unwired UV0/UV1") and draws dark red.
- Probe: `lbp-preview` example `story_group_plan_probe SNAPSHOT DIRS 27|x<guid>` (Things,
  overrides, primitive regions). Capture script pattern: part 80's flags; local captures in
  `local-analysis/d331-story-zoom/`. Runtime default result is still hard-coded (4, 22) in
  `lbp-runtime/src/lib.rs` `add_story_results`; the owner's profile settles on (4, 27).

## Next

1. Merges above, then a PR for `d331-story-zoom` (write part-82).
2. On the Story frame: load 0x19049's Earth overrides/decoration children onto the Pod Earth
   (transform podEarth x inverse(decoEarth)), swap the backdrop to 00019c73; badge at
   location x 1750, scale 1750 x 0.18 / 240; camera zoom; compare `game/b-story-settled.jpg`.
3. gmat 108277's sampler failure.
