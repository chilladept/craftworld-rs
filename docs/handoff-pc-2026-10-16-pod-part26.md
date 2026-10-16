# Handoff: Pod Story results and Story frame, part 26 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-79.md`, `docs/level-scripts/part-80.md`,
this note.

## Facts

- Open, CI green: PR 100 D328 (branch `d328-story-slots`, Story results from
  `lbp2story_slots.slt`). The PC classifier blocked `gh pr merge`; the owner was asked in the
  thread to say "merge 100". Merge it once they do.
- Open: PR 101 D329 (branch `d329-story-frame`, base `d328-story-slots`, stacked): viewer
  choose always on, `--level-scripts-choose-at N`, story slots read, items respawn per frame.
  After PR 100's squash merge, retarget PR 101 to main and rebase it onto main (drop the D328
  commit), check CI, merge.
- Worktree `E:\Projects\wt-d324` on `d329-story-frame`. Nothing running; RPCS3 not started.
- Session output must be a NEW file under `local-analysis/` (the tool refuses an existing one).
- Viewer capture: part 80's flags with `--snapshot`/`--game-dir` as part 23, run hidden via
  Start-Process -WindowStyle Hidden; capture in `local-analysis/d329-story-frame/`.
- Natives reached through `ui_call` must be listed in `UI_NATIVES` (lbp-script-vm natives.rs);
  table natives bump the `bound().count()` asserts (now 263) and `STATE_QUERIES` length.
- Locally failing, unrelated: two `lbp-viewer` diagnostics path tests.

## Next

1. Merge PR 100, then rebase, retarget and merge PR 101.
2. Draw the Story results on Earth (the planet's cursor and badges); compare the Story title
   with a game frame.
