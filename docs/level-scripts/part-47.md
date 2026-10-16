# Level scripts, part 47: the tent's back faces culled (2026-10-14, D296)

The Pod dissolve's green line read about twice as thick as the game's (median 4 viewer px at
tick 614 with `--no-edge-glow`, against the game's ~1.8 px in `open-t1`; the rule predicts 1.9).

- FOUND (RenderDoc replay of frames 5247 and 14790): every Pod draw (`pod_card`,
  `pod_card_edge`, `window_glassy`, the platform's card edge) culls back faces, front faces
  clockwise.
- Cause (MEASURED): the viewer drew the tent double-sided. Above the cut the near face of each
  post is discarded, and through the gap the inside of the post's far wall shows its own 2 px
  line about 7 rows higher; counted per column, the two lines make 4 rows.
- Probes at tick 614 (rows within 12 levels of COL0 per post column, mean in game px): base
  3.56; green term zeroed 1.87 (only moves the far line out of view); green term encoded 3.57;
  MSAA off 3.75; back faces culled 1.99; front faces culled 2.22 (only the far line). Game 1.77.
- Viewer: `dual::set_dissolve` sets `cull_mode = Some(Face::Back)` while a dissolve is on and
  clears it after. Only the Pod's own materials carry a dissolve. The game culls these
  materials always; with the tent closed nothing is cut, so the dissolve is the only time it
  shows (the glass is left as it was).
- MEASURED side effect: the green excess above the line with the glow on falls from
  31.1 / 27.1 / 21.9 / 14.1 / 8.2 to 22.9 / 20.2 / 16.5 / 9.6 / 5.5 at 1 / 2 / 3 / 6 / 10 px,
  still above the game's; the glow refit follows the final pass (D294).
- Tests: `d278_the_dissolve_setter_writes_and_clears` checks the cull is set and cleared.
