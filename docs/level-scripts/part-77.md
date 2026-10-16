# Level scripts, part 77: Story's search state runs (2026-10-16, D326)

Part 76 let the pad choose Story, but the state it pushed (`PodStateSearch`) faulted on its
first frame, so the pod computer stopped and nothing was drawn. Here the state runs: its
SearchDetails is a real script object, the focus planet's current result is an empty handle,
and the Story title bubble is built at the game's English font scale.

### What was found

- FOUND (the 01.33 listings): `SearchDetails.CreateStoryLevels__` is a static native whose
  declared return type is the script class `SearchDetailsStoryLevels` (base `SearchDetails`,
  with `.init__` / `.ctor__` and the virtuals the state calls: `GetScale` 3.0,
  `GetDefaultBrowseMode` 1, `DoTitle`). The engine hands back a script object of that class.
- FOUND: `PodStateSearch.PerFrameUpdate` reads `GetFocusPlanet().GetCurResult().GetType()`
  every frame; `SearchResultHandle` is a script class whose `GetType__` reads `_Type`, and its
  plain `.ctor__` makes an empty handle (`_Type` 0).
- FOUND: the script property `Gooey.FontScale` has code: its getter calls
  `GetStylesheetScalingFactor__` and its setter `SetStylesheetScalingFactor__f` (a table native
  until now only recorded), so the runtime's font scale was never set by the scripts.
- FOUND: `IsCJK__` is script code over `System.GetLanguageID__` that answers true for 0 (and
  9, 10, 11); the unbound default 0 took the CJK branch of `SearchDetailsStoryLevels.DoTitle`
  (font scale x1.5 instead of x2.5).
- INFERRED: no result is under the cursor yet, so the current result is the empty handle;
  the language is English (the PS3's CELL_SYSUTIL_LANG_ENGLISH_US, 1).

### What changed

- Script VM: `Host::native_object_class` / `World::native_object_class`: a factory native
  makes a script object (its fields, then `.init__`, then `.ctor__`) and the call answers it.
- Runtime: `SearchDetails.CreateStoryLevels__` makes a `SearchDetailsStoryLevels`;
  `Planet.GetCurResult__` makes an empty `SearchResultHandle`; `GetStylesheetScalingFactor`
  answers the Gooey's font scale and `SetStylesheetScalingFactor` sets it;
  `System.GetLanguageID__` answers 1.

### Session (pod.bin with pod_planets.bin, play 720, in use at 600, choose at 640)

| | Before | After |
| --- | --- | --- |
| Faults | `PerFrameUpdate` at `SearchResultHandle.GetType__+0` (null) | none |
| Top-level frames | (1, 1) | (1, 1), (2, 1) |
| Story frame | not built | named frame 1, 4800 x 1250, columns -0.5 / 1500 / -0.5, a style 15 bubble with "STORY" (style 16, font scale 2.5) |
| Pod computer draws | stop at 640 | 720 |

### Left out

- The search results (`SearchDetails.GetResults__`, `ResultCollection.GetNumResults__`), the
  focus planet (`Planets.GetFocusPlanet__` stays unbound, null), browse modes, paging and the
  planets' cursor: unbound, default answers.
- The viewer still chooses only under `--pod-menu-choose`; the Story frame was not drawn in a
  window here. Nothing in RPCS3 was started.

### Verification

Script VM: `factory_native_makes_a_script_object`. `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; workspace tests. Session files stay
in ignored `local-analysis/d326-story-search/`.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).
