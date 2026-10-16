# Level scripts, part 67: the planets zoom against the game's captures (2026-10-15, D316)

Research only, no code change. Part 66 put the viewer's planets zoom 2.5 to 4 ticks ahead of the
game's on the dissolve clock. Here the game's blend is read from the captures' own constants and
geometry instead of the Earth's radius on screen, and the lead is traced to the frames it was
measured on.

### What was found

- MEASURED (RenderDoc 1.46, lane B's six captures, `local-analysis/d316-zoom/`): the pod's
  `vc[4]` alpha (part 66) puts 7295, 7927, 8348, 8980 and 9601 at ticks 606, 609, 611, 616 and
  619 (178, 140, 114, 50 and 12 / 255); 6082 is before "in use" (alpha 1, camera at (0, -250,
  1130)).
- FOUND (the Earth draw's vertex program, `tc3.z = dot(pos, vc[16])`): `vc[16]` is a fog plane,
  the view direction over 52,649 plus an offset. Its zero distance along the view is 25804.6,
  21484.6, 17299.6, 14309.6, 7884.6 and 5949.6 in the six captures: 5804.6 + 20000 (1 - s),
  `SetDOFAndFog`'s FogBias x 2 (10,000 in view 0, 0 in view 1, part 46) blended by s.
- MEASURED: s is then 0.2160, 0.4253, 0.5748, 0.8960 and 0.99275 at 606 to 619, which is
  SmoothStep(0.05 n), the VM's blend at the same ticks, to four places. The game starts the
  transition on the same tick, steps it by 0.05 a tick and shapes it with SmoothStep.
- MEASURED (the planets level's draws, world positions from the post-VS clip positions through
  the inverse of `vc[0..3]`, 87 objects matched by index count): every object of the planets
  level, the Earth and the Moon included, moves with one rigid transform between captures
  (median residual 3 to 9 units, scale 0.998). Its turn is about x and equals
  2 x (the camera's pitch change) - 0.2 s within 3e-4 rad at all five ticks.
- MEASURED: sphere fits to the Earth's vertices give R 3236 to 3241 and the camera-to-Earth
  distances 32606, 27978, 23779, 20845, 14669 and 12852. `GetTransform` (twice the size about
  the camera's eye, the pitch, the offset blend; PlanetsView listing) with the VM's s, anchored
  at 6082, gives each distance within 0.1 %. The extra turn above is about the eye, so it keeps
  the distance (and the Earth's size on screen); its source was not found in the script.
- MEASURED (where part 66's lead came from): part 66 fitted the Earth on `th*.png`, desktop
  grabs of the lane B window (1264 x 681, with the final pass's smaller screen area). By room
  registration they are 0.83x the captured frame; the viewer's warp and the sweep frames are
  about 1.0x (sweep = 1.025x the frame). Against the model's radius curve the grabs are also
  0.5 to 1.8 ticks earlier than the captured frame.
- MEASURED: the Earth fitted on the captures' own output (`rt8348.png`, `rt8980.png`) sits at the
  viewer's ticks 611.8 to 612.2 and 616.9 to 617.5 (with and without the rest-radius scale):
  no lead; the viewer, if anything, is up to a tick behind.
- MEASURED: the viewer's Earth radius (d310 stills) over the game model's is a flat 0.946 to
  0.954 over ticks 600 to 611 and comes to 1.0 at rest (620). INFERRED: a static placement
  difference of about 5 % in view 0, not timing (a 3-tick shift would give about 1.3 at 611).

So the VM's planets view blend is the game's: no change to its start, rate or shape.

### What the viewer does

Unchanged.

### Verification

RenderDoc replays of lane B's captures 6082, 7295, 7927, 8348, 8980 and 9601 through
`qrenderdoc --python` on a never-shown desktop (`shading/rd_hidden.py`, 0 foreground samples),
closed after each; vertex constants, post-VS positions and the main target read there. The
PlanetsView and PlanetsWorld listings (`d133-script-listings-0133`) read for `GetTransform`,
`UpdatePlanetsView` and `UpdatePodCamera`. No RPCS3 run, no viewer run; the d310 stills reused.
Scripts and dumps stay in ignored `local-analysis/d316-zoom/`. No code change.

Counts: +0 newly viewable / +0 newly Complete (research).
