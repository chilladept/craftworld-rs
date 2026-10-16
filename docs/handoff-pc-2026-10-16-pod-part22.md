# Handoff: Pod picture, part 22 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-75.md`, this note.

## State

- Merged: PR 89 D324 (the menu's entrance: one layer, ring blur, alpha (age + 1) / 15; the
  focused pill's spring to 1.5 and its slab from the spring's peak; part 75).
- Open: branch `d325-pod-menu-input` (draft PR, WIP, no docs part yet). Worktree
  `E:\Projects\wt-d324` is on it. Nothing running; RPCS3 and RenderDoc closed.

## D325 so far (runtime only, `crates/lbp-runtime/src/gooey.rs`, example `level_scripts`)

- `GooeyInput { choose, back }`, `GOOEY_ACTIVATE` 256, `GOOEY_BACK` 512, `focus_since`,
  `build_frames`, `GooeyState::move_focus(dir, fonts, frame)` (nearest button within 60
  degrees, INFERRED). The focused button answers 256 once when chosen and its mask has 256;
  `EndFrameInput(512)` answers 512 once on back; `NeedsUpdate` is true while input waits.
- `level_scripts --menu=TICK:KEY,..` (up/down/left/right/choose/back). Session (pod.bin with
  pod_planets.bin, play 720, in use 600): down moves Story -> Community -> Recent Activity
  (focus 9, 10); back pops the main menu and the pod computer closes (request_host_use_pod
  false, planets view 0), as the game's circle does; choose on Story pushes PodStateSearch,
  which faults at `PodStateSearch.GetExtraPlanetZoom__+3` (null SearchDetails: unbound
  `SearchDetails.CreateStoryLevels__`, `Planets.SelectPlanet__i`, `Planets.SetCursorMode__i`,
  `PodComputerPlanets.SetWantSearchDetails__...`, `Gooey.InvalidateFrame__i`).
- Open problem: the menu is rebuilt only at ticks 601-603, 633-634, 664 (every ~30), so input
  acts up to ~1 s late. `NeedsUpdate` true every tick does NOT add builds: some path in
  `PodComputer.PerFrameUpdate__b` (listing 16e6e8c9..., UpdateUI call at pc 282, reached via
  pc 227 -> 276) skips the rebuild. Find that gate next (the game reacts at once).
- Script facts (subagent notes, `local-analysis/d325-menu-input/notes.md`): each entry pushes
  its state (Story PodStateSearch(CreateStoryLevels), Community PodStateCommunityMenu, Recent
  Activity PodStateStream, Store PodStateStoreMenu, Me PodStateMeMenu); masks 256 only for
  unlocked entries (flags 4/8/16/32); focus moves are engine code; the scripts never point the
  planets (native PlanetsView / Callout).
- Gooey EBOOT hunt (`local-analysis/d325-gooey-focus/notes.md`): the spring and slab rule not
  found; Gooey code uses TOC r2 0xdaa0c0; button node ctor 0x4d54e4; the ring-blur
  directions live in FUN_003162a4.

## Next

1. The rebuild gate above, then viewer keys (arrows move, Enter choose behind a flag until
   Story's state works, Backspace back) and the spring restarting from `focus_since`.
2. Story's search state: bind the SearchDetails / Planets natives it needs.
3. Part 76 doc (`docs/level-scripts/part-76.md`) and STATUS-CURRENT when D325 lands.
