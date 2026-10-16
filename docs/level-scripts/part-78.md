# Level scripts, part 78: Story focuses Earth (2026-10-16, D327)

Part 77 let `PodStateSearch` run for Story, but every native it asks of its SearchDetails,
its results and the planets was unbound: 742 unbound calls in the session, a null focus
planet, and the planets' view stayed on the main menu's (1). Here those natives answer, the
search state selects Earth, and the planets switch to the Story view.

### What was found

- FOUND (the 01.33 listings): `SearchDetails.GetResults__` returns the native class
  `SearchResults` (base `ResultCollection`, no script fields; `GetNumResults__` s32 on the
  base, `GetWantZoom__` bool); `Planets.GetFocusPlanet__` returns a `Planet`;
  `SearchDetails.GetPlanet__` and `GetCurrentPage__` are s32; `IsPaged__`,
  `IsBusyObtainingResults__`, `RequiresOnline__`, `CheckOverlayNeedsUpdate__` bool. The
  search state uses the focus planet and the results without a null check.
- FOUND: `OnActivateImpl` passes `Details.GetPlanet__()` to `Planets.SelectPlanet__i`, and
  `GetPlanetsViewType__` gives the Story views (6, or 7 when the results want zoom) only for
  planet 1.
- FOUND: with the results obtained, no results and page 0, the state calls
  `Details.ShowErrorMessage__` (a native); the scripts draw no no-results text themselves.
- INFERRED: the story levels are on Earth (planet 1, crafteroid 1), the focus planet is the
  selected crafteroid, and the search is local: obtained at once, one page, offline.

### What changed

- Runtime: `LevelWorld::search_results` (one SearchResults object made with the planets);
  the search state's natives answer in `gooey.rs` (`search_query`), and what it hands the
  planets and the pod computer is kept in `PodState::search` (`PlanetsSearch`: selected
  planet, cursor mode, browse mode, allowed browse modes, loader, earth control, calls).
- The results stay empty (the story levels the game lists were not read), so
  `ShowErrorMessage__` is called once and counted; nothing is drawn for it.
- Session report: `pod.at_end.search`.

### Session (pod.bin with pod_planets.bin, play 720, in use at 600, choose at 640)

| | Before | After |
| --- | --- | --- |
| Faults | none | none |
| Unbound native calls | 742 (24 natives) | 2 (`Gooey.InvalidateFrame__i`, `Slot.GetLockState__Q6SlotID`) |
| Planets view from 640 | 1 (main menu) | 6 (Story), transition 0.05 to 1.0 by 659 |
| Search state's planets | none | Earth selected, cursor mode 4, browse mode 1, allowed 0, loader 0, earth has control |

### Left out

- The story results themselves (the slots the game lists on Earth), the page natives and
  `ShowErrorMessage__`'s message; the planets' cursor and browse modes are kept, not acted on.
- The viewer still chooses only under `--pod-menu-choose`. Nothing in RPCS3 was started.

### Verification

`search_state_selects_the_focus_planet_and_keeps_its_modes` (lbp-runtime). `cargo fmt --all
--check`; `cargo clippy --workspace --all-targets -- -D warnings`; workspace tests. Session
files stay in ignored `local-analysis/d327-story-results/`.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).
