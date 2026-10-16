# Level scripts, part 36: the player's default colour as the Pod's flat colour (2026-10-14, D285)

Twelfth PR of the "Pod picture" follow-up to stage 6. A runtime change (the start value of
a player's colour) and a viewer change (how the flat colour reaches the shader).

### What was found

- FOUND (01.33, static reading, evidence only): a new profile's four colours are set by
  0x1589a8 from a table filled at boot from the script class `PlayerColour` (12 entries,
  each primary, secondary, tertiary and emphasis), at index 0 from both of its callers
  (0x17ff90, INFERRED a new profile, and 0x18b388, INFERRED an old profile without
  colours). Entry 0's primary colour is (150, 221, 60), packed 0xFF96DD3C; it is what
  `Native.GetPlayerColour` returns for a new player (the first of the four, part 24). The
  palette is only in the script: no packed word or float of it is in the EBOOT (MEASURED).
- FOUND (part 21): the Pod script sets the tent's shape colour to
  `Native.GetPlayerColour(avatar)` with alpha `1 - v`.
- MEASURED (the captured draws of part 29): the flat colour reaches the shader as vertex
  constant vc[4] = (0.34602, 0.75111, 0.05536, a), exactly (150/255)^2, (221/255)^2 and
  (60/255)^2, alpha not squared. The shader mixes it in after the lit colour's square
  root and takes no root of it, so the frame shows vc[4] x 255 = (88, 192, 14). INFERRED:
  the engine decodes the shape colour with x times x on upload (the upload code was not
  read).
- Whether the owner's profile holds the default or chose entry 0 is not decidable from
  the frame; either way the colour is entry 0. The viewer reads no profile.

### What changed

- `lbp-script-vm`: `natives::DEFAULT_PLAYER_COLOUR = 0xff96dd3c` (FOUND) replaces the
  white placeholder as the colour a `PlayerRecord` starts with. `level_scripts
  --player-colour` still sets another. `LevelWorld::player_colour` without any record
  returns it too (PLACEHOLDER for that case: the game reads a word that was not read).
- `lbp-bevy-viewer`: `pod_dissolve::uniforms` squares the stored red, green and blue
  before decoding them to linear, so the output's encoding shows the squares as display
  values, as the game's frame does. `pod_dissolve::shown_bytes` gives those bytes, and
  the first dissolve line prints them: `flat colour [0.588, 0.867, 0.235] (shown as
  display bytes [88, 192, 14])`. `dual.rs` is unchanged.

### Measured (PC, release build)

Same setup as part 35. Frames in ignored `local-analysis/rscript/stage6/shots/`
(`d285-*`); the same commands with main's build first (`d285-base-*`), which gave the
d284 frames byte for byte (all eleven).

- Equal byte for byte to main's build: the four captures without scripts, the shell under
  scripts closed at 600, the planets level at 600, in use from 600 at 619 and at 640
  (the platform's rim is kept and lit at 640, part 35, so it shows no flat colour).
- Changed, in use from 600: at 610, 57,441 pixels (y 64 to 688); at 615, 45,221 (y 577
  to 719); 45,281 at 615 under the default lighting. The line and the dark traces of the
  band are green where they were white.
- Edge colour, the strongly green pixels (green over 120, over red by 50 and blue by 80):
  the viewer's 615 has 5,281, median (88, 191, 14), the brightest band (green 170 to 200)
  (88, 191, 14); its 610 median (88, 191, 15). The game's `open-t1` has 1,278, median
  (84, 158, 24); in the same brightness band (93, 180, 24), and 230 brighter pixels at
  (121, 222, 36). INFERRED: the game's line is the same colour softened by fog and blur
  and lifted by a glow the viewer does not draw; the hue matches by eye.
- `level_scripts --all --play=60`, 378 levels, plain / with `--visit-triggers` / with
  `--visit-triggers --pulse-switches`: Things stopped by a fault 3 / 3 / 3, as D274. Plain
  equals `d274-play-all-novisit.json` in every value. With the visit, one difference
  each: pod.bin's shape colour is now unlike the level file's (the script's colour was
  white like the file's before), so `display.unlike_the_level_file` counts one more
  level and Thing under `changed` and `shape_colour`. Outputs:
  `d285-play-all-{novisit,visit,visit-pulse}.json` (D274: `d274-play-all-*`).

### Left out

- The players' own profiles (the viewer reads none) and the colours of players 2 to 4
  (both callers pass index 0; `CacheternatePlayerColours__`, which INFERRED changes a
  colour two players share, was not read).
- The fog over the flat colour and the game's glow on the line.

### Verification

`lbp-script-vm` `d273_pod_computer_natives` (a record starts with 0xff96dd3c, its vector
(150, 221, 60, 255) / 255); `lbp-runtime` `d273_the_pad_zone_the_engage_press_and_the_players_colour`
(a new player's `GetPlayerColour` is that vector); `lbp-bevy-viewer`
`d285_pod_flat_colour_is_the_square_of_the_stored_colour` (the default shows as the
captured vc[4] and as bytes (88, 192, 14)) and `d278_the_world_plane_measures_in_level_units`
(stored 0.5 shows as 0.25). fmt, the CI clippy line, the CI test set (612 pass), `cargo
check` of both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`
pass; the viewer's tests pass (89) but the one that fails on main
(`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
