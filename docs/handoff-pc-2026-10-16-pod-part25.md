# Handoff: Pod Story focuses Earth, part 25 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-78.md`, this note.

## Facts

- Merged: PR 99 D327 (`gooey.rs` `search_query`: the search state's SearchDetails, results
  and planets natives; `PodState::search` keeps what it hands the planets; one SearchResults
  object in `LevelWorld::search_results`; part 78). Session: unbound calls 742 -> 2
  (`Gooey.InvalidateFrame__i`, `Slot.GetLockState__Q6SlotID`), planets view 6 from 640.
- Branch `d328-story-slots` (this note only) in worktree `E:\Projects\wt-d324`. Nothing
  running; RPCS3 not started. Toolchain: dot-source the canonical repo's
  `scripts/use-local-rust.ps1` (the worktree has none).
- Session command: as part 23's note, output under `local-analysis/d327-story-results/`.
- Story slots: `gamedata/data/lbp2story_slots.slt` (GUID 000162ec, 82 slots), read by
  `lbp_level::slots::read` (slots.rs:216); `lbp_preview::story::load_story` names them.
  Da Vinci's Hideout = group slot 22 (docs/d149-davinci-levels.md). Which slots the game's
  Story search lists (types, order, Earth filter) is not established: EBOOT/Ghidra question.
- New owner rule: attach small progress screenshots at good opportunities (never committed).
- Locally failing, unrelated: two `lbp-viewer` diagnostics path tests.

## Next

1. D328: Story results: find how the native search fills SearchDetailsStoryLevels'
   results (Ghidra), then answer `GetNumResults__` / `FindResult` / `Planet.GetCurResult__`
   from `lbp2story_slots.slt`.
2. Viewer: draw the Story frame and lift the `--pod-menu-choose` gate; screenshot it.
