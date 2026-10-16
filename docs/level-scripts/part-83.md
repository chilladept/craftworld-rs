# Level scripts, part 83: the Story backdrop and its lighting (2026-10-16, D336)

Part 82 pasted The Cosmos's Earth onto the Pod's Earth while Story is up, but left the
Pod's bright room behind it. The game's Story frame is dark space with blurred coloured
dots. This part reads how the game swaps the backdrop and its lighting and does the same.

### What was found

- FOUND (01.33): after pasting the group's Earth (0x43cd64 -> 0x436b50 -> 0x42324c per
  planet), the swap calls two world-to-world copies. 0xdb2c4 moves the decorations level's
  backdrop Thing (PWorld +0x870) into the Pod world in place of the Pod's own (the old one is
  destroyed, the hierarchy re-homed, BackdropPlan +0x888 copied; no fade). 0xda074 copies
  the PWorld factors `lighting`, `colourCorrection`, `fog`, `fogTint` and `darkness` (+0x404
  to +0x414, the `Set*Factor` natives' offsets).
- FOUND: 0x42324c touches only the planet Thing (clears its decorations, clones the source
  Earth's parts and children, rebuilds the "earthshadow" child); no backdrop is placed
  around Earth.
- FOUND: the active lighting (0xd9990) is the backdrop Thing's PLevelSettings entry, so the
  Story world is lit by the decorations level's backdrop plan (00019c73): one preset, fog
  colour near black, fog 13660..53278. The Cosmos's factors are lighting 0, fog 0, fog tint
  0.15, darkness 0, so the fog colour is tinted over the Pod fog palette (D332).
- INFERRED: no separate dimming for Story; the darkness is this swap.

### What changed

- Viewer: `story_world::merge` also pastes the decorations level's backdrop items (51 for
  The Cosmos) unmoved onto the Pod's backdrop Thing as alternate items; spawned entities of
  the backdrop Thing carry `story_world::Backdrop`, and `follow_story_world` shows the pasted
  backdrop and hides the Pod's while Story is up.
- The Story's light (`story_world::StoryLight`) is built from the backdrop plan's preset under
  The Cosmos's factors (`at_lighting`, `with_fog_tint` with the Pod palette, `with_fog`), the
  shell's with its own direction (INFERRED, as P2r), and swapped in and out with the view.
  `PreviewLight::own_fog` keeps that light's fog instead of the Pod's live constants.

### Viewer run (part 80's flags, `--level-scripts-hold-at 760`)

With `--lbp-lighting` the frame matches the game's look: the room fades to dark with blurred
coloured dots, the orange planet at the lower left as in the game frame. With the Pod's
usual `--preview-lighting` (no LBP fog) the room shows grey and sharp. Captures stay in
ignored `local-analysis/d332-story-backdrop/`.

### Left out

- The copper continents: gmat 108277 is `proc_victoria_jelly_glassy`, the program family of
  `window_glassy` (D286): its colour is a x6 specular, rim and reflection term added after the
  square root, which the viewer gives only to that one GUID. Next.
- The badge card and its info bubble.

### Verification

`d336_story_swaps_the_pods_backdrop_for_the_pasted_one` (viewer). `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
