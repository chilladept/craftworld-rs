# Handoff: level globals, part 3 (2026-10-16, PC)

Read first (max 5):
- `docs/level-globals.md` section "Research for the next PRs"
- `local-analysis/level-globals/fog-tint-research.md` (EBOOT research on darkness / fog tint /
  fog; complete: FogColor is light-block offset 0x10, palette lerp at 8 x tint, fog
  near/far math; the viewer's darkness also scales exposure, which the game does not)
- `local-analysis/level-globals/final-pass-shader.md` last section (live-constant method)

## State

- D331 colour correction merged (PR 93). Game check done: live shader constants match.
- Branch `globals/fog-tint` cut from main in worktree `E:\Projects\wt-globals2` (target dir
  `E:\Projects\target-globals`); holds only this note, not pushed.
- RPCS3 closed; its config restored (fullscreen start back to true).

## Facts

- maingrapple (GUID 0001f527) is "Where In The World Is Avalon Centrifuge?": Story > The Cosmos
  > third main level (cursor: from Full Metal Rabbit press right). Starts at checkpoint Thing
  1750 (59064, 19916).
- RPCS3 driving: windowed start needs `Start games in fullscreen mode: false` in
  `config/config.yml` (restore after). Launch `local-analysis/d309-push/launch.py`; frames by
  F12 via `local-analysis/d309-push/step.py` (PrintWindow is black under Vulkan). Movement is
  the left stick: keys a/d/w/s; arrows are the D-pad; R1 = e. Pause > Return to Pod goes to
  the world map.
- Live fragment constants: scan guest memory for the cached `.fp` ucode, keep copies whose
  differing slots are only constant slots (words have their 16-bit halves swapped). Scratch
  script of part 3 not kept; rewrite from `final-pass-shader.md`.
- Viewer vs game scene mismatch at maingrapple's start (scrap shell over an orange planet in
  the game, a toy shop in the viewer): not investigated, not a grade issue.

## Next

1. Fog tint PR from the research note, then fog, then water; each its own small PR. The live
   constant method can check values in the game.
