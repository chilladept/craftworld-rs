# Level scripts, part 24: the pod computer is entered (2026-10-13, D273)

Stage 6 of `docs/rscript-plan.md` (the Pod), fourth PR. The Pod's world script
(`gamedata/levels/pod/pod.bin`) lets the party leader engage the pod computer when the
intro has set its progression flags, he stands in the pad zone and he presses a button.
It then asks the host for the pod computer, and while the computer is in use it runs its
change-over value from 0 to 1, swaps the pod's mesh and the planets level
(`gamedata/lbp2/pod/levels/pod_planets.bin`) moves its camera to another preset. Five
natives of that path were not bound, the request was only recorded, and the two levels
could not be played together. Six natives are new (249 -> 255).

### What the game does (FOUND, 01.33 executable; read for this PR)

One game object holds "our Pod" (a byte), "pod computer in use" (a byte) and the player
records. A player record points at its pad object and at an object with the player's
colours. None of this belongs to a level.

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Native.GetIsInPadZone__Q5Thing` | `bool (Thing)` | 0x64a52c -> 0x6d426c -> 0x13bac0 | False unless the level is our Pod and the Thing's PYellowHead has the primary player's number. Then it walks a list of the PWorld: an entry counts when its Thing's script instance passes a class test (0x620504) and a call by name without arguments (0x6205b4), and the Thing asked about is in the entry's list of Things inside (0x132cb4). The first such entry decides: false when the Thing has no PPos, else "world position z >= -300.0". No entry: false |
| `Input.IsJustClickedByID__iiQ6String` | static `bool (s32, s32, String)` | 0x629e18 -> 0x6d2f18 -> 0x3890a0 | False without a player record of that number. The second argument is the number of an action (an enumeration of 99); it is passed on unchanged when it is in a list filled at run time, else 0 is. With `ctx` the action's context mask (0x38677c): `(ctx & mask_a) != 0 && (ctx & mask_b) != 0 && down now && not down the frame before` on the record's pad. When `(ctx & mask_a) != 0` and the string is not null, the on-screen prompt of the action is asked for with the string as its key (0x388adc); that never changes the answer |
| `Native.GetPlayerColour__Q5Thing` | `v4 (Thing)` | 0x64a4dc -> 0x6d88b0 -> 0x139308 | The zero vector for a null Thing or one without a PYellowHead. Else a 32-bit colour, bytes A, R, G, B from the top, as (R, G, B, A) / 255. The colour is the first of four in an object of the record with the head's player number; without that record, the first record's; with none, a word that was not read |
| `PodComputer.SetPodTransitionTexture__g` | static `void (guid)` | 0x617428 -> 0x4015e8 | Once the texture with that GUID is loaded, sets it on three engine resources (GUIDs 12138, 12250, 119116). Stores nothing a script can read |
| `Game.CurrentLevelIsOurPod__` | static `bool ()` | 0x5f8b38 -> 0x5e8024 | The game object's byte. One function writes it, when a level becomes the current one (0x1ab508): true when the slot being entered equals a slot id constant, type and number |
| `Video.PressedStartButton__` | static `bool ()` | 0x5f61a4 -> 0x756520 | Over seven low-level pad records in order: the first with bit 0x80000 in one of its words has the press used up (0x687c6c: the mask leaves three words of the record and is ORed into a fourth) and the answer is true. None: false |
| `PodComputer.RequestHostUsePod__b` | static `void (bool)` | 0x5fbd04 -> 0x3fa9cc | Only when the level is our Pod and the Pod belongs to the local user (0x3fa974): stores the request (use or stop) with the time and sends a one-byte message to the host. It does not change "in use" |

- Action 8, the one the Pod's script passes: context mask 0x1001 (the only action with
  that mask); down when bit 0x80 of the pad block's button word is set. The same case
  reads other words of the block when one of two bytes of it is not 0.
- "In use" is written by one function (0x18ed30: value, with or without sound). With
  sound, and when the primary player's avatar exists, a change to true plays
  `pod/pod_comps_startup` and a change to false `pod/pod_comps_shutdown`. The receiver of
  the host's message (0x3fa868) calls it with the message's value and with sound, then
  clears the request.
- The same function is called without sound when the Pod is entered and a global byte and
  the progression mask 2 are set (the computer is open on arrival), and with false when
  the Pod is left (0x1ab508, 0x1a7580). Not modelled.
- FOUND in the script listing, and unlike the scoping note of part 21: the Pod's
  `EnterPuterTrigger` asks for the start-up sound when the leader walks into the trigger
  while the pod computer is in use, not while it is free.

INFERRED:

- The PWorld list of the pad zone is the level's PTriggers and the class is
  `Trigger_Pod`, whose function `IsPodComputerTrigger__` returns true. The two names were
  not read from the executable.
- The primary player is the one local player (the game picks one entry of the session's
  player list, 0x240a88).
- Bit 0x1 of a context mask is gameplay (the mode 1 the level scripts pass to the `Pad`
  natives) and bit 0x1000 the Pod. That 8 is in the wrapper's action list.
- The object with the colours is the player's profile.
- The slot id constant is the Pod's slot.
- The low-level pad word of `PressedStartButton` has the bit numbering of the pad's
  button word; the fourth word of the use-up call means "ignore until released".
- One local player is the host and owns the Pod he starts, and the host's answer is the
  request itself.

Not established: the contents of the action list and the tests of the other 98 actions;
which button bit 0x80 is; the prompt's icon; who writes the players' colours and what a
new profile holds; the value of the slot id constant; how many ticks pass between the
request and the answer; what the three resources of the change-over texture are.

### What the runtime does

`lbp-script-vm/src/natives.rs`, `lbp-runtime/src/pod.rs`.

- `LevelWorld::our_pod`: how the level was started. The caller sets it; false otherwise.
  `Game.CurrentLevelIsOurPod__` is a state query on it.
- `LevelWorld::in_pad_zone`: our Pod, the Thing is the primary player's Sackboy (a
  Sackboy `add_player` made; the primary player is the first player record), a PTrigger
  whose Thing's script class is or derives from `Trigger_Pod` has him in its list, and
  his world z is at least -300 (`PAD_ZONE_MIN_Z`). The call by name is not made.
- `PadState::is_just_clicked(action)`: the rule for action 8 with the constants above;
  no answer for any other action. The world answers false for those and counts them by
  action (`PodState::actions_not_read`): no mapping is made up. A prompt the game would
  ask for is counted by its key (`PodState::action_prompts`); nothing draws it. A player
  without a pad is false. `set_pad_buttons` gives a pad both mode masks 1, so the context
  test holds by bit 0x1.
- `PlayerRecord::colour` (0xAARRGGBB) and `colour_vector`. The start value is
  `PLACEHOLDER_PLAYER_COLOUR`, opaque white: a placeholder, not a game value.
- `SetPodTransitionTexture` is recorded only: the last GUID and the calls
  (`PodState::transition_texture`).
- `PressedStartButton`: the first of the first seven player records whose pad has bit
  0x80000 down; `PadState::use_up` takes the bit out of the frame's word and keeps it
  out until the button is released (`held_off`).
- `LevelWorld::request_host_use_pod`: in our Pod the request is kept and waits; elsewhere
  it is dropped and counted. `LevelWorld::begin_session_tick`, the first step of
  `LevelScripts::tick_level`, takes the answer: "in use" becomes the requested value, and
  when it changed and the world has the primary player's Sackboy the start-up or
  shut-down sound is counted in `sounds_played`. So `GetPodComputerInUse` follows the
  request one tick later. `RequestHostUsePod` left the recorded-only list.
- One session for several levels. `LevelWorld::hand_session_to` copies what the game
  keeps outside a level to another world: the progression flags, "our Pod", "in use" and
  a waiting request, the video and voice-over requests, and per player the colour and
  the buttons used up. `tick_session` runs one round: every world takes the answer, then
  the levels tick in the order given, and after each one its session state goes to the
  others. Each level keeps its own VM and its own level world.

`level_scripts`: `--our-pod`; without it a level is started as our Pod when it has a pod
Thing. That is the tool's stand-in: the game's rule is about the slot the level was
entered from, not about its content. `--enter-pad-zone=TICK` puts the Sackboy on the pod
computer's trigger from that tick on, `--press=TICK:MASK,..` gives player 0's pad a
button word on single ticks, `--player-colour=AARRGGBB` sets the stored colour,
`--with-level=PATH` plays a second level in the same session and reports the path of
events tick by tick (`runs.session`). `play_all.pod` has eleven new counts.

### Measured (PC, release build)

`level_scripts --all --play=60`, 378 levels, 13,547 scripted Things, plain / with
`--visit-triggers` / with `--visit-triggers --pulse-switches`, inputs as in part 11.
Outputs under ignored `local-analysis/`: `d273-play-all-{novisit,visit,visit-pulse}.json`
(D272: `d272-play-all-*`).

| 60 ticks per level | D272 | D273 |
|---|---|---|
| Things stopped by a fault | 3 / 3 / 3 | 3 / 3 / 3 |
| Fault kinds, levels with a fault | 1, 1 | 1, 1 |
| Natives in the table | 249 | 255 |
| Unbound natives called | 41 / 73 / 73 | 40 / 71 / 71 |
| Unbound calls | 61,480 / 862,986 / 868,663 | 61,419 / 862,919 / 868,592 |
| Native calls | 573,134 / 5,365,247 / 5,424,893 | the same |
| Recorded-only calls | 12,328 / 91,027 / 96,943 | 12,389 / 91,091 / 97,009 |
| Levels started as our Pod | - | 1 / 1 / 1 |
| `SetPodTransitionTexture` calls | 61 / 64 / 66 unbound | the same, recorded |
| `GetPlayerColour` calls on a Sackboy | 0 / 3 / 5 unbound | the same, bound |
| Things with a shape colour unlike the level file | 103 / 103 / 103 | 103 / 102 / 102 |

- One level differs between the runs: pod.bin. Its unbound calls go 61 / 67 / 71 -> 0.
  Everything else in the `play_all` block is equal but for the eleven new counts.
- The shape colour that is no longer unlike the file is the pod's, in the runs with a
  Sackboy. The script sets the avatar's colour on it; the placeholder colour is the white
  the file stores. The count follows the placeholder, not the game.
- No run of the 378 calls `GetIsInPadZone`, `IsJustClickedByID`, `CurrentLevelIsOurPod`
  or `PressedStartButton`: the engage test needs the progression mask 2, which a fresh
  profile does not have.

The two Pod levels in one session, 700 rounds, pod.bin first
(`d273-pod-session-700.json`): `level_scripts <snapshot> <game dirs>
gamedata/levels/pod/pod.bin <new json> --play=700
--with-level=gamedata/lbp2/pod/levels/pod_planets.bin --enter-pad-zone=460
--press=470:0x80 --player-colour=ff3366cc`. A fresh profile (flags 0).

| Round | Level | What changed |
|---|---|---|
| 452 | planets | The intro thread ends: the progression word is 3. The camera rests at eye (0, -250, 1130) |
| 460 | pod | The Sackboy stands on the pod computer's trigger and is in the pad zone |
| 461 to 470 | pod | `GetIsInPadZone` true and `IsJustClickedByID(0, 8, "BP_ENGAGE_POD")` asked each tick (10 calls, 10 prompts) |
| 470 | pod | Bit 0x80 is down: the click is true once, `RequestHostUsePod(true)` |
| 471 | both | "In use" is true. `pod/pod_comps_startup` counted in pod.bin. The change-over value is 0.05. The planets camera leaves its rest |
| 471 to 490 | pod | The change-over value goes up 0.05 a tick |
| 490 | pod | The value is 1.0 and the pod Thing's mesh is GUID 145753 |
| 496 | planets | The camera rests at eye (0, -300, 900), direction (0, 0.316, -0.949), and stays there to round 700 |

- On the way the planets camera has the same eye on two rounds three times (482, 490,
  494: the eased value turns around) before it rests.
- After round 700: no Thing stopped by a fault and no unbound native call in either
  level; the pod's shape colour is (0.2, 0.4, 0.8, 0), the stored colour's R, G, B with
  the script's own w of 1 - 1; the change-over texture is GUID 132243; one video and one
  voice-over request, in both worlds' lists.
- The same run with the planets level first (`d273-pod-session-700-planets-first.json`):
  the same events on the same rounds.
- A press that is held over the round on which the flags arrive is not a click
  (`d273-pod-session-700-early-press.json`: the button down on rounds 452 and 453, 248
  click tests, none true, no request).

### Left out

- The viewer. `--level-scripts` still plays one level: a shared session there means two
  script worlds and two VMs in the viewer's play state, which is a change of its own. So
  nothing draws the change-over yet (the mesh swap to `pod_platform.mol`, the fade, the
  texture, the shell layer's dolly) and nothing feeds the viewer's Sackboy and pad to
  these natives.
- The pod computer itself: `PodComputer.OnEnterPod__`, `PerFrameUpdate__b` and its
  menus are not called. "In use" is the engine's byte only.
- The pod computer open on arrival, and closed on leaving the Pod.
- The ownership test of the request (a Pod visited in someone else's game), the network
  message, the time of the request.
- The other 98 actions of `IsJustClickedByID`, and the other input devices of action 8.
- One VM for both levels: script statics and the PodComputer script object are per
  level here; the game has one of each.
- The order of the two levels in the game's frame.

### Verification

`cargo test -p lbp-script-vm`: new `d273_pod_computer_natives` (the colour's byte order,
the placeholder, action 8 by button, edge and both masks, no answer for another action,
a press used up until released, the natives' argument checks, a null Thing, a world
without the state, Start over the first seven pads in order). `cargo test -p
lbp-runtime`: 61 unit and 49 world tests. New world tests:
`d273_the_pad_zone_the_engage_press_and_the_players_colour` (each condition of the pad
zone alone, z at and below -300, no position, a second player, out of the trigger; the
click with and without a pad, held, released, another button, another action counted,
the prompts; Start; the colour by record; the texture kept),
`d273_the_host_answers_a_pod_computer_request_on_the_next_tick` (dropped outside our Pod;
in use from the next tick, the two sounds, an answer that changes nothing, no sound
without the avatar) and `d273_two_levels_share_the_session_state` (flags, media, the
request and its answer in both worlds, the sound where the avatar is, a used-up Start).
None needs game data. fmt, the CI clippy line, the CI test set and the Python tests pass;
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` passes and the viewer's
tests pass but the one that fails on main (`sackboy_play::tests::input_script_steps`);
the viewer's sources did not change. Nothing was run in the viewer and nothing was
compared against the game.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).
