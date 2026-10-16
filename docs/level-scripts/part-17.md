# Level scripts, part 17: the viewer draws the messages (2026-10-13, D266)

Stage 5 of `docs/rscript-plan.md`, second PR. Part 16 gave the level world the game's
message list. The Bevy viewer now draws it while a level plays under `--level-scripts`: a
box per open speech bubble at its Thing, and the subtitle at the bottom of the screen.
Nothing new was read in the executable; the look and the placement are INFERRED.

### What the viewer does

- Tables. `level_play.rs` loads the English base and patch translation tables for its
  level world, patch first, with the loader the `level_scripts` example uses since part 14
  (`lbp_preview::story::load_strings`). `LAMS.IsInLams` and `LAMS.Translate` now answer in
  the viewer as in the measuring runs, so a bubble's text is the translated string. A
  table that cannot be read is left out: the viewer still runs, a script then passes its
  own text for a missing key (the "Missing key" string of the bubble script, part 16),
  and a bubble has no close prompt line. The header line prints how many tables loaded.
- What is drawn. `LevelWorld::message_overlay` (`lbp-runtime/src/message_view.rs`) picks
  from `messages.open`, in list order:

| Message | Drawn | Label |
|---|---|---|
| Bubble with or without a prompt, any style, with text and an anchor that has a position | a box above the anchor: the speaker's name when `Message::actor` has one, the text, and the close prompt's string when `has_close_prompt` | INFERRED: every open bubble is drawn (the game's sort gives queued bubbles different states, not read) |
| Caption and think styles | as a speech bubble | INFERRED |
| Bubble whose text is empty | not drawn, counted | INFERRED |
| Bubble whose anchor has no position | not drawn, counted | no place to draw it |
| Subtitles | one box at the bottom centre; of several with text, the last of the list | INFERRED placement; the list holds one at a time (part 16) |
| Level-link and wormhole prompt | not drawn, counted | the HUD script builds their text (not read) |

- Close prompt. `LevelWorld::close_prompt` looks `CLOSE_PROMPT_KEY` up in the loaded
  tables. It is shown as a small line in the bubble. No key is wired to it: the player's
  close button (event 2) is still not modelled, so a prompt bubble stays until its script
  closes it.
- Text. `display_text` takes the game's markup out of a string, because the viewer has
  neither the game's fonts nor its icons. MEASURED on the two English tables (29,477
  strings): the markup is tags in angle brackets, and there are seven of them.

| Tag | Places | In the viewer |
|---|---|---|
| emphasis (`em`, `i`) | 1,821 and 17 | left out, the text between kept |
| line break (`br`) | 280 | a new line |
| pad button or stick icon (`icon NAME`) | 299 | the icon's name in square brackets |
| font face, colour, size (`font`) | 68 | left out, the text between kept |
| a player's or a level slot's icon (`player`, `slot`) | 46 | left out, the text between kept |

  Anything else in angle brackets stays as text (three strings have words in them), and
  so do square brackets (stage directions in subtitles). Control characters other than
  the new line are dropped. What the game does with an unknown tag was not read.
  `Message::text` keeps the string as the script passed it.
- Placing. `place_bubbles` and `place_subtitle`, in screen pixels. A bubble's box is
  centred on its anchor's screen point with its lower edge 24 pixels above it. The
  bubbles of one anchor Thing form a stack, the first of the list lowest, 6 pixels
  apart. A stack that would leave the screen is moved back inside as a whole, 8 pixels
  from the edge, so a bubble whose Thing is off screen sits at the nearest edge. An
  anchor behind the camera places nothing. Stacks of different Things may overlap. The
  subtitle is centred, 48 pixels above the lower edge, its text at most 0.7 of the
  screen's width; a bubble's text wraps at 300 pixels. All of this is INFERRED.
- Painting. `speech_overlay.rs` in the viewer measures the lines with egui, asks the
  functions above for the places and paints boxes with the fill and outline of the
  viewer's own windows, behind them. The anchor goes from level units to the screen by
  the level's display matrix (`LevelDisplay`) and the camera. The boxes are also drawn
  under `--qa-hide-ui`: they belong to the level, not to a panel. This is not the game's
  bubble art.
- Counts. The "Level scripts" window has a line with the messages open, drawn, opened,
  written over and closed. The viewer prints one line on each tick on which a message
  entered the list or was closed (`Messages at tick N: ...`), with the anchor Thing and
  place of each drawn bubble: the check that needs no screenshot.

To see it: `lbp-bevy-viewer --snapshot <d133 snapshot> --game-dir "<game dirs incl. the
d133 edat overlay>" --level-path
gamedata/lbp2/th_cosmos/final_levels/comp_01_rocket_funland/comp_01_rocket_funland.bin
--level-scripts`. That level opens one bubble without a prompt on the first tick, with
no player. Add `--level-focus-thing 246` to frame its Thing, or `--play-sackboy` to walk
into the triggers of a level (`--play-sackboy-at X,Y,Z` to start near a Thing).

### Measured (PC)

- The pulsed measuring run of part 16 again (378 levels, 60 ticks, `--visit-triggers
  --pulse-switches`): the whole `play_all` block is equal to the D265 run. Things stopped
  by a fault 4; requests 2,381, opened 1,187, written over 1,161, closed 1,152.
- Viewer, release build, the level above, 1280 x 720, no focus taken (`--qa-capture`):
  translation tables 2; `Messages at tick 1: open 1, opened 1, written over 0, closed 0;
  drawn: bubbles 1` at Thing 246. Two frames looked at: one framed on the Thing, with
  the box above the magic mouth; one from the Sackboy follow camera started at the
  Thing's place, where the Thing is above the frame and the box sits at the upper edge.
  The frames stay in ignored `local-analysis/`.

### Not modelled

- The game's bubble: art, fonts, the tail, where the HUD script puts it, bubble positions
  1 to 8, how a caption and a think bubble differ from speech.
- Which of several queued bubbles the game draws (the sort states of part 16).
- The player's close button. The prompt line is text only.
- Icons, emphasis, font faces, colours and sizes of the markup; the `%1`-style
  parameters inside the player and slot tags.
- The subtitle's formatting in the game (0x736f1c) and its place.
- The level-link and wormhole prompts.
- Other languages: the two English tables are the ones loaded (INFERRED, part 14).

### Verification

`cargo test -p lbp-runtime`: 46 unit and 36 world tests. New, all with made-up text: 6
unit tests in `message_view.rs` (markup tags and what is not a tag; which messages the
overlay draws; a bubble above its anchor and at each edge and corner; stacks of one Thing
and their order near the top; an anchor behind the camera; the subtitle's place) and the
world test `d266_the_overlay_reads_the_worlds_messages_and_tables` (the overlay through
the messaging natives, the close prompt through synthetic tables with the patch table
first, a moved anchor). fmt, the CI clippy line, the CI test set, the Python tests and
`cargo check` of both viewers pass; `cargo clippy -p lbp-bevy-viewer --all-targets -- -D
warnings` passes. Seen in the viewer window on one level, one bubble without a prompt.
Not seen by eye: a subtitle, a speaker's name, a close prompt line, a stack of several
bubbles, a caption. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
