# Handoff: Story shows The Cosmos world, part 29 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-81.md`, this note.

## Facts

- PR 101 (`d329-story-frame`, base retargeted to main, CI green) and PR 102 (`d330-earth-badges`,
  stacked) are unmerged: `gh pr view/edit/checks/create` now work, but the auto-mode check refuses
  `gh pr merge` (also close/reopen and assignee calls). The owner was asked to merge 101 by hand.
  After 101 merges: `git rebase --onto origin/main d329-story-frame` on 102, retarget, merge; same
  for 103.
- Draft PR 103 (`d331-story-zoom`, base `d330-earth-badges`), worktree `E:\Projects\wt-d324`.
  No part-82 doc yet (write it with the findings below; MODLOG/STATUS entries too).
- Done in 103: `story_world.rs` (viewer) loads the default group's planet decorations level
  (slot `planet_decorations`, group `lbp_runtime::STORY_DEFAULT_GROUP` = (4, 27), now the runtime
  default) with the scripted Pod and pastes Earth's subtree (Things under the decorations Earth,
  via new `LevelScene.parents`) onto Pod Thing 1 as alternate items, matrix podEarth x
  inverse(decoEarth) of the first EARTH_MESH (0x27bd) model items. 58 items, 8 meshes. The level's
  moon (Things 5, 6) is left out (it landed in front of the camera).
- `level_play::follow_story_world` shows them and hides Pod Earth + Moon (Thing 4) while
  `story_open`: `pod.search.selected_planet == Some(1) && want_search_details_calls > 0`
  (MEASURED tick 640; `SetSearchResults` is never called).
- Fix: `skin_bone_things` skips decoration items (Earth rocks collapsed to Earth's centre).
- gmat 108277 "sampler must be static" was the probe loading without the level defaults; the
  probe now loads as the scene does and `material_graph_dump` prints the diffuse verdict.
- Capture: scratchpad-style script = part 80 flags + `--level-scripts-hold-at 760`, game dirs as
  in part 27's handoff, `CARGO_TARGET_DIR=target-d320`. Captures in ignored
  `local-analysis/d331-story-zoom/` (cosmos4-760.png; compare jpg vs `game/b-story-settled.jpg`).

## Next

1. The zoom: the game turns Earth so the group's badge faces the camera (machine on top) and moves
   the camera close (Earth fills ~60% of the height); badge at location x 1750, scale
   1750 x 0.18 / 240 (part 81).
2. Backdrop swap to plan 00019c73 while Story is up (no runtime swap exists; `load_backdrop`).
3. Copper continents draw dark red (gloss/reflection not drawn); the badge card and its bubble.
