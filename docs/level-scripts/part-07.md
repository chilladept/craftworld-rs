# Level scripts, part 7: level scripts in the viewer (2026-10-09)

`lbp-bevy-viewer --level-guid HEX|--level-path PATH --level-scripts
[--level-scripts-flags MASK,..]` (module `level_play.rs`; bit masks since part 22) runs the level's scripts inside the
viewer, at the game tick: `Time<Fixed>` at `lbp_runtime::TICK_HZ` (30), one
`LevelScripts` step per `FixedUpdate`. The first step is `OnCreate__`; every later one
is a tick (resume, then `Update__` or the class's frame entry). The setup is the same as
`level_scripts --play`: load every script with its GUID, read the level,
`bind_objects`, `add_class_defaults`, the progression flags, `add_engine_objects`. A
"Level scripts" window shows the tick, create and tick counts, ticks with faults, the
first fault lines, the most-called unbound natives, the last camera set through
`Cam.SetCameraFromToUp` and the last `WRITE` lines. `--qa-hide-ui` hides it.

Nothing the scripts change is drawn yet (camera, positions, tints): this part is the
clock and the counts. Built and linted in the cloud (`cargo +1.99.0 clippy -p
lbp-bevy-viewer --all-targets -- -D warnings`, viewer tests pass); not run, as the cloud
has no game data or display.

PC check: the viewer on Pod planets (00018fdb) with `--level-scripts
--level-scripts-flags 2` on the second monitor. After a few seconds the window should
show ticks rising at 30 per second, 0 faults and the camera from (0, -250, 1130) to
(0, 100, -1500), as `level_scripts --play` reported in part 6. Then Rookie Test, where
the counts should match part 6 (create 189 / 23 / 9).

Counts: +0 newly viewable, +0 newly Complete (no asset change).
