# Level scripts, part 9: run past unbound natives, counted at run time (2026-10-12, D258)

Stage 1 of `docs/rscript-plan.md`.

- `Bindings::default_unbound` (constructor `Bindings::defaulting`): a native without a
  binding returns nothing, which the VM already turns into the default of the call's return
  machine type (false, 0, null, zero vector). The call is counted in `Bindings::unbound`
  and never as bound. Off by default, so the VM and runtime tests still see a missing
  binding as a fault. On in the viewer (`--level-scripts`, whose window now shows bound and
  defaulted calls) and in `level_scripts --play`; `--strict` turns it off there.
- `level_scripts ... --all NEW-JSON --play=TICKS [--strict]` plays every readable level on
  its own VM (no heap object or static outlives its level) and writes `play_all`: native
  calls bound and unbound, every unbound native with its calls and levels, and the faults
  that remain by kind. A bound native whose `World` lacks the feature still faults.
- 19 natives bound (116 in the table). Evidence for each: declared types from the script
  tables, and the rule read in the 01.33 executable through the Ghidra bridge (FOUND)
  unless marked.
  - `String`: `Length`, `IsNullOrEmpty`, `Find`, `SubStr__Q6Stringii`, `ToUpper`,
    `StringFromStringA`. A wide string: lengths and indices count UTF-16 units. Null has
    length 0, is "null or empty", is never found in (-1) and gives an empty substring.
    `SubStr(s, start, count)` is empty when `start` is at or past the end and stops at the
    end. Negative `start` or `count` is an error here (not observed in a run). `ToUpper`
    maps unit by unit (INFERRED to be the C library's wide upper-case).
  - `StringA`: `Length`, `IsNullOrEmpty`, `Find`, `SubStr`, on bytes. INFERRED to follow
    the `String` forms; their code was not read.
  - `Math`: `identity__`, `ceil__f`, `exp__f`, `pow__ff` (C library), `sign__f` (1 from
    zero up, -1 below, never 0), `lerp__fff` (t, a, b: a + (b - a) t), `Delt__fff` (from,
    to, step: move towards `to` by at most `step`, landing on it), `BiasCosine__f`
    (0.5 - 0.5 cos(pi x)).
  - `Resource.LoadTextureByFilename__gb`: the `__g` loader with a flag passed to the
    load. The flag's meaning is not settled; nothing is loaded either way.
  - `LAMS.MakeLamsKeyID__Q6String` now takes a null string as the empty string (FOUND).
- Not bound, of the plan's 10 Math natives: `DeterministicRandomBetween__ff` (FOUND: a
  30-bit linear congruential state kept by the world, top 14 bits scaled to the range; the
  state's start value is not known yet) and `ScreenToWorldSpace__pf` (needs the camera).

PC run, 378 levels read (20 not read), 60 ticks each, 13,547 scripted Things. Unit: native
call; scripted Thing (a Thing stops at its first fault).

| | Things stopped by a fault | Native calls bound | Unbound calls (distinct natives) |
|---|---|---|---|
| Before (main, unbound natives fault) | 4,940 | 26,186 | 4,169 (19) |
| After, `--strict` | 4,940 | 26,390 | 4,108 (18) |
| After (defaulted) | 1,722 | 51,030 | 76,269 (64) |

Before, the engine-owned objects also faulted in all 378 levels (the Gooey pool's
`.init__`); after, in none. Thread faults 1 before, 0 after. The static census of
`docs/rscript-plan.md` drops from 272 to 253 unbound natives reachable from create/update;
`LoadTextureByFilename__gb` was never called in these runs (it is reachable, not reached).

Most-called unbound natives after (calls / levels): `Native.IsEnemyAI__Q5Thing`,
`Native.SetWalkInput__Q5Thingp`, `Thing.IsDead__`, `Thing.IsInLimbo__` 11,529 / 26 each;
`Thing.GetBodyRoot__` 7,076 / 27; `Gooey.PerFrameUpdate__Q18Game.EPlayerNumberi` 6,222 /
101; `LAMS.IsInLams__i` 4,041 / 161; `HUD.PreloadGlyphs__Q11Fonts.EFontQ6String` 2,938 /
161; `EnemyWard.EnemyWardNotify__Q5Thing` 2,013 / 4; `Thing.UpdateScale__` 1,469 / 161;
`GameMessagingManager.Messaging_ShouldFilterMessageStrings__` 1,184 / 142;
`Thing.SetRenderMeshEditorColour__Q5Thingr` 1,180 / 135.

Faults that remain after (Things / levels):

| Fault | Things / levels |
|---|---|
| `Thing.IsSwitchTriggered__i`: no switch inputs (ScoreTweak 1,106, TweakSpinningObject 34, TriggerAirStone 3) | 1,143 / 56 at most |
| `PWorld.GameMode` member not provided (World script) | 220 / 219 |
| `Thing.SetLocalPosition__m`: no local transforms (TriggerCollectHandle 102, TriggerCollectGun 97) | 199 / 34 at most |
| Null object in `TriggerLevelLink.Init__` (after a defaulted native returned null) | 113 / 103 |
| Native member `Pos` of Race_Start (24) and Race_End (22) | 46 / 41 at most |
| Null Thing in `Pod.SetPodTransition__b` | 1 / 1 |

Rookie Test alone, 300 ticks: create 189 done, 22 yielded, 1 fault (the World script, on
`PWorld.GameMode`), 9 without the function; 221 instances, 1 stopped (part 6: create
189 / 23 / 9).

A defaulted native is a guess at "nothing there": a script that runs on may take a path the
game would not. The counts say which natives to bind next, not that those scripts are
right.

Verification: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings` clean, `cargo test` on the non-viewer crates passes (new tests
`stage1_strings_and_math`, `unbound_natives_fault_or_default`). Not run in the viewer.

Counts: +0 newly viewable, +0 newly Complete (no asset change).
