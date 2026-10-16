# Handoff: Story turns Earth to The Cosmos, part 30 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-81.md`, part 29's handoff "Facts", this note.

## State

- PRs 101 and 102 merged. Draft PR 103 (`d331-story-zoom`, worktree `E:\Projects\wt-d324`) is
  rebased onto main, base main. `gh pr merge` worked for 102 after `gh pr ready`.
- Done in 103 (this part): `LevelWorld::story_open` (moved from the viewer) and
  `LevelWorld::story_planet_point`: while Story is up, Earth's planet point is the default result's
  slot `location` x `EARTH_RADIUS` (1750), set before the planets' entries each tick. Capture
  matches the game: the machine on top, The Cosmos facing the camera, same Earth size.
  Test `d331_story_turns_earth_to_the_default_group`.
- FOUND (no zoom in Story): `SearchResults.GetWantZoom__` (0x6057bc) reads results +0x5d, cleared
  only by the results constructor (0x456a98), set only by a group search (0x44dfc0, from the group
  search constructor 0x44e774 = `CreateGroup__Q6SlotIDb`, also 0x706e00, 0x70b030) for SlotID types
  3, 4, 8, 9. Story results come from 0x449ce8, so it stays false (view 6). View 7 differs from 6 only
  in Offset.z (-2000 vs -3750) and would make Earth ~1.8x larger than the game frame.
  `SearchDetailsGroup.GetExtraPlanetZoom__` is 500 for types 4/9; StoryLevels is 0.
- Capture: scratchpad `cap.ps1` (part 80 flags, part 27's four game-dir roots, release build in
  `target-d320`, run from the main checkout). Captures: `local-analysis/d331-story-zoom/turn1-760.png`,
  comparison `cmp2.jpg`.

## Next

1. Write `docs/level-scripts/part-82.md` (part 29's facts + the above), MODLOG and STATUS entries,
   then mark 103 ready.
2. Backdrop: the game's Story frame is dark space with coloured dots and a dark room. The shell
   level has no backdrop Thing; the planets level's lighting presets come from its backdrop plan.
   The decorations level's LEVEL_SETTINGS names plan 00019c73; `story_world::merge` drops that
   level's backdrop items. Likely a lighting-preset swap plus the plan's meshes while Story is up.
3. Copper continents (gmat 108277) draw dark; the badge card and its info bubble.
