# Level scripts, part 66: two clocks in the change-over: the dissolve against the planets zoom (2026-10-15, D315)

Research only, no code change. Part 63 left the left glass panel's dissolve trailing the game:
at viewer tick 615 the glass still covers the strip of room seen through the band, where the
sweep frame `d0600` (matched to about 615.6 by the Earth's radius, part 62) has it gone. Part 64
matched the captures to ticks by the depth of field's range instead (8348 at 611, 8980 at 616).
This part reads the dissolve's own value in the captures and sets the clocks against each other.

### What was found

- FOUND (RenderDoc 1.46, the glass draw 3041 and the pod mesh draw 756 in 8348, the pod mesh
  draw 719 in 8980, `local-analysis/d315-dissolve/`): the vertex program hands `vc[4]` to the
  fragment program as the colour (the pod's shape colour; its red, green and blue are the
  profile's (0.346, 0.751, 0.055) here) and `in_pos - vc[5]` as the position over the camera.
  The fragment program takes `1 - alpha` as the change-over value v (part 29).
- MEASURED: `vc[4]`'s alpha is 0.44706 (114/255) in 8348 and 0.19608 (50/255) in 8980: v 0.55
  and 0.80, the script's 0.05 a tick on ticks 611 and 616 (1 - 0.05 n stored truncated to 8
  bits). These are the ticks of part 64's focus-range clock, exactly. The camera handed over is
  (0, -299.24, 903.50) in 8348 and (0, -299.49, 902.34) in 8980 (at rest at -299.48, part 40).
- MEASURED (`tz.py`'s Earth fit, `circ.py`, on the same two capture frames against the viewer's
  d310 stills): the Earth's radius puts 8348 at 607.1 to 607.5 and 8980 at 613.6 to 613.9 (with
  and without the rest-radius scale k). So at the same script tick the game's Earth is smaller
  than the viewer's: the viewer's planets zoom leads the game's by about 2.5 to 4 ticks (3.5 to
  3.9 at 611, 2.1 to 2.4 at 616). The zoom is the planets view blend (`PlanetsWorld`'s
  `ViewTransition`, 0.05 a frame through `SmoothStep`, part 46), started by the pod computer's
  per-frame step; the dissolve and the range are pod.bin's per-tick script. Part 50 read the
  game's blend as about two ticks ahead of the VM's from `open-t1`, and parts 50 and 52 a
  one-tick lead of the zoom over the dissolve; part 61 matched the dissolve to the zoom within
  half a tick on lane A's sweep. Against the dissolve clock in these captures the game's zoom is
  behind, not ahead (INFERRED: the cause is the blend's start or order in the VM, not read).
  [Correction, part 67: this lead is withdrawn. The Earth was timed on desktop grabs
  (`th*.png`, 0.83x the frame and 0.5 to 1.8 ticks earlier than the captured frame), not on the
  captures; on the captured frames the game's blend is the VM's, tick for tick. The dissolve and
  range clock (611 / 616) stands; the sweep offsets in the next point rested on the lead.]
- INFERRED (the offset above applied to part 62's sweep): on the script clock `d0550` is about
  tick 615.5 and `d0600` about 617.5, not 613.3 and 615.6. Lane A's sweep and lane B's captures
  were taken in different sessions; the offset is carried over, not measured on lane A.
- FOUND (the glass's fragment program in 8348): with `h` the height over the camera and v as
  above, the edge value is `(h + (-250 - 50 v) - 0.117647 x 100 - 500 + 1600 v) x 0.0025`
  (fc18 (50, -250), fc20 / fc21 0.117647 and 100, fc27 -500, fc28 1600, fc29 1/400), and the
  fragment is discarded above 1: the line is at `h = 1150 + 11.76 - 1550 v`, part 29's rule
  with the glass's constant (part 37). The mask takes 40 tiles (fc34) and the flat share is cut
  above 0.99023 (fc35). The pod mesh's program carries the same terms (fc26 (50, -250), fc30
  100, fc33 -500, fc34 1600).
- The viewer computes the same line: `pod_dissolve::line_y` (`pod_dissolve.rs:116`, camera y +
  1150 - 1550 v), the glass's 0.11765 through `dual::enable_glass`, and the edge test in the
  shader (`dual.rs`, `(above - dissolve_consts.x * green) / dissolve_consts.y + 1`). v is the
  script's alpha, the same 0.05 a tick. Its camera is the ported `Smooth3D` (part 40), at rest
  from 615; at 611 it is within a few units of the game's (INFERRED from the rule: within 6 by
  call 10), under 0.01 of a tick of line height.
- Looked at (`local-analysis/d315-dissolve/left_sheet.png`, the left half, viewer warped as part
  57): 8348 against the viewer at 611, and 8980 against 616 (the glass gone in both); the
  viewer's 615 still shows the strip, as `d0550` does.

So the viewer's dissolve is not late: on the dissolve's own clock it is the game's to the tick.
The 615 comparison set the viewer's 615 against a game frame of about 617.5.

### What the viewer does

Unchanged.

### Verification

RenderDoc replays of lane B's captures 8348 and 8980 (part 62) through `qrenderdoc --python`,
minimised and closed after each; constant buffers and program sources read with part 63's
`dump.py`. No RPCS3 run, no viewer run; stills of parts 63 and 64 reused. Scripts and dumps stay
in ignored `local-analysis/d315-dissolve/`. No code change.

Counts: +0 newly viewable / +0 newly Complete (research).
