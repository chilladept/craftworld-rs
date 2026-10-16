# Handoff: Pod Story search, part 24 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-77.md`, this note,
`local-analysis/d326-story-search/notes.md` (canonical repo, ignored).

## Facts

- Merged: PR 94 D326 (Story's search state runs: `Host::native_object_class` factory natives,
  `SearchDetails.CreateStoryLevels__` -> `SearchDetailsStoryLevels`, `Planet.GetCurResult__` ->
  empty `SearchResultHandle`, `Gooey.FontScale` via `Get/SetStylesheetScalingFactor`,
  `System.GetLanguageID__` = 1 English; part 77).
- Branch `d327-story-results` (this note only) in worktree `E:\Projects\wt-d324`. Nothing
  running; RPCS3 not started this thread.
- Session command: as part 23's note (`level_scripts ... --menu=640:choose`), output under
  `local-analysis/d326-story-search/`; choose on Story now runs with 0 faults and builds frame
  (2, 1) with "STORY" at font scale 2.5.
- Locally failing, unrelated: two `lbp-viewer` diagnostics path tests.

## Next

1. D327: Story's results and focus planet (`SearchDetails.GetResults__`,
   `ResultCollection.GetNumResults__`, `Planets.GetFocusPlanet__`, browse modes, paging) per the notes.
2. Viewer: draw the Story frame and lift the `--pod-menu-choose` gate once it looks right.
