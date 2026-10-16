# Level scripts, part 12: the switch network and switch reactions (2026-10-13, D261)

Stage 3b of `docs/rscript-plan.md`. Part 11 let scripts read switch signals, but the
signals stayed as the level file stores them. Now the network is evaluated each tick and
the scripts are told when an output wired to them turns on or off.

### What the game does (FOUND, 01.33 executable)

- The world update (0xeacd4) runs one switch pass (0xdd228): the switches are sorted by
  their wiring, a source before its targets, and updated once each in that order
  (0x1244a4). A wire that goes backwards in the order reads last tick's value.
- Each output is computed by 0x11bd58 through a jump table on the switch type (0x11be00,
  a second table 0x11bfd4 for an inverted switch). A signal is {activation, ternary,
  player}; an unwired port reads {0.0, 0, -1}.
- Per output: `on` = new ternary not 0 and (ternary changed or forced). The targets are
  visited when on/off changed, the player changed, or `on`. So a target sees a rising
  edge as true, a falling edge as false, and a sign flip while on as true again.
- The visit (0x11ddf0) acts by the target Thing's object kind. Toggle, counter, selector
  and timer change their state here, on the input edge. For 16 script GUIDs it calls
  `SwitchPerformReaction__b(on)` on the target Thing's own script; for one GUID
  `SwitchPerformReaction__bf(on, x)` with x = 1.0 when the output's ternary is not 0.
- The object kind (0x3ed2c) looks at other parts first and at the switch type before the
  script: a scripted Thing that also carries one of those parts or a switch of most types
  gets no reaction call.
- A microchip is off when its port 0 is wired and reads 0, or when the board it sits on
  is off (0x409dc). Everything on an off board outputs {0, 0, -1}; latches, counts and
  timers keep their state.

### What the runtime does

`crates/lbp-runtime/src/switches.rs`. `LevelScripts::tick_level` runs the script tick, the
trigger events, then `LevelWorld::update_switches`.

| Type | Name (INFERRED from the string order) | Output |
|---|---|---|
| 36 | microchip | output i = the signal on the board Thing's port i |
| 37 | circuit board | output i = the signal on the owner's port i + k |
| 23, 24, 40 | NOT, pass, node | input 0; NOT inverts through the inverted flag |
| 14 | AND | mode 0: smallest magnitude with the sign product; mode 1: product |
| 21 | OR | mode 0: largest magnitude; mode 1: sum, ternary clamped |
| 22 | XOR | the one active input; two or more give 0 |
| 28 | battery | the stored manual value |
| 20 | direction | magnitude of input 1 minus magnitude of input 0 |
| 27 | sign split | output 0 the negative part, output 1 the positive part |
| 17 | toggle | a latch, flipped by a rising edge on any port |
| 35 | selector | output i on when the index is i; port 0 steps, port i + 1 selects |
| 15 | counter | count / required; port 0 adds, port 1 resets |
| 16 | timer | count / length; advanced each tick; port 1 resets |

Every sensor type, the randomiser (19) and a board owned by a sequencer keep the stored
signal. Types without a handler in the game give {0, 0, -1}.

INFERRED: 30 ticks per second and timer lengths in ticks; that the first pass compares
against the stored signals; that the off flags are computed once before the first pass;
the order in which a wiring cycle is broken (lowest Thing index).

### Measured (PC, 378 levels, release build)

Census: 141,572 PSwitch Things. The AND / OR mode word holds 0 or 1 only (AND 5,671 / 56,
OR 4,084 / 5). 5,686 NOT gates are inverted, 108 are not. 28,278 microchips, each with a
board; 5,420 have port 0 wired. 4,298 of 4,599 battery outputs are stored off while the
battery is on: the files hold mostly unpowered outputs, and the first pass powers them.

| Run | Target visits | `__b` calls | `__bf` calls | Outputs unlike the file at the end | Things stopped by a fault |
|---|---|---|---|---|---|
| plain, 60 ticks | 75,140 | 204 | 36 | 24,612 | 597 (was 582) |
| `--visit-triggers`, 60 ticks | 467,601 | 222 | 37 | 24,665 | 598 (was 582) |
| `--visit-triggers --pulse-switches` | 770,915 | 3,052 | 1,345 | 9,486 | 1,794 |
| plain, 300 ticks | 265,180 | 226 | 37 | 24,469 | 598 |

Unit: one call of the script entry on one Thing; 144,608 outputs in all. In the plain run
18,100 Things end off under a microchip. `--pulse-switches` turns every output on for one
tick and off again, to reach the reactions the unvisited sensors would cause.

New faults the reactions reach: `Thing.Pos` in `Trigger_Speech_Bubble` (14 Things in the
plain run, 287 pulsed) and `PWorld.Frame` in `ScoreTweak` (1; 925 pulsed). 19 more unbound
natives are called in the plain run: get and set of the global setting (121 calls in 64
levels each), four Gooey frame natives, `Audio.IsSafeToTriggerMusic`, six speech-bubble
natives, two trigger messages, `Game.GotoScoreboard`, `Game.IsVcrPaused`,
`Thing.GetMicroChipProxyFromThing`, `Thing.GetWorldPositionForSensor`.

### Not modelled

Sensors (tag, player, impact, grab and the rest), the randomiser, the game's dirty flags
(every switch runs each tick here), sequencer boards and cursor chips, the three extra
conditions of `__bf`, object kinds 0x49, 0x53 and 0x54, the mesh, trigger-parent and
gameplay-data kind tests, the activating-player record, the force flag.

### Verification

`cargo test -p lbp-runtime -p lbp-level`: 21 unit tests in `switches.rs` and two world
tests (`stage3b_switch_reactions_go_to_listed_scripts_only`, the AND -> NOT -> script
chain); fmt, clippy and the CI test set pass. Not run in the viewer window; no switch
compared against the game in RPCS3.

Counts: +0 newly viewable, +0 newly Complete (no asset change).
