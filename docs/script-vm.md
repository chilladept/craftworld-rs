# Script VM (runtime plan item Rq)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Phase RS of
`docs/runtime-research.md`: the user chose (2026-10-08) to interpret their own compiled
scripts. This item is the VM core in the new runtime crate `lbp-script-vm`, built on the
U10 reader (`lbp_level::script`, D126). It runs synthetic bytecode only: the 427 scripts
on 01.33 sit in the encrypted `patch.edat` and are not readable yet.

## What it does

- `Vm::load(script, guid)` registers a class; `Vm::call(host, class, name, args, budget)`
  runs a function. `args` fill the function's argument rows in order.
- All 203 opcodes have a defined action or an explicit fault. `MOVs`, `EQs` and `NEs` are
  Nop-class in Toolkit's table and do nothing.
- Script-to-script calls (`CALL`, virtual `CALLVo` dispatching on the receiver's class and
  its superclasses), script objects and arrays on a VM heap, v4/m44 math, casts.
  D274: the virtual call on a Thing (`CALLVsp`) dispatches on the class of the script the
  Thing carries (`Host::thing_script` gives its GUID), when that class is or derives from
  the class the caller names; a null Thing or an unknown script is called as named
  (`docs/level-scripts.md` part 25).
- Everything engine-side goes to a `Host`: native functions by (class name, function
  name), Thing script members (`GET/SET_SP_MEMBER`), `ref_ptr` members, native members
  (`GET_SP_NATIVE_MEMBER`), `CASTsp` and `WRITE`.
- Limits, all explicit `Fault`s: instruction budget per call (per tick in the runtime),
  call depth 256, 2^20 heap objects, 2^20 array elements, registers past `stack_size`,
  branches leaving their function, unknown opcodes, type mismatches, divide by zero,
  null objects, failed `ASSERT`.
- `check::check(script)` counts opcode use, functions without a terminal instruction, and
  branch targets under both branch rules (below), for the PC census.

## Evidence

CONFIRMED_FROM_COMMUNITY_CODE (Toolkit d6a2ef80, via the U10 reader): instruction classes,
operand bit fields, function/argument/field/function-reference tables, machine types,
modifier bits (`native` = bit 1).

INFERRED (no executable or Toolkit statement; each is a place a real script can disagree):

| Rule | Choice | How it gets settled |
|---|---|---|
| Register model | One slot per 16-bit address in a frame of `stack_size` slots; values never span slots | Real scripts: no register read before write at another width (VM faults on type mismatch) |
| Arguments | `ARG` stages at the argument address; `CALL` copies staged values to the same addresses in the callee frame; host calls fill `shared_arguments` offsets in order | Argument rows' offsets vs `ARG` addresses in listings |
| Receiver | Member functions take the receiver as an argument; virtual dispatch uses the lowest staged address | Listings: argument row typed as the own class |
| Branch target | `pc + offset` (`Rules::branch_from_current`), alternative `pc + 1 + offset` | Settled by D133: all 15,866 branches in the 426 scripts land in range under `pc + offset` (15,399 under the alternative) |
| `LCv4` | Four consecutive f32 constants from `index` | Constant-table sizes vs `LCv4` indices |
| m44 layout | Column-major like the engine (element (row, col) at col * 4 + row, translation at 12..14); `MULm44` A * B is the engine's product (out[col*4+row] = sum a[k*4+row] * b[col*4+k]). In the `M44` instructions the first letter is the row and the second the column: `GET_M44_XW` reads row X of column W, index 3 * 4 + 0 = 12; `GET_M44_cW` reads column W, floats 12..15 (the translation); `GET_M44_rX` reads every fourth float from 0. Until D262 the VM indexed row * 4 + col, the transpose | CONFIRMED_FROM_LOCAL_DATA (emulated): the Pod World matrix read in RPCS3 is the transpose of the earlier row-major product (Pod thread, D180 trace, 2026-10-09). FOUND in the script tables (names and use only, D262): level scripts take the position of a world matrix as column W (115 uses) and the game's test script reads row X as XX, XY, XZ, XW; the test script still passes 512 of 513 checks. Tests `m44_product_is_column_major_a_times_b`, `m44_elements_rows_and_columns_follow_the_engine_layout` |
| Comparisons and casts | C semantics: wrapping int arithmetic, truncating division, `f as i32` | Script behaviour vs RPCS3 |
| `ARRAY_FIND` | Index of the first equal element or -1 | Script behaviour |
| Function lookup | By name through the superclass chain; first definition wins and repeats are counted (`ambiguous_calls`) | Census of duplicate names per class |
| Strings | `LCsa`/`LCsw` produce string values (MachineType has no string type) | Listings |
| No register | Operand `0xffff` = no register: reads give Void, writes are dropped, a typed `CALL` with that dest discards its result | D138 listings (`B` source, discarded `CALL` results); Register 65535 faults in the test script and the Gooey trace |
| Locals | Declared locals (the function's local-variable rows) start at their type's default; other registers start Void | D138 test script: `CheckEqual` got Void in TestLocals and the vector tests |
| Defaults | Every machine type defaults to zeroed storage, m44 included (not identity) | D144 TestMatrix44 662: the product of two never-written m44 locals must have a zero W row |
| `LCb` | Bit 63 (Toolkit), switchable by `Rules::lcb` (`Bit63`, `AnyHighBit`, `Bit32`) | Bit 63 calls NeverEvaluated in TestBool (D139); any high bit breaks 20 TestLogical checks (D140); bit 32 untested. PC compares with `script_tests --lcb=` |
| Construction | `NEW_OBJECT` allocates the class's and its superclasses' fields, then runs `.init__` (found through the superclass chain) with `this` in its single argument row, else register 0. `.ctor__` is called by the script itself | D139 listings: `.init__` calls the superclass `.init__` and sets the members, `.ctor__` only chains; checks read members only `.init__` sets |
| Superclass by GUID | Falls back to the class name that any loaded script's type references give that GUID | Scripts loaded from a directory have no GUID; Test's superclass is named only by GUID |
| `ARRAY_INSERT` index | Clamped to [0, len]: -1 inserts at the front, past the end appends | D145 TestArray: three appends, then inserts at 1, -1 and 100; the length and first two element checks fit only this rule (appending at -1 or at 100 alone broke them in D140/D144) |
| Array element type | Array instructions carry the element machine type; a Void element read, value written or resize fill becomes that type's default | D146 TestArray: an int element read back as Void faulted the following `EQi` (an empty array resized had no element to copy a zero from) |
| `LCs64` constants | Compressed long vectors hold zigzag values, `(n >> 1) ^ -(n & 1)`, decoded by the `lbp-level` reader; uncompressed u64 is two's complement | D143: with zigzag, TestS64 1551 (`1 << 63` = i64::MIN) and 1562 (AND of two constants) pass; without it no decode makes 1562 pass |
| Script threads | `Thread.StartImpl__(this)` is handled by the VM: it records the thread object (`Vm::take_started_threads`). The level schedule runs `ThreadMain__` on it at the end of the step that started it, until its first `System.Yield`, then resumes it once per tick after the Things' suspended scripts and before the updates | PC D167 listings: `Start__` asserts `!Running`, sets it and passes only `this`; `ThreadMain__` runs OnStart/Run/OnFinish and clears `Running`; waits are script loops over `Yield__`. Start timing and order INFERRED; RPCS3 (Pod intro) can settle them |

UNKNOWN: native function semantics (they live in the executable). Each one gets a host
binding from recorded findings, in call-count order (items Rr/Rs).

`script_tests` takes `--no-init` (instances without `.init__`) and `--lcb=bit63|any|bit32` so one PC run can compare the inferred rules. Test checks are counted from
the script framework's own counters (`TestBase` `.get.NumSucceeded__` / `.get.NumFailed__`;
`Check*` are TestBase script methods, not natives, D140) plus any `Test.*` native calls,
and `WRITE` lines (failed-check details) are kept per function. Its JSON also records each
test class's superclass chain, every native call that reached the host, and instructions per function.

Open after D145 (1 failed check of 480):
- TestLocals 417: a local declared inside a loop body without an initialiser is read
  before it is written, and the check expects 0 on every iteration. The source lines of
  the declaration emit no instructions and nothing resets the register, so the VM reads
  the previous iteration's value. The same function's second loop initialises its body
  local explicitly and passes. Re-zeroing locals on the back-branch would also reset the
  loop counter. Kept as is: the check may fail on hardware too (an RPCS3 run of the test
  script would settle it).

`Vm::last_site()` names the class, function and relative pc of the last executed
instruction, which after a fault is the faulting one. `script_tests` prints it with each fault.

## Verification

Cloud (Linux, Rust 1.97.0; CI pins 1.99.0): `cargo fmt --all --check` clean;
`cargo clippy -q -p lbp-script-vm -p lbp-level --all-targets -- -D warnings` clean;
`cargo test -q -p lbp-script-vm` 5 tests:
- a loop under both branch rules, plus budget exhaustion; the branch check tells the
  two encodings apart;
- script call, native call through the host, `WRITE`, wrong argument count;
- object fields, arrays (append, length, find), v4 cross product and component read,
  Thing member get and set through the host;
- virtual dispatch to a subclass override through an inherited caller, on a script object
  and (D274) on a Thing by the script it carries;
- faults: unknown opcode, register range, divide by zero, falling off the end, branch out
  of range, type mismatch, bad field reference, failed assert, call depth.

CI now also runs `lbp-level` (the reader this crate depends on) and `lbp-script-vm`.

Not tested: any real script (none readable yet), any native function.

## PC checks (once `patch.edat` is decrypted)

1. Run `check()` over every script (extend the U10 `script_census` example): branch
   totals per rule, unknown opcodes, functions without a terminal. The branch rule with
   every target in range becomes the default.
2. From listings: do member functions have a first argument row typed as their own class?
   Do `ARG` addresses equal the callee's argument offsets?
3. Native-call census (G10) ranks the host bindings to write first.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Next

Rs started in `docs/script-natives.md` (first bindings, `System.Yield` coroutines);
Rt: PScript instance data as Thing members and the create/update entry points.
