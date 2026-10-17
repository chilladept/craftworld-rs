# Level scripts, part 95: the goodies disc's pie (2026-10-17, D348)

Part 94 drew a part-filled goodies disc whole in the widget's colour. This part draws it as
the game does: two segments, the collected share in the colour and the rest in the shade.

### What was found

- FOUND (01.33, 0x4f5abc, the play stat's disc): one quad of centre +- radius and one draw.
  Collected 0 or equal to the total takes shader slot 5 (one stop, a plain disc in the
  shade or the colour); between, slot 6 (two stops) with the stops (theta, 2 pi), theta =
  2 pi x collected / total.
- FOUND (0x4a3824): the stops become the shader's angle limits b0 = (s theta + g) / 2 and
  b1 = b0 + s (2 pi - theta) + g (parameter 0x6a6c519c), s = (2 pi - 2 g) / 2 pi, the gap
  g = 0.1 radians (the global 0xdb86b4). The quad is turned by theta / 2 about the centre
  (0x4b54a8, matrix parameter 0x356ef243), so the collected segment starts at the shader's
  zero axis. Colour 0 is the widget's colour, colour 1 the shade (0, 0, 0, 0.3); the shade
  fills only the rest.
- FOUND: an edge softness of 0.1 (the global 0xdb86b0, parameter 0x33e74f53).
- INFERRED: where the shader's zero axis points and which way it turns are set in shader
  resource 0x109ee, not in the EBOOT; the viewer starts at straight up and turns clockwise.

### What changed

- Viewer: `MenuShape::Pie { start, sweep }` (a sector of the square capsule's circle with a
  pixel-wide soft edge) and `pie_segments`: the colour's segment from half a gap past the
  start, a gap, then the shade's. On the layer route a part-filled disc draws both; the UI
  route still draws it in the colour.

### Left out

- The shader's zero axis and turn (INFERRED), and its softness 0.1 (its units are the
  shader's; the viewer's edge is a pixel).
- No game frame of a part-filled disc was compared (the owner's Story levels shown are all
  at 100%).

### Verification

`d348_pie_is_a_clockwise_sector_from_the_top`, `d348_pie_segments_leave_the_game_gaps`.
`cargo fmt --all --check`; `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`;
viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
