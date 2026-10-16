# Level scripts, part 79: Story lists its slots (2026-10-16, D328)

Part 78 let the Story search state focus Earth, but its results were empty: `GetNumResults__`
answered 0, so the state called `ShowErrorMessage__`, and every native on a result was
unbound. Here the results are the slots of `lbp2story_slots.slt`, made the way the engine
makes them.

### What was found

- FOUND (01.33, Ghidra): `native_CreateStoryLevels__` (0x5f3f74) builds the Story search
  (ctor 0x44a0c8); its setup 0x449ce8 calls 0x40f120 with the game's slot list and no
  filter. That clears the results, adds one SearchResultSlot per slot in list order (groups
  included), then links each slot with a group as a child of the group's result.
  `GetNumResults__` (0x605b5c) is the size of that flat array.
- FOUND (0x449ce8): the default result is group (4, n), n the highest of 27 down to 22 whose
  lock state (0x1a7efc) is above 0; a mode bit forces 22.
- FOUND (the 01.33 listings): SearchResultSlot is a script class (base SearchResult);
  SearchResultHandle is script code (`_Type` 1 and `_SlotID` for a slot); the result,
  handle, planet-cursor natives and their return types; the pod computer compares
  `GetResultType__` with 1 and treats a parent handle that is not `Empty__` as a group.
  `SearchDetailsStoryLevels.WantVO__` asks the lock state of group (4, 23).
- INFERRED: the slot list is `lbp2story_slots.slt` (82 slots: 6 groups 22..27, 76 levels,
  all grouped); result type 1 is the slot kind; every lock state is 0 (the profile's were
  not read), so the default is group 22 (Da Vinci's Hideout) and the first-visit line plays.

### What changed

- Runtime: `LevelWorld::story_slots` (a caller supplies them, `SearchSlot::from_list`) and
  `story_results` (a SearchResultSlot, SlotID and SearchResultHandle per slot, with the
  group's index), made in `add_engine_objects`; an empty handle for no parent.
- `gooey.rs` `story_result_query`: `ResultCollection` (`GetNumResults__`, `GetResult__i`,
  `FindResult__`, `FindResultSlot__`, `FindResultIndex__`), `SearchResults` default result,
  `SearchResult` / `SearchResultSlot` natives, the Earth planet's cursor (`GetCurResult__`,
  `MoveCursorToResult__`, `PickDefaultResult__`), `GriefReport.SetContext__`. Handles and
  SlotIDs are matched by object identity (a native cannot read a script object).
- `Slot.GetLockState__Q6SlotID` is a state query answering 0.
- `level_scripts` reads `lbp2story_slots.slt` (GUID 000162ec) for the second level;
  `pod.at_end.search` adds the default and cursor results.

### Session (pod.bin with pod_planets.bin, play 720, in use at 600, choose at 640)

| | Before | After |
| --- | --- | --- |
| Faults | none | none |
| Unbound native calls | 2 | 1 (`Gooey.InvalidateFrame__i`) |
| Story results | 0, `ShowErrorMessage__` once | 82, no error message |
| Default result | none | 16 (group 22, Da Vinci's Hideout) |

### Left out

- The profile's lock states; which results the planet shows before a zoom; the cursor's
  movement; nothing is drawn for the results yet (the viewer step is next).

### Verification

`story_results_list_every_slot_and_link_levels_to_their_group` (lbp-runtime). `cargo fmt
--all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; workspace tests.
Session files stay in ignored `local-analysis/d328-story-results/`.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).
