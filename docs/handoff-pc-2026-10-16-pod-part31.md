# Handoff: Story backdrop and lighting, part 31 (2026-10-16, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-83.md`, this note.

## State

- PR 103 (`d331-story-zoom`, D331, base main) is ready and green (rust and python pass). The
  auto-mode check blocks `gh pr merge`, even with the allow rule in `.claude/settings.json`,
  and this time it also blocked `gh pr checks`. The owner was asked to type "merge" or merge
  it on GitHub.
- Draft PR 104 (`d336-story-backdrop`, D336, base `d331-story-zoom`), worktree
  `E:\Projects\wt-d324`. Docs done (part 83, index, STATUS-CURRENT). After 103 merges, run
  `git rebase --onto origin/main d331-story-zoom` on 104, retarget it to main, mark it
  ready, and merge it on green. D332 to D335 are used on main by the level-globals lane, so
  the next free number is D337.
- Capture script: this session's scratchpad `cap.ps1` (copy it from part 30's, which adds the
  `-Light` param, default `--preview-lighting`, and `-Out`, default
  `local-analysis\d332-story-backdrop`). Captures `bd4lbp-760.png` (`--lbp-lighting`, matches
  the game's dark look) and `bd4-760.png` (preview lighting: grey fog) are in that folder.

## Facts for the next step (copper continents)

- gmat 0x1a6f5 is `proc_victoria_jelly_glassy` (decoded with `local-analysis/shading/scripts/cgb.py`
  and `summary.py`, blob `79b44955`). It computes `out = fog(sqrt(4 light^2 d0 d1)) a +
  6 ta4 [(pow(R.L, CosinePower) + sat(N.V + rim)^2) light^2 + 0.3 refl luma(light^2)]`,
  where ta4 is the port-2 colour (1.446, 1.054, 1.274). That is the same family as
  `window_glassy` 0x1d14c (D286 glass route, `main.rs` `WINDOW_GLASSY`, applied to that GUID
  only).
- Textures: slot 0 0x1a6f2 (red jelly), slot 1 0x1a6f4 (normal map, read as `.yw`), slot 2
  0x0aa59 (a 2D rainbow environment map). Notes are in `local-analysis/d332-copper/`.
- Suggested fix: put 0x1a6f5 on the glass route (better: a program-family flag from
  lbp-preview) with opacity 1, and check the rim term (the jelly uses rim² in full). The
  reflection is still dropped, because the second diffuse sample takes graph slot 0.
  Visible only with `--lbp-lighting`.

## Next

1. Merge 103, then 104.
2. The copper continents on the glass route (D337).
3. The group's badge card and its info bubble.
