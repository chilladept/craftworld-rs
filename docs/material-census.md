# D13 material metadata coverage census — 2026-10-05

This measures existing reader acceptance beyond the32oracle samples. It adds no
format rules, renderer, extraction or behavior implementation. The selected local
tree contains one DATA.FARC and scanner metadata; coverage is not full-game coverage.

## Policy and architecture

Authored headless example: crates/lbp-catalog/examples/material_census.rs.
Uses existing validated schema2catalog, read-only source attachment and independent
material prefix/graph/tail readers. Format provenance remains the pinned Toolkit
rules in material-prefix.md, material-graph.md and material-tail.md. No community
source copied/translated. Current parser contract ordinary PS3 branch0heads920–951;
their strict limits/unsupported outcomes apply unchanged.

Selection includes confirmed graphics-material/GMTb locations plus locations
referenced by graphics-material MAP hints. Hint-only records are retained as hints;
they do not confirm payload type. Unknown magic is skipped without payload reads.
Known protected status or magic is blocked without reads. Unexpected protected
magic stops after the4bytepreflight. Clear inputs then use checked4096bytepages.

Locations deduplicate by exact snapshot-relative path/offset/size, preserving all
inspection observations, hash candidates and MAP alias indices. Candidate conflicts
remain lists; no winning hash/candidate chosen. Orphans are supported. Material MAP
rows retain raw identity, type hints versus confirmed type and all candidate lists;
missing inspection indices are unavailable in this selected snapshot. This is not
proof of an asset missing from the complete disc or all possible loose trees.

Limits:20,000unique locations and20,000selected MAP rows;1MiB/resource;
64MiBaggregate payload reads;120second deadline after attachment; existing8MiB
expanded/graph/tail budgets. Attachment metadata has its separate existing limit.
Reserve a full resource against aggregate remaining budget before opening it;
partial preflights/reads contribute actual bytes. Cancellation is checked before
locations/pages and through parser callbacks; partial reports label unattempted
locations and cancellation explicitly. CLI Ctrl+C terminates the process; no new
signal handler or graceful Ctrl+C report is claimed. Changed source fails the
final stability check rather than publishing an apparently stable report.

Deepest successful stage expands once. Failure falls back to graph then prefix
with bounded sequential expansions and independent outcomes. Blocked, unsupported,
parser budget, invalid-for-current-reader, cancellation and source read failures
are distinct. Invalid-for-current-reader is not a definitive corruption judgment.
No source payload hash scan; hashes remain validated table/candidate identities.
Counts reflect parser acceptance, not independent correctness or visual fidelity.

Version1JSON is deterministic for completed runs: sorted location tuples, sorted
identity sets/distributions and catalog-index MAP order; no absolute input paths,
timestamps or timing in the body. Environment-dependent read errors are normalized
to a stable category. JSON outputs require new files under ignored local-analysis.
No game bytes, shader data or animation keys; only compact metadata/statistics and
resource lookup identity. Reports remain private even when containing only metadata.

## Local results

CONFIRMED_FROM_LOCAL_DATA: BCUS98245 APP_VER01.00 selective A2directory snapshot.

| Measurement | Result |
| --- | --- |
| Material MAP rows / unique candidate locations | 5,536 / 5,163 |
| Eligible / attempted / completed stage inspections | 5,163 / 5,163 / 5,163 |
| Prefix, graph and tail parsed | 5,163 each |
| Payload bytes read | 44,060,339 (within64MiBbudget) |
| Unavailable material MAP rows, skipped reads, cancellations | 0 in this snapshot/run |
| Locations with multiple MAP aliases | 244 |
| Candidate hash conflicts / orphan locations | 0 / 0 in this snapshot |
| Revisions / graph box-wire shapes | 18 / 76 |
| Maximum stored size / box count / wire count | 18,637bytes /19 /19 |
| Raw sound values | 28distinct;5,043sound0,120nonzero |
| Parameter animation array distribution | 0:4,978;1:121;2:32;3:15;4:12;5:3;7:2 |
| Total parameter-animation records / largest component-value array | 307 /1,143values |
| Unknown residual tail bytes | 0 in all5,163successful tails |

New compared with the32oracle samples: revisions929/939, nonzero raw sound values
and arrays with2–7records. These are reader observations, not yet independently
validated follow-up evidence. Sound labels, animation binding/timing/interpolation,
all shader semantics and rendering correctness remain UNKNOWN.

## Verification and reproduction

Four new authored Rust example tests cover hinted/confirmed selection, missing MAP
rows, alias/duplicate locations, orphans, conflicting identities, known/unexpected
protection without further reads, per-resource/aggregate budgets, mid-read
cancellation, deterministic output and separate failure categories. Two Python
selection tests cover deterministic order, cap/uncovered-feature reporting, conflicts,
failed records, incomplete census and source-pin rejection.91workspace Rust tests,
fmt, Clippy and native viewer build pass;24Python tests pass (final two selector
tests repeated after the sample cap refinement). Viewer/parser libraries unchanged.

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-catalog --example material_census
cargo run -p lbp-catalog --example material_census -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d13-material-census.json
cargo run -p lbp-catalog --example material_census -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d13-material-census-repeat.json
cargo fmt --all --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
& 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' scripts/material_coverage_samples.py local-analysis/d13-material-census.json local-analysis/d10-material-tail-oracle-final.json local-analysis/d13-material-followup-samples-final.json
& 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -m unittest discover -s tests -v
```

Original and repeated9,209,187bytecensus reports byte-identical SHA256:
`a0de399385a5f9792a7017f6ee9cb90b13ff4568553f671842baf8f7316b0d40`.
Final protected-snapshot-magic hardening adds synthetic coverage; real inputs in
the earlier reports were already all clear. Final census is rerun after hardening.
Complete final Rust log: ignored d13-workspace-tests-final.log. No parsing failure
or new format assumption. Original SFO/MAP hashes and FARC size/read-only attributes,
clean reference source and default-deny boundaries verified; root no Git, no
commits/publishing or approval-review rejection. No native interaction/capture,
full disc/update coverage or new external oracle run in this milestone.

## Exact next task — D14 targeted coverage oracle validation

Authored script material_coverage_samples.py deterministically selects40samples,
prioritizing previously unrepresented revision/raw-sound/animation-array counts,
then graph-shape diversity. Initial32cap left the7-record cohort unrepresented;
raised to40within the existing48sample oracle cap. Final selection leaves no primary
feature uncovered and represents24graph shapes; not all76shapes validated. This is
a metadata manifest, not proof of reader correctness. Conflicting hashes are excluded
explicitly rather than choosing one. Original limited manifest remains ignored.

Run pinned `run-material-oracle.ps1 -Tail -SampleManifest
local-analysis/d13-material-followup-samples-final.json` with a new ignored report.
Compare its prefix/graph/tail metadata against independent Rust `material_parity
--tail`; preserve every rejection/mismatch and unchanged parser limits. Validate
nonzero raw sounds, new revisions and multiple animation records; repeat bounded
oracle output for determinism. Record applicability/conflicts before any parser
changes. No sound labels, animation semantics, shader execution, texture-role
guesses, rendering/gameplay or protected decoding.

Final post-hardening census report matches both earlier reports byte-for-byte;
same SHA256 and counts. Follow-up40sample payload total309,699bytes, below the
external16MiBaggregate budget. No new asset hashes were computed by the census.

## D14 targeted oracle validation complete — 2026-10-05

Pinned Toolkit `d6a2ef80b5a09999f08f23a803b516096055e5c9` isolated prefix/box/wire/
parameter-animation readers accept all40selected follow-up samples. Independent
Rust material_parity --tail matches every emitted prefix/graph/tail field exactly:
40compared,0mismatches,0failures;309,699archived bytes. Oracle guards/self-tests pass.
No parser, adapter, selector, renderer or fixture source change needed. No new
oracle conflict; existing extreme signed zigzag caveat remains unchanged.

Repeated oracle output byte-identical SHA256:
`f9ccf82200e893810efb574ff5d87bff5f75bbb3a1147d8236852023381859da`.
Private oracle/repeat/parity/coverage reports stay under local-analysis.

Combined with D10/D11 evidence:72distinct samples,0overlap, all18census revisions,
all28raw sound values and all7observed animation-array counts(0,1,2,3,4,5,7)
represented.29of76observed graph box/wire shapes;23parameter-animation records;
largest component-value array603in this combined set; no residual tails. Coverage
comparison asserts no missing primary revision/sound/array-count feature against
the census. This covers value cohorts, not all5,163materials or their behaviors.
Largest census array1,143values remains outside this oracle set.

CONFIRMED_FROM_LOCAL_DATA: independent canonical field equality on the40additional
samples. CONFIRMED_FROM_COMMUNITY_CODE rules/source sections/applicability are
unchanged from material-prefix.md, material-graph.md and material-tail.md. Newly
observed revisions929/939, nonzero raw sound values and multiple serialized
animation records now have sample corroboration. Sound labels/binding, animation
timing/interpolation, shader meaning and rendering correctness remain UNKNOWN.

```powershell
./scripts/run-material-oracle.ps1 -Tail -SampleManifest local-analysis/d13-material-followup-samples-final.json -Output local-analysis/d14-material-tail-oracle.json
. ./scripts/use-local-rust.ps1
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d14-material-tail-oracle.json local-analysis/d14-material-tail-parity.json --tail
./scripts/run-material-oracle.ps1 -Tail -SampleManifest local-analysis/d13-material-followup-samples-final.json -Output local-analysis/d14-material-tail-oracle-repeat.json
Get-FileHash -Algorithm SHA256 -LiteralPath local-analysis/d14-material-tail-oracle.json,local-analysis/d14-material-tail-oracle-repeat.json
```

Private coverage summary also checks previous/new sample statuses and equality on
any duplicate identifier, then compares the union of revision/rawsound/array-count
sets with the census. No duplicates found. No full-corpus oracle/native GUI,
new branch/platform or semantic tests. Rust/Python source unchanged; D13's91Rust/
24Python/fmt/Clippy/build checks not repeated. Original SFO/MAP fingerprints and
FARC size/ReadOnly attributes unchanged; reference source clean at required pin;
default-deny authored/private file audit verified; root no Git, no commits,
publishing or approval-review rejection. No failed experiment or format mismatch.

## Exact next task — D15 remaining graph-shape/boundary oracle samples

Use the existing census and both validated sample sets to select one representative
per47unrepresented graph box/wire shapes, plus the resource with largest observed
component-value array(1,143values) if not already selected. Deduplicate by identifier
only when all location/hash candidates agree; preserve conflicts as exclusions.
At most48samples fits the unchanged oracle limit; enforce existing resource/total
byte budgets. Keep metadata-only deterministic selection recipes and private reports.

Run pinned tail oracle and independent complete metadata parity; repeat for
determinism. Record any rejection/mismatch without weakening guards or assuming
game corruption. Verify the union covers all76observed graph count shapes and the
component-array maximum; this still does not prove every material or shader behavior.
No new parser/renderer, sound labels, animation semantics/key export, protected
decoding, shader execution or runtime work.


## D15 count-shape and array-boundary validation — 2026-10-05

Authored `scripts/material_shape_samples.py` selects the smallest unambiguous
representative per missing box/wire count pair, then adds the largest component
array if absent. Identifiers shared by conflicting or failed locations are
excluded. Agreeing aliases choose a deterministic location; agreement means equal
stored size and census stage metadata, not byte equality. The script reads JSON
only. Limits: 48 samples, 1 MiB per resource, 16 MiB aggregate; exclusive outputs
under ignored local-analysis. No parser/adapter/UI changes were needed.

Selected 48 resources (430,103 archived bytes): 47 missing count pairs plus the
1,143-component-value boundary. Toolkit pin
`d6a2ef80b5a09999f08f23a803b516096055e5c9` guarded readers accepted all 48.
Independent complete Rust prefix/graph/tail equality: 48 compared, zero mismatches
or failures. External synthetic guards passed. Existing signed zigzag conflict
remains unchanged. Repeated final selections and external runs are byte-identical:

- Manifest SHA256: `a93b3d6d650d0d9e4cdde63b259e825d4c57b0ad1128582fa1ed9bdeabb37f50`.
- Oracle SHA256: `5a03908e94dc6472325f6077235f3e9157697d7b656a1e2653cebc5c7dfc2881`.

Combined D10/D14/D15 evidence contains 120 distinct samples covering all 76 observed
box/wire count pairs and maximum component array (1,143). D14 revision/raw-sound/
array-count coverage remains intact. The union audit rejects wrong pin/schema/scope,
unparsed samples, conflicting duplicates or incomplete shape/maximum coverage.
Count pairs do not describe every topology, port combination or resource; no
full-corpus oracle, animation behavior or shader/rendering fidelity claim.

CONFIRMED_FROM_LOCAL_DATA: complete canonical equality on these 48 samples.
CONFIRMED_FROM_COMMUNITY_CODE: existing class/method provenance, rules, thresholds,
applicability and conflicts in material-prefix.md, material-graph.md and
material-tail.md remain unchanged. Sound binding, animation timing/interpolation,
port semantics and shader evaluation remain UNKNOWN.

```powershell
$pythonRuntime = 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
& $pythonRuntime scripts/material_shape_samples.py local-analysis/d13-material-census.json local-analysis/d15-material-shape-samples.json local-analysis/d10-material-tail-oracle-final.json local-analysis/d14-material-tail-oracle.json
./scripts/run-material-oracle.ps1 -Tail -SampleManifest local-analysis/d15-material-shape-samples.json -Output local-analysis/d15-material-tail-oracle.json
. ./scripts/use-local-rust.ps1
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d15-material-tail-oracle.json local-analysis/d15-material-tail-parity.json --tail
./scripts/run-material-oracle.ps1 -Tail -SampleManifest local-analysis/d15-material-shape-samples.json -Output local-analysis/d15-material-tail-oracle-repeat.json
& $pythonRuntime scripts/material_shape_samples.py local-analysis/d13-material-census.json local-analysis/d15-material-coverage.json local-analysis/d10-material-tail-oracle-final.json local-analysis/d14-material-tail-oracle.json local-analysis/d15-material-tail-oracle.json --verify
& $pythonRuntime -m unittest discover -s tests
```

Three authored Python cases cover deterministic selection, extra maximum-array
selection, union auditing, alias/report conflicts, failed-location exclusion and
all budgets. All 27 Python tests pass after final hardening, including exclusion
of a failed duplicate observation at the same location. Final repeated selection
matches the original. Rust source unchanged: current compiled parity passed;
D13's 91 Rust tests/fmt/Clippy/build were not repeated. No native capture, semantic
tests, other platform or full-game oracle. Original SFO/MAP hashes, FARC size/ReadOnly,
reference pin/clean source and default-deny boundaries verified unchanged. Root
remains outside Git; no commits/publication, new format conflict or approval rejection.

## Exact next task — D16 native material graph diagram

Add a bounded read-only diagram to the existing Material graph inspector using
already parsed box indices and wire endpoints. Keep metadata lists and earlier-stage
fallback. Use a deterministic diagnostic layout, label nodes with raw type/index
and edges with raw ports/swizzle, and explicitly show invalid endpoints. Provide
selection linked to metadata plus bounded pan/zoom/fit. Keep layout/UI separate
from parser libraries. Synthetic layout/invalid-endpoint checks and native small/
larger validated graph captures should precede handoff. No new parser, guessed
port semantics, shader/sound/animation evaluation, protected decoding or runtime.

D60 schema2 adds existing prefix wrap_s_raw/wrap_t_raw at every successful prefix
stage including partial graph outcomes. Schema1 lacked these fields. Parsing
and limits unchanged. [Evidence](mirror-preview.md).
