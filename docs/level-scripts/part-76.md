# Level scripts, part 76: the Pod main menu answers the pad (2026-10-16, D325)

Part 75 left the menu still: its focus stayed on Story and no entry could be chosen or left.
Here the runtime gives the menu's scripts the pad's focus moves, choose and back, and the menu
is built again on the tick the input comes, as the game reacts at once.

### What was found

- FOUND (the 01.33 listings, `PodStateMainMenu.UpdateUI`): a button's answer
  (`DoSpaceButtonNamed(id, label, style, state, mask, angle)`) is tested only for 256, and the
  script passes 256 as the mask only for an open entry (progression flags 4 / 8 / 16 / 32);
  the root frame's `EndFrameInput(512)` is tested only for 512, which pops the state. Each
  entry pushes a state of its own (Story `PodStateSearch` with `CreateStoryLevels`, Community
  `PodStateCommunityMenu`, Recent Activity `PodStateStream`, Store `PodStateStoreMenu`, Me
  `PodStateMeMenu`). The scripts never move the focus themselves (no SetFocus or MoveFocus in
  the menu's code): up and down are the engine's.
- FOUND (`PodComputer.PerFrameUpdate__b` and `NeedUpdateUI__`): the menu is built only when
  `NeedUpdateUI` is true, the or of the computer's own `NeedUpdate`, the planets'
  `CheckOverlayNeedsUpdate`, each active state's `NeedUpdateUI` (a counter `SetNeedsUpdate`
  sets to 2) and `Gooey.NeedsUpdate`. With the Gooey answering false the session builds the
  menu at ticks 601-603 and then only in pairs every ~31 ticks (633-634, 664-665, 695-696) (a state asking), so a key acted
  up to a second late. With it answering true while input waits, a key acts on its own tick.
- INFERRED (the engine's Gooey code was not read): the Gooey needs an update on the frame its
  focus moves or a choose or back waits; the focus moves to the nearest button within 60
  degrees of the direction (distance along it plus twice the distance across); a newly focused
  pill's spring (part 75) starts again from 1 at the move.

### What changed

- Runtime (`gooey.rs`): `GooeyInput { choose, back }`, `GooeyState::move_focus`,
  `focus_since`, `focus_moved`, `build_frames`. The focused button answers 256 once when chosen
  and its mask has 256; a root `EndFrameInput` whose mask has 512 answers 512 once on back; a
  choose or back no frame took lapses with the build. `NeedsUpdate` answers true while the
  focus moved since the last build or a choose or back waits.
- `level_scripts --menu=TICK:KEY,..` (up, down, left, right, choose, back).
- Viewer (`--level-scripts` with the Pod shell): while the pod computer is in use and the menu
  is drawn, the arrows and the D-pad move the focus, Backspace / Escape / Circle is the menu's
  back (the script then closes the pod computer), and Enter / Cross chooses under
  `--pod-menu-choose` only, since Story's state still faults. The focused pill's spring, slab
  and slab step go by the frames since the focus last moved.

### Session (pod.bin with pod_planets.bin, play 720, in use at 600)

| Input | Builds after the entrance | Result |
| --- | --- | --- |
| none | 633-634, 664-665, 695-696 | focus Story (6) |
| down 640, down 650, back 700 | 633-634, 640, 650, 664-665, 695-696, 700 | focus Community, then Recent Activity (10); at 700 the state pops, `RequestHostUsePod(false)`, in use false at 701, the camera goes back (rests at 716) |
| choose on Story at 640 | 633-634, then the build at 640 | Story answers 256 and `PodStateSearch` is pushed; its `OnActivateImpl` faults at +31 and its `Draw` at +3 on a null object (the SearchDetails: `SearchDetails.CreateStoryLevels__` and the Planets natives are not bound), so the pod computer's calls stop and nothing is drawn from 640 on |

### Left out

- Story's search state (the SearchDetails, `Planets.SelectPlanet`, `Planets.SetCursorMode`,
  `PodComputerPlanets.SetWantSearchDetails` and `Gooey.InvalidateFrame` natives): the next part.
- Which pad button gives choose and back in the game (the viewer's keys are its own); the
  engine's navigation rule; the planets turning to the focused entry (native, not script).
- The viewer's keys were not driven in a window here (no focus taken); the runtime session above
  is the check.

### Verification

Runtime: `d325_input_asks_for_a_build_and_answers_it_once`. `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; the workspace tests pass but
`sackboy_play::tests::input_script_steps`, which fails on main locally too. Nothing in RPCS3 was
started. Session files stay in ignored `local-analysis/d325-menu-input/`.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer input).
