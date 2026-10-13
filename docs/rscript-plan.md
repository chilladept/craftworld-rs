# RScript interpreter: plan to real levels

2026-10-12 America/Chicago. Goal (user, 2026-10-13 decision card): run the game's own
level scripts in the Rust + Bevy app so levels start to play. Scripts are interpreted in
our VM (user decision 2026-10-08), not replaced by native code. This plan follows
`docs/script-vm.md`, `docs/script-natives.md` and `docs/level-scripts.md` (parts 1-8).

## Where it stands

- The VM runs all 203 opcodes and passes 512 of 513 checks of the game's test script.
- `lbp-runtime` runs `OnCreate__`, then per tick resumes yielded scripts and threads and
  calls `Update__` (or a class's frame entry). The viewer does this at 30 Hz with
  `--level-scripts`.
- 99 natives are bound. An unbound native faults the script and its Thing stops.
- Nothing a script changes is drawn, and no engine event other than create and update
  reaches a script.

## Measured gap (2026-10-12)

Static census over the 49 script classes that the 378 readable levels use (13,547
scripted Things, D150), following script-to-script calls from the entry functions.
Unit: distinct native (declaring class, mangled name) without a binding. Names and
counts only; the listings stay in ignored `local-analysis/`.

| Reached from | Unbound natives |
|---|---|
| `OnCreate__`, `Update__`, `.init__`, `.ctor__` | 272 |
| every function of the classes in use | 583 |

By declaring class, create/update reach: Game 75, Thing 39, Native 16, Audio 15, Gooey
13, Score_Board 13, GameMessagingManager 11, Math 10, String and StringA 10, HUD 6.

Which classes matter (instances / levels; unbound natives reached from create/update):

| Class | Instances / levels | Unbound |
|---|---|---|
| PostItNote, Launcher, TriggerMusic, SmokeMachine and 14 more Tweakable classes | 7,900+ / up to 173 | 1 each: `Resource.LoadTextureByFilename__gb` |
| Trigger_Speech_Bubble | 1,469 / 161 | 60 |
| TriggerGlobalSettings | 1,180 / 135 | 18 |
| ScoreTweak | 1,106 / 48 | 22 |
| WorldMooCamera (the world script of 220 levels) | 220 / 220 | 37 |
| Score_Board | 102 / 101 | 80 |
| TriggerLevelLink | 113 / 103 | 30 |
| Pod | 1 / 1 | 31 |

The event entries the classes define, which nothing calls yet (unit: distinct class of
49): `Enter__Q5Thing` 18, `Render__` 18, `SwitchPerformReaction__b` 17, `Leave__Q5Thing`
11, `OnDestroy__` 8, `Inside__Q5Thing` 3. `UpdateUI__` (23) is the Popit tweak menu and
is out of scope here.

Reading: most scripted Things do nothing on create or update. They wait for a trigger
or a switch. So natives alone will not make a level play; the events are the larger
gap, and the natives a class needs can only be ranked honestly once its events fire.

## Stages

Each stage is one PR, with a PC run of `level_scripts --all` before and after.

1. **Run past unbound natives, and count them at run time.** An unbound native returns
   the default of its declared return type and is counted instead of stopping the Thing
   (a mode of `Bindings`, on for the viewer and `--play`, off in the VM tests).
   `level_scripts --all --play=N` then reports, over every level, calls made per
   unbound native and the faults that remain. Bind the pure ones in the same PR:
   String and StringA (length, find, substring, upper case, empty test, conversion),
   the 10 Math natives, and `LoadTextureByFilename__gb`.
   Result: a ranked list from real runs, and create/update finishing on most Things.
2. **Trigger events.** `Enter__`, `Leave__` and `Inside__` from the PTrigger part
   against the player Sackboy in the viewer's play mode (radius and layer rule from the
   executable, through the Ghidra bridge). This wakes speech bubbles, level links,
   collect triggers, music and global settings.
3. **Switch events.** `SwitchPerformReaction__b` and the switch-input natives from the
   level's switch and logic parts. Needs the logic network evaluated each tick; scope
   it from the PSwitch data first.
4. **Player and game state.** Players, the player's Thing (`GetYellowheadFromPlayerNumber`),
   score and multipliers, pause and mode queries: the Game group that ScoreTweak,
   WorldMooCamera and Score_Board need.
5. **Show what scripts do.** Speech bubbles and subtitles (GameMessagingManager) in the
   viewer UI with LAMS text, global settings (lighting, fog) applied to the scene,
   world transforms and tints applied to drawn Things, the script camera.
6. **Pod.** `Thing.FindThing`, the Pod world script and PodComputer threads, video and
   voice-over natives as recorded no-ops, up to the Pod intro sequence.

Audio natives stay recorded no-ops until there is an audio system. Online, Popit tweak
UI (`UpdateUI__`) and photo natives are not planned.

## Progress

- Stage 1 done (D258, `docs/level-scripts.md` part 9): over 378 levels at 60 ticks, Things
  stopped by a fault 4,940 -> 1,722 of 13,547; 76,269 calls to 64 unbound natives are
  defaulted and counted. Largest stops left: switch inputs 1,143, `PWorld.GameMode` 220,
  local transforms 199, TriggerLevelLink 113.
- Stage 2 done (D259, part 10): PTrigger kept by the reader; the inside test, hysteresis,
  layer rule and the Enter/Inside/Leave targets FOUND in the executable. 23,990 triggers
  in 378 levels, 4,662 with a script on their Thing or its parent. With a player visiting
  each trigger: 11,430 Enter, 895 Inside and 7,357 Leave calls run, no new fault, 29 more
  unbound natives reached (switch inputs and outputs lead). The viewer feeds the
  play-mode Sackboy to the triggers.

## Rules

- Each native binding states its evidence: declared types from the script tables, use in
  scripts, and the executable through the Ghidra bridge (2,662 script natives named).
  Decompiled code is evidence only and stays in ignored `local-analysis/`.
- Default-returning natives are never counted as bound. Reports keep three numbers:
  bound, defaulted, faulted.
- No script bytecode, listing, constant or game asset is committed.

Counts: +0 newly viewable, +0 newly Complete (plan only).
