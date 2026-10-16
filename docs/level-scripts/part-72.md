# Level scripts, part 72: the Pod main menu in the game's font, with the player's card (2026-10-15, D321)

Part 70 drew the main menu's text in the OS sans serif with text widths from a fitted model,
and the card as flat colours. Here the text is drawn from the game's own font at the size the
Gooey's text style gives it, the pills are measured with that font, and the card shows the
player's default picture and, given their GUIDs, the profile's pins.

### What was found

- FOUND (01.33 EBOOT, Ghidra bridge): the Gooey natives `GetFont__Q21Gooey.EGooeyTextStyle`
  and `GetFontSize__Q21Gooey.EGooeyTextStyle` read a table of 26 text styles (0x48a518 /
  0x48a584, table at 0xc25cec through the TOC; style s: a font index and a size, the size
  times a per-Gooey scale; a style past the table gives the first font at size 16). The main
  menu's buttons pass style 1 (script listing: `DoSpaceButtonNamed(id, label, 1, ...)`, font
  scale 1): font 1, size 32.
- FOUND: the engine's font GUID table (0xc26168): the Chinese fonts (0002347c, 0002347d,
  00020510, 0000eca2), then 000189f0 crikey_squat, 000189f1 crikey_tall, 000203db
  eigerdalsbla, 00004144 helveticaneueltstd-md, 000305bc icon_patch_font. The Omnes fonts
  (00004146 ...) are not in the EBOOT. The script enum names the fonts `FONT_HELVETICA`,
  `FONT_HEAVY`, `FONT_CRIKEY_SQUAT`, `FONT_CRIKEY_TALL`, `FONT_ICON_PATCH`; INFERRED that this
  is their value order, so font 1 is Eigerdals Black. MEASURED: the game frame's letters
  (`rt20_9601`, the 'y' tail, the 't', the weight) are Eigerdals Black's; Omnes Black, Omnes
  Semibold and Helvetica differ by eye.
- FOUND (local data): the fonts are loose files on the disc (`PS3_GAME/USRDIR/GAMEDATA/FONTS`,
  15 FNTb), outside DATA.FARC, so the snapshot listed 15 of 16 font rows as missing (D110). Read
  from the user's disc image into the ignored `local-analysis/d321-disc-fonts/USRDIR/gamedata/
  fonts/` (`local-analysis/d321-font/iso_fonts.py`; SHA-1 of omnesbla.fnt equals its MAP row).
  Eigerdals is revision 0x3b3, compressed: one byte a pixel. Omnes is 0x26e, uncompressed, two
  bytes a pixel.
- MEASURED: a glyph's pixels are a distance field, not coverage: 0x80 at the outline, rising
  0x1c to 0x20 a font pixel inwards (the first byte of the uncompressed fonts; the second byte's
  meaning stays UNKNOWN). Advances are in 1/64 font pixel. Eigerdals' 'H' is 18 font pixels tall.
- MEASURED (the game frame's alpha channel mapped into root-frame units through the runtime's
  tick-619 matrix, `local-analysis/d321-font/menu_units.py`): at style 1 the text is 1.96 to
  2.11 units a font pixel (Me 78.3 units of ink against 40 font pixels, cap height 35.4 against
  18; Store 138.7 against 68; Recent Activity 392.6 against 187). INFERRED rule: size / 16 (16
  being the table's fallback size), 2 units at size 32. With each advance rounded up to a whole
  font pixel the ink widths come out at Recent 172 / 174, Activity 200 / 200.6, Recent Activity
  390 / 392.6, Store 140 / 138.7, Me 80 / 78.3 (rule / game); unrounded advances are 6 to 19
  units short on the longer words, so the rounding is INFERRED.
- MEASURED: the pills are their text's rounded advance plus 49.0 to 57.7 units (Me 51.2, Store
  49.0, Story 51.7, Community 53.5, Recent Activity 57.7): `BUTTON_PAD_FONT` 52.6, fitted. The
  text is centred on the pill by its advance and cap height (ink 23.7 to 26.5 units from the
  pill's left end, 28.6 to 32.6 from its right, the side bearings).
- MEASURED (Me, against the field rasterized at its projected size, `fit_edge.py`): the
  outline at field value 125 to 135 and a soft edge about 2 screen pixels wide. Over the pill
  the text adds 0.42 of white in the picture's colours (pill (68, 96, 38), text (146, 164,
  128)); the alpha channel's 0.6 (part 70) is its coverage.
- FOUND (01.33): `DoPlayerBigIcon(GetLocalPlayerOnlineID(), 0, 0, 0, false)` draws the card;
  the GUID of `gamedata/texture_library/pod_computer/profile_default.tex` (000028d8) is in the
  engine's GUID table (0xc2726c) beside `level_default.tex` (000029ba). MEASURED: the game
  frame's card picture is that texture, 108 px across and turned 2.5 degrees (mean difference
  8.0 levels over the face, against 41 for its mean colour). INFERRED: the default for a player
  without a picture (the PSN avatar, offline in RPCS3).
- MEASURED (pin textures under `gamedata/texture_library/ui/pins/`, 276, matched on the frame):
  the three round icons are pins: `pin_general_night_owl` (0001e755, mean difference 11.1, next
  best 27.7), `pin_share_pod_time` (0001e75e, 21.4 / 43.4) and `pin_general_early_bird`
  (0001e753, 10.8 / 29.5). Which pins a card shows comes from the player's profile, which the
  viewer does not read (not established which of the profile's pins; INFERRED the latest or
  chosen three).
- MEASURED (card): a white frame (216, 221, 226), opaque, turned clockwise 2.9 to 3.8 degrees by
  its edges (2.5 by the picture's match; the source of the turn is not found); the picture 108 px
  of the 118 x 145 px card, square, 5 px from its sides; the pins 41 px across, centres at 18.5 /
  49.3 / 80 % across and 86 % down, level; a soft drop shadow (not drawn).

### What the runtime does

`lbp-runtime` (`gooey.rs`): `TEXT_STYLES` (the FOUND table), `GooeyFont` with the font files'
MAP paths, `text_style`, `units_per_font_pixel` (size / 16), `FontAdvances` (rounded-up
advances by character) and `GooeyFonts::text_width`. `layout_with(root, focus, fonts)` measures
buttons and text with the loaded fonts (pill = `BUTTON_PAD_FONT` + text) and falls back to the
fitted model for a font or a character that is not loaded; `layout` is `layout_with` without
fonts (the `level_scripts` report is unchanged). Placed buttons and texts carry their style and
font scale.

### What the viewer does

- `game_font.rs`: the Gooey's five fonts are read as loose files under the `--game-dir` roots
  (`USRDIR/<map path>` or `PS3_GAME/USRDIR/<map path>`, any case) when their size is the MAP
  row's, parsed by `lbp-resource`, and text is rasterized from the glyphs' distance field
  (bilinear, outline 130, edge 2 screen pixels) at the size it is drawn, centred on its advance
  and cap height.
- `pod_menu.rs` (scripted Pod, `--pod-camera N --pod-shell HEX`): `MenuAssets` holds the fonts,
  the card picture (000028d8 through the catalog) and the pins of `--pod-card-pins HEX,HEX,HEX`;
  the log prints `Pod menu assets: fonts [...]; card picture profile_default; pins 3`. The
  layout uses the fonts; each label is an image rasterized again when its projected size changes
  by 2 %, white at the old label alpha. The card has the picture, the pins (or the PLACEHOLDER
  circles without the option), the measured frame colour and turn. Without the font files the
  labels are the OS sans serif as before.
- The fonts are not in the archive the snapshot indexes: a game directory must hold the disc's
  loose `gamedata/fonts` (an extracted disc has them; here the overlay above is passed as a
  fourth `--game-dir` root).

### Measured (viewer stills, hidden window, level lighting, in use at 600)

`local-analysis/d321-font/shot.ps1` (stills `local-analysis/rscript/stage6/shots/d321-b-tick{616,
619,625}.png` with `--pod-card-pins 1e755,1e75e,1e753`, `d321-nopins-tick619.png` without).
Text span along each pill (px, `d321-font/text_span.py` and `story_span.py`; the same rule on
both pictures):

| label | game 619 | D321 619 | D319 619 | game 616 | D321 616 | D319 616 |
|---|---|---|---|---|---|---|
| Story (focused) | 107.3 | 111.1 | 90.2 | 72.4 | 74.3 | 60.8 |
| Community | 148.0 | 148.0 | 136.5 | 99.0 | 98.5 | 83.5 |
| Recent Activity | 202.0 | 200.5 | 166.5 | 134.5 | 134.0 | 110.0 |
| Me | 41.0 | 40.5 | 30.5 | 27.0 | 27.5 | 20.0 |

- Me's ink box: game x 745-784, y 291-309; D321 746-786, 291-309. The viewer's letters cover
  more pixels (Me 488 against 342 by the same whiteness rule; Story 1,983 against 1,521): the
  label's white is brighter over a brighter pill (Bevy blends the UI in linear light; the game's
  pill (68, 96, 38) and text (146, 164, 128) against the viewer's (95, 140, 55) and (213, 221,
  208)). Not changed here.
- Pills (`d319-menu/viewer_elements.py` against D320's still without the menu): Story 157 x 66
  (game 155 x 65), Store 100 x 43 (99 x 42), Me 68 x 42 (69 x 43); pill lengths in units by the
  rule: Community 344.6 (game 345.5), Recent Activity 446.6 (451.7), Store 194.6 (191.0), Me
  134.6 (133.2), Story 303.6 (302.2).
- Card: box x 577-703, y 211-365, centre (639.8, 286.6) (game 579-704, 214-366, (640.4, 288.2));
  D319's 120 x 148 box is now 127 x 155 (game 126 x 153) with the turn. The picture and the
  three pins match the game frame by eye (`local-analysis/d321-card/`).

### Left out

- The player's own picture (PSN avatar) and the profile's pins: the viewer does not read the
  profile; the pins are given by GUID (`--pod-card-pins`), else PLACEHOLDER circles.
- The text's and pill's blending in the picture's colours (the viewer's are brighter), the
  card's drop shadow, the source of the card's turn, the per-Gooey size scale (INFERRED 1),
  the font enum's values (INFERRED from the name order), the second byte of uncompressed glyphs,
  kerning (none found in the FNTb records), `ROOT_ROWS_DOWN` and the other fitted metrics of
  part 70 (row height, pill height, focus scale, the card's node).
- Direct reading of the fonts from the disc image (the loose files must be in a game directory).

### Verification

`lbp-runtime`: `d321_text_styles_and_font_widths_size_the_pills`. Viewer:
`d321_text_rasters_from_the_distance_field_centred_on_its_advance`,
`d321_loose_files_are_found_ignoring_case`, `d321_card_pins_option_reads_up_to_three_guids`
(synthetic font, no game data). `cargo fmt --all --check`; `cargo clippy -p lbp-runtime -p
lbp-bevy-viewer --all-targets -- -D warnings`; tests of both crates pass except
`sackboy_play::tests::input_script_steps`, which fails on main locally too. Research files stay
in ignored `local-analysis/d321-font/`, `d321-card/` and `d321-disc-fonts/`.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
