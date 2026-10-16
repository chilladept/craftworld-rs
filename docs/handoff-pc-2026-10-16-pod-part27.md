# Handoff: Story title bubble, part 27 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-81.md`, this note.

## Facts

- Merged: PR 100 (D328 Story results).
- Open, rebased on main, pushed: PR 101 (D329, branch `d329-story-frame`). Its base on
  GitHub may still read `d328-story-slots`: retarget it to main, check CI, merge. The PC
  classifier blocked `gh pr edit/merge/view` on it; the owner was asked in the thread to
  type "yes" so an allow rule for `gh pr merge` (no --auto/--admin) can be added to this
  folder's Claude settings. Do that once they answer, then merge.
- Open draft: PR 102 (D330, branch `d330-earth-badges`, base `d329-story-frame`, stacked):
  style-15 speech bubble for the Story title. After 101 merges: rebase onto main
  (`git rebase --onto origin/main d329-story-frame`), retarget, mark ready, merge on green.
- Worktree `E:\Projects\wt-d324` on `d330-earth-badges`. Nothing running; RPCS3 closed.
- Viewer capture: scratchpad-style script = part 80's flags with game dirs
  `game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay;local-analysis/d321-disc-fonts`
  (the fourth root loads the Gooey fonts) and `--snapshot local-analysis/d133-inventory-0133-edat.json`;
  `CARGO_TARGET_DIR=target-d320`. `--preview-lighting` uses the overlay route, not the menu layer.
- Game reference frames: `local-analysis/d330-earth-badges/game/` (pod menu, Story at 0.6 s on
  Earth, settled on The Cosmos world, one step right = The Factory). Boot recipe: d309-push/launch.py
  minimised, then bgpad x, enter, x, z (Square = Use Pod Computer).
- EBOOT findings (badges 0x6ffff4, sizes 0x16afb0, visibility 0x428174, decorations swap
  0x43cd64/0x42324c) are in part 81; decompiles in `local-analysis/d330-earth-badges/ghidra/`.
- Locally failing, unrelated: two `lbp-viewer` diagnostics path tests.

## Next

1. Merge PR 101, then PR 102.
2. The Story zoom: load the default group's planet decorations into Earth's world and draw its
   group badge (part 81's transform and sizes); compare with `game/b-story-settled.jpg`.
3. The title's remaining ~10 px offset (likely the camera zoom to Earth).
