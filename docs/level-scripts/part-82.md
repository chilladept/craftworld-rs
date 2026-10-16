# Level scripts, part 82: Story turns Earth to The Cosmos (2026-10-16, D331)

Part 81 drew the Story frame's title and found that the game's Story view settles on the
default group's world, not on Earth with 82 badges. This part loads that world (The Cosmos
on the owner's profile), pastes its Earth onto the Pod's Earth while Story is up, turns
Earth so the group faces the camera as in the game, and records why the view does not zoom.

### What was found

- FOUND: The Cosmos is story group (4, 27). Its planet decorations are a level, not a plan
  (g102473 = 0x19049): Thing 1 is Earth (mesh g10173) with a `MATERIAL_OVERRIDE` (body ->
  gmat 105200 blue rock, continent regions -> gmat 108277), child Thing 3 a decoration mesh
  (g144897), Things 5 and 6 a moon, and its `LEVEL_SETTINGS` names backdrop plan 00019c73.
  The Pod level (00018fdb) has the same Earth (Thing 1) and moon (Thing 4). The game's swap
  (0x42324c) clears Earth's world and pastes the level in.
- MEASURED (RPCS3, the owner's profile): the Story view opens at tick 640;
  `want_search_details_calls` goes above 0 there and `SetSearchResults` is never called.
- FOUND (the turn): while Story is up, Earth's planet point is the default result's slot
  `location` times Earth's radius (1750), so the group's badge faces the camera.
- FOUND (no zoom in Story): `SearchResults.GetWantZoom__` (0x6057bc) reads results +0x5d,
  cleared only by the results constructor (0x456a98) and set only by a group search
  (0x44dfc0, reached from `CreateGroup__Q6SlotIDb` 0x44e774, also 0x706e00 and 0x70b030)
  for SlotID types 3, 4, 8 and 9. Story's results come from 0x449ce8, so it stays false and
  the camera stays on view 6. View 7 differs from 6 only in Offset.z (-2000 vs -3750) and
  would draw Earth about 1.8 times the game frame's size. `SearchDetailsGroup.GetExtraPlanetZoom__`
  is 500 for types 4 and 9; StoryLevels' is 0.

### What changed

- Level: `PartData.material_override` keeps PMaterialOverride; level scenes apply it as a
  per-Thing mesh variant (a region's gmat swapped; its uvScale is not applied).
- Preview: `LevelScene.parents` (each Thing's parent); the decoration items are not skinned
  by their Things' bones (Earth's rocks had collapsed to its centre); the material probe
  loads as the scene does and prints the diffuse verdict. Example `story_group_plan_probe`.
- Runtime: `STORY_DEFAULT_GROUP` = (4, 27) is the default result;
  `LevelWorld::story_open` and `LevelWorld::story_planet_point` (set before the planets'
  entries each tick).
- Viewer: `story_world.rs` loads the default group's decorations level and pastes Earth's
  subtree onto Pod Thing 1 as alternate items (58 items, 8 meshes), matrix podEarth x
  inverse(decoEarth) of the first Earth-mesh model items. `follow_story_world` shows them
  and hides the Pod's Earth and moon while Story is up. The level's moon is left out (it
  landed in front of the camera).

### Viewer run (part 80's flags, `--level-scripts-hold-at 760`)

The viewer's frame matches the game's settled Story frame in layout: the machine on top,
The Cosmos facing the camera, Earth the same size. Captures stay in ignored
`local-analysis/d331-story-zoom/`.

### Left out

- The backdrop: the game's Story frame is dark space with coloured dots (plan 00019c73,
  dropped by the merge); the viewer keeps the Pod's room and lighting.
- gmat 108277 (the copper continents) draws dark red; the group's badge card and bubble.

### Verification

`d331_story_turns_earth_to_the_default_group` (lbp-runtime),
`d331_story_shows_the_pasted_world_and_hides_the_pods_planets` (viewer), the `d331_*`
material override and skinning tests (lbp-level, lbp-preview). `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; workspace tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
