# Level scripts, part 42: the dissolve's band mixed in the game's output (2026-10-14, D291)

Eighteenth PR of the "Pod picture" follow-up to stage 6, viewer only: no runtime change.
Part 36 left the glow on the tent's green line and the fog over its flat colour. This part
reads the tent's own shader for both, fixes the one difference in it, and leaves the glow,
which is a post pass, to the next part.

### What was found

- FOUND (`pod_card_edge`, the RPCS3 decompile of frame 5247): the shader ends `out =
  mix((COL0, 0), (lit, texalpha), saturate(t))`, with `lit` already in the game's encoded
  output (after its sqrt), then fogs `out.rgb` by view depth. Nothing on the band or the
  line adds light: the line is COL0 exactly. The mix also writes alpha = t, 0 on the line.
- FOUND (frame 5247's post chain): the glow is a separate post pass that reads the scene's
  alpha, `0.7 x (1 - a) x rgb` at half size, two 9-tap blurs at a quarter, added in the
  final pass (which also scales the whole frame by 1.2998). Not in the tent's shader.
- FOUND (vc[16] of both Pod captures): the fog's amount is 0 on the whole tent, closed and
  open. The game fogs after the mix and the viewer fogged before it, but with no fog there
  the order changes nothing visible.
- The real difference: the viewer mixed the flat and the lit colours as linear values,
  the game mixes their encoded values. In the band (keep between 0 and 1) the viewer was
  brighter toward the flat colour: lit 0.2 and flat 0.75 (encoded) at keep 0.5 give 0.475
  in the game and 0.564 in the viewer.

### What changed

- `dual.rs` shader: `lbp_output` is split into `lbp_encode` and `lbp_fog_encoded`. On the
  level-lighting route the dissolve mixes `srgb_encode(dissolve_colour)` (the game's COL0,
  D285) with the encoded lit colour, then fogs, then decodes. On the other routes (no fog
  of the game's) the encoded pair is mixed the same way. A whole kept share (keep 1)
  returns the lit colour as it is.
- A CPU mirror, `dissolve_mix`, for the tests.

### Measured

Pod with the scripts, in use at 600, held at 615 / 617 / 618, both lighting routes; D290's
build against this one. Frames in ignored `local-analysis/rscript/stage6/shots/` (`d291-base-*`,
`d291-new-*`, capture script `scratch/d287-shot.ps1`).

- Every changed pixel is in the band under the tent's line: 54,476 / 17,605 / 802 pixels at
  615 / 617 / 618 under the level lighting (43,690 / 17,677 under the default), at most 51
  levels, mean (-2, -10, -1) to (-6, -24, -1): the circuit traces ramp darker and greener
  into the line, as in the game's `open-t1`.
- The line itself is unchanged: 0 of 687 (615) and 0 of 4,299 (617) pixels at (88, 192,
  14) move.

### Left out

- The glow (next part): the alpha = t write and a full-screen post node.
- The final pass's x1.2998 over the whole game frame: whether the viewer's lit colours
  already include it is open (the game's line is 1.3 x COL0 in `open-t1`).

### Verification

`lbp-bevy-viewer`, no game data: `d291_the_line_shows_the_flat_colour` (keep 0 gives the
COL0 bytes (88, 192, 14) for any lit colour), `d291_the_band_mixes_in_the_encoded_output`
(0.475 at keep 0.5, and the encoded output follows the game's mix along the band) and
`d291_a_whole_kept_share_is_the_lit_colour`; the shader text test checks the new lines and
that the linear mix is gone. fmt passes; the viewer's tests pass (107) but the one that
fails on main (`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
