# Level scripts, part 1 (runtime plan item Rt)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Builds on the
script VM (`docs/script-vm.md`; the game's own test script passes 512 of 513 checks at
D145) and the native bindings (`docs/script-natives.md`). No script bytecode, listing,
level bytes or engine code was used or is committed.

## What it adds

- **`lbp-level`**: the PScript part now keeps its data (`PartData::script`): the script
  resource and, when the instance has a layout, one `ScriptField` per serialized layout
  field (name, machine type, value). Values: bool, char, s32, f32, v4, m44, Thing
  reference (Thing index), object reference (kind, object number and, on its first
  occurrence, the contents; part 5), and `NotStored` for a DIVERGENT field the instance does not
  serialize. The read order is unchanged (the module doc's SCRIPT rule). A PScript shared
  by back-reference is not repeated.
- **New crate `lbp-runtime`** (runtime only; the parsing crates do not depend on it):
  - `LevelWorld` implements the VM `World` from a read level: Thing handles are Thing
    indices; parent, first child and next sibling (children in Thing-list order); the
    world Thing (the one with PWorld); UID; world position = translation column of the
    POS matrix; `has_part` maps `PSpriteLight` to the reader's `SPRITE_LIGHT`; script
    members (`GET/SET_SP_MEMBER` on a Thing) start from the stored PScript fields;
    `WRITE` lines are kept.
  - `instances()`: every Thing with a PScript resource GUID, with its class when a
    script with that GUID is loaded.
  - `Runner`: starts a named function on an instance with the Thing as `this` in
    register 0, keeps scripts that yield, and resumes them once per `tick()`.
- **Example `level_scripts`** (PC): scripted Things per level, class resolution by GUID,
  stored field names against the class layout, function names over the classes in use,
  and optionally a run of named entry functions on one level.

## Evidence

CONFIRMED_FROM_COMMUNITY_CODE: the PScript / InstanceLayout / ScriptObjectUID layout
(Toolkit d6a2ef80, already recorded in the `parts.rs` module doc).

INFERRED (each is a place the PC run or RPCS3 can disagree):

| Rule | Choice | How it gets settled |
|---|---|---|
| Script members | A Thing's script fields are its `SP` members, by field name | `level_scripts`: stored names vs class field names (`fields_missing_in_class`) |
| `this` | The Thing, in register 0 | Faults when the first entry runs |
| Thing handle | Thing index in the level's list | Internal choice; never shown to scripts except through natives |
| Children order | Thing-list order | Scripts that walk children (`GetFirstChild`/`GetNextSibling`) |
| World position | POS world matrix translation, w as stored | RPCS3 comparison |
| Object references | Strings become string values; arrays and instances become VM heap objects (part 5); resources and audio handles are null and counted as `unconverted_values` | The PC count says whether they matter |

Entry names (settled, PC D152 `level_scripts --all`, unit: distinct class, 49 classes in
use over 378 levels): `OnCreate__` 24 classes, `Update__` 11. The other frequent names are
constructors (`.ctor__`, `.init__`), UI (`UpdateUI__`, `Render__`), switch reactions and
enter/leave handlers, which the matching engine events will call later.

## Verification

Cloud (Linux): `cargo fmt --all --check` clean;
`cargo clippy -q -p lbp-runtime -p lbp-script-vm -p lbp-level --all-targets -- -D warnings`
clean; `cargo test -q -p lbp-runtime -p lbp-level -p lbp-script-vm -p lbp-ui-script` all
pass. New tests:
- `lbp-level` `script_part_keeps_field_names_and_values`: an inline layout of four fields
  (bool, f32, a divergent s32, a wide-string object), with and without compressed
  integers.
- `lbp-runtime`: the Thing tree, UID, position, parts and members of a three-Thing level;
  a synthetic `Lamp.Tick` that increments a member, yields, and finishes on the next
  tick; missing function and missing class outcomes.

CI now also tests `lbp-runtime`.

Not tested: any real level or game script (PC only).

## PC checks

1. `cargo run -q -p lbp-runtime --example level_scripts -- <snapshot> <game dirs incl. the
   d133 edat overlay> --all local-analysis/<new>.json`. Report the summary line, the top
   `instances_by_class`, `unresolved_by_guid`, and the top 30 `function_names`.
2. Then, on one level with scripted Things (the empty test level, or the first story
   level), run with `--entry=<name>` for the likely create and update names from step 1,
   for example `--entry=OnCreate --entry=OnUpdate --ticks=30`. Report the printed run
   lines, the first faults and the top unbound natives.

Counts: +0 newly viewable, +0 newly Complete (no asset change).
