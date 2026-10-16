# Level scripts, part 10: trigger events (2026-10-12, D259)

Stage 2 of `docs/rscript-plan.md`: a level's PTrigger parts call `Enter__Q5Thing`,
`Inside__Q5Thing` and `Leave__Q5Thing` on its scripts when a player walks in.

- `lbp-level` keeps PTrigger (`PartData::trigger`): type, the stored in-list, radius
  multiplier, all-layers flag, hysteresis multiplier, enabled flag, z offset.
- `LevelWorld::add_player` / `move_player`: a player's Sackboy as a Thing of the level
  (BODY, POS, YELLOWHEAD, CREATURE) with a position and the z span of his body.
  `PYellowHead.PlayerNumber` is his index (INFERRED s32).
- `LevelWorld::update_triggers`, one tick of every trigger. FOUND in the 01.33 executable
  through the Ghidra bridge (trigger pass 0xe3bc0, per trigger 0x1364f0, test 0x135fe0,
  add 0x135a18, remove 0x1346fc):
  - The player's position is taken into the trigger Thing's own space (inverse world
    matrix), so the Thing's scale sizes the trigger. The radius is the radius multiplier,
    times the hysteresis multiplier while the in-list is not empty.
  - Type 0: x^2 + y^2 < r^2. Type 1: |x| < r and |y| < r. Type 4: x^2 + y^2 + z^2 < r^2.
    Any other type holds nothing. A trigger that is not enabled holds nothing.
  - Without the all-layers flag a Sackboy counts only when the trigger's z plus its z
    offset lies within the z span of his body.
  - Newly inside: appended to the in-list, Enter. No longer inside: removed, Leave. Then
    Inside for every Thing in the list.
  - Ordinary triggers test the players' Sackboys only.
- `LevelScripts::trigger_events` calls the scripts; `tick_level` is `tick` followed by it.
  FOUND: Enter and Leave go to the script of the trigger's Thing, or to its parent's when
  that Thing has none; Inside goes to the parent's script. The Thing that entered is the
  argument (register +4).
- Bound (118 in the table): `Native.GetNumInThings__Q5Thing` (FOUND 0x648770: the in-list
  count, 0 for a null Thing or no PTrigger) and `PartList.HasPYellowHead__Q5Thing`
  (INFERRED from its name). `thing.YellowHead` now resolves to the YELLOWHEAD part.
- Viewer: with `--level-scripts` and `--play-sackboy` together, the play-mode Sackboy is
  the player; the Level scripts window shows the events raised and run. His z span is a
  PLACEHOLDER (lane z +-100).
- `level_scripts`: `triggers` (census), and `--visit-triggers`: after the `--play` ticks a
  player stands on every trigger's Thing in turn, two ticks on it and one far away.

Not modelled, read in the same code: seven engine object kinds whose triggers test other
Things than players; four object kinds that alone get Inside (here any parent script with
the function gets it; the three classes that define it are the collect power-ups);
object kinds the engine handles without a script (19,328 triggers have no script on their
Thing or its parent); Sackboy states that never count (creature state 5, a YELLOWHEAD
flag); the world-bounds cull. INFERRED: the player's point is his Thing position (the game
asks his creature part); triggers run in Thing order, after the tick's updates.

PC run, 378 levels read, d133 snapshot with the edat overlay. Unit: Thing with a PTrigger
part; trigger event; scripted Thing.

| PTrigger census | Things |
|---|---|
| All | 23,990 |
| Type 0 (circle) / type 1 (box) | 23,715 / 275 |
| All layers | 5,397 |
| Not enabled | 5 |
| Stored in-list not empty | 2 |
| No POS | 2,235 |
| Script on its own Thing / on the parent / none | 2,388 / 2,274 / 19,328 |

Script classes behind the 4,662 scripted triggers: Trigger_Speech_Bubble 1,402 (parent),
TriggerGlobalSettings 1,180, TriggerMusic 981, Score_Board 205 (parent),
TriggerCollectGrapple 163, TriggerLevelLink 113, TriggerCollectGun 109,
TriggerCollectHandle 102, TriggerCollectCape 80, TriggerWormHole 79,
TriggerCollectGauntlets 75, 10 more below 50 each.

| 60 ticks per level | No player | Then a visit to each of 21,755 triggers |
|---|---|---|
| Enter / Inside / Leave raised | 0 / 120 / 0 | 39,212 / 101,918 / 39,178 |
| Enter / Inside / Leave run by a script | 0 / 60 / 0 | 11,430 / 895 / 7,357 |
| Events with no script to call | 0 | 117,648 |
| Events for a Thing already stopped | 0 | 1,410 |
| Things stopped by a fault (of 13,547) | 1,722 | 1,722 |
| Native calls bound / unbound (distinct unbound) | 51,030 / 76,329 (65) | 215,292 / 778,473 (94) |

The events add no fault. They add 29 unbound natives; the most called (calls / levels):
`Thing.GetInput__Q5Thingi` 57,400 / 166, `BaseWorld.IsScoreBoard__Q5Thing` 46,386 / 108,
`Thing.GetOutput__Q5Thingii` 8,249 / 150, `Game.IsOnSequencer__Q5Thing` 7,333 / 130,
`Audio.StopSound__Q11AudioHandle` 5,226 / 88, `Game.Get/SetGlobalSettingUID` 2,549 / 96
each, `GameMessagingManager.Messaging_NewGameEvent__iiQ5Thingb` 1,845 / 93,
`Native.GetInThing__Q5Thingi` 1,764 / 89, `Audio.IsSafeToTriggerMusic__i` 1,446 / 133.
The unbound call total grows mostly because the visit adds about 65,000 level ticks of
the enemy scripts' per-tick natives.

The visit stands on a trigger's Thing with his z span around the trigger, so it measures
what the scripts do once an event fires, not where a real player would walk. A defaulted
native still makes a script take a path the game might not.

Verification: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test` on the non-viewer crates (new:
`stage2_trigger_holds_the_player_by_shape_hysteresis_and_layer`,
`stage2_trigger_events_call_the_parent_script`), `cargo check -p lbp-bevy-viewer`. Not run
in the viewer window.

Counts: +0 newly viewable, +0 newly Complete (no asset change).
