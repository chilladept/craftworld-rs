# Level scripts, part 50: the final pass outside the Pod, and the zoom at the change-over (2026-10-14, D299)

Measurement only: no code change. Part 45's "Next" asked for one non-Pod game frame before
`--final-pass` could be on by default on the level-lighting route; part 46 left the view in
the viewer's Pod picture open. Both are measured here.

### The final pass in a level (FOUND, MEASURED)

- FOUND (RenderDoc replay of the blank Moon level's RSX capture, frame 40, create mode,
  paused; hidden desktop): the same post chain as the Pod's frames 5247 and 14790. The source
  pass (eid 4192) has every constant of frame 5247's (fc4 = 0.7, the 0.33 volumetric weight,
  the +-0.015 side streak, (0.06, 0.12, 0.15)); the two 9-tap blurs follow; the final pass
  (eid 5403) has 1.2998 on the scene before the glow add, the vignette's (1.0996, 1.7002),
  1.2, the smoothstep's 3, the 0.2 floor and the identity colour matrix, all equal to frame
  5247's, and ends, as there, in `lerp(c, scene, scene.a)`. It draws over the full 1280 x 720
  (no 0.875 screen area) and has one more term than 5247's program: a noise texture and a
  horizontal band texture scrolled by a time constant, mixed in at 0.2 (INFERRED the pause
  screen's effect).
- FOUND: in that replay the scene the final pass reads has alpha 255 on every pixel, and its
  output equals its input (the sky (125, 183, 188) in both). Frame 5247's copy had alpha 0
  everywhere (part 43), so there the scale and vignette are the output. The constants are the
  game's in both places; what turns them on is the scene's alpha.
- MEASURED (the game's own frames of non-Pod levels, on disk): the Moon in play mode
  (`s16-play`) and paused (`s08`) has its sky flat at (125, 183, 188) over 31 % of the frame,
  the fog colour (0.4905, 0.7193, 0.7397) in 8 bits: not x1.2998 and no vignette (the
  vignette would take a corner to 0.36). Its quilted floor is brighter at the bottom corners
  than at the bottom centre (145 / 138 against 114; the vignette predicts 0.72 of the centre).
  Bravery Test (0001e795, `live-bravery-a-20261010`, the spawn door): the ledge's front face,
  one wood across the frame, is flat from edge to edge (edge / centre 0.96 and 1.21).
- MEASURED (viewer at origin/main f653c64, Bravery Test, level lighting, `--play-sackboy-at
  -10765,2773,-200`; its camera is closer than the game's, so only like surfaces compare):
  the same ledge face edge / centre is 0.86 and 0.91 without the flag and 0.59 and 0.62 with
  it; with it the centre is x1.30 and the bottom-left corner x0.48 of the frame without it.
  The game's frame is the one without.
- So `--final-pass` stays off by default: the game draws the level-lighting route's levels
  as the viewer does without it. The Pod frames (part 45) are the place where the scene's
  alpha is 0. What clears it there (between frame 5247's source pass and its final pass) was
  not found; INFERRED the Pod or its menu, not the level lighting.

### The zoom at the change-over (MEASURED)

d299 release build of origin/main f653c64, hidden window (`scratch/d287-shot.ps1 -Prefix d299
-DefaultLight --level-scripts --level-scripts-in-use-at 600 --level-scripts-hold-at <t>
--final-pass`), stills `d299-on-tick{613..616,621}` in the ignored shots folder.

- The planets level does move under its world Thing as the VM's blend (part 23,
  `level_play::follow_world`): along the view its draw entities reach from -46071 to 69306 at
  rest (tick 452) and from -59927 / -61068 / -62121 / -63065 / -66694 at 613 / 614 / 615 / 616
  / 621 (near end), about 20,000 level units x `SmoothStep(transition)` (0.648, 0.718, 0.784,
  0.844, 1.0).
- Room behind the Pod against `open-t1` (`regroom.py`, the game = s x viewer about the
  centre; the shell's own s is 0.877, dx +2, dy 0): before part 46 (d294 still at 614) s 1.62,
  dx -96, dy -172, correlation 0.29; now s 1.00 / 0.96 / 0.91 / 0.88 / 0.78 at 613 / 614 / 615
  / 616 / 621 (614: dx -4, dy -16, 0.65; 616: dx 0, dy 0, 0.72, the best). So the room is now
  within 9.5 % of the game's at 614 (it was 85 %), and matches the shell's own relation at 616.
- Region means against `open-t1` (`d294cmp.py`, mean |d| over RGB), d294 at 614 / now at 614 /
  616: room behind the window 10.6 / 5.0 / 1.2; upper left 21.4 / 7.8 / 3.4; upper centre 20.6
  / 5.1 / 0.4; upper right 15.5 / 1.6 / 2.6; Earth box 49.6 / 15.6 / 4.7; Moon box 64.0 / 23.5 /
  3.8; right glass panel 7.0 / 3.6 / 3.0; left glass 2.0 / 0.4 / 5.3. The whole viewport 25.2
  -> 15.0 (614) and 11.6 (616, where the posts are already below the line's place).
- At rest, against `open-t2`: the viewer's 621 puts the Earth at r about 171 game px (t2 about 165)
  and the Moon at about 82 (80), their centres within 5 to 7 px; the room registers at s 0.93,
  dx -28 (correlation 0.49, t2's room is blurred by the depth of field).
- So the view at the line's time (614.2, part 35) is the viewer's of about tick 616: the
  game's blend runs about two ticks ahead of the VM's (transition about 0.75 against 0.66).
  Part 46 left one tick to the order of the pod computer's step against
  `UpdatePlanetsView`; the other is not found (the start of "in use" against the dissolve's,
  or the extra planet zoom, 0 in the VM, part 44).

### Verification

No code changed. RenderDoc replays on a never-shown desktop (`shading/rd_hidden.py`, 0
foreground samples); viewer stills from a hidden window. Nothing in RPCS3 was started or
captured.

Counts: +0 newly viewable / +0 newly Complete (measurement).
