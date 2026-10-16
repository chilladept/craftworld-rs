# Level scripts, part 70: the Pod main menu drawn from the scripts (2026-10-15, D319)

Part 46 pushed the main menu state (`PodStateMainMenu`) but answered its Gooey, callout and
other UI natives with defaults, so its `UpdateUI` skipped every frame (`StartFrameNamed` gave
false) and nothing was drawn. Here the runtime answers those natives, records the frame tree the
game's own script builds, makes the engine's per-frame draw call that gives the tree its matrix,
and the viewer draws it over the planets.

### What was found

- FOUND (script listings): `PodStateMainMenu.UpdateUI__Q5Gooey` builds a root frame named 1
  (layout (3, 1, 0.5), sizing `GetPuterWidth()` x 1250 = 2400 x 1250, borders (0, 0, 0, 150),
  transition time (15, 15), blur for transitions), in it a frame of three columns
  (`AddFrameColumn` (-0.5, 1), (0, 3), (-0.5, 0); sizing (-1, 500)): the left column's frame
  (layout (1, 3), borders 32) holds the buttons Story (id 6), Community (9), Recent Activity (10)
  and Store (11) split by `DoBreak`; the middle one (borders (64, 128, 64, 0)) the player's card
  (`DoPlayerBigIcon`); the right one (layout (0, 3)) Me (12). Then a break and the friends frame
  (1000 x 300, empty with no friends). `EnsureNodeOrDescendantHasFocus(6)` gives Story the focus.
  Each button's last float is 0.2 / 0.075 / -0.075 / -0.2 / 0.
- FOUND (script listings): the state's `Draw__Q5Gooeym(m)` calls
  `PodComputerState.SetupFrameMatrix(gooey, m, 1, 2.8, (0, -50, 800))`, which makes
  `GetWorld().GetWorldPosition() * inverse(PlanetsView(1).GetTransform((0, -300, 900), 0)) * m *
  scale(2.8) * translate(-w/2, -h/2 - 50, 800)` (w, h the root's `GetFrameSize`) and hands it to
  `SetItemViewMatrix(1, ...)`. (0, -300, 900) is the in-use camera's eye (a constant of
  PodComputerState). So the menu hangs in the planets world: at rest the world's matrix equals
  the main menu view's and the menu stands before the camera; during the zoom it moves and
  shrinks with the planets.
- FOUND (01.33): the pod computer object's draw function (0x404598) builds a constant matrix
  (0x404070, VMX code read with a separate disassembler: `B Rx(-1.24791) Rz(0.00706858)
  T(0, -100, 550)` with B's columns (1.08496, 0.010962, 0), (0, 0, -1.08502), (-0.010962, 1.08496,
  0), (0, -159.982, 168.926)) and passes it to a script call taking one matrix
  (0x89470c -> 0x5038a4 -> 0x6585e8), then makes the video call (part 46). INFERRED: that call is
  `PodComputer.Draw__m`, the class's only export with one matrix; it runs every state's `OnDraw`.
  MEASURED: with it the 2400 x 1250 root projects onto the 1280 x 720 picture from (-3, -1) to
  (1275, 661), 0.532 px a unit, at depth 3765: the menu is a screen-filling plane.
- FOUND (01.33, 0x62ad64 -> 0x5f0c20): `Math.inverse__m` copies out its callee's matrix;
  INFERRED a full 4 x 4 inverse.
- MEASURED (`local-analysis/d316-zoom/rt20_*`; the final target's alpha is the menu's coverage):
  the menu is there from the first capture after the push: tick 606 (alpha at most 82/255), 609
  (152), 611 (186), 616 and 619 (255). So it appears with the push on tick 601 and fades in over
  its 15-frame transition, while it zooms in with the planets. (`rt8980_up` is the scene before
  the menu pass; `rt20_8980_up` has the full menu.) Taken through the runtime's matrices at 616
  and 619, the game's pills land on the same root-frame units within 1 unit (Recent Activity
  (795.4, 614.6) / (795.8, 615.5), Store (923.1, 720.5) / (923.6, 721.5), Me (1445.3, 563.2) /
  (1446.0, 564.0)): the chain above is the game's.
- MEASURED (game frame, root-frame units): the last float is the pill's turn, clockwise on the
  picture (13.0 / 5.5 / -4.2 / -9.7 / -0.1 degrees measured against 11.5 / 4.3 / -4.3 / -11.5 / 0);
  the left column's pills end at one right edge (1020), 64 units from the card, Me starts 61
  units right of it; the rows are 103.6 apart; the focused pill is 1.56x the others; the
  unfocused pills cover 82/255 in the player's primary colour, the focused one is opaque
  (31-51, 204-211, 200-204), the third colour of `PlayerColour` entry 0 (29, 220, 205).

### What the runtime does

`lbp-script-vm`: a native the table does not bind goes to the new `World::ui_call` when its
class is in `UI_CLASSES` (Gooey, Callout) or it is in `UI_NATIVES` (24 others the step and menu
ask); a world that declines leaves it unbound as before. `Math.inverse__m` is bound (262
natives).

`lbp-runtime` (`gooey.rs`): `GooeyState` on the level world records the frame tree (frames with
layout, sizing, borders, spacing, columns, transition; buttons with id, label, style, state and
angle; breaks; the player card), the focus, the UID high bits and the item matrices.
INFERRED answers: `StartFrame*` true, buttons and `EndFrameInput` 0 (nothing pressed),
`NeedsUpdate` false, `GetFrameSize` the frame's own sizing, callouts none (null, never valid),
`GetPodComputerUserPlayerNumber` the primary player, offline / no friends / no DLC / no crossplay,
`HasPlayedIntroLevels` true (the game frame's Story is there). `LevelScripts::pod_computer_draw`
calls `Draw__m` with the FOUND matrix once a tick in the planets level, before `UpdateVideo__`.
`layout` places a recorded frame in root units: rows split by breaks, columns by
`AddFrameColumn`, alignment codes 0 start / 1 end / 3 centre (INFERRED from the menu: the root's
(3, 1) puts its rows at the bottom, the left column's (1, 3) its pills at the right), sizes from
a text-width model and metrics fitted to the game frame (`BUTTON_EM` 56.3, `BUTTON_PAD` 58.9,
`BUTTON_ROW` 103.6, `PILL_HEIGHT` 82, `FOCUS_SCALE` 1.56, card node 167 x 472 with the 234 x 291
card at its top, and `ROOT_ROWS_DOWN` 14.5, the one offset the rules do not explain).
`level_scripts` reports the tree, the layout and the matrix of every tick (`gooey`).

### What the viewer does

In the scripted Pod (`--level-scripts`) `level_play` hands the drawn menu to
`pod_menu::paint_script_menu` after each tick, which draws every item at the place the item
matrix, the level's display matrix and the camera give it, sized and turned by the same
projection, faded in linearly over the frame's transition time (INFERRED; the game also blurs
it, not drawn). Pills: the player's primary colour at 82/255, the focused one (29, 220, 205),
white bold text in the OS sans serif (not the game's font). The card is a PLACEHOLDER: the
game's white frame colour, a flat picture (the mean colour of the game's picture of the player)
and three round icons in the game frame's colours.

### Measured (viewer stills, hidden window, level lighting, in use at 600)

Centres and sizes in px (1280 x 720); game `rt20_9601_up` (tick 619), viewer `d320-d319c-tick619`
measured against D320's still of the same tick without the menu (Community and Recent Activity
touch there: the projected layout is given).

| item | game 619 | viewer 619 |
|---|---|---|
| Story | (501.6, 223.4) 155 x 65, +13.0 deg | (497.4, 222.0) 155 x 66, +11.4 deg |
| Community | (457.2, 276.2) 177 x 41 | (458.2, 275.4) 180 x 42 (projected) |
| Recent Activity | (433.0, 328.1) 233 x 42 | (431.9, 328.5) 233 x 42 (projected) |
| Store | (498.6, 382.2) 99 x 42, -9.7 deg | (498.0, 380.5) 100 x 43, -11.7 deg |
| card | (640.4, 288.2) box 125 x 152 | (639.6, 287.5) box 120 x 148 |
| Me | (765.6, 301.0) 69 x 43 | (766.5, 300.5) 70 x 42 |

At 616 (`rt20_8980_up`): Store (560.5, 385.7) 66 x 29 against (560.4, 384.5) 67 x 29; Me
(739.0, 331.5) 47 x 28 against (740.0, 331.5) 47 x 28. Stills at 606 / 609 / 611 / 625 / 640 in
`local-analysis/rscript/stage6/shots/d320-d319c-*`; scripts in `local-analysis/d319-menu/`.

### Left out

- Gooey's own layout and widget code (the alignment codes, the borders' order, the text metrics,
  the card's node are fitted, not read); the game's font; the player's picture; the blur of the
  transition; the friends frame's contents; button states other than focused (6 is passed for
  the others offline, its look not read); input (moving the focus, choosing an entry) and the
  planets' point once an entry is selected (part 69).
- The callouts (hint bubbles) are never made.

### Verification

`lbp-script-vm`: `d319_ui_natives_reach_the_world_and_inverse_inverts`; the table count asserts
are 262. `lbp-runtime`: `d319_the_draw_matrix_puts_the_root_frame_before_the_in_use_camera`,
`d319_the_gooey_calls_build_the_menu_and_its_matrix`,
`d319_the_layout_puts_the_columns_about_the_card`. Viewer:
`d319_menu_points_project_through_the_menu_matrix`. `level_scripts` session (pod.bin with
pod_planets.bin, play 700, in use at 600): unbound natives in the planets level 5,783 calls (28
kinds) -> 0, faults 0, frames built 9, draws 700, matrices from tick 601. fmt, clippy (-D
warnings) on the three crates and their tests pass, except `sackboy_play::tests::input_script_steps`,
which fails on main locally too.

Counts: +0 newly viewable / +0 newly Complete.
