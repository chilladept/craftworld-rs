# Handoff: Pod main menu, part 23 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-76.md`, this note,
`local-analysis/d326-story-search/notes.md` (canonical repo, ignored).

## Facts

- Merged: PR 92 D325 (menu input; `Gooey.NeedsUpdate` true while input waits makes
  `PodComputer.NeedUpdateUI` rebuild on the key's tick; viewer arrows / Backspace / Enter under
  `--pod-menu-choose`; part 76).
- Open: branch `d326-story-search` (draft PR, WIP), worktree `E:\Projects\wt-d324`. Nothing
  running; RPCS3 not started this thread.
- D326 so far: `Host::native_object_class` / `World::native_object_class` (script-vm): a factory
  native makes a script object (fields, `.init__`, `.ctor__`) and the call answers it.
  `LevelWorld` maps `SearchDetails.CreateStoryLevels__` -> `SearchDetailsStoryLevels`. With it
  choose on Story builds at tick 640 and the next fault is
  `PodStateSearch.PerFrameUpdate` -> `SearchResultHandle.GetType__+0` (null): the focus planet
  (`PodComputerPlanets.GetFocusPlanet__`, unbound, null) has no current result
  (`GetCurResult__`). Story needs the planets' focus / results natives (list in the notes).
- Session command (cwd canonical repo; output must be under local-analysis/):
  `level_scripts local-analysis/d133-inventory-0133-edat.json
  "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"
  gamedata/levels/pod/pod.bin OUT --play=720 --puter-in-use=600
  --with-level=gamedata/lbp2/pod/levels/pod_planets.bin --menu=640:choose`
  (exe in `E:\Projects\wt-d324\target\release\examples`). Tick 601 = PWorld frame 219866.
- Locally failing, unrelated: `input_script_steps` and two `lbp-viewer` diagnostics path tests.
- Repo auto-merge is off: merge green PRs with `gh pr merge N --squash`.

## Next

1. D326: decide how far Story goes: bind GetFocusPlanet / GetCurResult / SearchResults
   (GetResults non-null, GetNumResults) as small script-object or null-safe answers so the
   state runs; then its UpdateUI title bubble (DoTitle, no buttons) and Draw (GetScale 3.0).
2. Unit test for the factory hook (script-vm), part 77 doc, STATUS-CURRENT, merge.
