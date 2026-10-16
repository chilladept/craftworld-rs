# Level scripts, part 81: the Story title's speech bubble (2026-10-16, D330)

Part 80 drew the Story frame's "STORY" title as plain white text. A game frame of the
same moment shows it in Crikey Tall, yellow, inside a yellow speech-bubble outline with a
tail pointing at the planet. This part reads how the engine draws that bubble, draws it in
the viewer, and records what the Story view does next (it is not a view of Earth's badges).

### What was found

- FOUND (01.33, the bubble style enum at 0xd62d20): style 15 is
  `GCS_SPEECH_BUBBLE_OUTLINE` (13 `GCS_SPEECH_BUBBLE`, 10 `ROUNDED_BORDER`, 1 `FLAT_FILL`).
  The StartFrame path (0x637cb4 -> 0x487490) gives styles 0..11 their look from a jump
  table; above 11 it only stores the style (frame +0x17c), so the look of style 15 comes
  from the frame's own calls. The style-15 code (0x4878d4) takes the frame's rectangle, a
  radius capped at half its sides, and the tail value (frame +0x44c, default 20).
- FOUND (the script listing of `SearchDetailsStoryLevels.DoTitle`): `StartFrame(1)`, then
  `SetFrameStyle(15)`, `SetFrameSpeechBubbleOutlineTailSettings((50, 0, 0, 0), 30)`,
  `SetFrameOutlineWidth(35)`, `SetFrameCornerRadius(80)`, curvature and corner jauntiness 9,
  seed 2, `SetFrameBorders(45, 45)`, the text in style 16 at font scale 2.5, `EndFrame`. No
  tint or text colour is set, and no colour for the style is in the EBOOT.
- MEASURED (two game frames, 1280 x 720): the bubble is 129 x 76 pixels (x 221..350,
  y 312..388) with the tail's tip at x 367; the letters are 85 x 48 pixels; the outline is
  about 2 pixels wide; the yellow's brightest quarter is (210, 179, 64) and (204, 182, 53).
- FOUND (badges, for the next step): a result's badge sits at the slot's `location` (a unit
  vector, w 0, in all 82 story slots) times the planet's radius (Earth 1750, 0x420814), its
  axes from (0, 1, 0) and that direction (0x6ffff4), scale radius x angle / 240; angles
  0.18 for a story group and 0.12 for a level (0x16afb0), times per-type scales. On Earth,
  Story shows the groups only, until a group is zoomed (0x428174).
- FOUND (the world view): when a group is chosen the planet loads the group's planet
  decorations into Earth's own world (0x43cd64 -> 0x42324c), at Earth's centre and radius.
  The game frame after Story shows the default group's world (The Cosmos on the owner's
  profile) with one badge, not Earth with 82 badges.

### What changed

- Runtime: `SpeechBubble` (outline width, corner radius, tail offset and size) and
  `PlacedKind::Bubble`, placed for a style-15 frame before its children; a `SetFrame*`
  call's vector arguments are kept (the tail's offset was lost before).
- Viewer: the bubble is drawn in the measured yellow, the text in it too and opaque. On the
  menu layer (`MenuShape::Bubble`) it is a rounded ring open under a tail; on the overlay
  it is the node's border with a chevron child. INFERRED: the outline's width is the passed
  width over 8 (35 would be 16 pixels at the frame's 0.47 pixels a unit; the game's is 2).
- `lbp-preview` example `story_slot_probe`: each story slot's id, group, `location` and
  planet fields.

### Viewer run (part 80's flags, the fonts' loose files as a fourth `--game-dir` root)

| | Game | Viewer |
| --- | --- | --- |
| Letters | 85 x 48 px, yellow | 87 x 49 px, yellow (white before) |
| Bubble | 129 x 76 px at x 221, y 312 | 123 x 70 px at x 212, y 322 (none before) |
| Tail | tip 17 px past the edge | a chevron 14 px past it |

The viewer's bubble is about 10 pixels left of and below the game's; the game's Earth is
also nearer the camera at that moment, so the zoom, not the layout, is the likely cause.

### Left out

- The corner curvature and jauntiness (the game's corners wobble), the tail's offset
  (50, 0) and the tail's exact shape; the zoom to Earth; the group badges and the world
  decorations (next).

### Verification

`d330_style_15_frame_is_a_speech_bubble_behind_its_text` (lbp-runtime),
`d330_bubble_outline_is_a_ring_with_a_tail` (viewer). `cargo fmt --all --check`; `cargo
clippy --workspace --all-targets -- -D warnings`; workspace tests (the two known
`lbp-viewer` diagnostics path tests fail locally, as before). Captures stay in ignored
`local-analysis/d330-earth-badges/`.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
