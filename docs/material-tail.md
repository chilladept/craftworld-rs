# D10 material tail oracle — 2026-10-05

This pass establishes diagnostic tail layout through a pinned external reader.
No Rust parser or GUI change. Original inputs are read-only; no shader bytes,
animation keys or proprietary fixtures are exported. Output is ignored metadata.

## Provenance, applicability and confidence

Toolkit pin: `d6a2ef80b5a09999f08f23a803b516096055e5c9`. Reader invocation and
independent format evidence only; no copied/adapted/translated library source.
Reference MIT/inherited notices remain in the ignored checkout. Project license
remains undecided. Adapter: scripts/MaterialTailOracle.java; entry/runner:
MaterialOracle.java and run-material-oracle.ps1 `-Tail` (includes graph inspection).

| Rule | Evidence and pinned source | Applicability, conflicts and coverage |
| --- | --- | --- |
| Graph is followed by sound integer, then inline parameter-animation array | CONFIRMED_FROM_COMMUNITY_CODE: [RGfxMaterial::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java), [Revisions](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/Revisions.java) | Sound threshold 0x15b (347); parameter animations 0x2a2 (674). Current contract ordinary branch-zero PS3 heads 920–951 only, flags 0/7. Other revisions/branches excluded. Confirmed on same 32 local samples; sound semantics UNKNOWN. |
| Array has integer count and inline MaterialParameterAnimation records | CONFIRMED_FROM_COMMUNITY_CODE: [Serializer::array(values,class,false)](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/serializer/Serializer.java) | Guard scans all bounded records before isolated existing array-reader invocation. Count max128 is our diagnostic budget, not a format limit. Empty serialization cannot distinguish original null from empty array; report count0, no nullness claim. |
| Record uses base vector4, count plus component-major fixed float bytes, length plus byte name, component mask | CONFIRMED_FROM_COMMUNITY_CODE: [MaterialParameterAnimation::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialParameterAnimation.java) | Same layout as D7 embedded box animations; same count/mask/digest/base-bit/name comparison against isolated reader. Derived keys=count/popcount(mask&15), only when count>0; no playback/timebase interpretation. Max4096 component values and64 name bytes. |
| Sound and counts follow stream compression integer mode; IEEE bits stay fixed-width | CONFIRMED_FROM_COMMUNITY_CODE: [MemoryInputStream::i32/floatarray](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java) | Preserve raw unsigned sound bits including 0xffffffff; no signed zigzag on this field. Authored fixed/compact tests corroborate raw bits, NaN/signed zero, empty arrays, nonempty multi-component arrays and unknown mask bits. Actual samples all compact flag7 and sound0. |
| Known fields consume all residual payload bytes in selected samples | CONFIRMED_FROM_LOCAL_DATA: ignored final oracle/consistency reports | 32/32; 30 tails of2bytes (sound0/count0), 2 tails of989bytes (sound0/count1,241 values/keys). No unknown residual bytes in this sample set. This does not prove complete material interpretation, full-corpus coverage or shader correctness. |
| Other platform branch data follows the tail in the reader | CONFIRMED_FROM_COMMUNITY_CODE: RGfxMaterial::serialize DOUBLE11 UV/performance fields | Excluded by branch contract; no Vita/platform extension inferred. Unknown extra bytes are retained as counts, not silently swallowed or labelled corrupt. |

Confidence is high for these diagnostic fields within the pinned/sample scope.
Sound meaning, animation binding/name semantics, timing/interpolation, shared
parameter behavior and original rendering fidelity remain UNKNOWN. Zero unknown
tail bytes means byte-layout coverage for samples, not behavior compatibility.

## Guard and oracle contract

Stored input max1MiB/resource, total16MiB/run, max48 samples; expanded payload8MiB
from existing wrapper guard. Tail remainder max2MiB; array max128, each animation
max4096 values/64 name bytes. Checked source ranges/hash identity, clear GMTb magic,
branch/revision/compression gate, strict stream boundaries and mask/divisibility
guards precede untrusted reader allocations. Existing array deserialization only
receives an isolated, validated exact slice. No full RGfxMaterial deserialization,
shader execution, service access or decryption.

Thread interruption/cancellation is checked before and between records and before/
after isolated reader comparison. Overall sample loop uses a60second monotonic
deadline; bounded library calls cannot be interrupted internally. A tail failure
records `tail_status=rejected` and error while retaining valid prefix/graph; no
automatic retry. A deadline before a subsequent sample aborts the run without a
partial output. Diagnostic fields: sound_enum_raw, parameter_animation_count,
parameter_animations (base bits, value/key counts, component digest, raw mask/name
hex), tail_start, parsed_tail_bytes, unparsed_bytes. Existing graph/prefix fields
and JSON schema1 remain unchanged; tail-specific scope distinguishes the extension.

No new oracle conflict found. Existing fixed bytes() padding, unguarded array
allocation and component count/mask permissiveness are handled before invocation;
name NUL stripping/default charset is compared only on the observed ASCII names.
Raw diagnostic name bytes remain authoritative. D7 extreme signed zigzag conflict
remains recorded; this tail contains no signed zigzag fields.

## Reproduction and outcomes

```powershell
./scripts/run-material-oracle.ps1 -Tail -Output local-analysis/d10-material-tail-oracle-final.json
& 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' scripts/material_tail_report.py local-analysis/d10-material-tail-oracle-final.json local-analysis/d7-material-graph-oracle-guarded.json local-analysis/d4-material-prefix-oracle-final.json local-analysis/d10-material-tail-consistency.json
./scripts/run-material-oracle.ps1 -Tail -Output local-analysis/d10-material-tail-oracle-repeat.json
Get-FileHash -Algorithm SHA256 -LiteralPath local-analysis/d10-material-tail-oracle-final.json,local-analysis/d10-material-tail-oracle-repeat.json
& 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -m unittest discover -s tests -v
```

Java21 compilation and prefix/graph/tail authored self-tests pass. Fixed and packed
tail recipes cover every truncation, raw unknown sound/mask, base IEEE bits, empty
and nonempty arrays, digest/name comparison, extra opaque tail, count/value/name/
byte limits, invalid/nondivisible mask, revision rejection and mid-record-list
cancellation. Four new Python cases validate consistency, failures with prefix/
graph retention, raw unknowns/trailing bytes, malformed metadata and determinism.
All22 Python tests pass.32/32tail outcomes parsed; all previous prefix and graph
metadata exactly unchanged; two parameter animations total482 values/derived keys,
one mask4 and one mask8. Key arrays are not emitted.

Final and repeated oracle outputs byte-identical SHA256:
`1b11e74371c24a802c9acf6a323c9d110dde3276f2436a77b3343822b61748b3`.
Initial output preceded extra synthetic mask/empty-array guards; final run includes
them. No failed format experiment or mismatched sample. Rust/UI unchanged and
their tests/native capture not repeated in D10. Full corpus, other builds/platforms,
nonzero real sound and animation semantics not tested. Original SFO/MAP hashes,
FARC size/read-only attributes, clean reference pin and ignored/private exclusions
verified. No Git initialization, commits, publishing or approval-review rejection.

## Exact next task — D11 independent optional tail diagnostics

Add a small `lbp-resource::material::inspect_tail` diagnostic API using the documented
layout and existing bounded animation metadata helper. Share one prefix/graph/tail
expansion/walk, preserve existing prefix/graph APIs and schemas. Return raw sound,
array summaries and residual counts; retain unknown values/tails. Keep limits,
strict bounds, cancellation and explicit unsupported outcomes. Preserve names and
IEEE bits; no animation-key output or sound labels without evidence.

Compare all32 canonical tail samples and preceding prefix/graph metadata with the
external oracle. Synthetic fixed/compact/raw/stored/zlib cases, every tail
truncation, invalid count/mask/name/value budgets and mid-list cancellation before
integration. Run required Rust checks and update near-code provenance. Leave GUI
integration for a following small task; no playback, sound simulation, shader
evaluation, texture role guesses, textured renderer or runtime expansion.

## D11 independent optional tail reader complete — 2026-10-05

`lbp-resource::material::inspect_tail` returns `TailInspection` containing flattened
existing prefix/graph metadata plus `TailMetadata`. One bounded payload expansion
and one walk through prefix, graph and tail; graph's internal decode helper retains
the payload only for this optional continuation. Shared animation metadata helper
preserves raw bits/name/mask, counts/digests without returning keys. Limits128array
records,2MiBtail remainder,4096component values/64name bytes. Sound is raw u32;
trailing unknown bytes are counted. Existing inspect/inspect_graph APIs and JSON
schemas unchanged; their unparsed counts retain their original stage meaning.

Near-code provenance: material/tail.rs pinned RGfxMaterial/Revisions/Serializer/
MaterialParameterAnimation links and this evidence contract. This is independent
specification-based implementation; no reference source copied/translated.
No GUI change, sound meaning, animation binding or behavior inferred.

Four authored Rust integration cases verify fixed/compact integers and raw/stored/
zlib payloads; nonempty/empty arrays; unknown sound/masks/name bytes; NaN/signed-zero
base bits and component digest; every tail cut inside a rebuilt valid envelope
while prefix/graph remain readable; malformed count/value/name/mask/ULEB fields;
128-record boundary and exact2MiBremaining-byte boundary/overrun; protected markers,
unsupported revisions and cancellation before/between tail records. Prior graph
tests continue covering unknown nodes/cycles/signed wire extremes. Maximum budgets
are project limits, not claims about the original game's format limits.

Exact commands (outputs new/ignored):

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-resource --test material_tail
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d10-material-tail-oracle-final.json local-analysis/d11-material-tail-parity.json --tail
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d7-material-graph-oracle-guarded.json local-analysis/d11-graph-regression.json --graph
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d4-material-prefix-oracle-final.json local-analysis/d11-prefix-regression.json
cargo fmt --all --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

32/32canonical tail comparisons exact (including all prefix/graph fields);
32/32graph and32/32prefix regression exact; no mismatches/failures.85workspace
tests, fmt, Clippy and native viewer build pass. Complete test log retained in
ignored local-analysis/d11-workspace-tests.log. No failed experiment, new oracle
conflict or sample mismatch. No new native capture/manual GUI or full-corpus/other
build/platform/nonzero real sound/animation-behavior checks. D10 Python/oracle code
unchanged and not rerun in D11. SFO/MAP fingerprints/FARC size and read-only flags,
clean reference source and default-deny exclusions verified; root remains outside
Git, no commits/publishing/approval-review rejection.

## Exact next task — D12 native optional tail diagnostics

Add explicit optional tail inspection to Material tab through existing validated
read worker/cancellation/cache. Retain valid prefix/graph metadata when tail fails.
Display raw sound value, parameter-animation array summaries (base IEEE bits,
name bytes/mask, value/derived-key counts/digests), parsed/residual byte counts and
diagnostic JSON; virtualize the array. Do not label sound values or bind animation
names without evidence. Synthetic failure/cancellation/source-reset/layout checks
plus native QA on one empty and one nonempty array before completion. No animation
keys exported, playback, sound simulation, shader evaluation, texture-role guesses,
textured rendering or runtime/gameplay expansion.

## D12 optional native tail inspection complete — 2026-10-05

Material tab now offers Prefix, Inspect graph metadata and Inspect tail metadata.
Default stays prefix. Tail mode shows raw sound decimal/hex, array counts,
virtualized animation records, selected raw name/mask, base IEEE bits, value/derived
key counts and component digest, parsed/residual lengths and diagnostic JSON.
Sound/animation semantics remain explicitly unknown. Empty arrays do not claim
original nullness. No animation keys, shader data or sound execution.

Validated read-only source worker reads each input once. A successful request uses
one bounded expansion through the requested stage. Tail failure retries graph,
then prefix only as needed; fallback expansions are sequential and bounded with
independent errors. Existing cached prefix/graph survives same-resource deeper
inspection/cancellation; reload or source/selection reset clears caches. Mode
preference survives reset; queued deeper requests are honored once and failed/
cancelled work requires explicit retry. Existing graph slot menus unchanged.

Two new authored native tests cover valid prefix/graph with missing tail, successful
tail metadata, completed cancelled worker results, reset clearing cache/selections,
graph failure plus tail failure with prefix retained, empty/max128 arrays,
unknown sound/mask/name/base bits, invalid selected index and narrow headless layout.
17viewer/87workspace Rust tests, fmt, Clippy and native build pass. No parser or
oracle changes; D11 canonical parity remains evidence and was not repeated.

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-viewer
cargo fmt --all --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

Native QA command pattern (new PNG/logs only under ignored local-analysis):

```powershell
$qaProcess = Start-Process -FilePath target/debug/lbp-viewer.exe -ArgumentList @('--snapshot','local-analysis/a2-inventory-verified.json','--game-dir','game-data/a1-disc','--select-guid','00017049','--qa-tab','material-tail','--qa-capture','local-analysis/d12-tail-populated.png') -WindowStyle Hidden -PassThru -RedirectStandardOutput local-analysis/d12-tail-populated.stdout.log -RedirectStandardError local-analysis/d12-tail-populated.stderr.log
$qaProcess.WaitForExit(30000)
```

Three native captures inspected on RTX3080/Vulkan:

| GUID / material | Revision | Array / display result |
| --- | --- | --- |
| 000134dd garage_car.gmat | 935 | Empty,2parsed tail bytes,0residual; null/empty ambiguity stated |
| 00017049 cosmos_racing_game_top_02.gmat | 949 | One record,241values/derived keys,mask4;989parsed bytes,0residual |
| 0001770b cosmos_arcade_screen_space_02.gmat | 945 | One record,241values/derived keys,mask8;989parsed bytes,0residual |

All raw sound0. Both populated views fit IEEE bits and wrapped full digest within
the pane. Native captures/logs and complete d12-workspace-tests.log stay ignored.
No manual mode switching/queued-request interaction, expanded JSON, changed-source
interaction or other GPU/full-corpus behavior tested. Reset tests exercise the same
cache-clearing path as source/selection changes; do not infer native interaction
coverage from headless tests. No failed experiment or approval rejection. SFO/MAP
hashes unchanged, FARC1,023,990,298bytesReadOnly/Archive, reference clean/root no Git;
authored/private ignore audit verified. No commits or publishing.

## Exact next task — D13 bounded material metadata coverage census

Measure coverage beyond the32oracle samples using existing validated snapshot and
source attachment. Enumerate unique accessible clear GMTb locations deterministically;
classify protected, missing, unsupported revision/layout/budget and unparsed tail
outcomes separately. No asset extraction/new parser. Use existing prefix/graph/tail
readers with per-resource/aggregate byte and cancellation limits; retain aliases/
conflicting candidates and stage-specific success/error statistics. Report eligible,
attempted/skipped/unavailable counts and resource/revision/size/graph/tail distributions
without claiming full-game coverage from the selective local input tree.

Emit bounded diagnostic JSON only under local-analysis, plus terminal totals.
Validate with synthetic identities/duplicates/protection/errors/budget/cancellation
and deterministic ordering. If new shapes/errors appear, select a small bounded
oracle follow-up set before changing any format assumptions. Preserve the stable
viewer and all protected/read-only/publishing boundaries. No semantic animation,
sound behavior, shader evaluation, textured rendering or runtime work.
