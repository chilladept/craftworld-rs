# Level scripts, part 16: the message list (2026-10-13, D265)

Stage 5 of `docs/rscript-plan.md`, first PR. Part 14 kept every speech-bubble, prompt and
subtitle request with its arguments and did nothing with it. Now the level world runs the
game's message list by those calls: what is open, what a new request does to it, what
closes it. Nothing is drawn yet; the viewer reads the list in the next PR.

### What the game does (FOUND, 01.33 executable)

- The trigger messages and the two prompts go to message manager 4 of 5. A manager holds
  at most 8 messages (add 0x3e8dc0). A message keeps its type, up to five strings, its
  Thing, two counters that start at 0 and some flag bytes.
- Types, by the constant each native's core passes: bubble with a prompt 1 (0x7382e0),
  without 2 (0x7380f8), subtitles 3 (0x737320), level-link prompt 0x2b, wormhole prompt
  0x59, caption 0x5d (0x737f64) and 0x5e (0x737d7c), think 0x5f (0x737be8) and 0x60
  (0x737a00).
- A core first asks the manager for a message of its type about its Thing that is not
  closing (0x3e4124). When there is one, the call does nothing.
- With 8 messages held, the eighth is cleared to make room, unless its type's priority
  number is below the new type's. All nine types have priority 10 (type records filled
  by 0x3e3248).
- Bit 0 of a type's flags means one at a time: the new message is written over the first
  message of that type and keeps its id. Types 2, 3, 0x5e and 0x60 have the bit; the
  bubbles with a prompt and the two prompts do not, so those queue.
- Events (0x3e8aa0): event 3 reaches the messages about the Thing passed with it; events
  1, 2, 5, 6, 7 and 8 reach only the first message of the list, and only after one of its
  counters has passed a threshold; any other event reaches every message. A message then
  runs the action its type lists for the event (up to four pairs per type). All nine
  types list action 0x10 for event 3; the subtitles also for event 9. Types 1, 0x5d,
  0x5f, 0x2b and 0x59 list action 0x17 for event 2, types 2, 0x5e and 0x60 action 0x37.
- Action 0x10 (in the switch 0x3e528c): for a bubble or subtitle type it calls a function
  with a part of the message's Thing, then sets the message's closing byte. A closing
  message no longer blocks a new one of its type about the same Thing.
- `Messaging_NewGameEvent(manager, event, thing, flag)` (0x7353ec): an index above 4
  means manager 4. A speech bubble closes its own message with (4, 3, self, false).
- None of the nine types has a timed action. Nothing closes them by time.
- The request of a bubble with a prompt (0x7382e0): text, the actor name, a prompt
  string looked up by a LAMS key unless prevent close is set, the Thing, both player
  fields -1, "triggered once" in the byte the list sort reads, the bubble position, and
  prevent close in the byte actions 0x17 and 0x37 test before they do anything.
- Actor name (0x737910): an empty name is no name. When the level slot's type is 0 or 8
  the name is hashed as a translation tag and looked up in the loaded tables; a tag in no
  table gives a default string. For another slot type the name is used as passed.
- After each change the list is sorted and every message gets a state: one value with
  the "triggered once" byte set, one for the first other message when it heads the list,
  one for the rest (0x3e8928).

### What the runtime does

`crates/lbp-runtime/src/messages.rs`; `LevelWorld::messages` is a `Messages`.

| Item | Rule in `lbp-runtime` | Label |
|---|---|---|
| Types | `MessageKind` (Bubble, BubbleNoPrompt, Subtitles, LevelLinkPrompt, WormHolePrompt) and `BubbleStyle` (Speech, Caption, Think); `message_type` gives the game's number | FOUND |
| Duplicate | a request of a type about a Thing that has an open message of the type is dropped | FOUND |
| One at a time | a bubble without a prompt (per style) or a subtitle is written over the open one of its type and keeps its id | FOUND |
| Full list | at most 8 (`MAX_MESSAGES`); a ninth clears the eighth first | FOUND rule; which message is the eighth INFERRED (below) |
| List order | the order the messages were added | INFERRED: the game sorts with a comparison that was not read |
| Close | event 3 for manager 4 removes the messages about the Thing; event 9 removes the subtitles | FOUND that action 0x10 marks them closing; INFERRED that they leave at once |
| Other events | counted, nothing happens (`events_without_effect`); events for managers 0 to 3 are counted apart | FOUND that no type here lists another event except 2; event 2 not modelled |
| Time limit | none | FOUND |
| Age | `Message::age(frame)`: `PWorld.Frame` now minus the frame at opening; a message written over starts again | INFERRED (the game counts in the message; the restart is FOUND only for type 1's request) |
| Text | the first argument as passed: the script has already translated it | FOUND (script use) |
| Actor | `LevelWorld::actor_name`: the game's rule; `Message::actor_tag` keeps the argument | FOUND; INFERRED that the default string is empty, and the slot type 0 (part 13) |
| Anchor | `LevelWorld::message_anchor`: the Thing's microchip when it sits on one, else the Thing, when that has a PPos; `message_position` gives its place now | INFERRED: where the HUD draws a bubble was not read; the bubble's script measures its off-screen distance at this Thing |
| Prompt | `Message::has_close_prompt`: a bubble with a prompt unless prevent close; `CLOSE_PROMPT_KEY` is the string's LAMS key | FOUND for type 1 |
| Player | none: a trigger message has no player | FOUND (both fields -1) |
| Bubble position | kept as passed (0 to 8) | meaning not established |

Read API for the viewer: `LevelWorld::messages.open` (the open `Message`s in list order),
`Messages::of_thing`, the `Message` fields `id`, `kind`, `style`, `text`, `actor`,
`actor_tag`, `thing`, `anchor`, `bubble_pos`, `triggered_once`, `prevent_close`,
`has_voice`, `frame`, the methods `Message::age`, `message_type` and `has_close_prompt`,
and `LevelWorld::message_position`. `Messages` also counts requests, messages opened,
written over and dropped per type (`by_type`, `totals`), messages closed and pushed out,
and keeps a bounded log of what entered the list when `log_limit` is set (reports only).

Natives: the four caption and think natives are bound (212 -> 216 in the table). The ten
`GameMessagingManager` natives that open or close a message are listed in
`natives::MESSAGING` and are no longer in `RECORDED_ONLY` (29 -> 23): the world now keeps
state by them. They still reach the world through `World::engine_call`, and
`LevelWorld::requests` still keeps each call with its arguments (14 kept names).

`level_scripts`: each played level and `play_all` have a `messages` block.

### Measured (PC, 378 levels, 13,547 scripted Things, release build, 60 ticks)

Units: request (one call of a messaging native); message (one entry of the list).

| Run | Requests | Opened | Written over | Dropped as duplicate | Closed | Open at the end | Levels with a message |
|---|---|---|---|---|---|---|---|
| plain | 92 | 57 | 35 | 0 | 0 | 57 | 47 |
| `--visit-triggers` | 1,690 | 1,036 | 621 | 33 | 1,000 | 36 | 192 |
| with `--pulse-switches` | 2,381 | 1,187 | 1,161 | 33 | 1,152 | 34 | 216 |

By type, requests / opened / written over / duplicate:

| Type | plain | visit | pulsed |
|---|---|---|---|
| Bubble with a prompt | 8 / 8 / 0 / 0 | 156 / 156 / 0 / 0 | 214 / 214 / 0 / 0 |
| Bubble without a prompt | 57 / 23 / 34 / 0 | 457 / 221 / 227 / 9 | 1,058 / 306 / 743 / 9 |
| Caption without a prompt | 4 / 3 / 1 / 0 | 30 / 29 / 1 / 0 | 59 / 34 / 25 / 0 |
| Subtitles | 0 | 719 / 326 / 393 / 0 | 719 / 326 / 393 / 0 |
| Level-link prompt | 23 / 23 / 0 / 0 | 215 / 191 / 0 / 24 | 218 / 194 / 0 / 24 |
| Wormhole prompt | 0 | 113 / 113 / 0 / 0 | 113 / 113 / 0 / 0 |

- The caption with a prompt and the two think forms are not called in any run.
- Text of the bubbles and subtitles that entered the list (opened or written over): a
  string of a loaded translation table 69 / 1,289 / 1,974; another string 0 / 1 / 1;
  empty 0 / 63 / 66. With the two English tables loaded no text is left untranslated.
- Actor names: 16 / 69 / 218 messages came with a name, and every one is a tag of the
  loaded tables. Every message that entered the list has an anchor with a position.
- Close events for manager 4: 0 / 1,740 / 2,318, all (4, 3). 740 and 1,166 of them
  closed nothing: the Thing's message had been written over by another Thing's, or was
  never opened. No event for another manager.
- The list was full once, in the pulsed run (one message cleared from the eighth place).
- Things stopped by a fault: 4 -> 4 in all three runs; fault kinds 2, levels with a
  fault 2, as before. Native calls, trigger events and reaction calls are the same as in
  part 15.
- Unbound natives called 52 -> 51, 88 -> 87, 88 -> 87 (the caption native); unbound
  calls 73,329 -> 73,325, 1,002,475 -> 1,002,445, 1,010,588 -> 1,010,529. Recorded-only
  calls 13,648 -> 13,560, 95,656 -> 92,256, 102,810 -> 98,170: the messaging natives
  left that list.

### Not modelled

- Event 2, the player's button: action 0x17 on a bubble with a prompt calls a function
  with a part of the Thing and sets the "triggered once" byte, unless prevent close is
  set; action 0x37 on a bubble without a prompt was not read. Whether the bubble's
  script is told was not read. So a prompt bubble here stays until its script closes it.
- How long a closing message stays in the list, and the function action 0x10 calls on
  the Thing.
- The sort order and the three states, so which of several queued bubbles is on screen.
- Messages about another Thing that the game closes when a one-at-a-time message is
  added (0x3e4af0, by a Thing flag that was not identified).
- What a subtitle's text goes through before it is kept (0x736f1c), and the flag set
  from its last argument.
- The text of the level-link and wormhole prompts (the HUD script builds it), bubble
  placement and bubble positions 1 to 8, art, fonts.
- The bubble script's own off-screen close: the three screen natives are still unbound.
- The other four managers and every message type but these nine.

### Verification

`cargo test -p lbp-script-vm -p lbp-runtime`: 12 + 13 VM tests, 40 unit and 35 world
tests. New: 6 unit tests in `messages.rs` (type numbers, duplicate per kind, queue and
written over, the ninth message, close events per manager and Thing, the log) and the
world test `d265_the_messaging_natives_open_and_close_the_worlds_messages` (every
messaging native on a synthetic level with made-up text: fields, anchor on a chip, actor
name through a synthetic table and in another slot type, caption and think types,
subtitles, prompts, close). fmt, the CI clippy line, the CI test set, the Python tests
and `cargo check` of both viewers pass. Not run in the viewer window; nothing compared
against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).
