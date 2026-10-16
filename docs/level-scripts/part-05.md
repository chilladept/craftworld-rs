# Level scripts, part 5: object-reference members (2026-10-09)

The Pod planets run showed PlanetsView reading about 124 object-reference members the
VM did not model. Stored script objects now reach the VM:

- **`lbp-level`**: `ScriptValue::Object { kind, id, body }`. `id` numbers the level's
  script objects in first-occurrence order (the per-class sequence of the shared
  reference table), so a back-reference carries the `id` of its first occurrence; only
  the first occurrence has a `body` (`ScriptObject`): bool / s32 / f32 / v4 arrays, Thing
  arrays (Thing indices), object arrays (nested values), a resource (type and
  descriptor), a nested instance (`ScriptPart`), text (STRINGW and STRINGA), audio handle.
  The read order is the module doc's ScriptObjectUID rule, unchanged.
- **`lbp-script-vm`**: `Vm::new_array`, `Vm::array_mut`, `Vm::fields_mut` for hosts that
  load stored objects.
- **`lbp-runtime`**: `LevelWorld::from_things` collects every object body by number
  (members, array elements, instance fields). `LevelWorld::bind_objects(&mut vm)`
  allocates one heap object per number (so shared references stay one object, cycles
  included), fills object arrays and instance fields, then points members at them. An
  instance gets the class with its script GUID, class defaults first, stored fields on
  top. Strings stay string values. Resources, audio handles and instances of classes the
  VM has not loaded stay null and are counted in `unconverted`. Returns the number of
  heap objects built; `level_scripts` reports it as `objects_built`.

| Rule | Evidence | Applicability |
| --- | --- | --- |
| Object numbers follow the shared tag table (a back-reference is the same object) | INFERRED from the ScriptObjectUID layout (module doc of `parts.rs`) | Instances in levels read by `lbp-level` |
| An instance object starts from class defaults, stored fields override | INFERRED, the same rule as a scripted Thing's members (part 2) | Classes the VM has loaded |
| Resource objects are null | Not modelled: the VM has no resource value yet | Counted, so the PC count shows whether it matters |

Tests: `lbp-level` `script_objects_keep_contents_and_shared_identity` (string, f32
array, object array with a back-reference, a null and a nested instance);
`lbp-runtime` `stored_objects_become_shared_heap_objects`.

Not tested: real levels (cloud has no game data). PC check: `level_scripts` on
pod_planets (00018fdb) with `--play`, reporting `unconverted_values`, `objects_built`
and the PlanetsView faults.

Counts: +0 newly viewable, +0 newly Complete (no asset change).
