# Level scripts, part 80: the viewer draws Story (2026-10-16, D329)

Part 79 gave the Story search its results, so choosing Story runs without a fault. Before
this part the viewer still chose a menu entry only under `--pod-menu-choose`, did not read
the story slots, and spawned the menu's items again only when their count changed. Here
choosing is always on, the viewer's planets level gets the story slots, and the Story
frame is drawn.

### What changed

- `lbp_preview::story::read_story_slot_list` reads `lbp2story_slots.slt` (GUID 000162ec);
  the viewer and the `level_scripts` example both use it for `LevelWorld::story_slots`.
- Viewer: Enter / Cross choose the focused entry with no flag (`--pod-menu-choose` is
  gone). New QA flag `--level-scripts-choose-at N`: the focused entry is chosen on tick N
  (as `level_scripts --menu=N:choose`).
- `ScriptMenuFrame::built_at` (the frame's first build): the items are spawned again when
  the drawn frame changes, not only when their count does; a line is printed per frame.

### Viewer run (1280 x 720, `--qa-capture`, window hidden)

`--level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --level-scripts --window-16-9
--preview-lighting --qa-hide-ui --level-scripts-in-use-at 600 --level-scripts-choose-at 640
--level-scripts-hold-at 700`: the main menu is built on tick 601 (6 items), the choose is on
640, and the Story frame on 641 (1 item, the "STORY" title, at the left of Earth). The
capture stays in ignored `local-analysis/d329-story-frame/`.

### Left out

- The title's size and place were not compared with a game frame; the Story results (82)
  and the planet's cursor are not drawn; Back from Story was not tried in the viewer.

### Verification

`cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
workspace tests; the viewer run above.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
