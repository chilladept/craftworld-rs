# Level scripts, part 60: the glow alpha's fog form read from the material's program (2026-10-15, D309)

Viewer and parsing change, no runtime change. Part 58 split glowing materials by the way their
compiled program writes the alpha under fog, but nothing in the stored material told the forms
apart, so the viewer carried a list of 23 GUIDs. This part reads the form from each material's
own program.

### What was found (the game data)

- CONFIRMED_FROM_LOCAL_DATA: the compiled fragment programs are in the gmat's shader blob the
  viewer already reads (ten `CGB` records and their end offsets). In each record the u16 at 0x30
  is the size of the parameter block; the instruction list starts at the next 16-byte boundary
  as u16 pairs naming two 8-byte entries of a table shared by all ten programs (the blob after
  the last end offset). Instruction words follow the NV40 fragment encoding as Mesa's nvfx
  driver documents it publicly.
- The rule: the last write of the output alpha (`H0.w`) in variant 8, traced back. The glow sum
  `s` is an ADD of a DP3 and a constant; `MUL(ADD(RCP s, MAD(-RCP s, f, f)), const)` is the
  fogged form, `DIV(x, s)` or `RCP s` alone the unfogged one; anything else (flow control,
  conditional writes, writes inside an IF block) is unknown and takes the common, fogged form.
  INFERRED from the four programs with IF blocks (`burn`, `pod_card`, `pod_card_edge`,
  `window_glassy`): the IF's end word counts 32-bit words.

### What the viewer does

`lbp_resource::material::glow_alpha_form` classifies the program at load; the preview's surface
carries it and the viewer fogs the glow alpha unless the form is unfogged. The GUID list
(`dual::GLOW_ALPHA_UNFOGGED`) is gone.

### Measured

Census over the catalog's 8,386 gmats (`glow_alpha_census`, output in ignored
`local-analysis/d309-glow-alpha/`), none failing to read: 23 unfogged, exactly part 58's list;
556 fogged, part 58's 524 plus 32 not in its corpus (`air_pump_light_on`, `vin_candle`,
`proc_factory_neon` among them); 7,807 unknown (not glowing in this form). Since the 23 are the
same and every other material keeps the common form, the drawn picture is unchanged; no viewer
capture was taken.

### Verification

`lbp-resource`: `d309_fog_mix_of_the_glow_reciprocal_is_fogged`,
`d309_division_or_bare_reciprocal_is_unfogged`, `d309_other_programs_and_bad_blobs_are_unknown`,
`d309_conditional_or_flow_control_is_unknown`, `d309_writes_inside_an_if_block_are_conditional`
(synthetic instruction words, no game bytes); the viewer's `d307_glow_alpha_under_fog` checks the
three forms. fmt, clippy on `lbp-resource`, `lbp-preview` and `lbp-bevy-viewer` and `cargo check
--workspace --all-targets` pass; the viewer's tests pass (144) but the one that fails on main
(`sackboy_play::tests::input_script_steps`).

Counts: +0 newly viewable / +0 newly Complete (viewer fidelity).
