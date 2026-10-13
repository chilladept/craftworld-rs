# Handoff — 2026-10-06

## Goal / current milestone

Independent LBP2 Asset Explorer. A1/A2 scanner, B1/B2 metadata/hex/source explorer,
C1-C4 texture census plus GTF/TEX BC1/BC3 previews, and D1/D2 bounded MSH prefix,
geometry/native wireframe and D3 material navigation/header census are implemented
and verified. D4 establishes an external material-prefix oracle/contract. The product
prefix reader is now implemented in lbp-resource (D5), with canonical32sample parity.
Nine crates. Read AGENTS first; latest D60 verification, limits and next task
are at the end of this handoff.
Latest product slice: bounded grouped diffuse mesh preview with static color,
texture products/MIX and opt-in local matrix-animation base snapshots. Levels and
animation playback remain unsupported. See the D60 entry below.
Original image unchanged; current readers still report protected inputs explicitly.
Research permissions were revised below on 2026-10-06; read current AGENTS.md.

## Environment and immutable inputs

- Windows/PowerShell; Python 3.11.9/Pillow, Git and project-local Rust 1.99.0 available.
  Activate Rust with `. ./scripts/use-local-rust.ps1`; fmt and Clippy installed.
- Ignored local Temurin JDK 21.0.12.1+1 and Maven 3.9.11 installed with official
  checksum verification. Unmodified Toolkit cwlib built from pinned source.
- No MSVC installation found; Rust tests use x86_64-pc-windows-gnu and the ignored
  verified MSYS2 assembler/runtime fix. B1 native GUI builds on GNU; no MSVC required.
- Root is not a Git repository; no init, staging, commit, push or publication.
- Original ZIP/ISO unchanged, BCUS98245 APP_VER/VERSION 01.00. SCE EBOOT, NPD
  DATA.SDAT and protected packages remain untouched, with no decryption.
- Prepared ignored game-data/a1-disc contains only SFO/MAP/DATA.FARC: 1,027,812,196
  bytes, read-only copies. No further extraction during A2. No whole-disc extraction.

## Implemented / verified

- A1/A2's four crates provide bounded magic/header/dependency
  inspection. CLI retains --tables-only and defaults to A2 schema-version-2 JSON.
- Every candidate location inspected once, aliases retained; row-level metadata
  supplied only for unique candidates. Per-location diagnostics, raw IDs/flags/type
  IDs/nulls/duplicates preserved. Dependency links join MAP rows without recursion.
- Budgets: 1 MiB/resource metadata, 65,536 dependencies/resource, 256 MiB total reads,
  in addition to A1 table/file limits. No payload decoding in Rust.
- Output refuses overwrite/inside-input paths. Partial metadata failures publish
  diagnostic JSON with exit 2; fatal errors exit 1. Unknown/unsupported/protected
  and missing candidates remain explicit rather than being called corruption.
- Local A1 counts unchanged: 36,854 MAP rows / 35,867 hashes; 31,534 FARC entries /
  hashes; zero duplicate GUID/archive hashes. Table offset 1,023,107,338.
- 32,232 MAP rows have archive candidates; 4,622 do not. Minimal input has one loose
  candidate (DATA.FARC itself), 4,621 missing rows. Full ISO reconnaissance's 3,689
  loose matches / 933 unmatched has a different scope.
- A2 inspects all 31,534 FARC entries using 1,462,918 metadata bytes: 16,515 ordinary
  metadata_parsed, 21 metadata_partial_static_mesh, 13,862 identified_unparsed,
  1,136 unknown_magic. No malformed/unsupported/budget/I/O/protected cases in the
  final clear-FARC scan. Protected files outside this input remain a boundary.
- 116,739 raw dependencies: 116,693 GUID catalog candidates and 46 unresolved in
  selected MAP. A catalog match does not prove target bytes are available.
- 4,333 distinct MAP hashes lack an inspected location. This differs from missing
  row counts because of aliases. Resource type counts are per inspected location.
- Unmodified pinned Toolkit oracle agrees on every MAP/FARC field and 24 clear
  binary samples across 22 actual heads (306–951), with 123 normalized dependencies.
  Canonical Rust magic is stricter than Toolkit's lowercase-tolerant matching.
- Two complete root-form scans are byte-identical: 119,338,719 bytes, SHA-256
  d1334d780b9f31f940aaf80ce19dd154b71878ce567ed4d7f08d38ed53c029b8.
- 19 Rust tests + seven Python tests pass; fmt and Clippy (-D warnings) pass.
  Synthetic coverage includes header boundaries/LD flags, null/combined/unknown
  references, truncation/budgets, protected stops, safe metadata-only reads,
  mismatch between extension and magic, dependency aliases, partial JSON/exit codes.
- Ignore audit excludes images, game-data, references, tools, full inventories and
  binaries; authored Java adapter explicitly allowed alongside Rust/docs/scripts.

## Commands and evidence

```powershell
. ./scripts/use-local-rust.ps1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -m unittest discover -s tests -v
cargo run -p lbp-cli -- --game-dir game-data/a1-disc --json local-analysis/a2-inventory-verified.json
cargo run -p lbp-cli -- --game-dir game-data/a1-disc/PS3_GAME --json local-analysis/a2-inventory-repeat.json
./scripts/run-toolkit-oracle.ps1 -Output local-analysis/oracle/toolkit-a2-repeat.json
python scripts/verify_toolkit_oracle.py local-analysis/a2-inventory-verified.json local-analysis/oracle/toolkit-a2-expanded.json --repeat local-analysis/a2-inventory-repeat.json --output local-analysis/a2-verification.json
```

These filenames already exist; choose new names on rerun. Use only the verified A2
inventory/repeat, not the superseded initial a2-inventory.json. Oracle sample manifest:
local-analysis/oracle/samples-a2.json; library build log toolkit-build-local.log.
Exact Maven reproduction, semantics and limits: docs/resource-metadata.md.

## Provenance / limitations / failed experiments

Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9, FileDB::process, FileArchive
constructor, SerializedResource::process/processDependencies and enums; source-level
citations and MIT notices retained. No unlicensed/decompiled implementation copied.
Oracle uses normal cwlib constructors on bounded clear samples only; its plain
decompressor runs, with JVM heap 512 MiB. No pixels/payload structures exported.
Toolkit filters nulls, deduplicates descriptors and ignores type in equality;
comparison normalizes those semantics, while Rust retains every raw descriptor.

Initial sample ceiling 940 missed actual heads 941–951. Selected one representative
per additional head, corroborated each with Toolkit, then extended supported scope.
Static-mesh common header/dependencies are parsed; variable header/compression state
remain explicit null/partial. Textures/text/swizzled wrappers only magic-identified.
Unknown opaque formats, trailing ALSR data, other revisions/branches, overlays,
payload SHA-1 verification, pixel/mesh/level/animation decoding remain deferred.

An early protection check incorrectly treated any fourth byte 'e' as encrypted;
a synthetic false-positive test exposed plain text. Final reader requires a known
canonical wrapper or protected-container signature, stops after four bytes, and
leaves other opaque text unknown. Initial diagnostic JSON is superseded.
PowerShell split an unquoted JVM -D option; fixed quoting. A first Maven relative
cache landed under Documents/local-analysis; interrupted, corrected to absolute
project-local path, verified new cache creation/files and removed only that cache.
No approval rejection. No original game input changed.

No fuzzing, cross-platform CI, hostile filesystem-race defense, asset visual output or
texture/mesh oracle agreement claimed. Input directories must remain stable during
scans. Java oracle validates samples, not the entire semantic resource corpus.
Own-source distribution license remains undecided; no release work authorized.

## B1 implementation and verification

Two new crates: lbp-catalog (UI-independent typed schema-2 projection and direct
reference/search indexes) and lbp-viewer (eframe/egui 0.36.2, wgpu backend). Cargo.lock
pins the dependency tree. Native snapshot browser has archive/type/status filters,
GUID/hash/path lookup, metadata/raw projection, outgoing and incoming references,
history and copy IDs. Authored Demo has aliases, missing/unknown/protected states.
No original game-file opening, payload preview, live scan, or byte hex view in B1.
JSON loading/validation/indexing runs on a cancellable worker with a 256 MiB byte
ceiling and explicit entry/reference caps. Fixed-height lists are virtualized.

Exact commands: activate `. ./scripts/use-local-rust.ps1`; `cargo fmt --all -- --check`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo build -p lbp-viewer`. Added nine focused catalog/UI tests; 28 total Rust tests.
Headless command: `./target/debug/lbp-viewer.exe --check-snapshot local-analysis/a2-inventory-verified.json`.
Observed 4.434 s debug load and counts 36,854 / 31,534 / 116,739, matching A2.
Native render uses NVIDIA GeForce RTX 3080 / Vulkan, loads in approximately 4.6 s.
Local QA PNGs under ignored local-analysis/b1-*; viewed synthetic dependency and
real overview/dependencies/reverse/metadata/raw views. See docs/viewer.md for limits
and commands. No manual input interaction or other GPU/platform tested.

Initial compile used egui's removed style_mut API and deprecated show_inside;
updated to style_mut_of/theme and show. Test-only nested mutable borrow fixed.
Headless egui rendering requires clearing unconsumed texture deltas; the layout
test was corrected after that assertion fired, then all tabs/states passed.
Visual QA replaced unsupported arrow glyphs with text and made overview scrollable.
eframe's legacy screenshot feature is unsupported on wgpu; used the documented
viewport screenshot event instead. No original inputs changed; no reference/tool
binaries or snapshots added to authored source. No approval rejection.
Ignore audit used the existing reference checkout's Git metadata with this workspace
as work-tree and --no-index (no initialization/staging): snapshots, PNGs, game data,
references and binaries excluded; Cargo.lock/new Rust/docs eligible. Verified
inventory/SFO/MAP hashes still match the earlier fingerprints; DATA.FARC remains
read-only and no original data was opened by the viewer.

## B2 implementation and verification

Seven crates now: moved the existing CLI source/SFO modules to shared lbp-io;
scanner and catalog use identical directory containment/read rules. Source.open
rejects unsafe keys, non-regular/changed-size files and every nested symlink/reparse
component, including newly introduced ancestors with internal targets.

Catalog attachment requires schema-2 SFO/MAP fingerprints, compares full emitted
identity, all MAP rows and every FARC table field, and checks candidates/inspections
against approved ranges. Bound files retain size/creation/modification stamps;
every page rechecks them before/after reads. These are stable-input checks, not
full payload SHA verification or hostile-race defense. Extra files do not rescan
or expand a snapshot. Identical-location conflicting hash claims remain explicit.

Viewer attachment and hex I/O use cancellable workers. MAP resource Hex bytes and
independent Archive entries support candidate selection, offset and Prev/Next pages,
alias navigation and entries without MAP aliases. Max 4096 payload bytes/page plus
at most four protection-prefix bytes; protected previews return no bytes. One page
cached, no byte export. New snapshot/detach invalidates source and page jobs.

Exact checks: `cargo fmt --all -- --check`, `cargo test --workspace` (35 tests),
then `cargo test -p lbp-catalog` after adding case-normalized range lookup (36 total),
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo build -p lbp-viewer`.
Eight new tests cover source/hex/table validation, changed/case-varied files, protection,
cancellation, alias/conflict/orphan handling, ancestor junctions and UI mode transitions.
Headless: `./target/debug/lbp-viewer.exe --check-snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-entry 0:0`.
Source validates with 4,704,862 metadata bytes, all 31,534 FARC entries match;
4096-byte page at absolute offset 39,023,490. Local archive has zero hashes without
MAP aliases; orphan support tested synthetically. Native archive/resource hex PNGs
viewed under ignored local-analysis/b2-* on RTX 3080/Vulkan. No manual input QA.

Compile fixes addressed attachment reporting placed in the help-return branch and
an entry-filter closure moving mutable self. Inspection found shared filter cache
initialization; a regression test covers separate entry/resource cache updates.
Clippy's nested condition fixed. No new community parser/oracle claims; original
source modules and established protection rule reused. No protected executable
investigation, extraction, original writes, publishing or approval rejection.
See docs/source-attachment.md for limitations and reproduction commands.
Ignore audit confirmed new Rust/docs/lock eligible and screenshots/game-data/reference
outputs excluded using the existing reference Git metadata, without root init/staging.
SFO/MAP/verified inventory hashes still match earlier records; ZIP/FARC sizes and
modification times unchanged, DATA.FARC remains read-only. No whole-image hash/read.

## C1 implementation and verification

Added diagnostic-only Rust texture_census example, strict bounded zlib/DDS-prefix
inspection (flate2 dev dependency), authored Java Toolkit texture adapter and
metadata-only Python selection/parity report. Product scanner/UI unchanged.
13,624 texture locations: 11,633 GTF and 1,991 TEX. GTF: 6,950 BC3, 4,453 BC1,
174 A8R8G8B8, 56 B8. TEX: 1,203 BC1, 787 BC3, one 32-bit uncompressed.
9,817,800 requested diagnostic bytes; validated existing source first. All clear
headers/tables/TEX DDS prefixes succeed. Later compressed chunks not all inflated.

22 simple 2D samples, 524,137 archived bytes, payload SHA-1 verified. Strict
inflation equals unmodified pinned Toolkit buffer; every sample top mip decodes.
191 compared fields agree, zero failures. Canonical RGBA digests only under ignored
local-analysis. Both full census and oracle repeats identical. No Rust pixel parity
claim, no image export or visual QA. See docs/textures.md for hashes and commands.

Ten catalog + four new example tests pass; three new Python report tests pass;
Java synthetic guards pass. cargo clippy -p lbp-catalog --all-targets -- -D warnings
and cargo fmt --all --check pass. Initial Clippy constant-chunk lint corrected.
No need to rerun unchanged GUI rendering. Source-built pinned cwlib tracked source
remains clean. Original game inputs read-only; no extraction/protected decoding.

All 11,400 simple 2D GTF DXT declared expanded sizes exceed tightly packed standard
mip totals by 16–120 bytes. Placement/meaning UNKNOWN. Pinned DDSReader mip-skip
arithmetic differs from ceil-block arithmetic; authored synthetic 8x8 DDS top red
works but mip 1 throws ArrayIndexOutOfBoundsException. Higher-mip oracle rejected
for this purpose, source unmodified. Uncompressed Toolkit swizzle cites RPCS3 at
3d49976b whose root license is GPLv2; do not translate that helper under MIT.
Web fetches for pinned source initially cache-missed; ordinary read-only HTTPS
downloads succeeded and remain ignored. Refresh pinned GtfDecoder corroborates
top-mip BC1 dispatch but was not executed. C2 can use block specs and avoid both
uncertain paths. No approval rejection or publication.

Latest evidence: local-analysis/c1-texture-census-dds.json and repeat,
c1-texture-samples.json, c1-texture-oracle.json and repeat,
c1-texture-verification-final.json. Initial header-only c1-texture-census.json is
superseded. Outputs refuse overwrite; choose new names on rerun.

Final default-deny ignore audit: new authored Rust/Java/Python/docs and Cargo.lock
eligible; census/oracle JSON, original ZIP, FARC and reference jar excluded. Used
existing reference Git metadata with --no-index; root remains without .git.
SFO/MAP SHA-256 fingerprints match prior records and all three prepared inputs
retain ReadOnly. No whole-image hash was repeated.

## C2 implementation and verification

Eight crates: new UI-independent lbp-texture, bounded clear GTF BC1 top-mip parser
and CPU RGBA IR. Header preflight before full source reads; strict complete chunk
inflation with 16 MiB archived/32 MiB expanded/4 Mi pixels limits and cancellation.
No higher mip offsets or swizzle guessed. Viewer Texture tab supports MAP candidates
and archive entries, single-image CPU/GPU cache, workers/cancellation, Fit/zoom,
RGBA/channel/alpha checkerboard. Selection/source changes drop stale work/cache.

Initial three simple C1 samples matched bit-replicated RGB565, but broader oracle
failed 15/16. Confirmed normalized nearest endpoint expansion via pinned DDSReader
tables and authored 0x18e3 oracle (25,28,25,255). Independent arithmetic now matches
all 16 broader plus three original samples, exact RGBA hashes/dimensions/mips/flags.
19 distinct samples, zero final failures; full corpus not decoded. All broader
samples flag zero; bump notice is header-derived and not a local BC1 bump claim.
Failed initial report retained; use c2-bc1-parity-final.json/c2-c1-parity-final.json.

49 Rust workspace/all-target tests pass; final seven decoder + seven viewer tests
pass after QA channel changes. Clippy workspace then final affected targets and
fmt pass; native build passes. Three Python report tests pass. Native RGBA, alpha,
checkerboard and unsupported DXT5 screenshots inspected on RTX 3080/Vulkan under
ignored local-analysis/c2-*. No manual input delivery/other backend verified.
Initial Clippy loop/test initialization issues fixed. No encrypted/protected input,
asset export, bulk extraction, source translation, original writes or publishing.
See docs/texture-preview.md for provenance, commands, limits and report hashes.

Final ignore audit permits authored Rust/docs/lock and excludes parity JSON,
screenshots, FARC and original ZIP; root still has no .git. SFO/MAP fingerprints
match prior records, DATA.FARC size/ReadOnly attribute unchanged. No whole-image
hash repeated. No approval rejection. Minor documentation patch context and
Unicode string-match errors were corrected without affecting inputs.

## C3 completion — 2026-10-05

Clear TEX/DDS BC1 top-mip support is complete, alongside existing plain 2D GTF
BC1. Shared bounded inflation and independent CPU pixels preserve raw DDS fields,
mip count zero, optional GTF fields and explicit unsupported/protected outcomes.
No BC3, swizzle, higher-mip offsets or original rendering fidelity is claimed.

Pinned Toolkit oracle: 15 TEX samples (63,693 archived bytes) match exact RGBA,
dimensions, raw mips, no_srgb and all retained DDS fields. Existing 16-sample GTF
regression passes; refreshed C1 comparison passes all six supported BC1 samples,
including the original three GTF samples (19 distinct GTF samples across checks).
The first C1 regression used an older oracle lacking DDS fields and reported three
metadata mismatches. Refreshing the same pinned oracle resolved those; the failed
report remains ignored evidence, not a format conflict.

Commands passed after the final code changes:
- `cargo test --workspace --all-targets` (53 Rust tests).
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo fmt --all --check`.
- `python -m unittest discover -s tests -p test_texture_report.py -v` (3 tests).
- `cargo build -p lbp-viewer`.

Native TEX RGBA (GUID 00018946, raw mip zero), alpha (000075f3), and unsupported
DXT5 (000279ab) captures inspected on RTX 3080/Vulkan. Unsupported DDS is explicit;
raw-zero mip count displays as unspecified. No manual input, other GPU/backend,
full texture corpus, higher mip, BC3 or visual fidelity validation performed.
See [C3 contract](docs/tex-preview.md) for provenance, evidence and reproduction.

## Next task / blockers

C4: bounded plain 2D BC3/DXT5 top-mip support for GTF and TEX, using public block
specifications and the same pinned external oracle. Exact prompt: docs/roadmap.md.
Do not start C4 during this wrap-up. Unknown mip tails, swizzle provenance and
protected resources remain outside scope. Own-source license is undecided; root
has no Git repository and no staging, commit, publication or original writes occurred.

Final C3 audit: SFO/MAP SHA-256 match prior fingerprints; DATA.FARC remains
1,023,990,298 bytes with ReadOnly attribute. Default-deny ignore rules exclude
private parity/capture/game/reference-binary/ZIP paths and permit authored
Rust, oracle harness and documentation. Root still has no .git.

## C4 completion — 2026-10-05

Independent BC3 top mips and actual format labels complete. 35 samples (28 GTF,
7 TEX; 2,096,689 archived bytes) match Toolkit pixels/metadata. BC1 15 TEX + 16
GTF regressions pass. 57 Rust tests, final workspace Clippy/fmt/native build,
four Python tests and authored Pillow corroboration pass. Native GTF RGBA/TEX
alpha captures inspected. Initial synthetic oracle exposed Toolkit reversed-color
BC3 bug; specification-correct behavior retained and guard records conflict.
See docs/bc3-preview.md; higher mips/swizzle/protection remain excluded.
Next active work: D1 mesh reconnaissance/prefix census/oracle, not level rendering.

## D1 mesh prefix complete — 2026-10-05

Ninth crate lbp-mesh: bounded MSHb prefix only; no UI, topology or asset export.
36 samples from 19 of 119 metadata cohorts match canonical Toolkit helper fields,
605,145 archived bytes. 2,716 known MSH locations, flags 0/7. 61 workspace Rust
tests, Clippy and fmt pass. Initial float-decimal equality failures resolved with
exact IEEE bounds bits; resource metadata API integration compile error fixed.
See docs/mesh-prefix.md. Active next: D2 topology/range/bone-layout evidence and
bounded geometry IR before native mesh preview. Do not treat prefix parity as mesh
compatibility; original inputs/protected resources and publishing rules unchanged.

## D2 first native mesh wireframe complete — 2026-10-05

Independent bounded primitive/descriptor/bone-layout traversal reaches explicit
topology5/6. CPU geometry IR (non-serializable asset data) is separate from metadata
and native egui wireframe. 36 samples match Toolkit primitive/count/topology/index
hash metadata exactly; 32 strips/four lists. Prefix-only regression also36/36.
Native list/strip and final unsupported-SMH captures inspected. First native QA
failed because CLI tab allowlist lacked mesh; fixed. SMH originally hit size budget;
bounded magic preflight now reports unsupported before whole-source allocation.

65-test workspace run passed; final seven mesh/nine viewer tests, workspace
Clippy/fmt and native build pass. Added descriptors/legacy revision bound after
workspace run; final affected checks pass. No manual orbit/zoom input, other GPU,
full mesh corpus, original pose/rendering fidelity or performance audit tested.
See docs/mesh-geometry.md, docs/mesh-prefix.md, docs/bc3-preview.md for contracts.

Final audit: original SFO/MAP fingerprints still match; DATA.FARC is1,023,990,298
bytes ReadOnly/Archive. Ignored reports/images/reference binary/game ZIP excluded;
authored mesh/harness/docs allowed. No .git root, commits, publishing, bulk extract,
source translation, protected decoding or input writes. No auto-review rejection.

Next bounded task: D3 material reference navigation, supported material metadata
census/pinned oracle before texture/material shading. Higher mips, swizzle, posed
skinning, animation playback, full level rendering, gameplay and network remain
outside current implementation. Own-source distribution license still undecided.

## D3 material navigation/header census complete — 2026-10-05

Native Mesh material candidate menus preserve GUID/hash groups, aliases, conflicts,
raw nulls and unresolved identities. Targets navigate to existing metadata/dependency
browser. Census:5,163GMTlocations/18cohorts/13,426rawtexturedependency entries.
32selectedheaders in16cohorts match pinned Toolkit and56normalized dependencies.
Final full workspace67Rusttests, Clippy/fmt/native build and5Pythonreporttests pass.
Rendered material controls inspected; no manual menu interaction tested. Corrected
Windows source UTF-8/CRLF editing artifacts and below-threshold synthetic recipe.
See docs/material-navigation.md. No material shader/payload compatibility claimed.

All concrete scanner/explorer/first-texture/first-mesh preparation milestones are
reviewable. Future extension: corroborated material payload metadata and controlled
oracle before any textured shading. Animation playback, posed/skinned rendering,
higher mips, static meshes, full levels/runtime/physics/network remain future work.
Original boundaries, ignored data, no Git/publishing and undecided license persist.

## D4 material prefix oracle/contract complete — 2026-10-05

Authored guarded external adapter studies PS3 GMT scalar prefix, shader interval
boundaries, eight texture descriptors and wraps, without full material deserialization
or shader-byte output.32/32samples accepted;56normalized texture-slot references
agree with prior header/dependency evidence. No independent Rust material parser
parity claimed. Synthetic flags0/7/all truncations/descriptor modes pass;3new Python
consistency cases pass. Toolkit short fixed-width padding exposed by truncation133;
explicit final bounds guard fixes harness acceptance. See docs/material-prefix.md.

No Rust product code changed; prior67Rusttests/fmt/Clippy/native build stand without
redundant reruns. No new native QA/full material corpus/graph or shading tested.
Next precise task: independently implement this bounded prefix after shared binary
wrapper factoring, with canonical32sample parity before UI integration. All current
scanner/explorer/first visual milestones remain complete. Protection boundaries,
read-only originals, ignored reports/reference binaries and no commits persist.

Final D4 audit: all8Python report checks pass (texture4/material1/prefix3).
SFO/MAP SHA-256 still match recorded fingerprints; DATA.FARC remains1,023,990,298
bytes ReadOnly/Archive. Default-deny ignore check excludes private outputs/game/jar
and permits new authored harness/scripts/tests/docs. Toolkit tracked source clean;
root still has no.git. No rejected approvals, new extraction or publication.

## D5 independent material prefix reader complete — 2026-10-05

Added lbp-resource::material diagnostic reader and shared ordinary-binary payload
decoder; mesh uses the factored decoder with unchanged error variants. No new crate
or GUI integration. Scalar bits/raw enums, ten offset boundaries, eight raw texture
descriptors and wraps retained; opaque blob/tail bytes never returned/serialized.
Exact canonical32/32material oracle equality and36/36mesh geometry regression.
Four new authored integration cases cover variants, every truncation, malformed
counts/offsets/varints/identities, budgets/protection/cancellation.71workspace tests,
fmt/Clippy/native build pass; see docs/material-prefix.md for exact commands/scope.

Initial sandbox fmt/native parity execution returned OS access denied; approved
execution succeeded. No auto-review rejection, asset writes or parser mismatch.
Full material corpus/shader/graph/GUI integration/visual fidelity untested. Next:
bounded native Material metadata tab/slot navigation using existing attachment
workers; explicit null/unknown/conflict outcomes. No textured shading or game scope.

Final D5 audit: SFO/MAP fingerprints unchanged; DATA.FARC unchanged size and ReadOnly
attribute. Default-deny ignore check excludes both new reports/reference jar and
permits authored reader/test/parity source. Reference tracked source clean; no.git
at root and no commits/publication. Final dependency cleanup moves mesh flate2 to
synthetic-test dependencies only; final Clippy/build and affected tests pass.

## D6 native Material metadata tab complete — 2026-10-05

MAP rows/archive entries now offer Material: raw scalar/IEEE bits, shader interval
counts/bounds, eight texture descriptors/wraps and diagnostic-only JSON. Shared
GUID/hash helper retains separate alias/conflict/null/unresolved groups; targets
use existing metadata/history navigation. One validated, bounded/cancellable
worker/cache per source/key; no shader/blob bytes returned/exported. Full74Rust
tests (13viewer), fmt/Clippy/native build pass. Native real-material/unsupported
captures inspected on RTX3080/Vulkan. Exact limits/commands: docs/material-viewer.md.

Initial synthetic setup referenced nonexistent demo API; corrected. Headless layout
then dropped unapplied font texture deltas; explicitly cleared, final tests pass.
No manual menu click/scroll/cancel or other GPU/platform QA; full corpus/graph/
shading not tested. Existing parser32sample andmesh36sample evidence remains.
NextD7: corroborated MaterialBox/MaterialWire metadata reconnaissance/oracle before
any independent graph parser or inferred texture roles. No shading/runtime expansion.

## D7 graph metadata reconnaissance/oracle complete — 2026-10-05

Pinned MaterialBox/MaterialWire/MaterialParameterAnimation/BoxType/BrdfPort read;
guarded per-structure external oracle now corroborates32samples:139boxes/118wires,
55slot links,278embedded animation records. All current prefixes remain equal;
prefix32/56reference regression passes. Graph repeat byte-identical;3new Python
consistency cases and Java synthetic bounds/signed guards pass. No Rust graph
parser or UI change yet. Prior74Rust product checks stand, not redundantly rerun.

Conflicts recorded: Toolkit ignores parameter count; guard accepts8only. Extreme
packed s32zigzag0xffffffffincorrectly returns0; future parser uses unsigned shift.
Nonzero animation data with zero component mask is guarded before division. All
actual sampled links fit bounds; tails2/989bytes remain unknown. Exact pinned
rules/commands/budgets: docs/material-graph.md. NextD8: independent diagnostic graph
reader preserving prefix API, with32canonical parity and synthetic/cancellation
checks before UI. No graph execution, role guesses, shader/render/runtime expansion.

Final D7 audit: SFO/MAP fingerprints and DATA.FARC size/ReadOnly attribute unchanged;
default-deny ignores private reports/reference jar and allows authored graph adapter,
report/test/docs. Toolkit tracked source clean, root without Git, no commits or
publishing. No approval-review rejection or failed local graph sample experiment.

## D8 independent graph metadata reader complete — 2026-10-05

Optional lbp-resource::material::inspect_graph preserves prefix API/schema and
native Material UI. One shared expansion; raw box parameters/position/subtype,
bounded animation summaries/digests, signed wire indices/raw ports/swizzles,
slot/endpoint range statuses and opaque tail counts. Correct unsigned zigzag;
unknowns/negative/out-of-range links/cycles retained without traversal/evaluation.
32/32canonical graph and32/32prefix regression match. Five new authored integration
cases;79workspace tests/fmt/Clippy/native build pass. No graph GUI or new native QA.

Initial synthetic test compilation needed explicit u32integer type; corrected.
Initial parity report reused prefix row label; corrected final graph report retains
same exact metadata equality. No parser/sample mismatch or protected decoding.
Exact commands/evidence/limits: docs/material-graph.md. NextD9: optional native graph
metadata tables retaining prefix when graph fails, slot candidate navigation and
bounded animation summaries; synthetic state/layout and real sample QA first.
No graph evaluation/playback/shader execution/textured rendering/game expansion.

Final D8 audit: SFO/MAP fingerprints unchanged; DATA.FARC remains1,023,990,298bytes
ReadOnly/Archive. Default-deny ignore rules permit authored graph source/tests and
exclude new graph/prefix reports/reference jar. Reference tracked source clean;
root without.git, no commits/publishing, no approval-review rejection.

## D9 native optional graph diagnostics complete — 2026-10-05

Material tab now offers explicit graph inspection, virtualized box/wire lists,
selected raw parameters/position bits, bounded embedded-animation summaries,
slot/endpoint range statuses and diagnostic JSON. Only output/texture-sample kinds
have pinned community labels; raw unknowns retained. Separate GUID/hash slot menus
reuse alias/conflict/null-preserving reference navigation. Graph failures retain
valid prefix; graph-only cancellation retains cached prefix, source/selection reset
clears caches and preserves user graph preference. No shader/graph evaluation.

15 viewer /81 workspace tests, fmt, Clippy and native build pass. Real head935
native RTX3080/Vulkan QA:8slots,5boxes,4wires,417parsedgraphbytes,2unknown tailbytes;
ignored capture inspected. Source SFO/MAP hashes unchanged, FARC1023990298bytes
ReadOnly/Archive; reference clean/root no Git; ignore audit excludes all private
outputs and permits authored code. No publishing or approval rejection.
No new parser/oracle comparison; D8 evidence unchanged. No manual menu selection,
expanded animation panel/other GPU/full corpus tested. Exact commands and limits:
docs/material-graph.md.

Next D10: bounded external material-tail oracle using pinned existing reader.
Sound enum/parameter-animation array are leads only. Inspect same32samples with
budgets/unknown outcomes, document rules/conflicts before a Rust extension. No
shader execution, protected decoding, asset/key export or gameplay expansion.

Final D9 check: queued graph requests during initial prefix loading are honored
once; cancellations/errors require explicit retry. All81tests/fmt/Clippy/build
pass after refinement (ignored complete test log). Native layout capture preceded
this lifecycle-only change; manual queued-request interaction remains untested.

## D10 material tail oracle complete — 2026-10-05

Pinned Toolkit isolated array reader accounts for all remaining tail bytes in the
same32clearPS3GMTb samples (232,687 archived bytes). Raw sound0 in all32;30empty
arrays/tails2bytes;2arrays each1animation with241values/keys/tails989bytes. Zero
unknown residual bytes in this sample set; no sound/animation behavior inferred.
Prefix/graph metadata unchanged versus D4/D7. Final/repeated output byte-identical.

Added authored MaterialTailOracle.java, -Tail runner/entry mode, metadata consistency
report and4synthetic Python tests; explicit ignore allowlist for authored Java.
Fixed/packed/truncation/budget/mask/name/raw/cancellation/empty-array self-tests pass.
All22Python tests pass; no Rust/UI change or repeated Rust/native QA in D10.
Initial oracle succeeded; later final run adds mask/empty-array synthetic guards.
No failed sample, new conflict, protected decoding, key/asset export or publishing.
SFO/MAP hashes unchanged; FARC1023990298bytesReadOnly/Archive; reference source clean
at pinned commit; root no Git. Default-deny authored/private boundaries verified.

Provenance, exact commands, applicability, unknown semantics and test limits:
docs/material-tail.md. Next D11: independent optional material::inspect_tail API,
one bounded shared expansion/walk, raw sound/animation summaries/residual counts,
preserve prefix/graph APIs;32canonical parity plus synthetic bounds/cancellation
and required Rust checks before GUI integration. No playback/sound simulation,
shader evaluation/textured renderer or game-scope expansion.

## D11 independent optional material tail reader complete — 2026-10-05

Added material::inspect_tail, TailInspection/TailMetadata and two public budget
constants in lbp-resource. Shared internal graph decode retains expanded payload
for one bounded prefix/graph/tail walk; shared animation helper avoids key export.
Raw sound/IEEE/name/mask/digests/counts and opaque residual counts retained. Existing
prefix/graph APIs and schemas unchanged. No GUI or behavior change.

Four new authored integration cases cover raw/stored/zlib, fixed/compact, every
valid-envelope tail truncation, unknowns/bits/digests, counts/value/name/mask/ULEB,
max128records/max2MiBtail boundary, protection/revision and tail cancellation.
Tail32/32canonical, graph32/32regression, prefix32/32regression exact;85workspace
Rust tests/fmt/Clippy/native build pass. No mismatch/failed experiment/new conflict.
Complete ignored test log and three parity reports retained. No new native capture,
Python/oracle rerun/full corpus/other build/platform or sound/animation behavior
claimed. Input fingerprints/read-only FARC, clean reference pin and ignore audit
verified; root still no Git, no commits/publishing or approval rejection.

Provenance/reproduction/limits: docs/material-tail.md. Next D12: optional native tail
metadata via validated worker, independent prefix/graph fallback, raw sound and
virtualized parameter-animation summaries/counts/digests/JSON. Synthetic lifecycle/
layout checks plus real empty/nonempty native QA; no keys, playback/sound simulation,
shader evaluation/textured rendering or game-scope expansion.

## D12 optional native material tail diagnostics complete — 2026-10-05

Material tab explicit tail mode shows raw sound, virtualized parameter-animation
summaries/selected name/mask/base IEEE/counts/digest and diagnostic JSON. Empty
arrays preserve null/empty ambiguity. Successful request expands once; failure
falls back through graph/prefix with independent errors and bounded sequential
expansions. Same-resource deeper reads/cancellation retain cached earlier metadata;
reload/source/selection reset clears caches and preserves mode preference.

Two authored native tests added;17viewer/87workspace tests/fmt/Clippy/build pass.
Native RTX3080/Vulkan captures inspected for000134ddempty(head935) plus00017049
(head949/mask4) and0001770b(head945/mask8), each populated record241values/keys;
rawsound0 andzeroresidual throughout. Bits/digests fit pane. No new parser/oracle
run or manual mode/queued/source-change interaction/expanded JSON/other GPU/full
corpus claimed. No failed experiment or approval rejection. Original fingerprints/
FARC sizeReadOnly attributes/reference source/ignore audit unchanged; root no Git,
no commits or publishing. Exact commands/evidence/limits: docs/material-tail.md.

Next D13: bounded deterministic coverage census using existing material readers on
unique accessible clear GMTb locations; separate protected/missing/unsupported/
budget/unknown-tail outcomes, per-stage counts and distributions, explicit scope
and aggregate budgets. Ignored JSON only, synthetic validation, small oracle
follow-up for new shapes before parser changes. No asset extraction, guessed
semantics, playback/sound simulation/shader execution/textured renderer/runtime.

## D13 bounded material coverage census complete — 2026-10-05

Added headless material_census example using existing validated source and readers,
deterministic path/offset/size grouping preserving inspections/hash candidates/MAP
aliases, protected preflight and separate stage failures, count/byte/deadline/cancel
limits, private version1JSON. Hint-only selection never confirms resource type.
Four authored Rust cases cover aliases/duplicates/orphans/conflicts/missing hints,
protected markers, resource/aggregate budgets and mid-read cancellation/errors.

Available snapshot:5,536materialMAProws,5,163uniqueclear locations, all attempted;
prefix/graph/tail5,163parsed,44,060,339payloadbytesread within64MiB, no residual tails.
18revisions/76graphshapes;rawsound28values(120nonzero),185nonempty animation arrays
with307records total, maximum7records/1,143componentvalues.244aliased locations,
no hash conflicts/orphans/unavailable material rows in this snapshot. Full-game,
visual or behavioral compatibility not claimed. Repeated report byte-identical.

Added metadata-only follow-up selector and2Python tests. Initial32cap missed array
count7; expanded40within oracle48cap. Finalmanifest covers all newly observed
primary revision/sound/array-count features and24graphshapes; no oracle run yet.
91Rust tests/fmt/Clippy/native build and24Python tests pass (selector2retested after
cap refinement). Protected snapshot marker/orphan/duplicate tests hardened; final
census rerun after hardening. Source input fingerprints/read-only/reference/ignore
boundaries unchanged; no native QA/parser change/commits/publishing/approval rejection.
Exact reproduction, limits and selected-scope caveats: docs/material-census.md.

Next D14: run pinned tail oracle on local-analysis/d13-material-followup-samples-final.json,
compare independent Rust metadata including earlier stages and repeat oracle output.
Record mismatch/rejection/provenance before parser changes. No sound labels,
animation semantics, shader evaluation/rendering/runtime or protected decoding.
Final post-hardening census matches original/repeat byte-for-byte (same SHA256).
Selected40follow-up samples total309,699payloadbytes; no new asset hashes computed.

## D14 targeted material oracle validation complete — 2026-10-05

Pinned guarded external tail oracle40/40accepted; independent Rust complete prefix/
graph/tail canonical equality40/40,0mismatches/failures,309,699archivedbytes. Repeated
oracle byte-identical SHA256f9ccf82200e893810efb574ff5d87bff5f75bbb3a1147d8236852023381859da.
No source/adapter/parser/UI/fixture changes or new conflicts/failures.

Combined baseline/follow-up72distinct samples with no overlap now cover all18census
revisions,28rawsoundvalues and7arraycounts(0,1,2,3,4,5,7).29of76graphshapes;
23parameter animation records;max603componentvalues versuscensus1143. Private
coverage summary verifies no missing primary feature. This is sample field parity,
not full-corpus/semantics/rendering fidelity. Source input fingerprints/read-only
FARC/reference pin/default-deny audit unchanged; root no Git/no publishing/rejection.
D13Rust91/Python24/fmt/Clippy/build not repeated because source unchanged; bounded
oracle guards and current compiled Rust parity pass. No native capture/otherplatform.
Exact reproduction/provenance/limits: docs/material-census.md.

Next D15: bounded deterministic selection of47remaining graph-count shapes plus
largest component-array sample if needed(max48), existing pinned tail oracle and
complete independent parity/deterministic repeat. Assert combined76shape/array
maximum coverage; preserve mismatches/unsupported/protected boundaries. No parser
change, semantic guesses, soundlabels/playback/shaders/rendering/runtime.


## D15 remaining material count-shape validation complete — 2026-10-05

Added deterministic metadata-only selection/coverage script and three synthetic
Python cases. Selected 48 samples: 47 remaining graph count pairs plus maximum
1,143-component-value array, 430,103 stored bytes. Pinned guarded oracle accepts
48/48; independent complete prefix/graph/tail equality matches 48/48 with zero
mismatches/failures. Repeated oracle and final selection are byte-identical.

Combined 120 distinct samples cover all 76 observed graph count pairs and maximum
component array; earlier revision/sound/array-count cohorts remain covered. This is
sampled metadata parity, not every topology/material or shader/animation behavior.
All 27 Python tests pass. Rust source unchanged; current compiled parity passes,
D13 Rust tests/fmt/Clippy/build not repeated. No native capture or new format rule.
Input fingerprints/FARC ReadOnly/reference/ignore boundaries unchanged; root no Git,
no commits/publishing/rejection. Exact reproduction: docs/material-census.md.

Next D16: native diagnostic material graph diagram using validated indices/raw
wire fields, deterministic bounded layout, linked metadata selection and pan/zoom/
fit. Preserve invalid endpoints and fallback. Synthetic layout checks plus small/
larger native QA. No shader/sound/animation evaluation, new parser, protected
access, gameplay or runtime expansion.


## D16 native material graph diagram complete - 2026-10-05

Added independent bounded diagnostic index layout with directed routes, linked box/
wire selection, hover raw fields, pan/zoom/Fit and invalid-endpoint warning list.
Metadata/fallback remain available; camera clears on resource/reload changes.
Parser graph caps reexported without changing layout or validation rules.
Four authored checks include actual egui selection events; 95 workspace tests pass.
Final UI refinement: 21 viewer tests/fmt/Clippy/build pass. Native final 5-box and
19-box RTX3080/Vulkan captures inspected; both runs exit zero. No manual native
pan/zoom/invalid-link/otherplatform claim. Inputs/reference/ignore boundaries intact.
Resolved import visibility, headless texture-delta handling, Clippy module ordering
and Windows-default-encoding issues; no format failures or approval rejection.
Exact commands/provenance/limits: docs/material-diagram.md.

Next D17: explicit private deterministic JSON export of cached material diagnostics,
versioned stage statuses/metadata/source-relative location, new ignored files only.
No additional asset reads, payload exports, new parser or shader/sound/animation
semantics. Test containment/overwrite/determinism/fallback and verify real output.


## D17 cached material diagnostic JSON export complete - 2026-10-05

Added explicit native destination/Save controls, deterministic version1 envelope,
three stage outcomes, current-mode flags, stable error categories and cached fields.
No additional game reads, payload or host-path export. New .json files only under
canonical ignored local-analysis, exclusive creation and 8MiB serialization budget.
Busy/uncompleted exports refused; cached earlier stages survive deeper failure.
Added envelope schema and QA export option using the same production save function.

Four authored tests; 99 workspace/25 viewer tests/fmt/Clippy/build pass. Two native
head949 exports exit zero; screenshot inspected, JSON19153bytes byte-identical,
all cached fields exactly match existing pinned oracle. Draft202012 schema validation
and four invalid mutations pass using ignored jsonschema4.23.0 developer tool;
bundled validator absence resolved without product dependency changes. No new parser
rule or semantics inferred. No manual Save/field-editing/otherGPU/fullcorpus claim.
Original input hashes/FARC ReadOnly/reference/ignore/root-no-Git boundaries intact;
no publishing/rejection. Exact evidence/limitations: docs/material-export.md.

Next D18: inspect actual texture preview cache, add cached metadata-only JSON export
using shared private writer/guard, no pixels or extra source reads. Versioned stage
outcomes, synthetic bounds/refusal checks and native pinned-oracle field comparison.
No new codec, shader evaluation, bulk extraction or runtime expansion.


## D18 cached texture diagnostic export complete - 2026-10-05

Added explicit Texture destination/Save controls and version1 metadata envelope/schema.
Projection whitelists header/counts; no pixels, asset bytes or extra source reads.
Factored shared location/stage/error/private-output helpers to app::diagnostics;
Material envelope unchanged. Two new texture cases, relocated/hardened output cases;
101workspace/27viewer tests/fmt/Clippy/build pass.

Native GTF BC3 GUID00017440 and TEX/DDS BC3 GUID0000c9cd exports exit zero and
screenshots inspected. Both reports match all cached fields in prior oracle-validated
C4 parser output; directly shared external oracle header fields also match. GTF
RGBA/R repeat byte-identical; Material regression byte-identical toD17. Schemas
validate and four invalid mutations per texture are rejected. No fresh pixel/oracle
run, manual Save/errorinteraction/otherGPU/fullcorpus claim or new format rules.
Input fingerprints/FARC ReadOnly/reference/ignore/noGit boundaries intact. No
publishing/rejection. Exact commands/scope: docs/texture-export.md.

Next D19: inspect mesh cache and add cached metadata/summary JSON export using the
shared private writer. Exclude vertex/index/bone arrays and geometry/image data;
versioned stages, synthetic cache/refusal checks and native existing-oracle comparison.
No extra asset reads, new parser, shaded rendering or runtime expansion.


## D19 cached mesh diagnostic export complete - 2026-10-05

Added native Mesh destination/Save controls and version1 summary schema. Explicit
projection exports cached prefix/topology/descriptors/counts/bounds/digests plus
vertex count; no geometry/index/bone arrays, pixels or extra source reads. Shared
private output guards unchanged. Two new cache cases;103workspace/29viewer tests/
fmt/Clippy/build pass.

Native listGUID000150e5 andstripGUID00013051 metadata equals prior validated D2
parser output; direct pinned oracle canonical equality also passes with existing
IEEE bounds-bit rule. Unsupported staticGUID000161b2 exports explicit unsupported/
null metadata. All4native runs exit zero, three captures inspected; list repeat
byte-identical. Schemas valid, five bad mutations per supported report rejected.
No new parser/codec/oracle or manual Save/otherGPU/fullcorpus claim. Original
fingerprints/FARC ReadOnly/reference/ignore/noGit boundaries intact; no publishing
or approval rejection. Reproduction/scope: docs/mesh-export.md.

Next D20: bounded Back/Forward route history for MAP aliases, archive entries and
Overview, storing inspection tab; validated indices, no duplicate pushes, Forward
cleared by new route, both cleared by snapshot replacement. Centralize transition
cache/job cancellation and avoid rendering old detail after navigation; clear hiding
filters on restoration. Authored state/toolbar input checks and native captures.
No parser/source-format changes, original writes or dependencies.

## D20 bounded cross-mode navigation complete - 2026-10-05

Central checked route transition now handles MAP aliases, archive entries and both
overviews with tab restoration and Back/Forward capped at128 visits each. New
routes clear Forward; invalid/repeated destinations do not alter history; snapshot
replacement clears both. Resets cancel inspector/hex work and discard old cache,
retain validated source, reset pending jumps and queue restored list selection.
Toolbar/alias jumps stop rendering the old detail. Back/Forward clear hiding filters.

Five new authored checks;108workspace/34viewer tests/fmt/Clippy/build pass. Native
synthetic production-handler roundtrip and validated local mesh regression exit
zero; both screenshots inspected. Synthetic verifies both directions/tab state;
headless actual toolbar pointer events tested separately. Material route cancellation
verified. Initial harness texture-delta cleanup failure and two Clippy nested-if
warnings fixed; no weakened guards. Input fingerprints/FARC ReadOnly/reference/
ignore/noGit boundaries intact. No new format rules, original writes or publishing.
Limitations and commands: docs/navigation.md. Candidate/camera state is not restored.

Next D21: keyboard Back/Forward through the same checked transitions, visible
shortcut help and consumed events; synthetic checks for enabled/disabled/repeated
actions and typing preservation. No parser/dependency/source-format changes.

## D21 keyboard history navigation complete - 2026-10-05

Alt+Left/Right use checked Back/Forward before panel binding; visible shortcut help.
Exact Alt-only events consumed, repeats ignored, at most one new action per frame.
Plain/Shift/Ctrl arrows and text remain available to editors. Three new input tests
prove disabled/repeated actions, modifiers and focused caret/text insertion;
111workspace/37viewer tests/fmt/Clippy/build pass. Native synthetic roundtrip exits
zero; shortcut help screenshot inspected, no manual native keyboard claim.
Initial Rust2021 let-chain error corrected to match guard. Input fingerprints/
FARC ReadOnly/pinned reference/ignore/noGit intact. No parser/dependency changes.
Commands and limits: docs/navigation-shortcuts.md.

Next D22: existing C1 census has one clear32-bit TEX/DDS256x256 with standard
BGRA byte masks. Review existing pinned oracle/spec before adding bounded top-mip
decoding, synthetic channel/pitch/truncation/budget tests and native pixel parity.
Preserve DDS metadata/unsupported outcomes; no higher mips or GTF swizzle work.

## D22 TEX/DDS BGRA8 preview complete - 2026-10-05

Independent standard-mask32-bit DDS top-mip decoder; PixelFormat replaces the block-
only Rust enum, existing BC JSON names preserved. Handles documented row pitch and
strict locally corroborated legacy LINEARSIZE header, raw metadata retained. Source
budgets/cancellation/unsupported/protected outcomes intact; no GTF swizzle copying.
Four authored tests;115workspace/37viewer tests/fmt/Clippy/build pass. Initial
ambiguous numeric-offset compile error fixed with usize. Fresh pinned one-sample
oracle exactly matches RGBA and all DDS fields;67combined pinned texture samples
have0mismatches/0failures. Native RGBA/R captures inspected, schema valid, four bad
mutations rejected, cached output equals parity metadata and repeats byte-identically.
Inputs/fingerprints/FARC ReadOnly/reference/ignore/noGit intact. No dependency or
publishing change. Single BGRA local sample/padded rows synthetic only; no higher
mips, GTF uncompressed or GPU-fidelity claim. Contract: docs/bgra-preview.md.

Next D23: animation metadata reconnaissance using pinned RAnimation. Existing
snapshot has854 .anim MAP rows (extension only); establish accessible magic/revision
cohorts, bounded scalar/count prefix and external metadata oracle before independent
implementation. No playback/posing, executable investigation or guessed parser.

## D23 animation census and external prefix oracle complete - 2026-10-06

Snapshot-only selector:854extension aliases/748unique hashes, separately748confirmed
ANMb locations/21revisions/2,785,763stored bytes.35cohort samples;33modern prefixes,
2oldhead844 rejections. Full unmodified RAnimation corroborates scalar/count/optional
position/fat fields on23preflight-safe samples;10locator-array skips,0reader failures.
No playback/locator-record claim.205,629archived bytes/run, metadata only. Three
Python tests and Java revision/packed-length/truncation/actual-reader synthetics pass.
Census/manifest/final oracle repeat byte-identically. Sandbox canonical-path WinError5
resolved with scoped authorized execution, no weakened guard/rejection. Rust/UI
unchanged;115Rust tests last runD22. Input hashes/FARC ReadOnly/reference/ignore/noGit
intact. Provenance/conflicts/limits: docs/animation-prefix.md.

Next D24: independent modern CPU ANMb prefix reader using existing resource wrapper/
decompression/integer guards. Match33oracle prefixes; raw scalars/counts/optional
bits/nulls/opaque-byte metadata, explicit oldhead844 unsupported. Synthetic revision/
truncation/budget/cancel and local parity before native tab. No posing/playback.

## D24 independent animation prefix reader complete - 2026-10-06

CPU-only modern ANMb diagnostic module under lbp-resource, shared wrapper guards
and privately factored existing Material integer decoder. Raw scalars/counts,
optional position bits/fat/nulls and bounded opaque byte-length/start/tail counts;
no keyframes/locator records/semantics. Four authored cases;119workspace/37viewer
tests/fmt/Clippy/build pass.33local modern prefixes exactly match D23 pinned oracle,
2old sampled layouts explicit Unsupported; report repeat byte-identical30,715bytes.
32material-prefix regression matches, including shared integer path. No failures,
new dependency, UI or original writes. Input hashes/FARC ReadOnly/reference/ignore/
noGit unchanged. Scope/commands/provenance: docs/animation-parser.md.

Next D25: native Animation prefix inspector, validated source and bounded cancel/
cache, MAP/entry candidates, raw metadata/unparsed outcomes. Integrate route/source
reset; synthetic lifecycle/layout and native selected-oracle comparison. No playback.

## D25 native Animation prefix inspector complete - 2026-10-06

MAP/entry Animation tab, bounded validated-source worker/cache, raw scalar/count/
optional-bit/byte metadata, scrollable details and explicit unsupported outcomes.
Preflight rejects old/protected/non-ANMb before body reads; cancellation, short-page
guards and route/detach/snapshot reset. Four authored viewer tests plus shared
protection test;124workspace/41viewer tests/fmt/Clippy/build pass. Initial closure
type inference fixed. New protected-buffer test found existing exactly4byte-helper
caller mismatch: helper now checks leading4bytes of any buffer, preserving short/
interior behavior. No decryption attempted or guard weakened. Final scroll wrapper
verified with narrow tests/native after full suite. Four native captures inspected:
891/946MAP and946entry cached fields equal pinned oracle; old844Unsupported/no cache.
No manual scrolling/Reload/Cancel/candidate/otherGPU claim. No format/dependency or
original writes. Hashes/FARC ReadOnly/reference/defaultdeny/noGit intact.
Contract/reproduction: docs/animation-inspector.md.

Next D26: cached Animation metadata-only JSON export via shared private writer,
versioned envelope/schema/outcomes, synthetic refusal/reset and native oracle/repeat/
schema checks. No blob/keyframes or additional source reads.


## D26 cached Animation export complete - 2026-10-06

Fourth cached metadata-only exporter, explicit raw/null prefix projection, version1
schema, private exclusive bounded writer and UI/QA production path. Busy/incomplete/
reset refusals and sanitized outcomes; no blob/keyframes or source rereads. Two new
viewer tests;126workspace/43viewer tests/fmt/Clippy/build pass. Four native runs exit0:
three supported reports exactly match D23 pinned oracle fields; old844unsupported/
null. All schema-valid; repeat1310bytes identical; five schema mutations rejected.
Head946PNG inspected; no manual interaction claim. Inputs/reference/defaultdeny/
noGit unchanged. Contract/commands/limits: docs/animation-export.md.

Next D27: bounded level/plan header census from existing snapshot and pinned reader
study; establish safe diagnostic prefix/oracle coverage before any implementation.
No full Things, rendering, gameplay or executable investigation.


## D27 level/plan reconnaissance complete - 2026-10-06

Snapshot-only census:282confirmedlevels/45cohorts/44574615storedB;7342plans/97cohorts/
10428191B. Header dependencies parsed:45946level/47672plan entries. Names separate
from magic/type, explicit branch/flags/status/counts. Modern branch0plan sample
manifest48resources/28cohorts/313643B, repeat byte-identical; census49366B/manifest7946B.
Four authored Python tests pass. Initial non-escalated private-root WinError5 resolved
with scoped authorized execution. Pinned web-page fetch cache-miss; local clean
Toolkit source used. No original payload reads, new Rust/parser/GUI or full oracle
claim. Level immediately enters recursive Thing; plan has bounded opaque-array
prefix. Evidence/gates/conflicts/unknowns: docs/level-plan-recon.md.

Next D28: bounded pinned-reader external plan-prefix oracle; preflight ordinary
modern headers and nonempty dependency tables, wrapper/identifier/byte-array bounds,
metadata/hash/tail only. No full Thing/RLevel deserialization. Implement independent
plan prefix only after oracle evidence. No current product/toolchain blocker.


## D28 bounded external plan-prefix oracle complete - 2026-10-06

Authored PlanOracle.java/runner and explicitignoreallowlist; unchanged pinned clear
wrapper + MemoryInputStream i32/bytearray calls after strict bounds. No full RPlan/
RLevel or Things.48samples/28modernheads,313643readB, allparsedprefixes/zero rejected;
three30321B reports byte-identical. Synthetic flags/raw/zerochunk/unknowninner/duplicates/
truncation/revision/protection/dependency/integer/expansion guards pass. Initial
relative-seek adapter mistake caught in synthetic test; fixed SeekMode.Begin.
All48storedinner revisions equalouter (observation, notvalidationrule). Blob lengths/
hashes/tail only. Original hashes/FARCattrs/reference/ignore/noGit unchanged. No Rust
changes/checkclaim. Full rules/coverage/limits: docs/plan-prefix.md.

Next D29: independent CPU plan prefix via existing binary/integer guards, bounded
rawinner/opaquearrayhash/bounds/tail metadata, synthetic tests and48record external
oracle parity/repeat. No Thing/body/inventory semantic parsing or native UI yet.


## D29 independent CPU plan-prefix reader complete - 2026-10-06

lbp-resource::plan immutable bounded PLNb metadata reader, existing binary/integer
helpers unchanged, SHA256opaque array with4096byte cancellation points. Rawinner,
arraylength/start/hash, prefix/unparsed/tail counts; no payloadreturned or Things.
Four authored tests pass;130workspace/43viewer tests/fmt/Clippy/build pass.
48modern local prefixes allfieldequal D28externaloracle,313643readB/run; zero mismatch/
failure,30744B repeat identical. No newdependency/sharedformat/GUI changes. Original
hashes/FARCReadOnly/reference/defaultdeny/noGit preserved. docs/plan-parser.md.

Next D30: native Plan prefix inspector with validated bounded source worker/cache,
MAP/entry candidates, raw/hash/bounds/unparsed/outcomes, reset/cancel/navigation and
synthetic/native oracle checks. No inventory edits or Thinggraph/body interpretation.


## D30 native Plan prefix inspector complete - 2026-10-06

PlanMAP/entrytab and bounded cancellable validatedsource worker/cache, early32byte
preflight, rawinner/opaquearrayhash/bounds/tail/unparsed diagnostics. Route/source/
snapshot reset and Planentryhistory support. Four authored tests;134workspace/
47viewer tests/fmt/Clippy/build pass. Fournative runs exit0/PNGs inspected; supported
small/largeMAP and largeentry allcachedfields equal D28oracle, old622Unsupported/
no cache. No manualinteraction/allGPU/bodycompatibility claim. Originalhashes/
FARCReadOnly/reference/defaultdeny/noGit preserved. docs/plan-inspector.md.

Next D31: cached Plan metadata-only JSON export through shared privatewriter,
explicitprojection/version1schema/outcomes/refusalreset tests and native oracle/
schema/repeat. No original payload arrays, new reads or Thing/inventory semantics.


## D31 cached Plan export complete - 2026-10-06

Fifth cachedmetadata exporter, explicitprefix/hash/bounds projection/version1schema,
shared privatewriter/UI/QApath. Busy/incomplete/reset refusals; stable sanitized
outcomes, no Thing/inventory bytes or source reread. Twoauthored tests;136workspace/
49viewer tests/fmt/Clippy/build pass. Fournative runs exit0; threesupportedreports
allfieldequal D28oracle; old622unsupported/null. Allschema-valid, repeat979B identical,
fiveinvalid mutations rejected. LargePNG inspected, no manualinteraction claim.
Originalhashes/FARCReadOnly/reference/defaultdeny/noGit preserved. docs/plan-export.md.

Next D32: modernInventoryItemDetails scalarprefix/SlotID/signed64 rule study and
bounded pinned-reader oracle, stopping before recursiveCreationHistory. Establish
raw field/gate coverage before independent optional parser. No Things or edits.


## D32 external plan inventory-prefix oracle complete - 2026-10-06

Authored bounded PlanInventoryOracle/runner, pin/ignoreguards and actualSlotID +
communityinteger primitives, stopsbeforeCreationHistory.48modern samples pass,
313643readB, tworeports70846B identical; existingD28planprefixes unchanged. All local
date/slot/sound raw0, types/subtype/colour/keys vary; no semantic inference. Synthetic
flags/unknownbits/SlotID/truncation/u64overflow pass. Found/reproduced pinned compact
s64 arithmetic-shift conflict forhighbit encoded values; rawencodedhex preserves
wirevalue, no timestamp normalization. No localconflict occurrence. No Rust changes
or fullInventoryItemDetails/Thing deserializer claim. Inputs/reference/noGit/ignore
intact. Fullsources/rules/conflict/coverage: docs/plan-inventory-prefix.md.

Next D33: independent optional CPU planinventoryscalar-prefix stage, raw64encodedhex,
rawuint32slot/sound/colour/type/subtype/keys, boundedextents; no enumloss/date semantics/
references. Preservecurrentprefix/API and failure-stage distinction.48oracleparity,
prefixregression/repeat plus relevantRustchecks before nativeintegration.


## D33 independent optional plan inventory prefix complete - 2026-10-06

CPUinspect_inventory combinedstage, privateexistingprefix helper, no doubleinflation/
hash or publicpayloadbytes. Rawencoded64hex/raw32fields/extents; no enum/date/reference
semantics. Basevalid+inventoryResult separatesdeeperfailure/cancel; caller must honor
canceltoken before publish. Fournewtests;140workspace/49viewer tests/fmt/Clippy/build
pass. Initialnewzlibfixture had equalcompressed/expandedsize (storedchunkmarker),
correctedrecipe, parserrule unchanged.48inventoryoraclematches +48base matches,
zero mismatch/failure,61628B repeat identical; standaloneprefixregression byte-identical
D29report. Originals/reference/defaultdeny/noGit intact. docs/plan-inventory-parser.md.

Next D34: optionalnativePlan inventorymode using combinedCPUstage, separateerrors/
requestedstate, preservedbase on deeperfailure, boundedsource/cache/reset/cancel and
synthetic/nativeoraclechecks. Existingversion1export staysprefix-only; v2next.


## D34 optional native Plan inventory inspector complete - 2026-10-06

Added optional inventory mode, single-read combined worker, separate deeper errors,
base preservation, lossless raw-field UI and both-cache cancellation/reset handling.
143 workspace / 52 viewer tests, fmt, workspace Clippy and build pass. Final test-only
route assertions: nine Plan tests and viewer Clippy pass; format repair/check pass.
Five native runs exit 0; four inventory PNGs inspected. Three supported inventory
caches and four supported base caches exactly match D32 oracle; old622 unsupported
without cache. Two v1 exports schema-valid and byte-identical to D31 (979 bytes).
Original hashes/FARC read-only/reference pin/default-deny/no-root-Git preserved.
No manual UI interaction or full corpus/Thing claim. docs/plan-inventory-inspector.md.

Next D35: version2 cached Plan diagnostic envelope with requested mode, explicit
base/inventory stage outcomes and projections, deeper-failure base retention,
shared privatewriter/refusals, synthetic/schema/native oracle/repeat checks.
Retain v1 schema; no recursive/date/title semantics. No current blocker.


## D35 stage-aware Plan diagnostic export complete - 2026-10-06

Version2 cached base/inventory metadata projections, requested mode, explicit stage
outcomes and partial base preservation; incomplete/contradictory caches refused.
Version1 schema retained (description typo fixed only). Two newtests;145workspace/
54viewer tests, fmt/Clippy/build pass. Initial test Clone assumption fixed by move/
expected projection, no CPU change. Five native runs exit0; largePNG inspected.
Four base/three inventory projections match D32oracle; all schema-valid,1687B repeat
identical. Four synthetic partial outcomes valid; ten invalid mutations rejected.
Historical v1 valid. Hashes/FARC read-only/reference/default-deny/noGit intact.
No manual UI/full-corpus claim; docs/plan-export-v2.md. No current blocker.

Next D36: deterministic MAPresource sort (original row/path/GUID/SHA1), stable ties,
virtualized filtered lists, selection/alias/navigation invariants; synthetic and
native verification. Practical browsing improvement, no new format or source reads.


## D36 deterministic MAP resource sorting complete - 2026-10-06

Added MAP-row/path/GUID/SHA1 list ordering and --sort option. Case-preserving paths,
case-folded hex IDs, stable row/index ties; aliases and underlying catalog indices
preserved. Cached filtered virtualization, selection jumps, navigation/reload retained.
Two new tests;147workspace/56viewer tests, fmt/Clippy/build pass. Initial modified
synthetic row IDs broke dependency links; repaired fixture links without weakening
validation. Five snapshot-only native runs exit0; first10/count/selection match
independentPython ordering.36854rows/allorders,7342filteredplans; twoPNGs inspected.
Invalid --sort rejected exit1. No original source attached during native checks.
Hashes/FARC read-only/reference/default-deny/noGit intact. docs/resource-sorting.md.

Next D37: archiveentry table/offset/size/SHA1 sorting with documented crossarchive/
tie ordering, orphan/alias/selection/filter/navigation invariants, authoredmultiarchive
checks and native snapshot-only expectedordering captures. No format or source reads.
No current blocker; maximum1millionrow sorting latency not benchmarked.


## D37 deterministic archive-entry sorting complete - 2026-10-06

Table/offset/size/SHA1 global ascending presentation order, ties archiveorder then
originalentrynumber/vectorposition. Aliases/orphans/selection/filter/navigation
preserved; cached virtualized list and --entry-sort. Two authoredmultiarchive tests;
149workspace/58viewer tests, fmt/Clippy/build pass. Helperlegacyencoding failure fixed
with explicitUTF8 and punctuation restored; finalchecks pass. Five snapshot-only
native runs exit0, first10/count/selection/aliases match independentordering.31534all,
1hashfilteredselection; twoPNGs inspected. Invalidoption exit1. No originalattached.
Hashes/FARC read-only/reference/defaultdeny/noGit intact. docs/archive-sorting.md.
No current blocker; maximum-budget sorting latency not benchmarked.

Next D38: bounded literal MAPpath-prefix folder navigation, childfolders/counts and
parent/root breadcrumbs; combinedfilters/sorting/alias/index preservation. Explicit
unusualpath handling, authored boundary/lifecycle tests and native snapshot-only
counts/selection. No filesystem traversal or new format/source reads. Highertexture
mips remain deferred pending independentoffset evidence (knownToolkit mipbug).


## D38 literal MAP folder browsing complete - 2026-10-06

Added one-level childfolders/counts, Root/Up/prefix breadcrumbs and --folder, combined
with filters/sorting. Metadataonly; unusualpaths stayroot, bounded4096B/32components
links, depth/byteexhaustedrows preserved. Virtualized children and resource lists,
alias/index/detail preservation; navigationreveal/reset rules documented.
Three authoredtests;152workspace/61viewer tests/fmt/Clippy/build pass. Finaltest-only
budget/filter assertions: focusedthree/fmt/viewerClippy pass. Four snapshot-only
native runs exit0; allcount/first10child+resource/order/selection match independent
Python.36854root/36852gamedata/13selectedplans/0absent; twoPNGs inspected.
Invalidprefix/checkcombo rejectedexit1. No originalattached; hashes/FARC read-only/
reference/defaultdeny/noGit intact. docs/folder-browsing.md. No current blocker.

Next D39: optionalneutral solidmesh preview from verifiedposition/topology alongside
wireframe, UI-side only, generatedfaceshading, explicitocclusion/fidelitylimits and
existingbudgets/lifecycle. Authoredgeometry/extreme/degenerate checks + meshmetadata
regression/native supported/unsupported captures. No parser/material/bone/morph work.


## D39 optional neutral solid mesh preview complete - 2026-10-06

Added two-sided generatedflatshading and boundedaverage-depth sortedtriangle submission
alongsidewireframe; --mesh-view option, mode retainedonreset, no parser/newdependency.
Explicitnumerical/index/budget errors and degeneratefacecounts; UIstatesocclusionlimit.
Two new tests;154workspace/63viewer tests/fmt/Clippy/build pass. Four native runs exit0;
list/strip/unsupportedPNGs inspected. List8faces;strip2472faces+168degenerates; unsupported
no scene. AllfourJSONschema-valid/byte-identicalD19 baselines, includingwireframe.
Expectedinvalidmodeexit1 causednativecheckloopfinalexit1; not a nativefailure.
Helpmojibake dash fixed. Hashes/FARC read-only/reference/defaultdeny/noGit intact.
No manualinteraction/fullcorpus/material/depthcorrectness claim. docs/mesh-surface.md.
No current blocker; maximum-budgetredrawperformance not benchmarked.

Superseded by user steering: proposed D40 mesh focus-preview toggle and orthographicaxis presets, freeingcanvasheight
and inspectingflat/edge-onresources. Retainorbit/zoom/modes/metadata/source lifecycle/
export. Syntheticaxis/lifecycle checks + nativefocusedcircuitboard/default/unsupported
captures. No new format/material/normal/bone parsing or rendererdependencies.

## D40 first material-linked textured mesh complete — 2026-10-06

User requested substantive asset compatibility instead of more optional UI polish.
Added independent checked UV reader and first Direct diffuse preview: ordinary MSH
one primitive → unambiguous GMT → exactly one direct texture sample → decoded top
mip on UV0. Identity/unanimated/unswizzled only, repeat/clamp matching axes. No game
shader/lighting/alpha/bone/morph fidelity claim. Base geometry survives linked-stage
failure; source/reset/cancel clears caches. Wireframe/neutral paths retain no linked
reads. No new renderer dependency or copied community implementation.

36 UV samples match pinned Toolkit full-channel getter; oracle repeat byte-identical.
Pinned partial-range getter has a demonstrated byte-seek bug, avoided independently.
Floor 000150e5 → material 00014c31 → texture 000150eb previews successfully:
24 vertices/8 triangles, GTF BC1 512×256, exact top-mip pixel oracle parity. Four native
regression captures exit0; unsupported multi-primitive/SMH boundaries explicit.
All four exports schema-valid/byte-identical to D39. PNGs inspected, no manual input
claim. 153 Rust tests including67viewer pass; fmt/Clippy/build pass. See exact commands,
provenance, limits and private artifact details in docs/textured-mesh.md. This supersedes
earlier aggregate totals;153 is the exact current Rust log sum. Final floor-label
capture exits0 and preserves the same export bytes; final fmt check passes.

Failed attempts resolved: PowerShell adapter argument syntax; LocationKey implicit
serialization compile error (explicit fields instead); Clippy constant-chunk iterator
warning (array chunks). No unchanged failed experiment needs repeating. SFO/MAP hashes,
FARC read-only size, clean pinned reference, ignore audit and no-root-Git unchanged.

Next D41: multi-primitive direct-diffuse meshes. Preserve primitive ownership in the
CPU geometry IR, verify triangle partitions against the pinned oracle, bind each
eligible primitive independently, keep unsupported groups visible as diagnostic
geometry, and test ambiguity/cancellation/budgets plus native supported/partial cases.
Reuse D40's proven graph subset; no general graph evaluator, level renderer or runtime.
No current blocker. Broader graph/shader fidelity and full corpus remain unverified.

## Bevy frontend recommendation / revised next task — 2026-10-06

User asks whether to transition to real Bevy model previews. Recommendation: yes,
stage the transition around the verified D40 floor slice. D41 now means shared CPU
preview loader plus a small Bevy consumer with depth-tested perspective/orbit and
the same unlit direct-diffuse material. Keep parser/catalog/source rules and eframe
diagnostics intact; inspector panels migrate after the 3D slice works. Multiple
primitive bindings become D42. No Bevy code/dependency/build claimed in this pass.
Official Mesh/StandardMaterial and bevy_egui integration/support-table research
recorded in docs/bevy-transition.md. Documentation-only change; no Rust checks needed.
Existing visible explorer/user test session left running; no game-data writes.

Next-chat handoff review: add a minimal synthetic Bevy build/render gate on the
existing Windows GNU toolchain before extracting the working preview loader. Official
Windows setup uses MSVC; compatibility is not established by eframe success. There
is no root rust-toolchain.toml. Then shared CPU extraction/eframe regression, the
known floor with minimal GUID/status/camera controls, adapter/lifecycle checks and
native verification. Full browser migration/multiple materials remain later. Details
in docs/bevy-transition.md. Documentation-only review; no Bevy compilation claimed.

## User research-policy revision — 2026-10-06

User explicitly permits local decryption and decompilation research on their own
game copy. Earlier blanket prohibitions are superseded by current AGENTS.md.
Original inputs remain read-only; decrypted files/decompiler output/keys/research
artifacts stay ignored and outside distribution. Product contains no original LBP
code or assets, and no recovered-source copying/adaptation/translation. Independently
expressed behavior/format findings may inform original implementation; absence of
copied code alone is not a strict clean-room-process claim.

No decryption/decompilation was performed in this policy pass, and existing reader
protected outcomes were not changed. D41 Bevy preview still uses accessible verified
data and does not need executable research. Eventual ISO-sourced runtime assets remain
the goal; D41's extracted-directory path is not a claim of implemented ISO loading.

Full updated new-chat implementation prompt is saved in docs/next-chat-prompt.md.
It incorporates the research-permissions revision and the reviewed D41 acceptance
checks. Use this version instead of the earlier chat prompt containing the blanket
decryption/decompilation prohibition. Documentation-only handoff update.

## D41 Bevy preview implemented and verified — 2026-10-06

Implemented the revised D41 Bevy slice, superseding the earlier pending decision.
Bevy 0.19.1 / bevy_egui 0.42.0 exact pins and Cargo.lock resolution; existing Windows
GNU Rust 1.99.0 works. Minimal authored native scene compiled and its inspected
depth capture showed foreground red occluding background blue before loader extraction.
No MSVC/toolchain replacement or unrelated stack installation.

Added renderer-independent lbp-preview, moving the existing direct-diffuse predicate,
candidate resolver and bounded mesh preparation from eframe. Same parser/source
budgets and unsupported/protected/ambiguous outcomes; short-page mesh-loop guard added.
Both frontends use the shared loader. No decryption/executable research was needed
or performed. Original LBP code/assets/keys are absent from product additions.

lbp-bevy-viewer accepts --snapshot, --game-dir, --select-guid; background snapshot/
attachment/asset preparation keeps rendering responsive. GUID/source load/reload,
cancel, visible outcomes/identity/stats, perspective orbit/pan/zoom/reset/frame.
Scene replacement removes owned entities/Mesh/material/Image assets; job replacement
cancels its token and drops the receiver. Source-path changes invalidate context.
Unit tests cover cancellation/stale receiver and explicit owned GPU asset removal.
No manual native interaction claim; rapid GUI reload/source replacement stress,
other GPUs/platforms and hostile-filesystem races remain untested.

Adapter preserves vertex/index/UV pairing, raw CPU geometry/UVs, original axis/order
and performs display-only f64 centering/normalization. Double-sided, unlit, opaque,
sRGB, no tonemapping, top mip only; generated bounded diagnostic normals labeled.
Finite/invalid/degenerate/index/UV/texture-budget tests pass. Native authored scene
shows top-left UV orientation, repeat versus clamp, opaque display of alpha64 yellow,
and depth occlusion with foreground submitted before background. Native floor shows
24 vertices/eight triangles/one primitive and 512×256 BC1/Repeat. Four-primitive
00013051 retains neutral geometry with explicit unsupported material; 000161b2
reports unsupported static layout. Inspected PNGs remain private local-analysis.

Exact checks (after activating . ./scripts/use-local-rust.ps1):
- cargo build -p lbp-bevy-viewer (initial feasibility): passed, 4m02s.
- cargo test --workspace: final rerun passed, 157 tests; d41-tests-final.log.
- cargo test -p lbp-bevy-viewer -p lbp-preview -p lbp-viewer: passed, 71 tests;
  d41-focused-tests.log (65 viewer, two preview, four Bevy).
- cargo clippy --workspace --all-targets -- -D warnings: passed;
  d41-clippy-final.log.
- cargo fmt --all --check: passed; d41-fmt-final.log.
- cargo build -p lbp-bevy-viewer -p lbp-viewer: passed; d41-build-verified.log.
- lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir
  game-data/a1-disc --select-guid 000150e5 --mesh-view textured --qa-tab mesh
  --qa-capture local-analysis/d41-eframe-floor.png --qa-mesh-json
  local-analysis/d41-eframe-floor.json: native exit0, inspected; JSON byte-identical
  to d40-floor-label.json, SHA256 636b78a5cbf37ec2650069d2c0f4a553f37783008a403919fb8a06d000460be1.
- lbp-bevy-viewer.exe --synthetic --qa-capture local-analysis/d41-synthetic-final.png:
  native exit0, inspected.
- lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir
  game-data/a1-disc --select-guid 000150e5 --qa-capture local-analysis/d41-bevy-floor.png:
  native exit0, inspected. Same command with GUID00013051/d41-bevy-partial.png and
  GUID000161b2/d41-bevy-static.png: exit0, inspected fallback/error.

Resolved failures: sandbox DNS blocked initial Cargo downloads; authorized network
build succeeded. Sandbox denied rustfmt and four existing synthetic attachment tests;
filesystem-capable reruns passed without product safety changes. egui 0.36 renamed
input-focus methods; corrected. Clippy UI adjacent-if formatting corrected. An optional
Python jsonschema check could not run (module absent); no schema-validation rerun
claimed. Export equality to the existing validated baseline and Rust export tests pass.
No automatic approval rejection. No proprietary fixture/oracle outputs added.

Root remains without Git metadata. Read-only SFO/MAP SHA256 match docs/local-data.md;
FARC stays read-only at 1,023,990,298 bytes. Existing default-deny check-ignore permits
authored new Rust/crate/docs and excludes captures/game data. No input writes, Git
init/staging/commit/push/release/publication. Bevy native backend/device identity was
not separately logged; eframe regression logs RTX3080/Vulkan. Existing pinned oracle
evidence reused rather than rerun: shared loader introduces no new format rules.

Launch and first-user-test walkthrough: docs/bevy-preview.md. Interactive floor
preview opened at handoff; unrelated applications remain untouched.
Next D42: preserve primitive ownership/material groups using this proven pipeline.
Broader graphs, alpha/game-color/lighting fidelity, skinning, levels/gameplay and
direct ISO loading remain deferred.

## D42 primitive ownership and material groups — 2026-10-06

Continued from completed D41. CPU Geometry now retains one triangle span per
primitive, including empty/degenerate/overlapping source ranges, while existing
metadata/digests/flat order/export remain unchanged. Shared lbp-preview adds a
multi-primitive loader with per-group outcomes, deterministic first-eight distinct
material attempt budget and within-load success/failure cache. Existing zero-flag,
identity UV0 direct-diffuse predicate is reused; nonzero alternative flags now
explicitly reject alongside alternative kinds. Eframe stays single-primitive.

Bevy now pools draws by cached binding and combines unsupported groups in one
neutral draw; per-primitive diagnostics preserve identity/count/reasons. Full mesh
bounds align every group. Adapter rejects gaps/overlaps in the triangle partition
and binding-count mismatches before GPU allocation. Scene owns/removes all group
entities/Mesh/material/Image handles; four-group cleanup test passes. Maximum eight
material attempts/nine draws, stored reads <=138 MiB, cumulative expansion work
<=464 MiB, retained CPU pixels <=128 MiB; temporary image copies mean these are not
total-process-memory estimates. Details: docs/primitive-groups.md.

Pinned oracle d6a2ef80b5a09999f08f23a803b516096055e5c9 run via authored existing Java
adapter: 36 samples, 82 primitives, 16 multi-primitive meshes, 605145 archived bytes.
Primitive spans and canonical triangle SHA256 match exactly: zero mismatches/failures.
All 36 geometry metadata records also equal the previous D2 oracle records.
No proprietary code/assets/keys copied into additions. No decryption/decompilation
needed/performed; original inputs remain read-only. New knowledge is CPU ownership
corroboration, not general material semantics or original rendering fidelity.

Exact checks after . ./scripts/use-local-rust.ps1:
- ./scripts/run-mesh-oracle.ps1 -Groups -Output local-analysis/d42-groups-oracle.json:
  passed, 36 samples / 605145 bytes.
- cargo run -p lbp-catalog --example mesh_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d42-groups-oracle.json local-analysis/d42-groups-parity.json
  --groups: passed, 36 compared / zero mismatches / zero failures; d42-groups-parity.log.
- cargo test --workspace: passed, 160 tests; d42-tests-final.log (three added tests:
  primitive spans, repeated binding cache/budget, mixed draw ownership/alignment;
  existing scene cleanup test extended to four groups).
- cargo fmt --all --check: passed; d42-fmt-final.log.
- cargo clippy --workspace --all-targets -- -D warnings: passed; d42-clippy-verified.log.
- cargo build -p lbp-bevy-viewer -p lbp-viewer: passed; d42-build.log.
- target/debug/lbp-bevy-viewer.exe --synthetic-groups --qa-capture
  local-analysis/d42-synthetic-groups.png: exit0, inspected red/green texture groups,
  shared red binding and neutral unsupported group aligned correctly.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00013051 --qa-capture
  local-analysis/d42-bevy-board.png: exit0, inspected four neutral primitive groups
  (892/1062/415/271 triangles), each with explicit complex-graph rejection.
- Same Bevy command with 000150e5 / d42-bevy-floor.png: exit0, inspected 24 vertices /
  eight triangles / UV0 / 512x256 BC1 / Repeat.
- target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 000150e5 --mesh-view textured --qa-tab mesh
  --qa-capture local-analysis/d42-eframe-floor.png --qa-mesh-json
  local-analysis/d42-eframe-floor.json: exit0, inspected. Export SHA256
  636b78a5cbf37ec2650069d2c0f4a553f37783008a403919fb8a06d000460be1, identical D41/D40.

Resolved experiments: sandbox rustfmt AccessDenied, handled by filesystem-capable
rerun. Initial authored edit used Windows default encoding and then doubled CRs;
repaired to UTF-8/LF, restored display punctuation, format/tests/build passed.
Clippy caught tests-after-items and one-range container spelling; corrected without
lint suppression. Final CLI-help wrapping was formatted after the usage-text edit;
fmt/Clippy reruns pass. Private logs retain these failures. No auto-review rejection.
No repeated unresolved approach/toolchain changes or game input writes.

SFO/MAP hashes unchanged against docs/local-data.md; default-deny ignore audit excludes
private oracle/captures and game data, permits authored source/docs. No root Git
initialization, staging/commit/push/release/publication. Native group captures use
Bevy's existing backend without separately logged GPU/backend identity; no new
GPU/platform claim. Manual interaction, rapid GUI reload stress, real multi-texture
supported asset, full corpus and game visual fidelity untested. Synthetic mixed
binding display plus real single-texture floor are narrower evidence.

Updated interactive Bevy floor opened at handoff. Next: use per-group unsupported
graph diagnostics to select and evidence one additional common material subset;
no general graph evaluator, gameplay or runtime expansion. D42 contract/provenance,
budgets and limitations: docs/primitive-groups.md.

## D43 diffuse-only graph projection — 2026-10-06

Continued D42 with one bounded material subset: a direct identity-UV0 texture wired
into diffuse may now coexist with ignored material branches. Exactly one output /
unique diffuse wire, zero swizzle/source port, subtype zero and no selected sampler
inputs/animation; multiply/mix/color/ThingColor and transforms remain unsupported.
Malformed endpoints reject even on ignored branches. Both offset zero signs are
accepted with raw parameter bits preserved. Shared CPU Asset records selection and
ignored counts; both frontends visibly label diffuse-only projection and ignored
nodes/wires. Existing per-group caching/budgets/read-only access and GPU lifecycle
remain unchanged; no additional branch resources are loaded.

Pinned RGfxMaterial selection helpers called on bounded reader-produced structures:
32 established material samples / 232687 bytes, zero route index/kind/slot mismatches;
14 pass projection policy, ten with additional nodes. All original graph records
remain identical to D7. For a visible example, private manifest selected the first
48 distinct material GUIDs encountered in D42's 36-mesh primitive order, requiring
one hash/one FARC location, no loose/conflicting candidates and <=1 MiB inputs.
48 linked materials / 404629 bytes, zero route mismatches; eight pass projection
policy. These are two sample sets (possibly overlapping), not a full corpus count.
Graph eligibility does not assert linked texture/mesh compatibility.

Observed real asset improvement: bottle top 00003114, 536 vertices / 531 triangles,
256x256 BC1 Repeat, ignores three nodes / three wires. Inspected Bevy capture clearly
shows the colored cap texture; eframe capture also shows it and the projection
label. Garage light 00013edb: 480 vertices / 459 triangles, primitive0 431 triangles /
128x128 BC1 Repeat / ignores one node/wire; primitive1 28 triangles remains neutral
with explicit unsupported reason. Inspected native capture shows textured housing
and neutral center. Circuit board primitive2 now loads 256x256 BC1 / 415 triangles /
one ignored node/wire; other three groups remain neutral. Initial default view
largely occludes that group, so board is diagnostic load evidence, not a strong
visible-texture proof. Bottle/light were selected to resolve that verification limit.

Exact checks after . ./scripts/use-local-rust.ps1:
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer: passed, 73 tests;
  d43-focused.log. Existing predicate test expanded with ignored branches/cycles,
  selected-input/duplicate-output/duplicate-wire rejection, signed/nonfinite offsets,
  node-kind/subtype/port/animation/swizzle guards. Existing test count is unchanged.
- ./scripts/run-material-oracle.ps1 -Diffuse -Output local-analysis/d43-diffuse-oracle.json:
  passed, 32 samples / 232687 bytes, existing synthetic guards and known signed oracle
  conflict retained. Output graph metadata equals prior D7 records.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d43-diffuse-oracle.json local-analysis/d43-diffuse-parity-final.json:
  passed, 32 compared / zero mismatches / 14 projected; d43-diffuse-parity-final.log.
  Initial report d43-diffuse-parity.json also agrees; final adds selected-index/slot checks.
- ./scripts/run-material-oracle.ps1 -Diffuse -SampleManifest
  local-analysis/d43-linked-material-samples.json -Output local-analysis/d43-linked-material-oracle.json:
  passed, 48 samples / 404629 bytes.
- Same diffuse_parity command with d43-linked-material-oracle.json /
  d43-linked-material-parity.json: passed, 48 compared / zero mismatches / eight projected;
  d43-linked-material-parity.log.
- cargo test --workspace: passed, 160 tests; d43-tests.log.
- cargo clippy --workspace --all-targets -- -D warnings: passed; d43-clippy.log.
- cargo fmt --all --check: passed; d43-fmt.log.
- cargo build -p lbp-bevy-viewer -p lbp-viewer: passed; d43-build.log.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00013051 --qa-capture
  local-analysis/d43-bevy-board.png: exit0, inspected; limitation above. Same command
  with 00003114/d43-bevy-bottle.png, 00013edb/d43-bevy-garage-light.png and
  000150e5/d43-bevy-floor.png: exit0, inspected supported/neutral/floor results.
- target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00003114 --mesh-view textured --qa-tab mesh
  --qa-capture local-analysis/d43-eframe-bottle.png --qa-mesh-json
  local-analysis/d43-eframe-bottle.json: exit0, inspected. Same with 000150e5 /
  d43-eframe-floor.png / d43-eframe-floor.json: exit0, inspected; floor export SHA256
  636b78a5cbf37ec2650069d2c0f4a553f37783008a403919fb8a06d000460be1, identical D42/D41/D40.

No compile/test/Clippy/oracle failures in this milestone. No decryption or executable
research needed/performed; no original LBP code/assets/keys bundled. Community shader
source was read as behavior evidence, never copied/adapted/translated into product.
Product parser remains independent; no strict clean-room-process claim. Oracle
selection helpers choose first duplicates; product rejects them explicitly. Raw
shader data remains opaque, other material branches deliberately unevaluated.

SFO/MAP hashes unchanged; reference Git source clean; root still has no Git metadata.
Default-deny audit permits authored code/docs and excludes local evidence/game trees.
No staging/commit/push/release/publication or game input writes. Captures/logs/private
sample manifests remain ignored. Bevy backend/GPU identity not separately logged;
eframe logs RTX3080/Vulkan. Manual camera/reload interaction, other GPUs/platforms,
full corpus, animation, game alpha/lighting/material fidelity remain untested.
Stored pose may occlude groups. No pixel-equivalent original-game claim.

Interactive updated Bevy bottle preview opened at handoff. Launch, provenance,
policy/conflicts/test scope: docs/diffuse-projection.md. Next D44: bounded direct-
diffuse UV1 channel selection using the already verified interleaved UV arrays;
validate with a local sample and retain original CPU arrays/channel identity.
Affine transforms and new graph operations remain separate evidenced milestones.

## D44 direct-diffuse UV1 sampling — 2026-10-06

Shared selector accepts raw channel0/1 only and retains original channel identity.
Loader validates/clones requested raw coordinates without mutation, reordering,
transforms or UV0 fallback; missing/invalid UV1 rejects before texture reads. Both
frontends label UV0/UV1 explicitly; Bevy feeds selected coordinates to its normal
single diffuse attribute. Eframe remains single-primitive. No parser layout or
geometry/export changes, new graph operations or budget expansion.

Material external route/channel helper comparison: established 32 samples /232687
bytes, zero mismatches /15 graph-eligible (14 UV0/one UV1); separate48 linked sample
set /404629 bytes, zero mismatches /14 eligible (eight UV0/six UV1). Sets may overlap;
eligibility does not prove linked texture/mesh compatibility. UV full-channel oracle:
36 meshes /605145 bytes, zero mismatches/failures; UV metadata unchanged from D40.
Valkyrie384 vertices has three distinct UV channel digests, selected channel1 retained.

Native Bevy Valkyrie00010e8d: 384 vertices/693 triangles/two UV1 textured primitives,
350 triangles/256x256 BC3 +343 triangles/128x128 BC3, Repeat, each one ignored node/
wire. Capture inspected: visible distinct textured parts. UFO00020155: 880 vertices/
1168 triangles/six primitives, one56-triangle UV1 group/32x256 BC1/three ignored
nodes/four wires; five unsupported groups neutral. Capture inspected; stored pose
not transformed. This also verifies a real asset with multiple supported texture
bindings, previously only synthetic in D42. No game pose/lighting/alpha fidelity claim.

Exact checks after . ./scripts/use-local-rust.ps1:
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer: passed,74 tests;
  d44-focused.log. One new raw UV-selection/no-fallback test; predicate test extended.
- ./scripts/run-material-oracle.ps1 -Diffuse -Output local-analysis/d44-diffuse-oracle.json:
  passed32 samples/232687 bytes, channel parameter included in external evidence.
- Same with -SampleManifest local-analysis/d43-linked-material-samples.json -Output
  local-analysis/d44-linked-material-oracle.json: passed48 samples/404629 bytes.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d44-diffuse-oracle.json local-analysis/d44-diffuse-parity.json:
  passed32/zero mismatches/15 eligible; d44-diffuse-parity.log. Same with
  d44-linked-material-oracle.json/d44-linked-material-parity.json: passed48/zero/14;
  d44-linked-material-parity.log. Census output v2 now retains channel fields;
  old oracle route fields remain compatible but do not assert new channel evidence.
- ./scripts/run-mesh-oracle.ps1 -Uv -Output local-analysis/d44-uv-oracle.json: passed,
  36 samples/605145 bytes; synthetic channel/negative-zero guard and pinned nonzero-
  start getter conflict still verified.
- cargo run -p lbp-catalog --example mesh_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d44-uv-oracle.json local-analysis/d44-uv-parity.json --uv:
  passed36/zero mismatches/zero failures; d44-uv-parity.log.
- cargo test --workspace: passed161 tests; d44-tests.log.
- cargo clippy --workspace --all-targets -- -D warnings: passed; d44-clippy.log.
- cargo fmt --all --check: passed; d44-fmt-final.log.
- cargo build -p lbp-bevy-viewer -p lbp-viewer: final pass; d44-build-final.log.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00010e8d --qa-capture
  local-analysis/d44-bevy-valkyrie.png: exit0, inspected. Same with00020155/
  d44-bevy-ufo.png: exit0, inspected supported/neutral groups.
- target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00010e8d --mesh-view textured --qa-tab mesh
  --qa-capture local-analysis/d44-eframe-valkyrie.png --qa-mesh-json
  local-analysis/d44-eframe-valkyrie.json: exit0, inspected explicit one-primitive
  limitation/retained metadata. No real single-primitive UV1 eframe sample verified.
- Same eframe command with000150e5/d44-eframe-floor.png/d44-eframe-floor.json: exit0,
  inspected UV0 floor; export SHA256636b78a5cbf37ec2650069d2c0f4a553f37783008a403919fb8a06d000460be1,
  byte-identical D43/D42/D41/D40.

Resolved experiments: two native build attempts could not replace locked Bevy exe
(d44-build.log/d44-build-retry.log). Restricted process listing showed no process;
elevated listing found task-owned D43 PID22312. After verifying its workspace exe
path, closed that preview only; final build succeeded. No unrelated apps closed,
no toolchain or filesystem safety changes. Metadata candidate query initially hit
optional null confirmed_type; null guard corrected, then read-only dependency query
found UFO candidate. No automatic approval rejection, test/Clippy/oracle failures.

Current sampler evidence maps raw1 to second UV pair; Toolkit's other-selector
fallback is deliberately not adopted (product rejects>1). Existing nonzero-start
UV getter conflict unchanged. Community source consulted as behavior evidence;
no copying/adapting/translating shader/recovered game code. No decryption/executable
research needed/performed; original inputs read-only, no original LBP code/assets/
keys bundled. No clean-room-process claim. SFO/MAP hashes unchanged; reference source
clean; root still no Git metadata. Ignore audit permits authored files/excludes
local evidence/game trees. No staging/commit/push/release/publication.

Manual camera/reload interaction, native stress, other GPU/platform/corpus and game
fidelity remain untested. Eframe device logs RTX3080/Vulkan; Bevy backend identity
not separately logged. UV1 eframe native rendering unverified because selected real
samples have multiple primitives; shared CPU selection/test and UV0 regression pass.
Updated interactive Bevy Valkyrie UV1 preview opened at handoff.

Next D45: bounded finite affine scale/offset sampling for direct UV0/UV1 texture
routes, using pinned subtype-zero sampler evidence and a controlled local example;
retain original UV arrays and label derived sampling coordinates. Subtype1 matrix
transforms/animation/wired sampler inputs/general graph evaluation remain deferred.
Contract/provenance/raw-coordinate policy: docs/uv1-preview.md.


## 2026-10-06 — Post-D44 clickable launcher and Bevy asset browser

User-requested usability work: authored root Launch Asset Explorer.cmd delegates to
scripts/launch-viewer.ps1, resolves its own checkout location, checks local
prerequisites, activates the existing runtime environment and opens the built
viewer. No installation, machine execution-policy change, game modification or
publication. Default-deny ignore policy permits this exact launcher only.

Added a searchable, virtualized catalog menu with show/hide files without a
current preview. Default filtering retains unchecked ordinary mesh candidates;
confirmed type overrides extension inference. Successful display means geometry,
not complete material/game fidelity. All MAP rows/aliases remain distinct. Failed
attempts are source-scoped and optionally visible with their reason; cancellation
does not cache failure. Source edits invalidate results. Attachment now succeeds
independently of preview failure, and optional GUID permits attachment-only browse.
No new format parser, interpretation, proprietary assets/code/keys or recovered
source; D45 affine sampling remains deferred.

Verification (project-local GNU Rust environment activated):
- cargo fmt --check; cargo test -p lbp-bevy-viewer: seven tests pass, including
  confirmed/hinted/unchecked status, reveal-all rows, failure cache reset and scene
  cancellation/cleanup. cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings
  passes. cargo build -p lbp-bevy-viewer passes.
- cargo test -p lbp-preview -p lbp-catalog: 14 tests pass. No parser changes.
- Native CLI capture with snapshot a2-inventory-verified.json, game a1-disc,
  GUID00010e8d and --browse-query valkyrie: four browser matches; selected UV1
  textured geometry visibly rendered alongside menu. Capture inspected locally.
- Native GUID000161b2 with --show-unavailable --browse-query guid:000161b2:
  unsupported ordinary-binary-MSH outcome, retained catalog, visible failure row.
- Final attachment-only --qa-capture local-analysis/browser-attach-only-final.png:
  2731 shown / 36854 catalog rows, blank GUID, attached status; inspected layout.
  Logs/captures remain ignored local diagnostics, not distributable fixtures.
- CMD launcher invoked with Windows temp working directory: exit0, responsive
  LBP2 Asset Explorer window. Closed only that verified task-owned viewer to finish
  the waiting process-tree test; final launcher can be reused by double-click.
- Isolated ignored Git ignore audit permits authored CMD/PS1/Rust, denies unknown
  CMD/private capture/game/reference source. Root still has no Git metadata.
  SFO/MAP SHA256 unchanged from prior entries; original inputs remain read-only.

Initial sandbox rustfmt canonicalization denial was resolved using reviewed local
execution. An approval-review usage-limit failure delayed one check; retry after
user continuation succeeded. Initial egui API compile errors were corrected using
installed dependency signatures. Incremental hard-link cache warning is incidental;
no remaining code/check failure. Native mouse/keyboard interactions, scrolling
stress, other installations/platforms and renderability of the whole corpus were
not exhaustively tested. No commit, push, release or publication.


## 2026-10-06 — D45 bounded static diffuse UV scale/offset

Implemented the next support milestone requested by the user. Direct, unanimated
subtype-zero diffuse samplers now accept bounded finite scale/offset on UV0/UV1.
Raw selected UVs and original material parameter bits are preserved; a separate
sampling array applies component-wise UV × scale + offset. Identity axes preserve
signed zero. Both Bevy and eframe label raw/derived sampling and use the new array.
Negative/zero scale is supported; nonfinite/overflow/budget/cancel outcomes reject.
Existing matrix/animation/wired-input/swizzle/descriptor/ambiguous-source guards
and eight-binding group limits remain. Original pose/lighting/alpha fidelity is
not established. Contract, per-rule source/pin/conflicts/bounds: docs/affine-preview.md.

Research question: static sampler order and parameter meaning. Consulted the
unchanged local Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9,
MaterialBox constants/constructor and GfxAssembler TEXTURE_SAMPLE static branch.
Recorded mathematical behavior and independently expressed arithmetic; no copied,
adapted or translated shader/recovered code. Animated branch convention differs
and stays excluded. Attempted pinned GitHub page fetch returned cache miss; local
pinned source answered the question. No decryption/decompilation needed/performed.
Numeric limits are authored preview policy, not inferred game-format constraints.

Exact verification commands (after . ./scripts/use-local-rust.ps1):
- cargo fmt --check: pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer: pass78 tests (6/7/65).
  Covers affine operation order, negative/zero scale, signed-zero identity, raw UV1
  preservation, finite/overflow/length/cancellation guards and prior UI lifecycles.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass. Final layout-only change also checked with Bevy Clippy/fmt.
- cargo build -p lbp-bevy-viewer -p lbp-viewer: pass. Final Bevy build after fixture
  layout adjustment passes. Only previously opened task-owned PID29892 was closed
  after checking its exact workspace executable path; no unrelated apps closed.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d44-linked-material-oracle.json local-analysis/d45-linked-parity.json:
  pass48/zero route/channel/raw-transform-bit mismatches; eligible14→16 (2× and3×).
- Same command with d44-diffuse-oracle.json and d45-diffuse-parity.json:
  pass32/zero mismatches; eligible15→16 (UV1 half-scale). Sets may overlap;
  eligibility is not whole-mesh/texture renderability. Existing external records
  reused; unchanged reader pin/metadata evidence, no new Java shader generation.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00002635 --browse-query guid:00002635
  --qa-capture local-analysis/d45-bevy-toro.png: native capture inspected;
  760vertices/701triangles/5primitives,61-triangle UV0 strap now2×/256×256BC1,
  two prior supported UV1 groups, two explicit neutral groups.
- Same native command with00016c87/d45-bevy-clive.png: inspected;
  2432vertices/2657triangles/7primitives;50/16-triangle UV1 groups now half-scale,
  512×512BC1;2195-triangle identity group unchanged, four explicit neutral groups.
- target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 000150e5 --mesh-view textured --qa-tab mesh
  --qa-capture local-analysis/d45-eframe-floor.png --qa-mesh-json local-analysis/d45-eframe-floor.json:
  inspected UV0 identity floor; geometry export SHA256
  636b78a5cbf37ec2650069d2c0f4a553f37783008a403919fb8a06d000460be1,
  unchanged from prior milestones. Native device label RTX3080/Vulkan.
- target/debug/lbp-bevy-viewer.exe --synthetic-affine --qa-capture
  local-analysis/d45-bevy-affine-fixture.png: authored offset/reflection fixture.
  Final capture inspection recorded below.

No tests/Clippy failures or original-input changes. SFO/MAP SHA256 unchanged;
FARC still1,023,990,298 bytes/ReadOnly. Isolated ignore audit permits authored Rust/
docs and excludes private JSON/captures/game/reference files; reference pin/source
clean, root no Git metadata. No original LBP code/assets/keys in product, no strict
clean-room claim, no commit/push/release/publication. Nonzero offsets have authored
coverage only; no original-game pixel parity, matrix/animated/general graph or
whole-corpus compatibility claim. Native interaction stress and other GPUs/platforms
remain untested. Next D46: choose one common unsupported diffuse route from bounded
diagnostics and establish behavior evidence before extending operations.


D45 capture follow-up: the first authored affine capture was edge-on and did not
verify the intended pattern. Input interference is suspected, not proven. Added
QA-only input suppression (normal interactive controls unchanged), reran Bevy's
seven tests/fmt/Clippy/build successfully, and requested a new exclusive capture
local-analysis/d45-bevy-affine-fixture-fixed.png. Original failed capture retained
as ignored evidence. This changes the verification conditions rather than repeating
an unchanged failed experiment.


D45 final authored capture inspected: raw left quadrants retained, middle2× Repeat
with quarter offset shows shifted checker cells, right Clamp horizontally reflects
red/green and blue/yellow, foreground red retains depth occlusion over blue. Fixed
capture completes with no stderr error. No original-game pixel-parity claim. Final
interactive Toro preview opened from the rebuilt viewer; existing CMD launcher
continues to target that updated binary.

## 2026-10-06 — Viewable/complete compatibility status and test preferences

Recorded user preferences in AGENTS.md: test programs on the second/non-primary
monitor, sparse image inspection, and newly viewable/newly complete asset counts
at every compatibility milestone. Complete means all textures showing and correct
geometry, independently of renderer completion.

The clickable launcher's Bevy browser now labels Viewable versus Complete, reports
textured/nonempty primitive coverage, and displays session viewable/complete/failed
counts across filters. Counts are distinct attempted GUIDs; complete is a subset
of viewable. Unchecked candidates/aliases retain their earlier separate meanings.
Complete requires two explicit visual confirmations on the actually displayed GUID;
neutral primitive groups/skipped geometry block confirmation. Diffuse-only success
does not automatically certify all textures or geometry. Reload replaces evidence;
source changes/restart clear it. No persisted/whole-corpus census or legacy eframe
ledger added. Compatibility delta: 0 newly viewable / 0 newly complete assets.
Existing D45 material eligibility counts cannot establish new whole-asset counts.

Bevy starts hidden and selects an OS-reported non-primary monitor before showing
the window; never defaults to the main screen. With no secondary monitor it remains
hidden. With more than two monitors it selects the first reported non-primary one.
Headless ECS regression verifies primary-only stays hidden, secondary placement,
and preserving later user window movement. Physical desktop placement remains
untested; no native window or image viewing was used for this change.

Exact checks after . ./scripts/use-local-rust.ps1:
- cargo fmt --all; cargo fmt --all --check: pass.
- cargo test -p lbp-bevy-viewer: 9 tests pass, including evidence/count/reset and
  monitor placement regressions.
- cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings: pass.
- cargo build -p lbp-bevy-viewer: pass; clickable launcher targets updated binary.

Initial sandbox formatter canonicalization and Clippy CARGO_MANIFEST_DIR checks
failed with Access denied. Reviewed execution outside the sandbox resolved both;
no approval rejection. Initial 8-test pass preceded the final monitor regression;
final 9-test/fmt/Clippy/build pass supersedes it. No format/parser changes, game
input writes, proprietary assets/code/keys, commit, push or publication. Visual
layout/manual checkbox interaction/native placement and whole-corpus counts remain
untested. Docs: docs/bevy-preview.md and architecture.md. Next D46 support milestone
must establish a measured asset baseline/delta rather than material-only totals.

## 2026-10-06 — D46 static texture × COLOR diffuse compatibility

Implemented one bounded MULTIPLY(TEXTURE_SAMPLE,COLOR)→DIFFUSE route in either
input order. Static finite color components0..1, UV0/UV1/affine rules retained;
duplicate/extra inputs, cycles, swizzles, animation/wiring, nested/multi-texture
graphs stay unsupported. Both frontends label tint and consume the same derived
display pixels; raw RGBA/UV/material bits remain intact. Nonidentity adds at most
16MiB/binding (128MiB/eight bindings); identity has no second pixel buffer.
CPU lookup tables/cancellation/budgets tested. No new disk parser or game code.
Per-rule source/pin/applicability/conflicts: docs/tinted-preview.md.

Question/provenance: which common unsupported diffuse route can be bounded next?
Existing80-material metadata census found4texture×static-COLOR roots. Unchanged
Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9, BoxType/BrdfPort and
GfxAssembler::resolve COLOR/MULTIPLY cases establish parameter/operation behavior.
Authored Java adapter queries RGfxMaterial helpers for route indices/tint bits.
No generated shader/template copied or executed. Independently expressed product
arithmetic; no recovered code/decryption/strict clean-room claim. Khronos sRGB
conversion informed authored display policy; original game color calibration,
post-filter GPU rounding and full material fidelity remain UNKNOWN. Wiki direct
fetch403/DataFormat HTML size-limit failure recorded; search result supplied the
standard conversion. No further reference downloads needed.

Asset-level before/after (actually ran baseline before changing selector): same
D42 set36mesh hashes/36distinct catalog GUIDs; viewable36→36, all-diffuse15→15,
newly visually verified complete0. **+0 newly viewable / +0 newly complete.**
Three previously viewable meshes gain6nonempty textured groups:
- Victorian frame0001e458:1→3/4; nurse000201dc:1→3/8; Thor head000156e7:1→3/6.
Metadata-only reports remain ignored: d46-compatibility-baseline.json,
d46-compatibility-after.json, d46-compatibility-delta.json. Delta comparison checks
same hashes/GUIDs and unchanged vertex/triangle counts. No full-corpus extrapolation.
The new bounded compatibility_census example reports GUID sets/group outcomes;
up to48hashes/64MiB total mesh inputs plus established per-mesh linked-read bounds.
Fully loaded diffuse groups are not certified Complete. App confirmations unset.

Exact verification (after . ./scripts/use-local-rust.ps1; local GNU toolchain):
- cargo fmt --all; cargo fmt --all --check: pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:81tests pass(7/9/65).
  Subsequent preview/Bevy pass16tests; final preview7tests include mid-pass tint
  cancellation, identity/raw preservation, linear RGB/alpha, finite bounds and
  selected-graph duplicate/animation/swizzle/cycle/multi-texture rejections.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass. Final preview-only Clippy/fmt after cancellation coverage also pass.
- cargo build -p lbp-bevy-viewer -p lbp-viewer: pass; launcher binary updated.
- ./scripts/run-material-oracle.ps1 -Diffuse -Output local-analysis/d46-linked-material-oracle.json
  -SampleManifest local-analysis/d43-linked-material-samples.json; same script
  with default manifest/-Output local-analysis/d46-diffuse-oracle.json:48/32
  parsed samples,404629/232687archived bytes; synthetic Java guards pass.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d46-linked-parity.json;
  analogous d46-diffuse-oracle.json/d46-diffuse-parity.json:48/32zero mismatches,
  eligibility16→18each (4newmaterial hashes). Exact source/color/multiply indices,
  selected channel, transform/tint bits corroborated; not shader/pixel agreement.
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d42-groups-oracle.json local-analysis/d46-compatibility-baseline.json;
  after support change with d46-compatibility-after.json: same counts/scope above.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00027987 --browse-query guid:00027987
  --qa-capture local-analysis/d46-bevy-vulture.png:
  1200vertices/1713triangles/one supported512×512BC1UV0
  group,4×Repeat/staticlinear0.14901961gray tint. Secondary monitor160v0 selected
  before visibility, logged by app; capture inspected once. Feather texture visible,
  original pose/fidelity unverified. QA process exits normally/stderr empty.
  Native sample is outside fixed36-asset baseline, no added corpus-count claim.

Initial build missed the new Asset field in the authored fixture; fixed. Initial
Clippy requested is_multiple_of/as_chunks; corrected and checked. First sandbox
native launch returned Access denied before opening; retained d46-bevy-frame logs,
then reviewed execution ran vulture successfully on non-primary display. No approval
rejection. Multi-tool patch context failed atomically, then corrected; inputs untouched.
Only one image viewed. No native eframe/manual checkbox/camera interaction, other
GPU/platform, full corpus or original-game pixel parity tested. Current native
sample does not establish Complete; lighting/alpha/pose/full shader remain limited.

SFO SHA256 b6491a83d66901a7fc18bb494dc2448811046d959de7a8cb9aa85ef4e9b99d39;
MAP SHA25651de580f2badf74db647999b212afbc3fb509d4c389adeb0ef14008cb4398350.
FARC remains1023990298bytes/ReadOnly. Source-bound readers verify these identities;
no whole-image hash repeated. Isolated ignore audit permits authored Rust/Java/docs
and denies private diagnostic JSON/PNG/game files/reference JAR. Reference pin/source
clean; root has no Git metadata. No proprietary assets/code/keys in product, original
input writes, commit, push, release or publication. Next D47: bounded static COLOR-only
diffuse materials (23observed roots), with correct no-texture group status and an
asset-level before/after baseline; do not invent fallback textures.

## 2026-10-06 — D47 static COLOR-only diffuse compatibility

Both frontends now display bounded static COLOR→DIFFUSE as a supported material
with no required diffuse texture. Added explicit MaterialAsset::Texture/Solid and
SolidSelection IR; solid has material identity/raw color bits, no texture/UV fields.
Loader returns it before UV decoding/texture resolution; no fallback image invented.
Bevy uses unlit linear constant color; eframe uses constant encoded vertex color,
preserving prior geometry/painter ordering. Alpha retained, opaque display policy.
One output/unique zero-swizzle source-port-zero diffuse edge; subtype-zero selected
nodes, no named/keyed animation/incoming COLOR wires, finite0..1components only.
Cycles/ThingColor/wired/animated/HDR cases remain unsupported. Read/cancel/cache
limits unchanged; supported solid bindings add no pixel/UV buffers.

Bevy panel now separates textured and solid groups; neutral bindings remain failed.
All supported groups enable visual review, not automatic completeness. Session
summary includes ready-for-completeness-review count; no required diffuse texture
is explicit in per-group detail and confirmation text. Raw geometry/pixels retained.
Format/provenance/conflicts/coverage: docs/solid-color-preview.md; architecture updated.

Evidence: unchanged Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
GfxAssembler::resolve COLOR and BoxType/BrdfPort. Reused D46 oracle's pinned reader
metadata/route records; exact source indices/raw color bits matched.48sample
eligibility18→36;32sample18→23;zero mismatches,23newmaterial hashes. No new disk
layout, shader/template execution, recovered code/decryption or copied/translated
reference source. Linear color/opaque policy is authored display behavior;
original calibration, alpha/pose/lighting/full shader fidelity remain UNKNOWN.

Actually captured before baseline prior to implementation, using same36D42mesh
hashes/36distinct catalog GUIDs: viewable36→36, all-diffuse15→23,0newly visually
verified complete. **+0newly viewable / +0newly complete.**13previously viewable
assets gain23solid-color nonempty groups. Eight new all-diffuse candidates:
0001354e,00013edb,00014cc4,00015daf,0001f732,0001f915,0001f972,000279ca.
Remaining5improved meshes still have neutral groups. Corpus totals remain unknown;
material eligibility/all-diffuse coverage is not Complete. Metadata reports ignored:
d47-compatibility-baseline.json, d47-compatibility-after.json, d47-compatibility-delta.json.
Delta comparison checks same hashes/GUID sets and unchanged vertex/triangle counts.

Exact commands/checks after . ./scripts/use-local-rust.ps1:
- cargo fmt --all; cargo fmt --all --check: pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:81tests pass(7/9/65).
  Extended meaningful checks: solid has no texture/UV accessors; duplicate/wired/
  animated/swizzled/nonfinite/ThingColor outcomes; solid coverage permits confirmed
  Complete; eframe constant vertex colors replace neutral shading without an image.
  Final preview7tests/previewClippy/fmt pass after the explicit no-UV/texture assertion.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass. cargo build -p lbp-bevy-viewer -p lbp-viewer: pass; launcher updated.
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d42-groups-oracle.json local-analysis/d47-compatibility-baseline.json;
  after implementation with local-analysis/d47-compatibility-after.json: counts above.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d47-linked-parity.json;
  analogous d46-diffuse-oracle.json/d47-diffuse-parity.json:48/32zero mismatches,
  36/23projected. Existing external records reused; no new Java execution required.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 0001f915 --browse-query guid:0001f915
  --qa-capture local-analysis/d47-bevy-fez.png:512vertices/666triangles,4supported
  solid groups/0textured. Red body/black tassel visible, all groups labeled no required
  diffuse image. Native UI shows1ready for review,1viewable/0complete, confirmations
  unset. Monitor160v0 non-primary selection logged before visibility. One image
  inspected; QA process exits/stderr empty. Native eframe, manual interaction, other
  GPU/platforms, original-pose/game-pixel parity and whole-corpus rendering untested.

No failed compilation/tests/Clippy or approval rejection this milestone. All work
performed within read-only-input scope; reviewed local Rust/native execution used
because earlier turns established sandbox path/executable denial. One image viewed.
SFO/MAP SHA256 match D46, FARC1023990298bytes/ReadOnly; no whole-image hash repeated.
Ignore audit permits authored solid.rs/docs and denies privateJSON/PNG/game/reference
JAR. Reference pin/source clean, root still no Git metadata. No proprietary game
bytes/keys/recovered code bundled, original writes, commit, push or publication.
Next D48: establish bounded static subtype-one 2×3 sampler matrix behavior and
same-asset baseline before extending UV transforms (3observed unsupported roots).

## 2026-10-06 — D48 static subtype-one UV matrices

Shared preview now accepts bounded static subtype-one TEXTURE_SAMPLE matrices:
raw parameters[0,1,2,3,6,7] yield row-major coefficients[a,b,c,d,e,f], derived
sampling `(a*u+b*v)+c, (d*u+e*v)+f` on explicit UV0/UV1. SamplingTransform retains
matrix versus scale/offset identity; both frontends consume the separate sampling
array and label the operation. Raw geometry/UV/texture bytes remain unchanged.
Identity rows preserve signed zero; coefficients/derived values bounded, finite
inputs,100kvertices, cancellation every256vertices/before return. Selected named/
keyed animation, wired samplers and unknown subtypes remain unsupported. Existing
graph/output/tint/cache/source budgets retained. Pin/method/source/conflicts/rule
labels/test applicability: docs/matrix-preview.md. No shader translation/execution.

Before implementation captured original36mesh baseline and expanded37hash/37GUID
baseline (added catalog-linked tutorial floor, not an external geometry oracle).
Same expanded cohort after:37→37viewable,23→23all-diffuse,0verifiedComplete.
**+0newly viewable / +0newly complete.** Exactly one asset0001a560 gains one
968triangle stitching group (128×256BC3 Repeat). Its5928vertices/7783triangles
unchanged; four groups still neutral with unsupported diffuse multiply routes.
Original36sample cohort unchanged36viewable/23all-diffuse. Delta checks cohort
hash/GUID identity and vertex/triangle counts; full corpus totals unknown. App
reports1viewable/0complete,1/5supported groups; completeness confirmations unset.

Exact verification commands after . ./scripts/use-local-rust.ps1:
- cargo fmt; cargo fmt --all --check: pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:83tests pass(9/9/65).
  New matrix checks: asymmetric cross terms/rotation, raw/signed-zero preservation,
  nonfinite/overflow/bounds/budget/cancellation; selector unknown-subtype and named/
  keyed animated matrix rejection. Existing monitor/completeness tests retained.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass. cargo build -p lbp-bevy-viewer -p lbp-viewer: pass.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d48-linked-parity.json;
  analogous d46-diffuse-oracle.json/d48-diffuse-parity.json:48/32compared,
  zero mismatches,36/24eligible (previous36/23). Reused pinned metadata oracle;
  no new external reader execution. Two other observed matrix roots animated.
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d42-groups-oracle.json local-analysis/d48-compatibility-baseline.json;
  same with d48-compatibility-after.json. Expanded selection d48-mesh-samples.json
  with d48-expanded-baseline.json/d48-expanded-after.json: counts above.
  PowerShell same-cohort/geometry delta in local-analysis/d48-compatibility-delta.json.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 0001a560 --browse-query guid:0001a560
  --qa-capture local-analysis/d48-bevy-tutorial-floor.png: correct selection and
  matrix diagnostic logged, patterned stitching visible, other groups neutral.
  Monitor160v0 non-primary selected before visibility; QA exits, stderr empty.
  One image inspected. Launcher now also uses hidden Start-Process; app handles
  second-monitor placement. Native eframe/manual confirmations/camera interaction,
  other GPU/platforms, added mesh external geometry parity, whole-corpus rendering
  and original-game pixel/pose/full shader fidelity untested.

No failed tests/build/Clippy or approval rejection. Reviewed Rust/native executions
used established sandbox path/executable workaround. SFO/MAP SHA256 unchanged from
D47; FARC1023990298bytes/ReadOnly; no whole-image hash repeated. Ignore audit allows
authored matrix.rs/docs and excludes derived JSON/PNG,game inputs/reference JAR.
Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9 clean; root no Git metadata.
No original input writes, proprietary data/code/keys bundled, recovered code use,
commit, push or publication. Next D49: gather evidence and a same-asset baseline
for a bounded two-texture diffuse multiply route (four tutorial-floor groups remain
unsupported), before adding independently implemented sampling/composition.

## 2026-10-06 — D49 bounded static two-texture diffuse products

One MULTIPLY→DIFFUSE now supports two distinct direct static subtype-zero texture
samplers on the same UV0/UV1 channel with integer positive scales1..16/zerooffsets
and Repeat on both axes. Unique unswizzled source-port-zero inputs on multiply
ports0/1, subtype-zero unanimated output/multiply, unflagged present descriptors,
unwired/unanimated samplers. Nested products, matrices, other channels, fractional/
negative scales, offsets and Clamp explicitly unsupported. Independent arithmetic
product, no reader shader copied/translated/executed. Evidence/pin/applicability/
conflicts/bounds/tests: docs/product-preview.md. No new disk layout/parser changes.

Periodic nearest grid uses checked LCM of each source extent×scale; bounded4096
peraxis/4Mpixels. RGB multiply linear then sRGBencode/round, alpha linear. Both
frontends explicitly use nearest for products (linear-filtered precomposition would
not equal two independently filtered samples). Single textures remain linear.
Asset retains both original textures/keys/raw transforms; separate product pixels
and extent. Sampling array uses raw channel for product, existing transformed array
for single textures. Source stability/cancellation/cache bounds retained. Eight
bindings ceilings now256MiBraw/128MiBderivedpixels,266MiBcumulativeinput and
720MiBcumulativeexpansionwork; transient65,536byte table. UI labels product/filter/
grid; diagnostics distinguish secondary resource and display extent from originals.

Actual before baseline d49-compatibility-baseline.json captured before code changes,
using same d48-mesh-samples.json37hash/37distinctGUID cohort. After/detailed:
37→37viewable,23→25all-diffuse,0verifiedComplete. **+0newly viewable / +0newly
complete.** Seven assets gain13textured groups/5733triangles:00010d89,00002635,
0001da30,00002d94,000201dc,000156e7,0001a560. New all-diffuse candidates00002635/
00010d89. Delta verifies same hashes/GUIDs/vertex/triangle counts and records groups
under local-analysis/d49-compatibility-delta.json. Full corpus totals unknown;
material eligibility and all-diffuse readiness are not Complete. Added floor still
has no external geometry oracle. Tutorial floor1/5→4/5supported groups, remaining
2000triangle matrix-sampler product neutral;5928vertices/7783triangles unchanged.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo fmt; cargo fmt --all --check: pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:85tests pass(11/9/65).
  Preview11tests rerun after selector extensions and final pixel tests. New checks
  asymmetric source sizes/scales/two-axis periodic ownership, linearRGB versus
  encoded-byte multiplication, alpha, source extents, LCM/pixel limits, cancellation
  during table/output, invalid scales, selected animation/channel/fractional/nested
  rejection. Existing second-monitor and completeness distinctions tests retained.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass. cargo build -p lbp-bevy-viewer -p lbp-viewer: pass.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d49-linked-parity.json;
  analogous d46-diffuse-oracle.json/d49-diffuse-parity.json:48/32compared,
  0mismatches,40/26eligible versus36/24. Reused unchanged pinned metadata records;
  compares both source indices/slots/channel/raw coefficients and root input edges.
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d49-compatibility-baseline.json;
  repeated after implementation with d49-compatibility-after.json and final secondary
  texture diagnostics with d49-compatibility-detailed.json:counts above.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 0001a560 --browse-query guid:0001a560
  --qa-capture local-analysis/d49-bevy-tutorial-floor.png:three new product groups,
  grids768×512/512×512/1024×768, fabric/checker patterns visible, neutral group
  retained. App shows4/5supported,1viewable/0complete, confirmations unset. Native
  startup logs monitor160v0 non-primary before showing; process exits/stderr empty.
  Exactly one image viewed to resolve product pattern display. No native eframe,
  manual confirmation/camera interaction, original-game pixel/pose/filter parity,
  other GPU/platform or whole-corpus rendering tested. Nearest/opaque/unlit/top-mip
  projection remains limited; no visual completeness certification made.

No failed compilation/tests/Clippy or approval rejection. Rust/native work used
reviewed established sandbox path/executable workaround. SFO/MAP SHA256 unchanged
from D48; FARC1023990298bytes/ReadOnly, no whole-image hash repeated. Ignore audit
allows authored product.rs/docs and excludes privateJSON/PNG/game/reference JAR.
Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9 clean; root no Git metadata.
No original writes, proprietary assets/code/keys bundled, recovered-source use,
decryption, commit/push/release/publication. Next D50: gather independent evidence
and a same-asset baseline for the remaining matrix product; do not infer original
filtering/visual fidelity from this authored nearest projection.

## 2026-10-06 — D50 independent static matrix/affine products in Bevy

Targeted research: floor_tut_purple material0001a547 has a static sampler matrix
with cross terms/offsets multiplied by an identity sampler. D49's axis-aligned grid
cannot represent it exactly. Unmodified pinned Toolkit metadata confirms raw
coefficient slots/root/input routes; no coefficients snapped, generated shader
copied/translated/executed or recovered code used. New DualAsset keeps secondary
texture/key/derived UVs; primary uses its own static transform. Independent sampler
selection accepts finite bounded affine/matrix pairs on same UV0/UV1, otherwise
retains graph/animation/wire/descriptor/Repeat guards. No nested products or other
channel pairs. D49 integer products still nearest-grid and retain their grid budgets.

Bevy0.19.1 ExtendedMaterial/MaterialExtension API inspected in pinned installed
source. Authored embedded unlit extension samples primary UV0 and secondary UV1
GPU attributes with independent linear filters, multiplies in linearRGB and keeps
opaque output/postprocessing. Disabled extension retains existing single/solid/grid
preview behavior. GPU channel numbers do not replace original material channel
identity. Scene cleanup now removes secondary image as well as primary/material/
mesh/entity; regression includes enabled extensions. Legacy eframe explicitly keeps
geometry and reports independent sampling unsupported. Census gains are for Bevy
preparation, not universal frontend coverage. Rule labels/pin/source URLs/methods,
applicability/conflicts/bounds/tests/API scope: docs/dual-preview.md. Same source/
pixel/read/cache budgets; extra secondary UV array≤800kbytes perbinding (≤6.4MB
across eight). Raw textures/UVs/geometry/transforms unchanged. No extra bakedpixels.

Before baseline d50-compatibility-baseline.json captured before compatibility code,
same d48-mesh-samples.json37hash/37distinctGUID cohort. After/detailed:37→37viewable,
25→26all-diffuse,0verifiedComplete. **+0newly viewable / +0newly complete.** One
asset0001a560 gains its2000triangle group;4/5→5/5supported,5928vertices/7783triangles
unchanged. Same-cohort/GUID/geometry delta in d50-compatibility-delta.json. Original
floor still lacks external geometry oracle; no original-game pose/pixel correctness
certification. App shows5/5supported,1ready for review,1viewable/0complete and
manual texture/geometry checks unset. Complete means all textures showing/correct
geometry, independent of renderer completion; readiness/eligibility not Complete.
Corpus-wide totals unknown. Existing48/32oracle eligibility unchanged40/26 because
targeted matrix material was not included in those original sample sets.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d50-compatibility-baseline.json;
  after with d50-compatibility-after.json; final secondary frontend/filter diagnostics
  with d50-compatibility-detailed.json:counts above. Neutral diagnostics now retain
  materialGUID/hash to make targeted research reproducible without asset exports.
- ./scripts/run-material-oracle.ps1 -Diffuse -SampleManifest local-analysis/d50-matrix-material-sample.json
  -Output local-analysis/d50-matrix-material-oracle.json:1material/8732bytes, pinned
  source clean. Authored metadata manifest from catalog alias/candidate fields.
  Synthetic oracle guards pass; known Toolkit signed ZigZag extreme conflict retained.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d50-matrix-material-oracle.json local-analysis/d50-matrix-parity.json:
  1compared/1eligible/0mismatches. Second-sampler comparison now respects subtype
  parameter indices, not contiguous four-bit assumption. Analogous d46-linked-material-oracle.json/
  d50-linked-parity.json and d46-diffuse-oracle.json/d50-diffuse-parity.json:
  48/32compared,0mismatches,40/26eligible. Original external datasets reused.
- cargo fmt; cargo fmt --all --check:pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:85tests pass(11/9/65).
  Final preview11 rerun after asymmetric matrix/nonfinite route checks; Bevy9 rerun
  after secondary image ownership regression. Existing affine/matrix math/rawbit/
  cancellation/channel and monitor/completeness tests retained.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass. cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 0001a560 --browse-query guid:0001a560
  --qa-capture local-analysis/d50-bevy-tutorial-floor.png:final matrix group displays
  fabric pattern, all5groups supported, earlier grid/single groups visible. Runtime
  shader stderr empty; QA exits. Monitor165v0 non-primary selected before showing.
  Exactly one image viewed to resolve shader/product display; no screenshots used
  for routine source checks. Native legacy eframe/manual confirmations/camera,
  other GPU/platforms, shader pixel oracle, original pose/game filtering/mips/full
  shader channels and whole-corpus rendering untested. Top-mip/unlit/opaque remains
  authored diagnostic policy, no Complete certification from this capture.

Initial Bevy test compilation failed E0618 because local material handle shadowed
helper function; fixed by qualified dual::material calls, focused tests/build pass.
No runtime shader failure or approval rejection. Rust/native execution used reviewed
established sandbox path/executable workaround. SFO/MAP SHA256 match D49;
FARC1023990298bytes/ReadOnly, no whole-image hash repeated. Isolated ignore audit
allows authored dual.rs/docs and denies privateJSON/PNG/game/reference JAR. Toolkit
pin d6a2ef80b5a09999f08f23a803b516096055e5c9 clean; root no Git metadata. No original
input writes, proprietary game bytes/code/keys bundled, decryption/recovered-source
use, commit/push/release/publication. Next D51: gather evidence and same-asset
baseline for bounded nested diffuse products or channel pairs; maintain frontend
scope and distinguish full diffuse preparation from visual completeness.

## 2026-10-06 — D51 independent UV0/UV1 product channel pairs

Bevy static two-texture diffuse sampling now selects/transforms each original
channel independently, including UV1×UV0. Grid baking explicitly requires matching
original channels (identical/integer transforms do not make different channels
interchangeable). No fallback for missing UV1; channels>1/nonfinite/wrong count/
bounded transform failures remain unsupported. Existing graph/animation/wire/
descriptor/Repeat/cancellation/source/cache guards retained; no nested products.
No new disk layout or shader edit. GPU adapter validates/inserts secondary sampling
coordinates while preserving primary coordinates/indices. UI labels both original
channels, diagnostics include second channel/source/frontend. Legacy eframe still
reports independent sampling unsupported and retains geometry. D50 memory/read/
pixel/coordinate budgets unchanged. Provenance/pin/methods/coverage/applicability/
conflicts: docs/channel-pair-preview.md, prior UV1/dual contracts remain relevant.

Actually captured before baseline local-analysis/d51-compatibility-baseline.json
before code changes, same d48-mesh-samples.json37hash/37distinctGUID cohort.
After/detailed:37→37viewable,26→30all-diffuse,0verifiedComplete. **+0newly viewable
/ +0newly complete.** Four assets gain4groups/1577triangles:0000ffff(2triangles),
0000ef9d(651),00013051(892),0000ffa9(32). All four become all-diffuse candidates;
that is not Complete. Delta verifies same cohort/GUID/vertex/triangle counts under
d51-compatibility-delta.json. Full corpus totals unknown; scope Bevy preparation.
Two previously blocked cross-channel material routes in existing48records become
eligible; material-route count is not asset count. No new oracle dataset needed.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d51-compatibility-baseline.json;
  after with d51-compatibility-after.json; final channel diagnostics with
  d51-compatibility-detailed.json:counts above, deterministic GUID sample sets.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d51-linked-parity.json;
  analogous d46-diffuse-oracle.json/d51-diffuse-parity.json:48/32compared,
  0mismatches,42/26eligible versus40/26. Reused unchanged Toolkit pin
  d6a2ef80b5a09999f08f23a803b516096055e5c9 reader records; independently compare
  root/input indices, both channel selectors/slots/raw transform bits.
- cargo fmt; cargo fmt --all --check:pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:86tests pass(11/10/65).
  New GPU regression distinct primary/secondary bits including signed zero, original
  triangle index ownership, short/nonfinite secondary rejection. Selector records
  both channels; integer cross-channel route cannot use grid. Existing requested
  UV1 absence/no-fallback and source/transform/cancellation/completeness tests retained.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass. Final preview all-target Clippy/fmt pass after diagnostic channel field.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00013051 --browse-query guid:00013051
  --qa-capture local-analysis/d51-bevy-channel-pair.png:2560vertices/2640triangles,
  fourgroups;892triangle group uses transformedUV1×rawUV0, combined green circuit
  trace pattern visible. Other groups retain one texture/two solid colors. App
  shows4/4supported,1ready for review,1viewable/0complete, checks unset. Monitor165v0
  non-primary selected before visibility; QA exits/stderr empty. Exactly one image
  viewed for this visual question. No native legacy eframe/manual confirmations/
  camera interaction, original game pose/pixel/filter/shader parity, otherGPU/
  platforms or whole-corpus rendering tested. All textures/geometry correctness
  still requires separate review, independently of renderer completion.

No failed compilation/tests/Clippy, runtime shader errors or approval rejection.
Reviewed Rust/native execution used established sandbox path/executable workaround.
SFO/MAP SHA256 match D50; FARC1023990298bytes/ReadOnly, no whole-image hash repeated.
Ignore audit permits authored Rust/channel-pair docs and denies privateJSON/PNG/game/
reference JAR. Toolkit source clean at pin; root no Git metadata. No original writes,
proprietary assets/code/keys bundled, recovered code/decryption, commit/push/release/
publication. Next D52: investigate bounded nested diffuse multiplication (remaining
roots in metadata); capture asset baseline and preserve explicit depth/cycle limits.

## 2026-10-06 — D52 bounded three-texture nested diffuse products

Exactly two static subtype0 MULTIPLY nodes with three distinct static sampler leaves
now supported in Bevy. Third sampler must share original channel/exact transform bits
with one inner leaf; either outer input order/reversed wire list accepted. Three
original textures/keys retained with only two derived sampling arrays. Optional
third GPU texture samples secondary coordinates; scene cleanup regression removes
all three images. Grid baking excluded for this route. Parser UI-free; no disk
layout changes/general traversal/recursion. Graph endpoint/port/swizzle/animation/
channel/transform/Repeat/descriptor/cancel/source/cache guards retained. Deeper
products/cycles/duplicate leaves and unmatched transforms fail explicitly.
Eight bindings now≤384MiB raw pixels+128MiB derived pixels,394MiB cumulative input,
976MiB cumulative expansion including repeated UV decode. Third source retains
16MiB input/32MiB expanded/4Mpixels caps. Exact source URLs/pins/methods/evidence
labels/applicability/conflicts/tests:docs/nested-preview.md. Legacy independent
sampling remains unsupported with geometry available. App labels three samplers
and source-session Viewable/Complete counts with separate manual confirmations.

Beforecode census:d52-compatibility-baseline.json. After:d52-compatibility-after.json;
delta:d52-compatibility-delta.json,all ignored under local-analysis. Same37hash/
37distinctGUID cohort,unchanged GUID/vertex/triangle counts:37→37viewable,
30→30all-diffuse,0verifiedComplete. **+0newly viewable/+0newly Complete**.
One asset improved:handkerchief00007a6d,0/3→2/3supported;64+96=160triangles across
two new textured groups. Remaining98triangle four-texture route neutral. Corpus
totals unknown;scope Bevy preparation,not universal frontend/game visual fidelity.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d52-compatibility-baseline.json;
  same command with d52-compatibility-after.json after changes:counts above.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d52-linked-parity.json;
  analogous d46-diffuse-oracle.json/d52-diffuse-parity.json:48/32compared,
  0mismatches,44/26eligible versus42/26. Reused pinned external metadata;
  projection schema4 retains inner/root multiply and third sampler selectors.
- cargo fmt --all; cargo fmt --all --check:pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:86pass(11/10/65).
  New guards in selector regression:valid triple/reversed ordering/both outer
  input orders/exact signed-zero transform sharing/duplicate port or leaf/
  animated third sampler/cycle/deeper nesting/no grid. Cleanup includes thirdimage.
  Final cargo test -p lbp-preview --lib:11pass after extra outer-order guard.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:
  pass, including final check. cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.
- Start-Process -WindowStyle Hidden target/debug/lbp-bevy-viewer.exe --snapshot
  local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid
  00007a6d --browse-query guid:00007a6d --qa-capture local-analysis/d52-bevy-three-texture.png:
 184vertices/258triangles/3groups. Non-primary monitor165v0 selected before showing;
 process15884 QA-exited. stdout/err in matching ignored .out.log/.err.log;stderr empty.
 Exactly one image inspected to resolve three-texture shader rendering:ornate floral
 borders and pale fabric bands displayed,center neutral. UI2/3supported,1viewable/
 0complete,0ready for review,Complete checks unavailable. No visual certification.

Initial baseline execution and formatter encounter Windows sandbox Access denied/
canonicalization failures; established reviewed execution workaround succeeds.
Initial Rust test compile failed E0308:three-texture label arm accidentally inserted
in texture() rather than label(); moved to correct method,all checks pass. No shader
runtime errors or approval rejection. Source audit first MAP lookup used wrong
directory; resolved read-only OUTPUT/BLURAYGUIDS.MAP before hashing. SFO/MAP SHA256
match D51;FARC1023990298bytes ReadOnly/Archive,no wholeimage rehash. Toolkit pinned
d6a2ef80b5a09999f08f23a803b516096055e5c9 clean. Isolated defaultdeny ignore audit
permits authored Rust/nesteddocs,denies privateJSON/PNG/game/referenceJAR.
No original writes,proprietary assets/code/keys bundled,decryption/recovered source,
commit/push/release/publication or strict clean-room claim. Native legacy,manual
confirmation/camera interaction,original pose/game pixels/filter/mips/full shader,
otherGPU/platforms and corpus-wide rendering untested. Next D53:investigate bounded
four-texture products with two shared sampling coordinate sets; capture baseline
and preserve depth/cycle/source/memory limits before extending.

## 2026-10-06 — D53 four-texture diffuse chain

Fixed chain ((A×B)×C)×D with three static subtype0 multiplies/four distinct static
texture leaves now supported in Bevy. C shares B's original channel/exact transform
bits; D shares A's. Either input order on each multiply/reversed wire list accepted.
At most three multiply nodes inspected, no recursion; repeated nodes/cycles,
duplicate leaves, invalid source/ports/swizzle/animation and deeper/balanced trees
remain explicit failures. Output/endpoints checked before indexing. Four original
textures/keys retained; fourth samples primary GPU UVs, third samples secondary;
no extra coordinate arrays. All four GPU images owned/removed on scene replacement.
Repeat/present unflagged descriptor/resolution/source/cancel/cache/finite/channel
bounds retained. Grid excluded. Eight bindings≤512MiB raw pixels+128MiB derived,
522MiB cumulative stored reads,1232MiB cumulative expansion including UV decoding;
each texture still16MiB input/32MiB expansion/4Mpixels. No parser UI dependency/new
disk rule/reader shader copied. App labels four original samplers,distinct session
Viewable/Complete counts and manual all-textures/correct-geometry confirmations.
Legacy eframe remains explicit geometry fallback for independent sampling.
Evidence URLs,pins,methods,labels,applicability/conflicts/tests:docs/four-product-preview.md.

Beforecode:d53-compatibility-baseline.json;after:d53-compatibility-after.json;
delta:d53-compatibility-delta.json under ignored local-analysis. Same37hash/
37distinctGUID cohort,unchanged GUID/vertex/triangle counts:37→37viewable,
30→31all-diffuse,0verifiedComplete. **+0newly viewable/+0newly Complete**.
Handkerchief00007a6d improves2/3→3/3groups,gaining98triangle group. Full diffuse
coverage only enables visual review. Corpus totals unknown;Bevy preparation scope.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d53-compatibility-baseline.json;
  same with d53-compatibility-after.json after changes:counts above.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d53-linked-parity.json;
  analogous d46-diffuse-oracle.json/d53-diffuse-parity.json:48/32compared,
  0mismatches,45/26eligible versus44/26. Existing pinned record
  ce658d5650822969f1780545df8c61bdc1bb55d3:roots3→5→7,leaves1/2/4/6,
  UV1/UV0/UV0/UV1 identity,slots0/1/2/3. All roots/edges/selectors/raw coefficients
  compared independently; diagnostic schema5 retains deepest root/fourth sample.
- cargo fmt --all; cargo fmt --all --check:pass,including final comment-reference check.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:86pass(11/10/65).
  Selector regression validates four selected indices/roots/ignored counts,
  reversed wire order/all multiply input orders,no grid,coefficient/signed-zero
  mismatch,animated fourth,invalid source port,duplicate leaf and cycle rejection.
  Existing source/budget/cancel/UV-no-fallback guards retained. GPU cleanup includes
  fourthimage. Monitor regression preserves primary-only hidden behavior.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:pass.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.
- Start-Process -WindowStyle Hidden target/debug/lbp-bevy-viewer.exe --snapshot
  local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid
  00007a6d --browse-query guid:00007a6d --qa-capture local-analysis/d53-bevy-four-texture.png:
  184vertices/258triangles/3groups. Non-primary monitor165v0 selected before showing,
  process7508QA-exited,matching.out.log/.err.log,stderr empty. One image inspected
  specifically for fourth-sample shader rendering:ornate center motif now visible,
  prior floral borders/pale fabric retained,3/3supported. App1viewable/0complete,
  1ready for review,manual confirmations unset. No visual completeness certification.

No failed compilation/tests/Clippy/runtime shader or approval rejection. Established
reviewed local Windows execution workaround used. SFO/MAP SHA256 match D52;
FARC1023990298bytes ReadOnly/Archive,no wholeimage rehash. Toolkit pin
d6a2ef80b5a09999f08f23a803b516096055e5c9 clean;isolated defaultdeny ignore audit
allows authored Rust/fourproductdocs and denies privateJSON/PNG/game/referenceJAR.
No original writes,proprietary assets/code/keys bundled,decryption/recovered source,
commit/push/release/publication or strict clean-room claim. Native legacy/manual
confirmation/camera,original game pixels/pose/filter/mips/full shader,otherGPU/
platforms and corpus-wide rendering untested. Remaining sample gaps include one
shared material00001d92 with unsupported wrapping across3assets. Next D54:
investigate that descriptor/wrap contract with targeted pinned metadata and capture
asset baseline before extending; keep explicit per-axis semantics and budgets.

## 2026-10-06 — D54 independent U/V single-texture wrapping

Targeted pinned headless metadata identifies material00001d92 sack_boy_stitching_decal,
hash02a5c9ea46d5479b73c7664f0f3effe12e4e346e,7865bytes atFARCoffset37592114.
Direct static identity UV1 slot0 requests S=ClampToEdge(3),T=Repeat(1),present
unflagged GUID texture. Independent raw array/route comparisons corroborate both
axes. Matching-axis guard was preview policy,not a format requirement.
Asset now stores typed TextureWrapping(U,V) rather than one repeat boolean;
single/tinted routes accept1/3 independently. Bevy maps separate U/V sampler modes;
raw/derived UVs/pixels unchanged,no folding/defaults/shader edit. App labels both
axes and retains separate session Viewable/Complete confirmation/counts. Legacy
eframe's uniform wrap option cannot express mixed axes;explicit failure retains
geometry before image installation,prior textures cleared by existing load reset.
Products still require Repeat on both axes for every source. Unknown/missing/
mirror/border/legacyCLAMP/mirroronce modes rejected. D53 four-source/cache/pixel/
coordinate/cancel/source budgets unchanged,no parser/UI dependencies added.
SourceURLs/pins/classes/evidence/applicability/conflicts/coverage:docs/axis-wrap-preview.md.

Beforecode:d54-compatibility-baseline.json;after:d54-compatibility-after.json;
delta:d54-compatibility-delta.json ignored under local-analysis. Same37hash/
37distinctGUID cohort,unchanged GUID/vertex/triangle counts:37→37viewable,
31→33all-diffuse,0verifiedComplete. **+0newly viewable/+0newly Complete**.
Three assets gain3groups/81triangles:0001da30 gains38triangles3/6→4/6;
000201dc gains8triangles7/8→8/8;000156e7 gains35triangles5/6→6/6. Last two
become full diffuse preparation candidates,not Complete. Corpus totals unknown;
scope Bevy CPU preparation,not universal frontend or visual certification.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d54-compatibility-baseline.json;
  same with d54-compatibility-after.json after changes:counts above;diagnostics raw
  U/V fields and addressing frontend scope retained.
- ./scripts/run-material-oracle.ps1 -Diffuse -SampleManifest local-analysis/d54-wrap-material-sample.json
  -Output local-analysis/d54-wrap-material-oracle.json:1target7865bytes,
  parsedgraph;synthetic prefix/graph bounds pass. Known existing Toolkit extreme
  signed-ZigZag conflict reported,independent parser doesn't adopt it. Original FARC
  read-only;derived manifest/JSON/classes remain ignored. Toolkit pin/source checked.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d54-wrap-material-oracle.json local-analysis/d54-wrap-parity.json:
  1compared0mismatches1eligible. Both complete raw S/T arrays also independently
  compared for existing datasets using samecommand with d46-linked-material-oracle.json/
  d54-linked-parity.json and d46-diffuse-oracle.json/d54-diffuse-parity.json:
  48/32compared0mismatches,45/26eligible unchanged. Graph eligibility was already
  true for the new addressing route;material counts not asset counts. Schema6 adds
  raw arrays and wraps_equal.
- cargo fmt --all; cargo fmt --all --check:pass,including final evidence-reference check.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:87pass(12/10/65).
  New wraptest all4combinations/rawretention/uniformfrontendNoneforMixed;absent
  U/V and0/2/4/5/6/7/8/255 eitheraxis reject. Adapter regression both mixed pairs
  asserts independent address_mode_u/v and unchanged linear filters;prior uniform
  modes/sRGB/extents/geometry/UV/cache/cancel/completeness tests retained.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:pass.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.

Headless descriptor checks sufficient for this bounded mapping;no native test window
opened and0images inspected. No native GPU sampling/game pixel/pose/filter/mips/full
shader/otherplatform/camera/manualComplete or wholecorpus validation. No failed
compile/tests/Clippy or approval rejection;established reviewed Windows execution
workaround used. SFO/MAP SHA256 match D53;FARC1023990298bytes ReadOnly/Archive,
no wholeimage rehash. Toolkit pinned d6a2ef80b5a09999f08f23a803b516096055e5c9 clean.
Defaultdeny isolated ignore audit permits authored wrapping/Rust/docs and denies
privateJSON/game/referenceJAR. No original writes,proprietary assets/code/keys,
decryption/recovered source,commit/push/release/publication or strictcleanroomclaim.
Next D55:target pinned metadata for remaining material0000be15 unsupported diffuse
multiply inputs;establish actual static/instance-dependent semantics and baseline
before extending,keep unresolved/animated/instance-dependent outcomes explicit.

## 2026-10-06 — D55 static COLOR / two-texture product MIX

Initial targeted material0000be15 neon_lifebar,6954bytes/hash
d83895a7dae40936d571d1c7bb531c19f9bbfdfa,uses instance ThingColor8 for diffuse
multiply and swizzled sampler input. Static compatibility hypothesis rejected;
no scene color invented. Pinned metadata also gathered for five remaining targets
(50368bytes):lamp static MIX;two glow matrix samplers have named b1/c1 animations,
each201keys/603components;LCD custom missing diffuse wire;LCD stretch uses instance
ThingColor. Raw records remain ignored;animated/instance/missing routes unsupported.

Static lamp0001dc46 vic_lab_wall_lamp_glassy,10175bytes/hash
3b974d545f61c79b424213c6e10737dfad1b7c2a,supported as exactly one static MIX of
unwired finite COLOR and one two-texture MULTIPLY. Stored parameter0 weight finite0..1;
ports0/1 direction preserved,no wired weightport2. Static nodes/unique unswizzled
inputs/endpoint/output/distinct leaves/fixed-depth guards retained. Both samplers
sameoriginal UV0 integer Repeat scales1/3;no channel fallback/general traversal.
DiffuseBlend retains MIX index/color/weight bits/input direction;COLOR andproduct
indices retained. CPU grid calculates product then linear RGB/alpha mix before
single output encoding,no intermediate quantized image. Optional192KiB bounded
RGB LUT (plain product64KiB),existing4Mpixels/4096axis/LCM/cancel bounds retained.
No extra image/source/UVarray or shader changes;D53 four-source ceilings unchanged
with explicit temporary table allowance. App labels static MIX,color,weight,direction
and separate Viewable/Complete review. Existing nearest-grid frontend behavior.
RuleURLs/pins/classes/evidence/applicability/conflicts/tests:docs/static-mix-preview.md.

Beforecode:d55-compatibility-baseline.json;after:d55-compatibility-after.json;
delta:d55-compatibility-delta.json under ignored local-analysis. Same37hash/
37distinctGUID cohort,unchanged geometry/GUID counts:37→37viewable,33→34all-diffuse,
0verifiedComplete. **+0newly viewable/+0newly Complete**. Lampmesh0001e458 gains
one190triangle group,3/4→4/4supported. Full diffuse preparation enables review;
not proof of all textures/correct geometry. Corpus totals unknown;Bevy CPU scope.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d55-compatibility-baseline.json;
  same with d55-compatibility-after.json after changes:counts above/static_mix fields.
- ./scripts/run-material-oracle.ps1 -Diffuse -SampleManifest local-analysis/d55-material-sample.json
  -Output local-analysis/d55-material-oracle.json:1neon6954bytes;
  analogous d55-remaining-material-sample.json/d55-remaining-material-oracle.json:
  5records50368bytes;d55-mix-material-sample.json/d55-mix-material-oracle.json:
  1lamp10175bytes. Synthetic prefix/graph bounds pass;known Toolkit extreme signed
  ZigZag conflict printed,independent reader doesn't adopt. Pinned source clean;
  all originals read-only,derived manifest/JSON/classes ignored.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d55-mix-material-oracle.json local-analysis/d55-mix-parity.json:
  1compared0mismatches1eligible. Same with d55-material-oracle.json/
  d55-instance-parity.json:1compared0mismatches0eligible. Existing
  d46-linked-material-oracle.json/d55-linked-parity.json and d46-diffuse-oracle.json/
  d55-diffuse-parity.json:48/32compared0mismatches46/26eligible versus45/26.
  MIX root/edges/color bits/weight/input direction/both sampler slots/channels/
  coefficients/wraparrays checked. Schema7 retains blend metadata. Five-record
  research set not passed wholesale to route comparator because one has no diffuse.
- cargo fmt --all; cargo fmt --all --check:pass,including final evidence references.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:89pass(14/10/65).
  Earlier narrow cargo test -p lbp-preview --lib:14pass. New pixel tests linear
  RGB/alpha/asymmetricmix/endpoints/sourcepreservation/midoutputcancel;weighttest
  endpoints/rawsignedzero/nonfinite/outofrange. Selector tests valid route/reversed
  wirestorage/bothinputdirections/selectedignoredcounts/wiredweight/animatedCOLOR/
  swizzle/crosschannel/cycle. Existing source/cache/LCM/pixel/UV/completeness retained.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:pass.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.

Headless CPU/grid tests sufficient for this operator;0native windows/0images,
no monitor placement needed. No native new-route display/manualComplete/camera/
game pixel/pose/fullshader/alpha/filter/mip/otherGPU/platform/corpus verification.
No failed compile/tests/Clippy or approval rejection;initial static-neon hypothesis
rejected from new pinned evidence rather than retried unchanged. Established
reviewed Windows execution workaround used. SFO/MAP SHA256 match D54;
FARC1023990298bytes ReadOnly/Archive,no wholeimagehash repeat. Toolkit pin
d6a2ef80b5a09999f08f23a803b516096055e5c9 clean;defaultdeny isolated ignore audit
allows authored blend/Rust/docs,denies privateJSON/game/referenceJAR. No original
writes,proprietary assets/code/keys,decryption/recovered source,commit/push/release/
publication or strictcleanroomclaim. Next D56:investigate bounded named sampler
animation key layout/time semantics for glowmaterials0001d958/0001d959;gather
pinned/controlled evidence before adding an explicitly timed preview,no guessed
static frame or instance defaults.

## D56 — local animation base snapshots (2026-10-06 America/Chicago)

Question: can the two named matrix-row curves on glow materials be previewed at
an established time? Pinned MaterialParameterAnimation serialization exposes
component-major values/base/name/mask but does not establish key rate, time,
interpolation or wrapping. GfxAssembler binds named rows but baked shader values
come from a global material animation list; equivalence to local sampler bases
and the game's resolver remains UNKNOWN. Timed playback is deferred. New scope
is explicitly opt-in local stored base-value snapshots, with no guessed frame.
No decryption/executable investigation or recovered-source implementation needed.
Sources/pin/methods/applicability/conflict/tests:docs/animation-base-preview.md.

Direct subtype1 UV0/UV1 sampler only, two distinct named XYZ row curves, static
output, exact unswizzled direct diffuse wire, unwired sampler. Existing1024node/
4096wire endpoints, UV bounds/read/texture/cache/cancel guards apply. Each curve
has1..4096component values divisible by3 with derived count matching. Original
static matrix bits and both full base rows/key counts retained separately; only
XYZ derives sampling UVs. No new key decoding/buffers/shader resource. Default
and legacy remain static-only. Bevy option/--animation-base reloads and resets
verification; snapshots explicitly label game texture placement unverified and
cannot be confirmed Complete. Complete still means all textures and correct
geometry, independent of renderer completion. Diagnostic census schema2/oracle8
retain mode/rows/static bits/key counts/time:null.

Baseline d56-compatibility-baseline.json and static-after.json, opt-in
base-after.json/base-detailed.json and delta.json under ignored local-analysis:
same37mesh hashes/37distinctGUIDs; unchanged GUIDs/geometry.37→37viewable,
0→0verifiedComplete: **+0newly viewable/+0newly Complete**. Static all-diffuse
34→34; opt-in snapshots35. Geometry boy0001da30 gains2groups/223triangles,
4/6→6/6prepared. All-diffuse is preparation coverage, not Complete certification.
Corpus totals unknown. Two glow material records are not two asset counts.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d48-mesh-samples.json local-analysis/d56-compatibility-baseline.json;
  before implementation. Same arguments with d56-compatibility-static-after.json:
  37viewable/34all-diffuse/0Complete. With d56-compatibility-base-after.json and
  --animation-base:37/35/0; final d56-compatibility-base-detailed.json same counts.
- ./scripts/run-material-oracle.ps1 -Diffuse -SampleManifest local-analysis/d56-animation-material-sample.json
  -Output local-analysis/d56-animation-material-oracle.json:2records23682stored
  bytes;synthetic metadata checks pass;known Toolkit extreme signed ZigZag conflict
  printed unchanged and not adopted. Research output remains ignored.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d56-animation-material-oracle.json local-analysis/d56-animation-parity.json
  --animation-base:2compared0mismatches2projected. Final d56-animation-parity-detailed.json
  includes original static matrix-bit comparison as well as full base rows,
  derived coefficients/key counts:2/0/2. Existing d46-linked-material-oracle.json/
  d56-linked-parity.json:48compared0mismatches46projected;d46-diffuse-oracle.json/
  d56-diffuse-parity.json:32compared0mismatches26projected (default static mode).
- cargo fmt --all; cargo fmt --all --check:pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:89pass(14/10/65).
  Final raw-static-bit assertion: cargo test -p lbp-preview --lib:14pass.
  Tests cover asymmetric base rows/signed zero/raw UV/static coefficient retention,
  mode-off rejection/invalid names/masks/nonfinite rows/wired sampler/endpoints;
  existing bounds/source/cache/cancel tests retained. App test blocks snapshot
  Complete even with both manual checks set.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:pass.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 0001da30 --browse-query guid:0001da30
  --animation-base --qa-capture local-analysis/d56-bevy-animation-base.png:
  hidden Start-Process;log verifies non-primary monitor165v0 placement before show;
  6groups loaded with2snapshot labels,source session1viewable/0Complete/0failed.
  One image viewed to verify enabled option,6/6groups,distinct counts,disabled
  Complete review and rendered textures. No shader errors. Screenshot exposed a
  redundant neutral-geometry explanation;final text now describes unverified game
  texture placement and only reports neutral/skipped geometry when applicable.
  Final cargo test -p lbp-bevy-viewer,cargo fmt --all --check,
  cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings and
  cargo build -p lbp-bevy-viewer verify this wording correction. No extra image.

Not tested:game-time/global uniform resolver/native game pixels/pose,playback,
manual Complete,interactive mode toggle/rapid cancellation,full shaders/alpha/
mips/filter fidelity,other GPUs/platforms or corpus. Native capture predates only
the final wording correction. No failed compile/test/Clippy/approval rejection;
two initial patch-context mismatches were corrected before any files were changed.
SFO/MAP SHA256 unchanged from D55;FARC1023990298bytes ReadOnly/Archive.
Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9 source clean. Isolated ignore audit
allows authored Rust/docs and denies private JSON/PNG/game/reference JAR. No
original writes/proprietary bundling/keys/recovered code/commit/push/publication
or strict clean-room claim. Next:resolve remaining instance-dependent materials
from pinned metadata and controlled local evidence;do not invent ThingColor or
animation timing. D56 complete;final UI wording checks passed (10tests,fmt,Clippy,build).

## D57 — constant COLOR/single-texture MIX (2026-10-06 America/Chicago)

Remaining ThingColor routes require instance input absent from standalone meshes;
no default invented. Existing pinned metadata revealed bounce-pad casing00012fd2
with a constant black COLOR/texture MIX. D55 graph/arithmetic evidence applies
without new disk parsing. Independent fixed-depth selector now allows one static
unwired UV0/UV1 sampler with existing bounded matrix/scale-offset transform,
COLOR finite0..1 and stored MIX weight finite0..1. Exactly two unswizzled inputs
on unique ports0/1;weighted port2/named animation/wired sampler/ThingColor stay
unsupported. Existing output/endpoints1024node/4096wire/source guards retained.
Evidence URL/pin/method/applicability/conflicts/tests:docs/single-mix-preview.md.

DiffuseBlend retains root/color/weight/input direction. No second sample or
multiply index for the single route. Linear RGB/linear alpha mixing before one
encoding uses1024byte LUT and existing<=16MiB derived diffuse buffer;original
pixels/raw UVs preserved. Texture-only endpoints return no additional allocation;
cancellation before bounded LUT,every4096output pixels,at completion. No new
shader or extra source/UV buffer. Existing per-axis wrapping and both frontend
display paths/labels reused. Bevy still displays distinct source-session GUID
Viewable/Complete counts and manual texture/geometry review;no coverage-based
Complete claim. Corpus totals remain unknown.

Beforecode d57-mesh-samples.json extends prior37hash cohort with catalog-linked
bouncepad00012fd4,hash b9955d73fa4789190a56ec0640c6adfe50573d22. Added mesh is not
an external geometry oracle. d57-compatibility-baseline.json/after.json/delta.json:
same38hashes/38distinctGUIDs,unchanged geometry/GUIDs:38→38viewable,34→34all-diffuse,
0→0visuallyverifiedComplete. **+0newly viewable/+0newly Complete**. Bouncepad
0/2→1/2supported,one232triangle casing group;other95triangle surface stillwired
sampler unsupported. Material eligibility counts are not asset counts.

Exact commands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d57-mesh-samples.json local-analysis/d57-compatibility-baseline.json:
  beforecode38viewable/34all-diffuse/0Complete. Same with
  local-analysis/d57-compatibility-after.json aftercode:same totals,232triangles gained.
- ./scripts/run-material-oracle.ps1 -Diffuse -SampleManifest local-analysis/d57-single-mix-material-sample.json
  -Output local-analysis/d57-single-mix-material-oracle.json:1record8742stored bytes;
  synthetic bounds pass,known Toolkit extreme signed ZigZag conflict printed,
  unchanged and not adopted. Pinned source clean,all derived output ignored.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d57-single-mix-material-oracle.json local-analysis/d57-single-mix-parity.json:
  1compared0mismatches1eligible. Existing d46-diffuse-oracle.json/d57-diffuse-parity.json:
  32compared0mismatches27eligible versus26. Existing d46-linked-material-oracle.json/
  d57-linked-parity.json:48compared0mismatches46eligible unchanged. New parity branch
  checks single MIX root,exact source ports/swizzles,input direction,color/weight
  bits plus existing primary sampler coefficient/slot/channel/wrap comparisons.
- cargo fmt --all; cargo fmt --all --check:pass.
- cargo test -p lbp-preview --lib:15pass. cargo test -p lbp-preview -p lbp-bevy-viewer
  -p lbp-viewer:90pass(15/10/65). New singleMIX pixel test covers linear RGB/alpha,
  asymmetric direction,endpoints,source preservation,bad pixel length,early/mid
  output cancellation. Selector tests cover matrix UV1/exact slot/reversed wires/
  both port directions,wired alpha factor,named sampler,wired sampler/ThingColor.
  Existing color/weight/UV/source/bounds/cache/cancel/completeness regressions pass.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:pass.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.

Headless CPU/metadata verification sufficient for constant mixing through the
existing pixel/display path;0native windows/0images,no placement necessary.
Not tested:new-route native display/game pixel/pose/full shader/alpha/mips/filter
fidelity/manualComplete,other GPUs/platforms/revisions/corpus. No failed tests/
compile/Clippy or approval rejection. Optional lookup of absent older local mesh
oracle failed harmlessly;the added mesh is explicitly catalog-linked only.
SFO/MAP SHA256 unchanged from D56,FARC1023990298bytes ReadOnly/Archive,Toolkit
pin d6a2ef80b5a09999f08f23a803b516096055e5c9 sourceclean. Defaultdeny isolated
ignore audit allows authored Rust/docs,denies privateJSON/game/referenceJAR.
No original writes/proprietary assets/code/keys/recovered source/decryption/
commit/push/publication or strictcleanroomclaim. D57 complete. Next:inspect the
bounce-pad surface's sampler wiring and alpha-driven COLOR/texture MIX routes
from pinned metadata before a bounded implementation;do not substitute instance
values or assume nonlinear alpha mixing commutes with filtering.

## D58 — same-sample alpha MIX (2026-10-06 America/Chicago)

Target bouncepad surface00012fd3/4640ecf1ae1810a3cf19580c45ba9aa672d1259b is
actually direct ThingColor (6476stored bytes),not a wired sampler;instance input
is absent,no default invented. Pivot:polaroid00006a16/05e7d877fa19251c37c9563143f95c036c2bd667
uses COLOR/sample MIX with same sample alpha swizzle on factorport2. Independent
fixed selection/baking implemented;existing static COLOR/product MIX unchanged.
Evidence URLs/pin/methods/applicability/conflicts/tests:docs/alpha-mix-preview.md.

Four selected nodes/four edges:staticoutput,directdiffuse,staticMIX,staticunwired
COLOR finite0..1 and staticunwired UV0/UV1 sampler with existing bounded transform.
Ports0/1unswizzled,port2exact same sample alpha[97,0,0,0,0],allsourceports0.
1024node/4096wire endpoints validated;other factors/swizzles/cycles/ThingColor/
namedanimation/wiredsampler/missingUV/texture/wrap remain unsupported.
AlphaDiffuseMix separate fromconstantDiffuseBlend;overridden weightbits retained,
not validated asactiveweight. RawUV/originalpixels intact.192KiB heap LUT+2KiB
transfer table;existing<=16MiB derived buffer. Temporarymaximum194KiB;existing
four-source read/decompression/cache ceilings unchanged. Cancelbeforework,
every16alpha rows/every4096pixels/atcompletion. Bothfrontends nearest only;
nonlinear alpha mixing does not commute withlinear filtering,no filteredgame
parity claim. App label statesfactor/colorport/UV/wrap/nearest;distinctGUID
Viewable/Complete review unchanged,noautomaticComplete. Oracle schema9/census3.

Concreteblocker:polaroid selected wrap[4,4],pinned TextureWrap calls4 BORDER.
Bordercolor/gameedgebehavior notestablished;noRepeat/Clamp substitution.
48structural4node/4wire/unanimatedtail material records (376627storedbytes) show
onlythis same-samplealpha case;noother supported-wrap asset established.
Beforecode d58-mesh-samples.json adds catalog-linked polaroid00006a1a,
5c8cb186d0b5b87089f90d3cd863cc87eddc650c,toprior38mesh hashes (no externalgeometry
oracleforaddedmesh). d58-compatibility-baseline/after/delta.json:same39hashes/
39distinctGUIDs,identicalgeometry/groupcoverage:39→39viewable,34→34all-diffuse,
0→0verifiedComplete. **+0newly viewable/+0newly Complete,0improvedsampledassets**.
Polaroid1/2groupssupported;32trianglealphagroup remainsblockedbyBORDER.
Metadataeligibilityisnotpayloadcompatibilityorassetcount. Corpustotalsunknown.

Exactcommands after . ./scripts/use-local-rust.ps1:
- ./scripts/run-material-oracle.ps1 -Diffuse -SampleManifest local-analysis/d58-surface-material-sample.json
  -Output local-analysis/d58-surface-material-oracle.json:1record6476bytes.
  Samewith d58-alpha-material-sample.json/d58-alpha-material-oracle.json:
  1record6804bytes. Samewith d58-alpha-search-samples.json/d58-alpha-search-oracle.json:
  48records376627bytes. Syntheticbounds pass;knownToolkit extreme signedZigZag
  conflictprintedunchanged/notadopted,pinnedsourceclean,allresearchignored.
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d58-mesh-samples.json local-analysis/d58-compatibility-baseline.json:
  beforecode39/34/0. Samewith d58-compatibility-after.json:39/34/0unchanged.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d58-alpha-material-oracle.json local-analysis/d58-alpha-parity.json:
  1compared0mismatches1projected. Existing d46-diffuse-oracle.json/d58-diffuse-parity.json:
  32compared0mismatches28projected versus27;d46-linked-material-oracle.json/
  d58-linked-parity.json:48compared0mismatches46projectedunchanged.
  Newparitychecksroot/allports/sources/swizzles/color/direction/overriddenweight,
  existingprimaryslot/channel/coefficients/wraparrays retained.
- cargo fmt --all; cargo fmt --all --check:pass.
- cargo test -p lbp-preview --lib:initialcompilefailed due label inserted intexture()
  matchinstead oflabel();fixedandrerun16pass. cargo test -p lbp-preview
  -p lbp-bevy-viewer -p lbp-viewer:91pass(16/10/65). Pixeltests alpha0/255/128,
  linearRGB/alpha,bothdirections,originalpixels,badcolor/length,early/table/midoutput
  cancel. Selector asymmatrixUV1/reversedwires/nonfiniteoverriddenweight/wrong
  factorchannel/source;existingbounds/cycles/UV/instance/animation/source retained.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:pass.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.

Headlessonly0windows/0images;nativefilterednew-route display cannot beverified
onthisblockedlocalmaterial. Not tested:fullpayloadbinding/nativeGPU/gamepixels/
pose/borderbehavior/filteredshader/alpha/mips/manualComplete/otherGPU/platform/
revision/corpus. ExploratoryPython dependency lookup failed onnullconfirmedtype,
correctedwithexplicitnullhandling;PowerShell rg wildcardfilename failed,corrected
byrg--files;neitherchangedoriginals. Noapprovalrejection. SFO/MAP SHA256 unchanged
fromD57,FARC1023990298bytesReadOnly/Archive,Toolkitpin d6a2ef80b5a09999f08f23a803b516096055e5c9
clean. IgnoreauditallowsauthoredRust/docs,deniesprivateJSON/game/referenceJAR.
Nooriginalwrites/proprietarybundling/keys/recoveredcode/decryption/commit/push/
publication/strictcleanroomclaim. D58completewithinboundedalpha-route scope.
Next:research BORDER mode4,colorandedge semantics plusfrontenddevice support;
keep polaroidunsupporteduntil evidence supports explicit bounded addressing.

## D59 — interior-only BORDER preview (2026-10-06 America/Chicago)

BORDER4 color/out-of-range semantics remainUNKNOWN. New controlled evidence:
polaroid00006a1a UV0 referenced range [0.0004995167,0.0004994869]..[0.9995005,0.9995005];
UV1 reaches1.4112475 and is not substituted. Entire selected UV0 channel passes
1/4096interior margin. Authored inference:nearest sampling selects in-range texels
on this domain,so existing displayClampToEdge can be used without invoking border
color. No linear filtering/general BORDER/RSX fidelity claim. Khronos nearest
sampling reference and pinned Bevy optionalhardwareborderfeature evidence,
URL/pin/method/applicability/conflicts/tests:docs/interior-border-preview.md.

Globalfrom_raw stillrejects4. Proposed addressing allows4 onlyforvalidated
same-samplealpha MIX (bothfrontendsnearest). Beforetexturedecode/publication,
BorderDomain validatesALLderivedsamplingUVs inselectedmeshchannel,notonlyfirst
primitive,safeacrosssharedmaterialcache. BORDERaxes within[1/4096,1-1/4096],
otherRepeat/ClampToEdge axesretained,allfinite/nonempty/<=100kvertices. NoUVfold/
clip/change/fallback;rejectborderreachable/unknown/missing/non-nearestroutes.
Cancellation256vertices/final,noaddedUV/imagebuffer/source/shader/devicefeature.
OriginalBORDERraw/rangebits retainedseparatelyfromeffectivedisplayaxes;app label
statesvalidateddomain/nearest/unusedbordercolor. Census4source/displaywrap/range/
margin/scope;legacydiagnostic source/displayaxes separate. Complete meansall
textures/correctgeometry,renderercompletionseparate;noautomaticcertification.

Beforecode d59-compatibility-baseline.json;after d59-compatibility-after.json;
delta d59-compatibility-delta.json,same d58-mesh-samples39hash/39distinctGUIDcohort:
identicalGUIDs/vertices/triangles,39→39viewable,34→35all-diffuse,0→0verifiedComplete.
**+0newly viewable/+0newly Complete**. Polaroid gainsone32trianglegroup1/2→2/2,
texture256x256/24890storedbytes. FullpreparationisnotComplete;corpustotalsunknown.

Exactcommands after . ./scripts/use-local-rust.ps1:
- cargo run -p lbp-preview --example border_domain_probe -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc 00006a1a:referencedrawchannelranges above. Firstcompilecalled
  nonexistentprimitive_triangleshelper;correctedtovalidatedgeometryranges.
  Tool2MiBinput/100kvertices/50ktriangles,pagedexactreads,cancel/finite/sourcechecks.
- cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d58-mesh-samples.json local-analysis/d59-compatibility-baseline.json:
  beforecode39/34/0. Samewith d59-compatibility-after.json:39/35/0.
  Finaldelta script verifiescohort/geometryequalityandgainedgroup/trianglecount.
- cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json
  game-data/a1-disc local-analysis/d58-alpha-material-oracle.json local-analysis/d59-alpha-parity.json:
  1compared0mismatches1projected. Existing d46-diffuse-oracle.json/d59-diffuse-parity.json:
  32compared0mismatches28projected;d46-linked-material-oracle.json/d59-linked-parity.json:
  48compared0mismatches46projected. Sourcewrap4arrayspreserved;eligibilityunchanged.
  ExistingpinnedD58oracle reused,no newdisklayoutorproprietaryfixtures.
- cargo fmt --all; cargo fmt --all --check:pass.
- cargo test -p lbp-preview --lib:17pass. Initialfullchecksfailed onBevy synthetic
  Assetfixture missingborder_domain field (tests/Clippy/buildsamecause);addedNone,
  then cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:92pass(17/10/65).
  Guardtests mixedaxes/inside/boundary/outside/nonfinite/missingunknownmodes/
  non-nearest/empty/originalUV/earlymidcancel. Existingalpha/UV/source/cache/
  completenesschecksretained. No failed runtime tests afterfixturecorrection.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings:pass.
  cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.
  Finallegacydiagnosticsource/displayfields:cargo fmt --all --check,
  cargo clippy -p lbp-viewer --all-targets -- -D warnings,cargo build -p lbp-viewer:pass.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00006a1a --browse-query guid:00006a1a
  --qa-capture local-analysis/d59-bevy-interior-border.png:hiddenStart-Process;
  logverifiesnon-primary165v0placementBEFOREshow. Two groupsloaded,sourceBORDER
  range/nearestlabel,source-session1viewable/0Complete/0failed. Oneimageviewedto
  resolvenewtexture/UIquestion:photograph visible,2/2groups,manualCompletechecks
  unchecked. No shadererrors. Windowexitedaftercapture. Nolegacywindowopened.

Not tested:manualComplete/originalgamecomparison/bordercolor/out-of-domain/
linearfilter/mips/fullshader/pose/otherGPU/platform/revision/corpus. UVdomain
mathematicalinferenceandconservativemarginareauthoredpolicy,notformalGPUrounding
proof. FullKhronoswebchapterfetchfailed4MiBlimit;primarysearchreturnedrelevant
nearest/wrappingsections;rawtagURLunavailable. No original writes. Optionalbad
lbp-toolpath/PowerShellwildcardlookupfailedread-only,correctedwithactualpaths.
Noapprovalrejection. SFO/MAP SHA256unchangedfromD58;FARC1023990298ReadOnly/Archive;
Toolkitpin d6a2ef80b5a09999f08f23a803b516096055e5c9 sourceclean. Ignoreauditallows
newauthoredRust/example/docs,deniesprivateJSON/PNG/game/referenceJAR. No proprietary
bundling/keys/recoveredsource/decryption/commit/push/publication/strictcleanroomclaim.
D59complete. Next:inspect MIRROR2 materials and explicitfrontendmirror-address
mapping for a bounded compatibilityslice;generalBORDERcolor/edge behaviorstillunknown.


## D60 completed — MIRROR and independent product axes — 2026-10-06

Implemented typed MIRROR2 on independent U/V axes. Bevy uses MirrorRepeat; legacy
uses explicit uniform mirrored-repeat options and still rejects mixed axes/dual
products. Two-source GPU diffuse MULTIPLY preserves primary and secondary typed
wrapping. Repeat-only LCM baking remains guarded on both samples. Constant product
MIX and shared three/four-source chains retain Repeat-only restrictions. Product
labels show both transforms/axis policies. No shader or disk parser change.
Material census schema2 adds existing raw wrap arrays; compatibility schema5 adds
secondary axes and marks dual products Bevy-only. Rule evidence, pin, API sections,
applicability/conflicts/test coverage:docs/mirror-preview.md. Parser/UI separation,
read budgets/cancel/cache/stability/descriptor guards remain.

Measured asset counts (distinct catalog GUID; static Bevy cohort):
- Original39hash/39GUID before/after single Mirror mapping:39viewable/35full-diffuse/
  0verifiedComplete unchanged. Four of five Mirror materials use it only outside
  the selected diffuse route; raw slot eligibility is not an asset count.
- Extended40hash/40GUID baseline taken after single mapping but before independent
  product support:40viewable/35full-diffuse/0Complete. Final40/36/0.
  Newly viewable0; newly Complete0. Candlestick00002487 already had geometry;
  primitive0 gains1278triangles,3/4 to4/4groups.1032vertices/1888triangles unchanged.
  Full diffuse preparation does not establish all textures/correct geometry.
  Corpus-wide asset totals unknown. No external geometry oracle for this new mesh.
- App displays measured source-session counts and explicit Viewable/Complete
  definitions. Complete still requires manual all-textures and correct-geometry
  verification, separate from renderer completion; verification resets on reload.

Exact verification commands (activate with . ./scripts/use-local-rust.ps1):
- cargo run -p lbp-catalog --example material_census --
  local-analysis/a2-inventory-verified.json game-data/a1-disc
  local-analysis/d60-material-wrap-census-final.json:5163locations/5536MAP rows,
  44060339payloadbytes, all prefix/graph/tail parsed, no cancellation. Bounds20k
  locations/64MiBstored aggregate/1MiBstored per resource/8MiBexpanded/120seconds.
  Earlier d60-material-wrap-census.json matches final exactly after schema1->2
  normalization (the provisional run already contained wrap arrays).
- ./scripts/run-material-oracle.ps1 -SampleManifest
  local-analysis/d60-mirror-material-samples.json -Output
  local-analysis/d60-mirror-material-oracle.json -Diffuse:5records45383bytes,
  synthetic bounded checks pass; known extreme ZigZag conflict printed unchanged.
  Full raw wrap arrays independently compare equal for all5materials.
- cargo run -p lbp-preview --example diffuse_parity --
  local-analysis/a2-inventory-verified.json game-data/a1-disc
  local-analysis/d60-mirror-material-oracle.json local-analysis/d60-mirror-parity.json:
  5compared/0mismatches/5projected. Projection is not visual or asset certification.
- cargo run -p lbp-preview --example compatibility_census --
  local-analysis/a2-inventory-verified.json game-data/a1-disc
  local-analysis/d58-mesh-samples.json local-analysis/d60-compatibility-baseline.json;
  repeated to d60-compatibility-after.json:39/35/0 unchanged.
- Same command with d60-mesh-samples.json ->d60-product-compatibility-baseline.json:
  40/35/0; after ->d60-product-compatibility-after.json:40/36/0; final corrected
  diagnostic frontend metadata ->d60-product-compatibility-final.json:40/36/0.
  Authored metadata comparison writes d60-compatibility-delta.json, asserts same
  GUID/vertex/triangle counts, one1278triangle gain and unchanged Viewable/Complete.
- cargo fmt --all; cargo fmt --all --check:pass.
- cargo test -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer:92pass(17/10/65).
  Covers9axis pairs/missingunknown modes, Bevy descriptor mapping and grid rejection
  on either mirrored sample, retained route/UV/cancel/cache/completeness tests.
- cargo test -p lbp-catalog --example material_census:4pass.
- cargo clippy -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer -p lbp-catalog
  --all-targets -- -D warnings:pass; repeated after final diagnostics:pass.
- cargo build -p lbp-bevy-viewer -p lbp-viewer:pass.
- target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json
  --game-dir game-data/a1-disc --select-guid 00002487 --browse-query guid:00002487
  --qa-capture local-analysis/d60-bevy-mirror-product.png:hidden Start-Process
  -WindowStyle Hidden. Log verifies non-primary165v0 placement before showing;
  all4groups loaded, independent UV0 Repeat/Repeat and UV1 Mirror/Repeat labels,
  source-session1viewable/0Complete/0failed. One image resolves texture/UI question:
  patterned candlestick is textured;4/4groups and unchecked Complete confirmations.
  No shader errors; capture autoexit confirmed. No other native windows/images.

Failed attempts: multi-file patch context mismatch applied nothing, corrected to
actual Bevy image call. Initial restricted Windows run failed canonicalization/
access and2filesystem-dependent legacy tests; repeat with required access passes
all92. A label formatting edit briefly lacked2placeholders; corrected before
successful Clippy/build/native. Documentation write hit default cp1252 encoding;
rewritten explicitly UTF-8. Incorrect short crate/doc paths and root git diff
(root has no Git repository) failed read-only; corrected to crates/ and existing
scope documents. No unchanged approach repeated without addressing its cause.
No automatic approval rejection.

Not tested: original game pixel/geometry/pose comparisons, all required textures,
manual Complete certification, bump/normal branches, mip/filter/alpha fidelity,
legacy native Mirror, otherGPU/platform/revision/full corpus. General BORDER color,
CLAMP5 and mirror-once6–8 remain unresolved. No decryption/decompiler work needed.
Source fingerprints unchanged: SFO SHA256B6491A83D66901A7FC18BB494DC2448811046D959DE7A8CB9AA85EF4E9B99D39;
MAP SHA25651DE580F2BADF74DB647999B212AFBC3FB509D4C389ADEB0EF14008CB4398350;
FARC1023990298bytes ReadOnly/Archive. Toolkitpin
d6a2ef80b5a09999f08f23a803b516096055e5c9 sourceclean.
Isolated ignored local-analysis/d60-ignore-audit Git check-ignore permits authored
Rust/docs; excludes private JSON/PNG/game FARC/reference JAR/unknown.dat. Root has
no Git repository; no staging/commit/push/publication. No proprietary files/keys/
recovered source bundled; no strict clean-room claim.
D60 complete. Next: inspect selected diffuse blockers among the remaining4partial
samples and choose a bounded evidenced route; do not infer Complete from coverage.

## D61 — supplied instance color and texture review (code 2026-10-06, recorded 2026-10-07)

Code was written 2026-10-06 but the session ended before docs/STATUS. Recorded
2026-10-07 after re-running checks. Opt-in user-supplied linear RGBA for ThingColor
(never a game default), port-2 alpha UV subtraction, one nested tint fold, and an
"other required texture slots unrendered" review flag that blocks Complete.
Evidence/rules: [docs/instance-preview.md](docs/instance-preview.md).

Counts (distinct catalog GUID; 48-hash cohort = D60's 40 + 8 catalog-linked meshes):
before code 48/43/0 (viewable/full diffuse/Complete); default after 48/43/0;
supplied white 45; supplied (0.25,0.5,0.75,0.5) + animation base 46.
**+0 newly viewable / +0 newly Complete.** Supplied-color coverage is preview only.

Evidence files (local-analysis/, from 2026-10-06): d61-compatibility-{baseline,final,
reviewed}.json, d61-instance-{white,final,reviewed}.json, d61-blocker-material-oracle.json
(6 records), d61-mesh-{geometry,groups,uv}-parity-final.json: 48 compared/0 mismatches,
d61-global-animation-evidence.json. Exact 2026-10-06 command lines were not recorded.
Re-check 2026-10-07: fmt pass; tests lbp-preview 21, bevy 10, legacy 65 pass; Clippy
and build pass. Not tested then: native window, real instance colors, game pixels.

## D62 — git baseline, partial-sample blockers, subtype-0 animation base (2026-10-07)

Git: local repo initialized at the project root with the existing default-deny
.gitignore; 230 authored files staged (no file over 200 KB, no game/reference/
local-analysis content); baseline commit d7d33b2. Local only, no remote, no push.
The baseline includes the D61 code (see above).

Blockers on D60's 4 partial samples (d62-blocker-material-oracle.json, 8 records):
- 00012fd4 bounce pad, 00002d94 neon lifebar: ThingColor instance input. Covered
  only with a supplied color (D61); real game value unknown.
- 0001da30 geometry boy: animation base (D56) + ThingColor.
- 00020c92 logic LCD: prim 2 ThingColor alpha on sampler port 2 (D61, supplied);
  prim 1 base_logic_lcd_custom 3d32e960… has NO diffuse wire (uses outputs 4
  self-illumination and 6 reflect). What the game shows for unwired diffuse is
  UNKNOWN; not invented. This is the only blocker left with every option on.
- Added in the 48 cohort: 00015fa4 eve_floor_pipe2 bubbles used a subtype-0
  single-curve animated sampler. Implemented this milestone.

Change: animation-base selector accepts subtype 0 with one named curve, base
(x,y,z,w) → scale (x,y), offset (-(z·x), -(w·y)) per Toolkit GfxAssembler 191–196.
Rules/evidence: [docs/animation-base-preview.md](docs/animation-base-preview.md) D62 section.

Counts (48-hash cohort, distinct GUID): default 48/43/0 → 48/43/0; animation base
48/44/0 → 48/45/0 (00015fa4 3/4 → 4/4 groups, +4 triangles); supplied color +
animation base 46 → 47. **+0 newly viewable / +0 newly Complete.** Geometry identical.

Commands (after `. ./scripts/use-local-rust.ps1`; A=a2-inventory-verified.json, G=game-data/a1-disc):
- run-material-oracle.ps1 -Diffuse d62-blocker-material-samples.json → d62-blocker-material-oracle.json: 8 records, synthetic checks pass, known ZigZag conflict unchanged.
- Same for d62-bubbles-material-samples.json → d62-bubbles-material-oracle.json: 2 records.
- compatibility_census A G d61-mesh-samples.json: d62-base-baseline.json (code stashed, --animation-base) 48/44/0; d62-default-after 48/43/0; d62-base-after 48/45/0; d62-instance-base-after (--instance-rgba 0.25,0.5,0.75,0.5 --animation-base) 48/47/0.
- diffuse_parity: d62-bubbles-base-parity (--animation-base) 2 compared/0 mismatches/2 projected; d62-bubbles-static-parity 2/0/0; d62-animation-parity (D56 oracle, --animation-base) 2/0/2; d62-diffuse 32/0/28; d62-linked 48/0/46; d62-alpha 1/0/1; d62-mirror 5/0/5.
- First parity run on the 8-record blocker oracle failed "diffuse absent" (LCD material has no diffuse); rerun on a bubbles/fan-only manifest.
- cargo fmt --all --check pass; cargo test (preview 21, bevy 10, legacy 65, resource/catalog suites) pass; python unittest 34 pass; Clippy -D warnings pass; build pass.
- Native: lbp-bevy-viewer --select-guid 00015fa4 --animation-base --qa-capture d62-bevy-bubbles-base.png, hidden start; log confirms non-primary monitor 165v0 before showing; 4/4 groups, labels show subtype-0 derived UV × (1,1) + (-0,-3); no shader errors. One image viewed; Complete checks stay locked.

Not tested: game pixels, motion/time, real instance colors, unwired-diffuse behavior,
legacy viewer subtype-0 path, other revisions, corpus totals (unknown).
Originals read-only; FARC/SFO/MAP untouched. No publication, keys or recovered code.
Next: decide how to present materials with no diffuse wire (00020c92 prim 1): look
for evidence of the game's default for an unwired diffuse port before any route.

## D63 — unwired diffuse as explicit black (2026-10-07)

Last open blocker: base_logic_lcd_custom has no diffuse wire (glow/reflect only).
Pinned Toolkit GfxAssembler::setupPath returns float4(0.0) for an unwired port; the
game's own behavior is UNKNOWN. Added a narrow labelled zero-diffuse solid route
(static parameterless output, no Blender nodes). Other-channel samplers stay flagged
as unrendered, so Complete stays blocked. Rules/evidence: [docs/unwired-diffuse-preview.md](docs/unwired-diffuse-preview.md).

Counts (48-hash cohort, distinct GUID): default 48/43/0 unchanged (00020c92 1/3 → 2/3
groups); supplied color + animation base 48/47/0 → 48/48/0.
**+0 newly viewable / +0 newly Complete.** Geometry identical.

Commands (after `. ./scripts/use-local-rust.ps1`; A, G, mesh manifest d61-mesh-samples.json as D62):
- compatibility_census: d63-default-baseline (stashed) 48/43/0; d63-all-baseline (stashed, --animation-base --instance-rgba 0.25,0.5,0.75,0.5) 48/47/0; d63-default-after 48/43/0; d63-base-after 48/45/0; d63-all-after 48/48/0.
- diffuse_parity: d63-blocker 8/0 mismatches/1 projected; d63-blocker-base 8/0/5; d63-diffuse 32/0/28; d63-linked 48/0/46; d63-alpha 1/0/1; d63-mirror 5/0/5; d63-animation 2/0/2; d63-bubbles 2/0/2.
- fmt --check pass; tests pass (preview 21 incl. new cases); Clippy -D warnings pass; python 34 pass.
- cargo build -p lbp-viewer pass; cargo check -p lbp-bevy-viewer pass. Full Bevy build failed: exe locked by the user's open viewer window (started 08:29), which I did not close.
- Failed: first test compile missed a DiffuseTint path; PowerShell Set-Content added a BOM to linked.rs (removed); old instance test expected an error for no-diffuse graphs (updated to pass-through).

Not tested: native capture (exe locked), game pixels for unwired diffuse, glow/reflect rendering, corpus count of unwired-diffuse materials.
Next: relink and capture 00020c92 once the viewer is closed; then widen the cohort (corpus-wide census of which diffuse routes remain unsupported) to pick the next blocker by asset count.

## D64 — corpus route census and same-sample multiply (2026-10-07)

D63 follow-up: Bevy relinked after the user closed the viewer; capture of 00020c92
(d63-bevy-lcd-unwired.png) placed on non-primary 165v0, black unwired label, no
shader errors (log only, image not viewed).

New diagnostic `diffuse_route_census` over all 5163 materials: 4175 texture, 459 solid,
7 unwired, 34 animation-base-only, 122 instance-only, 366 unsupported. Mesh-weighted
ranking and shapes: [docs/route-census.md](docs/route-census.md). Implemented the
cheapest exact gap: one sample wired to both multiply ports (texel squared).

Counts (distinct GUID): new 26-hash squared cohort 26/0/0 -> 26/23/0 (groups 36 -> 65
of 68, geometry identical); D61 48 cohort 48/43/0 unchanged.
**+0 newly viewable / +0 newly Complete.**

Commands (after `. ./scripts/use-local-rust.ps1`):
- diffuse_route_census A G d60-material-wrap-census-final.json -> d63-diffuse-route-census.json and d63-diffuse-route-shapes.json: 5163 materials, 44060339 bytes. First run failed "Hex page size must be 1..4096" (page 65536), fixed.
- run-material-oracle.ps1 -Diffuse d64-square-material-samples.json -> d64-square-material-oracle.json: 17 records.
- diffuse_parity d64-square 17/0 mismatches/17 projected; regressions d64-diffuse 32/0/28, d64-linked 48/0/46, d64-alpha 1/0/1, d64-mirror 5/0/5, d64-blocker 8/0/1.
- compatibility_census: d64-square-baseline (linked.rs stashed) 26/0/0; d64-square-after 26/23/0; d64-cohort48-after 48/43/0. A combined 74-sample manifest failed the 48-sample census budget; split instead.
- fmt, Clippy -D warnings, tests (preview 21), python 34, both builds: pass.
- Native: --select-guid 0000454e (snow_lake) capture, non-primary 165v0, 4/4 textured; one image viewed.

Bug found: that image showed mojibake bullets in labels. The D63 PowerShell
Get-Content/Set-Content round trip had double-encoded 11 non-ASCII lines in linked.rs
(committed in 923c4f7). Repaired by reversing cp1252 -> UTF-8; diff to D62 shows only
intended text; recapture log d64-bevy-qa2.out.txt has clean bullets. CLAUDE.md now
forbids PowerShell text rewrites of source.

Not tested: game pixels, filtered/colour-space fidelity of squared products, external
geometry oracle for the 26 added meshes, corpus-wide binding (selection only).
Next: three-texture product with an independent third transform (largest blocker,
84 meshes blocked by nothing else); needs a three-sampler Bevy path or a bounded bake.

## D65 — three-texture product with independent third transform (2026-10-07)

Largest D64 census blocker. The third sample now re-derives its coordinates as a
scale/offset of the displayed primary or second coordinates on the same raw channel
(f64 then one f32 rounding), passed to the Bevy shader as uniform 107. Legacy keeps
refusing independent-sampling products. Rules/evidence: [docs/third-sample-preview.md](docs/third-sample-preview.md).

Counts (distinct GUID): new cohort d65-third-mesh-samples-pinned.json (48 of 85 meshes
blocked only by these materials) 48/0/0 -> 48/29/0, groups 92 -> 134 of 235, geometry
identical. D61 48 cohort 48/43/0 and D64 squared 26/23/0 unchanged.
**+0 newly viewable / +0 newly Complete.**

Commands (after `. ./scripts/use-local-rust.ps1`):
- run-material-oracle.ps1 -Diffuse d65-third-material-samples.json -> d65-third-material-oracle.json: 48 records, 466968 bytes.
- diffuse_parity d65-third 48/0 mismatches/48 projected; d65-diffuse 32/0/29 (was 28: one D46 material now projects), d65-linked 48/0/46, d65-alpha 1/0/1, d65-mirror 5/0/5, d65-square 17/0/17.
- compatibility_census d65-third-baseline2 (code stashed) 48/0/0; d65-third-after2 48/29/0; d65-cohort48-after 48/43/0; d65-square-after 26/23/0.
- fmt, Clippy -D warnings, tests (preview 21, bevy 10, legacy 65, others), python 34, both builds: pass.
- Native: --select-guid 00002c6e (japan_vase_cracked) capture, non-primary 165v0, 2/2 textured, label shows third from second x [0.5,0.5]; clean UTF-8; one image viewed.

Failed: first census run lacked toolkit_commit in the new manifest ("mesh oracle pin
differs"), rewritten as -pinned. Two test assertions assumed the wrong primary/second
order; made role-aware. git stash round trip left lib.rs/linked.rs as LF; restored CRLF.

Not tested: game pixels, filtering/mip fidelity, matrix-base thirds, legacy rendering.
Next: the 8-distinct-material mesh preview budget (78 groups in this cohort alone);
measure corpus impact before raising it with bounded memory.

## D66 — per-mesh material budget (2026-10-07)

Replaced the 8-distinct-material count with a 64-load cap plus a 640 MiB retained-bytes
aggregate (the old implied total); Bevy draw cap 9 -> 65. Policy and worst cases:
[docs/material-budget-preview.md](docs/material-budget-preview.md).

Counts (distinct GUID): new cohort d66-many-material-mesh-samples.json (all 22 meshes
with >8 materials) 22/0/0 -> 22/5/0, groups 157 -> 449 of 504; D65 cohort 48/29/0 ->
48/30/0, groups 134 -> 212 of 235; D61 48/43/0 and D64 26/23/0 unchanged. Geometry
identical. **+0 newly viewable / +0 newly Complete.** No mesh hit the memory budget.

Commands (after `. ./scripts/use-local-rust.ps1`):
- compatibility_census: d66-many-baseline (lib.rs/linked.rs stashed) 22/0/0; d66-many-after 22/5/0; d66-third-after 48/30/0; d66-cohort48-after 48/43/0; d66-square-after 26/23/0.
- fmt, Clippy -D warnings, tests (preview 21, bevy 11 incl. new draw-budget test, legacy 65, others), python 34, both builds: pass.
- Native: --select-guid 0001d050 (cosmos_room_default_lbp2, 59 primitives) first capture FAILED "material draw budget" (Bevy adapter cap 9 not covered by the census); after raising MAX_DRAWS, d66-bevy-many2.png: non-primary 165v0, 58/59 groups, 1 neutral, 24 s incl. startup; one image viewed.

Failed: the census reported 0001d050 viewable while Bevy refused it; the census does not
run adapter grouping. rustfmt again left lib.rs as LF; CRLF restored before commit.
Not tested: peak process memory on the largest meshes, legacy viewer, game pixels.
Next: "third texture needs a scale/offset sample on its channel" (29 groups across the
D65/D66 cohorts), then rerun the corpus route census to re-rank blockers.

## D67 — third-texture role swap (2026-10-07)

Third samples on a channel neither displayed sample uses now swap the second/third
roles (commutative componentwise product). Rules/evidence/counts: D67 section of
[docs/third-sample-preview.md](docs/third-sample-preview.md).

Counts (distinct GUID, baseline D66 runs): D65 cohort 48/30/0 -> 48/38/0; D66 cohort
22/5/0 -> 22/7/0; D61 48/43/0, D64 26/23/0 unchanged; geometry identical.
**+0 newly viewable / +0 newly Complete.** Corpus selection: unsupported 366 (D63) -> 235.

Commands (after `. ./scripts/use-local-rust.ps1`):
- run-material-oracle.ps1 -Diffuse d67-thirdbase-material-samples.json -> d67-thirdbase-material-oracle.json: 18 records, 177564 bytes.
- diffuse_parity: d67-thirdbase 18/0/18; d67-third 48/0/48; d67-diffuse 32/0/29; d67-linked 48/0/46; d67-square 17/0/17; d67-mirror 5/0/5; d67-alpha 1/0/1.
- compatibility_census: d67-third-after 48/38/0; d67-many-after 22/7/0; d67-cohort48-after 48/43/0; d67-square-after 26/23/0.
- diffuse_route_census -> d67-diffuse-route-shapes.json: 5163 materials, 44060339 bytes.
- fmt, Clippy -D warnings, all tests, python 34, both builds: pass. rustfmt LF drift restored to CRLF.
- Native (log only, no image): --select-guid 000024dc, non-primary 165v0, label "third from primary coordinates", clean UTF-8, no errors.

Not tested: game pixels, float rounding order of permuted products, legacy viewer.
Next: two-texture products that also multiply a static COLOR (about 90 materials /
80 meshes across several shapes; currently mislabelled "sampler must be static").

## D68 — official update 01.33 acquired as a separate overlay input (2026-10-07)

User asked for the newest game version. Disc/ISO left untouched (read-only rule); all 32
official packages 01.01..01.33 from Sony's update server were verified and extracted in
order into ignored `game-data/a1-update-0133/` (1,742 files, 3.71 GB; APP_VER 01.33).
Evidence, tables and overlay plan: [docs/update-overlay.md](docs/update-overlay.md).

Counts (patch MAP vs disc MAP, distinct GUID): 23,229 patch rows = 21,270 new GUIDs,
1,014 replaced hashes, 945 unchanged. 32 accumulating `patches/patch_NNNN.farc`
(22,353 unique hashes, 21,437 not on disc). 9,665 reachable binary resources use
revisions 952..1016, beyond current readers (<= 951).
**+0 newly viewable / +0 newly Complete** (no code change; overlay not wired in).

Commands (scripts under ignored `local-analysis/lbp2-update-lists/`):
- ver.xml fetch: 32 packages, 4,047,336,256 bytes.
- dl.py: 32/32 verified (SHA-1 of file minus 32-byte footer == XML == footer).
- pkgx.exe (csc build of pkgx.cs) x32 in version order: all succeeded.
- compare.py: SFO, MAP, FARC coverage counts above. revisions.py: revision histogram.

Failed: whole-file SHA-1 never matches Sony's XML (cost one aborted download pass).
Not tested: scanner/viewer on the overlay, `patch.edat` (NPDRM, holds 528 .ff/.sbu rows
by size inference), EBOOT 01.33, text resources.
Next: user decision on wiring the overlay (second input root, patch MAP wins per GUID,
hash lookup across all FARCs), then revision 952..1016 support. Prior D67 Next (two-texture
x COLOR products) remains open.

## D69 — 01.33 becomes the working input (2026-10-07)

User decision: move to the latest update outright. Disc + installed update are read as
layers (`--game-dir "game-data/a1-disc;game-data/a1-update-0133"`); patch MAP rows win
per GUID; heads to LBP2_MAX 0x3f8 accepted. Rules, evidence and tables:
[docs/update-overlay.md](docs/update-overlay.md) (D69 section).

Counts: MAP rows 36,854 -> 58,124 (rows); parsed headers 16,515 -> 27,624 (inspected
locations); materials 5,163 -> 8,159, all parse; texture-route 4,306 -> 6,828 (material
locations). New 48-mesh 01.33 cohort (post-951 meshes, distinct GUID): 48 viewable /
45 full diffuse / 0 Complete, baseline 0 (absent on 01.00). Existing cohorts unchanged.
**+48 newly viewable in the tested cohort / +0 newly Complete.** Corpus totals unknown.

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- lbp-cli --game-dir "disc;update" -> d68-inventory-0133-verified.json: 58,124 rows, 33 FARCs, 0 ambiguous, partial false.
- run-{material,mesh,plan,animation}-oracle.ps1 -Archive patch_01xx.farc on d68-samples-*.json: 66/66 parsed.
- {material,mesh,plan,animation}_parity + diffuse_parity vs those: 29/0, 18/0, 14/0, 5/0, 29/0 mismatches.
- compatibility_census on 01.33: d68 new 48/45/0; D61 48/43/0; D65 48/38/0; D66 22/7/0; D64 26/23/0.
- material_census d68-material-census-0133b.json: 8,159/8,159; diffuse_route_census: texture 6,828, solid 603, unsupported 386.
- fmt check, Clippy -D warnings (workspace), all tests, python 34, both viewer builds: pass.
- Native Bevy --select-guid 0002e3db (01.33 Van Gogh costume): non-primary 165v0, 8/8 primitives, 0 failed; image not viewed.

Failed: first overlay census ran 33 s vs 2 s (each 4 KiB read reopened ~2,040 stamped
files); fixed by checking identity files per read and archives per sweep. Diffuse route
census aborted on 65 FARC hashes with no MAP row (superseded copies); now counted.
Not tested: legacy viewer on 01.33, EDAT/EBOOT, text resources, game pixels, corpus-wide
mesh census.
Next: rerun the D67 Next item (two-texture x COLOR products) on 01.33, re-ranking
unsupported diffuse shapes from d68-diffuse-route-shapes-0133.json (386 unsupported).

## Batch 2026-10-07b — cloud D72, D71, D73 merged (2026-10-07)

First on-demand batch: cloud agents built three branches; the PC Integrator verified
and merged them (owner's standing approval, 2026-10-07). Details per milestone:
[d72-ci.md](docs/d72-ci.md), [d71-container-bounds-tests.md](docs/d71-container-bounds-tests.md),
[d73-revision-gate-tests.md](docs/d73-revision-gate-tests.md).

- D72: GitHub Actions CI (ubuntu-latest: fmt, non-GUI test/clippy, Python unittest).
- D71: synthetic FARC, MAP and SFO bounds/mutation tests (lbp-archive, lbp-io, lbp-core).
- D73: synthetic tests for the 01.33 revision gates 952..=0x3f8 (lbp-resource).
- D74 (PR #6) closed as a duplicate of D71; not merged.
**+0 newly viewable / +0 newly Complete** (tests and CI only; no parser or viewer change).

Commands (after `. ./scripts/use-local-rust.ps1`), each branch alone in a separate
worktree, then again on merged main:
- cargo fmt --all --check: pass.
- cargo test -q -p lbp-core/-p lbp-io/-p lbp-archive/-p lbp-resource: pass (merged: 2/19/20/38 passed).
- cargo clippy -q (those four crates) --all-targets -D warnings: pass.
- python -m unittest discover -s tests: 34 run, OK (1 skipped).
- git merge-tree: no conflicts among the three branches.

Not tested: viewer builds, oracle/parity runs (no format code changed), CI run on GitHub.
Note: an earlier PC-local batch-driver session produced D70/D71 on a local branch; only
its pushed D71 was merged here.
Next: unchanged — two textures x static COLOR (docs/backlog.md item 1), done on the PC.

## Batch 2026-10-07c — cloud D77, D76, D75 merged (2026-10-07)

Cloud implementers built three branches; the PC Integrator verified them on 01.33 and
merged all three (standing approval). Detail: [d77-branch-4431.md](docs/d77-branch-4431.md),
[d76-blend.md](docs/d76-blend.md), [d75-two-tex-color.md](docs/d75-two-tex-color.md).

- D77: branch 0x4431 (Vita DOUBLE11) headers read at heads 0x3c1..=0x3e2, branch rev
  <= 0x87; payloads refused. 01.33 scan (resource locations): metadata_parsed
  27,624 -> 27,740, unsupported 117 -> 1 (FNTb font). 116/116 now pass the gate.
- D76: BLEND research doc only (proposed rule, test plan); no code.
- D75: two textures x static COLOR under two multiplies. Route census on 01.33
  (material locations, 8,094): texture 6,828 -> 6,938, instance-only 207 -> 225,
  unsupported 386 -> 258; no other class changed. Doc predicted 273..358; the
  sampler-rule shapes all qualified (COLOR was mislabelled as a sampler).
- Counts (distinct catalog GUID): new D75 cohort d75-mesh-samples.json (48 of 74 meshes
  using D75 materials): 48/0/0 -> 48/44/0 (viewable/all-diffuse/Complete). Cohorts
  d68-new 48/45/0, D61 48/43/0, D64 26/23/0 unchanged.
**+0 newly viewable / +0 newly Complete**; +44 newly all-diffuse in the D75 cohort.
Corpus totals unknown.

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests lbp-resource/lbp-mesh/lbp-cli/lbp-preview, both viewer builds: pass.
- lbp-cli --game-dir "disc;update" -> d77-inventory-0133.json: exit 0, partial false, unsupported 1.
- diffuse_route_census -> d75-diffuse-route-shapes-0133.json: counts above.
- run-material-oracle.ps1 -Diffuse on the 128 changed materials (12 manifests d75-samples-*, max 48 each) + diffuse_parity: 128 compared, 0 mismatches, 110 projected.
- diffuse_parity regressions: d64-square 17/0/17, d65-third 48/0/48, d67-thirdbase 18/0/18, d46-diffuse 32/0/29, d46-linked 48/0/46, d68 patch_0120 26/0/20, patch_0131 3/0/3.
- compatibility_census baseline = worktree at b905b17 (pre-D75), after = merged main.

Failed: one oracle run hit the 48-sample budget (split); d75-mesh-samples-2.json (26
meshes) failed "sample absent from catalog" (meshes without MAP rows), not counted.
Not tested: native viewer capture, legacy viewer on 01.33, game pixels, Vita payloads.
Next: backlog rank 2 (animated-sampler products, 65) or rank 4 (MIX, 14); D76 BLEND
rule ready for implementation.

## Batch 2026-10-07d — cloud D79, D80, D78 merged (2026-10-07)

Last cloud batch (later batches run locally on plan usage, owner request). Detail:
[d79-fntb-font.md](docs/d79-fntb-font.md), [d80-thing-color.md](docs/d80-thing-color.md),
[d78-animated-product.md](docs/d78-animated-product.md).

- D79: FNTb font face header + glyph table reader. 01.33 scan (resource locations):
  metadata_parsed 27,740 -> 27,741, unsupported 1 -> 0. Clippy 1.99 rejected three
  `chunks_exact` calls; fixed on the branch by the Integrator (e328bf1), then merged.
- D80: THING_COLOR research doc only (instance-color sources, supplied-color proposal).
- D78: animation-base opt-in extended to 2+ texture products (one animated leaf).
  Route census on 01.33 (material locations, 8,094): unsupported 258 -> 209,
  animation-base-only 60 -> 106, instance-only 225 -> 228; texture route unchanged
  (6,938; opt-in). Branch had converted linked.rs and compatibility_census.rs to LF;
  merged with CRLF kept.
- Counts (distinct catalog GUID, default and --animation-base): d68-new 48/45/0,
  D61 48/43/0, D75 cohort 48/44/0, all unchanged.
**+0 newly viewable / +0 newly Complete.** Corpus totals unknown (D81 running).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests lbp-resource/lbp-cli/lbp-preview, both viewer builds: pass.
- lbp-cli on 01.33 -> d79-inventory-0133.json: exit 0, partial false, no unsupported.
- diffuse_route_census -> d78-diffuse-route-shapes-0133.json: counts above.
- run-material-oracle.ps1 -Diffuse on the 49 changed materials (6 manifests d78-samples-*) + diffuse_parity --animation-base: 49 compared, 0 mismatches, 46 projected.
- diffuse_parity regressions (default): d75 DATA-1 48/0/41, d64-square 17/0/17, d65-third 48/0/48, d67-thirdbase 18/0/18, d46-diffuse 32/0/29, d46-linked 48/0/46, d68 patch_0120 26/0/20.
- compatibility_census: cohorts above.

Failed: first D79 merge failed Clippy; reset and re-merged after the fix.
Not tested: native viewer, real glyph semantics (FNTb +8 word unknown), animated
playback, game pixels.
Next: MIX (14) and BLEND (14, D76 rule) on the PC; D81 corpus census and D82 wrap modes in progress.

## Batch 2026-10-07e — local D81, D82, D83, D84 + audit fixes merged (2026-10-07)

First batch run as local subagents on plan usage (own worktrees). Detail:
[d81-corpus-census.md](docs/d81-corpus-census.md), [d82-wrap-modes.md](docs/d82-wrap-modes.md),
[d83-mix-shapes.md](docs/d83-mix-shapes.md), [d84-blend-route.md](docs/d84-blend-route.md).

- D81: new `corpus_mesh_census` example. First corpus-wide totals on 01.33 (distinct
  catalog GUID, 4,261 mesh GUIDs / 4,245 hashes): viewable 4,257, all-diffuse 3,657,
  Complete 0 (not certified). 2 hashes over the triangle limit, 2 without geometry,
  37 unmapped hashes excluded. Matches compatibility_census on D61 (48/43/0).
- D82: legacy CLAMP (raw 5) shown edge-clamped; mirror-once 6-8 stay refused (meaning
  unknown). 0 material locations on 01.33 use raw 5-8.
- D83: two-texture alpha MIX on the Repeat grid. Route census texture 6,938 -> 6,940,
  unsupported 209 -> 207. Parity on the 8 alpha-MIX materials: 0 mismatches, 2 projected.
- D84: static BLEND phase 1 (D76 rule, thresholds 0.15/0.16). Selects 0 of the 14
  BLEND materials (none use those thresholds; all masks unswizzled); reasons only.
- Audit fixes (PR #13): CI pinned to Rust 1.99.0 with --locked, viewer check job,
  workflow doc rewritten for on-demand PC batches, daily-batch script removed.
**+0 newly viewable / +0 newly Complete** vs the D81 baseline (post-batch corpus rerun:
4,257 / 3,657 / 0, unchanged).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests lbp-preview/-resource/-viewer/-bevy-viewer/-cli/-archive/-io, both viewer builds, python 34 OK.
- diffuse_route_census -> d84b-diffuse-route-shapes-0133.json: texture 6,940, solid 603, unwired 10, anim-base 106, instance 228, unsupported 207.
- diffuse_parity: d83-alpha2 8/0/2; d75 DATA-1 48/0/41; d78 DATA --animation-base 38/0/36; d68 patch_0120 26/0/20; d64-square 17/0/17; d65-third 48/0/48; d67-thirdbase 18/0/18; d46-diffuse 32/0/29; d46-linked 48/0/46.
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b84-corpus-mesh-census-0133.json: 4,257 / 3,657 / 0, 241 s.

Failed: D84 merge conflicted in linked.rs tests (both added a test at the same place);
kept both. rust-toolchain.toml not added (local rustup names the toolchain "stable";
a "1.99.0" pin would force a download).
Not tested: native viewer on the 2 alpha-MIX materials, CI viewer job on GitHub.
Next: remaining MIX shapes need per-sample UVs; BLEND needs non-0.15/0.16 threshold
evidence; animated-sampler products with 2+ animated leaves; THING_COLOR (D80 proposal).

## Batch 2026-10-07f — local D85, D86, D87 merged (2026-10-07)

Local subagents on plan usage (own worktrees; resumed once after a session restart).
Detail: [d85-optin-corpus-census.md](docs/d85-optin-corpus-census.md),
[d86-thing-color.md](docs/d86-thing-color.md), [d87-small-families.md](docs/d87-small-families.md).

- D85: corpus_mesh_census gains --animation-base and --instance-rgba (shared parser
  with compatibility_census); default output byte-identical.
- D86: under a supplied instance color, a static diffuse-root MULTIPLY/MIX of two
  constants (ThingColor/COLOR/CONSTANT) folds to one COLOR. 8 materials; default unchanged.
- D87: CONST4 alone (9) and COLOR x COLOR (7) as solid colors; TEX with CONST/CONST2 on
  sampler port 1 (9) folded into the sampler scale. 25 materials.
- Route census on 01.33 (material locations, 8,094): texture 6,940 -> 6,949, solid
  603 -> 619, instance-only 228 -> 236, unsupported 207 -> 174.
- Corpus mesh census on 01.33 (distinct catalog GUID, 4,261; baseline D81/D85):
  viewable 4,257 in every mode; all-diffuse default 3,657 -> 3,665, --animation-base
  3,709 -> 3,717, both opt-ins (RGBA 0.25,0.5,0.75,0.5) 3,932 -> 3,954. Complete 0.
**+0 newly viewable / +0 newly Complete** (all-diffuse +8 default).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests lbp-preview (42)/-resource/-viewer/-bevy-viewer/-cli, both viewer builds, python OK.
- diffuse_route_census -> b87-diffuse-route-shapes-0133.json: counts above.
- diffuse_parity: d86 disc 5/0/5 and patch_0128 3/0/3 (--instance-white), d86 disc default 5/0/0; d87 oracles 6/0/6, 1/0/1, 18/0/18; regressions d83-alpha2 8/0/2, d75 48/0/41, d78 --animation-base 38/0/36, d68 patch_0120 26/0/20, d64 17/0/17, d65 48/0/48, d67 18/0/18, d46-diffuse 32/0/29, d46-linked 48/0/46.
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b87-corpus-{default,anim,both}.json: counts above.

Failed: session restart stopped all three agents once (resumed from their worktrees).
D87 merge conflicted in diffuse_parity's report line (D86 and D87 both bumped schema
to 15); kept D86's instance_white field, schema now 16.
Not tested: viewer captures, real instance colors (D86 materials can't be Complete).
Next: THING_COLOR shapes with textures under the instance color; ADD (8); UV channel
raw 2 (4); remaining MIX per-sample UVs; BLEND thresholds.

## Batch 2026-10-07g — local D88, D89, D90 merged (2026-10-07)

Local subagents on plan usage. Detail: [d88-thing-color-textured.md](docs/d88-thing-color-textured.md),
[d89-add-uv2.md](docs/d89-add-uv2.md), [d90-texture-census.md](docs/d90-texture-census.md).

- D88: under a supplied instance color, THING x animated texture (7; also needs the
  animation-base opt-in) and texture x (THING x COLOR) (1). Default unchanged.
- D89: ADD of two textures on the Repeat grid and texture x COLOR + COLOR (4 locations).
  ADD refused under a supplied instance color. UV channel raw 2 stays unsupported
  (conflicting Toolkit evidence; documented).
- D90: new corpus_texture_census. 01.33: 22,973 of 23,247 texture hashes decode (23,195
  of 23,477 GUIDs); 274 unsupported (GTF 0x85 A8R8G8B8 187, 0x81 B8 80, 0xa5 3, cubes 4).
  Oracle: 16 Rust-decoded samples 0 mismatches; Toolkit also decodes 0x85/0x81/0xa5.
- Route census on 01.33 (material locations, 8,094): texture 6,949 -> 6,953,
  instance-only 236 -> 244, unsupported 174 -> 162; solid 619 unchanged.
- Corpus mesh census (distinct catalog GUID, 4,261; viewable 4,257 in every mode):
  all-diffuse default 3,665 -> 3,669, --animation-base 3,717 -> 3,721, --instance-rgba
  3,894 -> 3,899 (baseline rerun on 2ef7fce by D88), both 3,954 -> 3,967. Complete 0.
**+0 newly viewable / +0 newly Complete** (all-diffuse +4 default).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests lbp-preview (47)/-resource/-viewer/-bevy-viewer/-cli, both viewer builds, python OK.
- diffuse_route_census -> b90-diffuse-route-shapes-0133.json: counts above.
- diffuse_parity: d89 5/0/2, 1/0/1, 1/0/1; d88 (--animation-base --instance-white) 4/0/4, 1/0/1, 2/0/2, 1/0/1; d86 --instance-white 5/0/5, 3/0/3; d87 6/0/6, 18/0/18; regressions d83 8/0/2, d75 48/0/41, d78 38/0/36, d68 26/0/20, d64 17/0/17, d65 48/0/48, d67 18/0/18, d46 32/0/29 and 48/0/46.
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b90-corpus-{default,anim,inst,both}.json: counts above.
- corpus_texture_census (D90 agent) -> d90-texture-census-0133.json, repeat byte-identical.

Failed: D88 merge conflicted with D89 in linked.rs (binding call) and
diffuse_route_census (instance-only check); kept both. diffuse_parity schema 18.
Not tested: viewer captures, real instance colors, ADD sum vs game (linear clamp is policy).
Next: GTF 0x85/0x81 texture decode (267 hashes, Toolkit-confirmed); THING_COLOR in
sampler ports (14) and 2+ animated samplers (12); BLEND thresholds (38 with THING).

## Batch 2026-10-07h — local D91, D92, D93 merged (2026-10-07)

Local subagents on plan usage. Detail: [d91-gtf-uncompressed.md](docs/d91-gtf-uncompressed.md),
[d92-multi-animated.md](docs/d92-multi-animated.md), [d93-counts-in-app.md](docs/d93-counts-in-app.md).

- D91: GTF 0x85 A8R8G8B8 and 0x81 B8 decode (swizzled, and 0x20 linear rows 0xa5/0xa1).
  Texture census on 01.33: decoded 22,973 -> 23,215 of 23,247 hashes (23,195 -> 23,445
  of 23,477 GUIDs). Left: 7 unsupported (cubes 4, volume 1, B8 remap 0x02ff 2) and 25 B8
  over the 4 Mi pixel budget. Toolkit pixel parity on 20 new samples: 0 mismatches.
- D92: animation-base opt-in accepts 2+ distinct animated leaves, each with its own base
  rows; nested-tint ThingColor forms refused. Default unchanged.
- D93: both viewers show the corpus mesh/texture census counts (unit, scope, baseline;
  "not measured" without files). Flags --mesh-census, --texture-census, --census-baseline;
  Bevy --print-census prints headlessly.
- Route census on 01.33 (material locations, 8,094): animation-base-only 106 -> 121,
  instance-only 244 -> 247, unsupported 162 -> 144; texture 6,953, solid 619 unchanged.
- Corpus mesh census (distinct catalog GUID, 4,261; viewable 4,257 in every mode):
  all-diffuse default 3,669 (unchanged), --animation-base 3,721 -> 3,733, --instance-rgba
  3,899 (unchanged), both 3,967 -> 3,980. Complete 0.
**+0 newly viewable / +0 newly Complete** (texture decode +242 hashes).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests lbp-preview/-texture/-catalog/-resource/-viewer/-bevy-viewer/-cli, both viewer builds, python OK.
- diffuse_route_census -> b93-diffuse-route-shapes-0133.json: counts above.
- diffuse_parity: d92 oracles (--animation-base --instance-white) 17/0/14, 1/0/1, 1/0/1, 3/0/2, 2/0/0; d88 4/0/4, 1/0/1, 2/0/2, 1/0/1; regressions d78 38/0/36, d75 48/0/41, d83 8/0/2, d68 26/0/20, d86 5/0/5, d87 18/0/18, d89 5/0/2, d64 17/0/17, d65 48/0/48, d67 18/0/18, d46 32/0/29 and 48/0/46.
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b93-corpus-{default,anim,inst,both}.json; corpus_texture_census (D91 agent) -> d91-texture-census-0133.json.
- lbp-bevy-viewer --print-census with those files: counts above, baseline deltas correct, exit 0.

Failed: none. Schema bumps: diffuse_parity 19, compatibility_census 12 (D92).
Not tested: viewer window layout of the census panel; B8 identity-remap colour on
hardware; patch-archive textures vs oracle; animated playback.
Next: BLEND (thresholds, 38 with THING); ThingColor wired into sampler ports (14);
25 B8 textures over the pixel budget; cubes/volumes.

## Batch 2026-10-08a — local D94, D95, D96 merged (2026-10-07)

Local subagents on plan usage. Detail: [d94-blend.md](docs/d94-blend.md),
[d95-texture-rest.md](docs/d95-texture-rest.md), [d96-thing-sampler-ports.md](docs/d96-thing-sampler-ports.md).

- D94: BLEND factor = clamp((m - p0)/(p1 - p0), 0, 1) for any finite p0 < p1
  (Toolkit + shader-constant byte search in the BLEND materials' own shader blobs:
  CONFIRMED_FROM_LOCAL_DATA for 0.67/0.8 and 0.3/0.7; 0/1 STRONG_INFERENCE). All 50
  BLEND locations still refused: RGB (red) mask colour space UNKNOWN. 0 class changes.
  Its byte dumps moved to ignored local-analysis/d94-blend-shader-blobs/.
- D95: B8 pixel cap 16 Mi (byte caps unchanged, new 64 MiB RGBA cap; viewers still
  refuse > 4 Mi pixels at GPU upload). BC1 cubes decode via layered API (face 0 2D,
  faces 1-5 data only; 128-byte face padding measured locally, conflicts with Toolkit).
  Volume and remap 0x02ff stay refused. TextureOracle.java extended.
- D96: under a supplied instance color, constants/ThingColor on sampler ports 0/1/2 fold
  into the sampler scale/offset (6 materials); instance option no longer refuses graphs
  without ThingColor. Default unchanged.
- Texture census on 01.33: decoded 23,215 -> 23,244 of 23,247 hashes (23,474 of 23,477
  GUIDs); 3 unsupported (volume 1, remap 0x02ff 2).
- Route census (material locations, 8,094): instance-only 247 -> 253, unsupported 144 -> 138.
- Corpus mesh census (4,261 GUIDs; viewable 4,257 every mode): all-diffuse default
  3,669, --animation-base 3,733 (unchanged), --instance-rgba 3,899 -> 3,902, both
  3,980 -> 3,983. Complete 0.
**+0 newly viewable / +0 newly Complete** (textures +29 hashes).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests for 7 crates, both viewer builds, python OK.
- diffuse_route_census -> b96-diffuse-route-shapes-0133.json; corpus_texture_census -> b96-texture-census-0133.json (220 s).
- diffuse_parity: d96 DATA 4/0/3 and patch_0128 2/0/2 (--animation-base --instance-white), default 4/0/0 and 2/0/0; regressions d78 38/0/36, d75 48/0/41, d83 8/0/2, d68 26/0/20, d86 5/0/5, d87 18/0/18, d89 5/0/2, d88 4/0/4, d92 17/0/14, d64 17/0/17, d65 48/0/48, d67 18/0/18, d46 32/0/29 and 48/0/46.
- D95 agent: Toolkit texture parity 7/0 (B8 masks and cube face 0), D91 samples 20/0.
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b96-corpus-{default,anim,inst,both}.json; viewer --print-census reads them (exit 0).

Failed: none. diffuse_parity schema 20.
Not tested: viewers on large B8 or cube textures; cube faces 1-5 and patch-only textures
vs oracle; real instance colors.
Next: BLEND mask colour space (needs evidence); remaining THING_COLOR (MIX on non-grid,
FRESNEL/ADD); UV channel raw 2; viewer Complete QA.

## Batch 2026-10-08b — local D97, D98, D99 merged (2026-10-08)

Local subagents on plan usage. Detail: [d97-blend-mask.md](docs/d97-blend-mask.md),
[d98-last-textures.md](docs/d98-last-textures.md), [d99-product-trees.md](docs/d99-product-trees.md).

- D97: BLEND red masks sampled sRGB-decoded, alpha masks linear (STRONG_INFERENCE from
  Toolkit cwlib-gl GlTexture/MaterialBaker, MIT, plus local noSRGB-marker statistics).
  Two-sample BLEND on the Repeat grid. 9 THING_COLOR BLEND locations under the instance
  color; 41 still refused (products under inputs 37, three samples 3, off-grid 1). No
  executable investigation. Raw probes in local-analysis/d97-research/.
- D98: GCM remap word decoded (0x02ff = alpha-only mask; sources PSL1GHT and Toolkit,
  MIT; RPCS3 GPL-2.0 read only, nothing copied). Volume slices as one swizzled 3D block
  (RPCS3 only; noise-like data could not confirm); slice 0 via the layered API.
- D99: static MULTIPLY trees of texture/COLOR leaves (16) and two-texture alpha MIX
  off the integer grid via the Bevy dual path (6); legacy viewer still refuses dual.
- Texture census on 01.33: 23,244 -> 23,247 of 23,247 hashes decode (all 23,477 GUIDs).
- Route census (material locations, 8,094): texture 6,953 -> 6,975, instance-only
  253 -> 267, unsupported 138 -> 102.
- Corpus mesh census (4,261 GUIDs; viewable 4,257 every mode): all-diffuse default
  3,669 -> 3,683, --animation-base 3,733 -> 3,747, --instance-rgba 3,902 -> 3,929,
  both 3,983 -> 4,010. Complete 0.
**+0 newly viewable / +0 newly Complete** (all-diffuse +14 default; textures +3).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests (lbp-preview 61 and 6 other crates), both viewer builds, python OK.
- diffuse_route_census -> b99-diffuse-route-shapes-0133.json; corpus_texture_census -> b99-texture-census-0133.json.
- diffuse_parity: d99 27/0/22 (default), 27/0/27 (--instance-white); d97 sets 0 mismatches (merged-a --instance-white 31/0/9); regressions d78, d75, d68, d86, d87, d89, d88, d92, d96, d64, d65, d67, d46 x2 unchanged; d83-alpha2 now 8/0/8 (was 2; D99 off-grid MIX, expected).
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b99-corpus-{default,anim,inst,both}.json; viewer --print-census reads them (+14 vs b96).

Failed: D99 merge conflicted with D97 in linked.rs (source label, grid check) and
diffuse_parity (report fields); kept both, diffuse_parity schema 22. First resolution
attempt cut the wrong boundaries; redone from restored markers before compiling.
Not tested: Bevy dual alpha-MIX shader visually; BLEND output pixels; volume layout on
real data; legacy viewer for D99.
Next: products under BLEND inputs (37); remaining 102 unsupported by reason in
docs/backlog.md; viewer QA toward Complete.

## Batch 2026-10-08c — local D100, D101, D102 merged (2026-10-08)

Local subagents on plan usage. Detail: [d100-grid-graph-bake.md](docs/d100-grid-graph-bake.md),
[d101-multiply-rest.md](docs/d101-multiply-rest.md), [d102-sampler-rest.md](docs/d102-sampler-rest.md).

- D100: general static graph evaluator (TEX, COLOR/CONST4, CONST within -16..16,
  MULTIPLY, ADD, MIX, BLEND) baked per texel on the integer Repeat grid; fallback after
  the fixed routes; ThingColor only under the instance color without other rewrites.
- D101: four texture leaves without shared pairs (fourth relative to primary/second;
  Bevy only), COLOR-only trees fold to solids, CONST broadcast / CONST4 fold.
- D102: subtype-1 matrix rows with any XYZ mask, and one animated texture x static
  COLOR, under the animation-base opt-in.
- Route census on 01.33 (material locations, 8,094): texture 6,975 -> 6,989, solid
  619 -> 620, animation-base-only 121 -> 126, instance-only 267 -> 296, unsupported
  102 -> 53.
- Corpus mesh census (4,261 GUIDs; viewable 4,257 every mode): all-diffuse default
  3,683 -> 3,695, --animation-base 3,747 -> 3,762, --instance-rgba 3,929 -> 3,962,
  both 4,010 -> 4,046. Complete 0.
**+0 newly viewable / +0 newly Complete** (all-diffuse +12 default).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests (lbp-preview 68 and 6 other crates), both viewer builds, python OK.
- diffuse_route_census -> b102-diffuse-route-shapes-0133.json: counts above.
- diffuse_parity: d100 36/0/9 (default), 36/0/36 (--instance-white); d101 8/0/5, 8/0/8 (--animation-base); d102 3/0/3, 1/0/1; regressions d99, d78, d75, d83, d68, d86, d87, d88, d92, d96, d64, d65, d67, d46 x2 unchanged; d89-0 now 5/0/4 and d97-a 31/0/17 (D100 projections of previously refused rows).
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b102-corpus-{default,anim,inst,both}.json.

Failed: D100 merge conflicted with D101/D102 (instance_textured.rs structure,
product_tree tests); kept both. D101's overbright-CONST (4.89) refusal is now taken by
the D100 evaluator (baked, clamped); its test updated. Integrator decision, noted here.
Not tested: viewer rendering of evaluated graphs and the Bevy fourth-sample path;
legacy viewer for D101; game pixels.
Next: 53 unsupported left (mostly off-grid graphs, UV channel raw 2, FRESNEL, five
leaves); viewer QA toward Complete.

## Batch 2026-10-08d — local D103, D104, D105 merged (2026-10-08)

Local subagents on plan usage. Detail: [d103-offgrid-bake.md](docs/d103-offgrid-bake.md),
[d104-uv-channel-2.md](docs/d104-uv-channel-2.md), [d105-instance-graphs.md](docs/d105-instance-graphs.md).

- D103: off-grid graph bakes. One UV channel: leaves resampled bilinearly in linear
  light onto a shared repeating tile (<= 4096/axis, 4 Mi px). UV0 x UV1 root products:
  per-channel bakes multiplied by the Bevy dual path. Resampling is preview policy,
  labelled "preview approximation (not game behaviour)" in labels, census and parity.
- D104: sampler UV channel word 2 samples UV0. Evidence CONFIRMED_FROM_LOCAL_DATA: the
  materials' compiled fragment programs (decoded with the public NV40 encoding) fetch
  TEXCOORD0.xy for every word-2 sampler (54 fetches); matches Toolkit's shader generator.
  Research output in ignored local-analysis/d104-uv-channel/.
- D105: under a supplied instance color, the graph evaluator runs on the original graph
  with only ThingColor substituted when the rewritten projection gives nothing (2).
- Route census on 01.33 (material locations, 8,094): texture 6,989 -> 7,003,
  instance-only 296 -> 301, unsupported 53 -> 34.
- Corpus mesh census (4,261 GUIDs; viewable 4,257 every mode): all-diffuse default
  3,695 -> 3,700, --animation-base 3,762 -> 3,767, --instance-rgba 3,962 -> 3,971,
  both 4,046 -> 4,055. Complete 0.
**+0 newly viewable / +0 newly Complete** (all-diffuse +5 default).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests (lbp-preview 76 and 6 other crates), both viewer builds, python OK.
- diffuse_route_census -> b105-diffuse-route-shapes-0133.json: counts above.
- diffuse_parity: d103 15/0/7, 15/0/9 (--instance-white); d104 3/0/3, 1/0/1; d105 2/0/0, 2/0/2 (--instance-white); regressions d100, d101, d99, d78, d75, d83, d86, d87, d88, d92, d96, d64, d65, d67, d46 x2 unchanged; d89-0 5/0/5 and d97-a 31/0/23 gain projections of previously refused rows.
- compatibility_census: d68-new 48/45/0, D61 48/43/0, D75 48/44/0 (unchanged).
- corpus_mesh_census -> b105-corpus-{default,anim,inst,both}.json.

Failed: D103 merge conflicted with D104/D105 (route-census scope line, add.rs test);
kept both, route census schema 5. D105's off-grid refusal test now expects D103 to
evaluate that graph (Integrator update).
Not tested: viewer rendering of resampled bakes, dual multiply, and the 4 UV-word-2
materials; game pixels.
Next: 34 unsupported (mostly BLEND/MIX mixing UV0 and UV1 outside a product, FRESNEL,
animated/wired ThingColor graphs); viewer QA toward Complete.

## Batch 2026-10-08e — first level batch D106-D108 + queued D109-D112 (2026-10-08)

Asset work paused at the owner's request; level work started from
[level-research.md](docs/level-research.md). Detail: [d106-level-oracle.md](docs/d106-level-oracle.md),
[d107-level-reader.md](docs/d107-level-reader.md), [d108-shape-geometry.md](docs/d108-shape-geometry.md),
[u1-translation.md](docs/u1-translation.md), [mesh-bones.md](docs/mesh-bones.md),
[runtime-research.md](docs/runtime-research.md), [u2-font-glyphs.md](docs/u2-font-glyphs.md).

- D106 (L0/L1): Toolkit LevelOracle. 01.33: 398 level resources, 385 parse to the last
  byte (13 failures: Vita .cpv, branch 0x4431); 680,391 Things, 40 part types. 76 LBP2
  story levels (th_* dirs, inferred): 29,068 RENDER_MESH vs 68,519 SHAPE; their 240
  mesh GUIDs all viewable. Recommended first level: lbp2/th_tutorials/levels/tutorial_23.bin
  (91 Things, 14 meshes, 36 shapes). Scene dumps byte-identical on repeat.
- D107 (L3/L4/L5/L6/Lw): new crate lbp-level (no UI deps): serializer core, Thing headers,
  POS/RENDER_MESH/SHAPE/GENERATED_MESH/GROUP/REF/BODY parsed, other parts read through.
  236 of 398 levels walk to the last byte; the other 162 are out of band (122 heads
  < 0x341, 20 LEERDAMMER, 20 DOUBLE11). Parity vs D106: 236/236 levels, 4,180/4,180
  scene Things bit-exact. Reader recurses natively (examples use a 512 MiB stack).
- D108: shape geometry research (doc only). Shapes store parameters only; first
  implementation = straight extrusion, no bevel, labelled approximation. Read LBP Asset
  Studio (unlicensed, ported from the PS3 binary) for behaviour hypotheses only, labelled.
- D109 (U1, PR #14): .trans reader. All 40 tables: 20 real (28,144-29,540 keys) match
  Toolkit on every key (0 differ, 0 tag hash differences); 20 empty 4-byte placeholders.
  U0 UI census (d110-ui-census-0133.json): .ff scripts 427 rows, 0 bytes present (likely
  in encrypted patch.edat); fonts 16 rows, 1 present; UI textures 1,935.
- D110 (R1, PR #15): mesh bones/mirror tables. Skeleton census: 4,243 of 4,245 mesh hashes
  decode (same 2 failures), 91,018 bones, 0 bad links/cycles, 60 name-hash mismatches,
  14 inverse outliers; Sackboy 43-bone skeleton shared by 2,047 meshes.
- D111 (PR #16): Sackboy/physics/logic research plan (docs only).
- D112 (U2, PR #17): FNTb glyph decode. icon_patch_font.fnt: 4/4 glyphs, 0 malformed;
  glyph sheet checked visually (clean icon shapes).
**+0 newly viewable / +0 newly Complete** (corpus 4,257 / 3,700 / 0 rechecked after D110).

Commands (after `. ./scripts/use-local-rust.ps1`; outputs in local-analysis/):
- fmt check, workspace Clippy -D warnings, tests for 12 crates, both viewer builds, python 36 OK.
- level_census -> b108-level-census-0133.json (byte-identical to d107); level_parity -> b108-level-parity-0133.json: 236/236, 4,180/4,180.
- run-translation-oracle.ps1 on all 40 extracted .trans (d109-trans/): 40/40 pass.
- corpus_skeleton_census -> d110-skeleton-census-0133.json; corpus_mesh_census -> b110-corpus-default.json (4,257/3,700/0).
- font_glyphs on icon_patch_font.fnt -> d112-icon-patch-font.*.

Failed: none. Integrator fixes: .gitignore conflict (LevelOracle + TranslationOracle
lines), lbp-viewer mesh.rs kept CRLF (D110 branch had converted it).
Not tested: level rendering (no viewer change yet), Vita/LEERDAMMER levels, plans in
lbp-level, 15 missing fonts, .ff scripts.
Next: L2/L5 level scene in Bevy for tutorial_23 (RENDER_MESH things at POS first), then
Lx straight shape extrusion.

## Batch 2026-10-08f — first level on screen: D113, D114 (2026-10-08)

Detail: [d113-level-scene.md](docs/d113-level-scene.md), [d114-shape-extrude.md](docs/d114-shape-extrude.md).

- D114 (Lx): `lbp_level::shape_mesh` straight extrusion (own hole-bridging triangulation,
  mirrored transforms, half-depth slabs, planar UV 1/500), labelled preview approximation.
  01.33 in-band levels: 121,167 of 121,286 drawable shapes extrude; 119 rejected (bad
  outlines); half-depth layer tiling 2,367 adjacent pairs vs 0 at full depth.
- D113 (L2/L5): Bevy level mode `--level-guid` / `--level-path` / `--level-report`; scene
  built in `lbp_preview::level_scene` (no Bevy). RENDER_MESH Things at POS (static bind
  pose), shapes batched per gfx material through the existing material route.
- Integrator: `--qa-hide-ui` capture flag (panels off) added to lbp-bevy-viewer.
- tutorial_23 (GUID 0001db4f): 91 Things; 14/14 RENDER_MESH drawn (8 meshes, 2 with neutral
  groups); 36 SHAPE, 9 extruded and drawn (27 have no GENERATED_MESH: collision only),
  1 flat fallback colour; 3 decorations not drawn. Rookie Test (headless): 333 of 455
  meshes drawn (122 need bone placement), 599 of 857 shapes.
- Visual check: native capture on non-primary monitor 165v0
  (local-analysis/d113-tutorial23-clean.png): recognisable, upright, sensibly placed level
  pieces; no backdrop, no bevels, unlit. Sent privately to the owner; not committed.
**+0 newly viewable / +0 newly Complete** (assets unchanged).

Commands (after `. ./scripts/use-local-rust.ps1`):
- fmt check, workspace Clippy -D warnings, tests (lbp-preview, -bevy-viewer, -mesh, -level, -viewer, -resource, -catalog), both viewer builds.
- lbp-bevy-viewer --snapshot ... --level-path lbp2/th_tutorials/levels/tutorial_23.bin [--qa-hide-ui] --qa-capture local-analysis/d113-tutorial23[-clean].png: exit 0.

Not tested: backdrop/REF plans, bevels, lighting, bone-attached props, other levels in
the window, legacy viewer level mode (none).
Next: queued PRs #18 (skin weights/morphs), #19 (animation keyframes), #20 (UI layer);
level: backdrop plan, bevels, bone-attached props.

## Batch 2026-10-08g — level v2 (D119-D122) + queued PRs D115-D118, D123 (2026-10-08)

Docs: d119-level-backdrop, d120-bone-props, d121-preview-lighting, d122-shape-bevels,
mesh-skin, animation-keys, u6-ui-layer, skinned-preview, u4-slot-pin-palette (docs/).

- D119: backdrop = PWorld backdrop Thing's PLevelSettings backdropMesh (STATIC_MESH) +
  backdrop plan; tutorial_23 static mesh 000190ca 409/409 primitives. Plan parity 42/42.
- D120: POS Things placed POS x inverse bind of bone 0 (Toolkit StaticMeshBuilder);
  joints placed from contact points (inferred). rookie_test 333 -> 455/455 meshes.
- D121: --preview-lighting from PLevelSettings (sun, ambient, fog), approximate.
  Integrator fix: light uses the display bounds (fog had covered the view).
- D122: RBevel reader (146/146 Toolkit parity) and --shape-bevels (inferred geometry);
  90,374 of 121,167 shapes bevelled in the census.
- D115 (PR #18): skin weights/morphs; 4,245/4,245 meshes, Sackboy 29-morph face set.
- D116 (PR #19): ANMb keys; 767/772 decode, 531 on the 43-bone set; fat byte mostly
  absent; locators are sound cues; trailing bytes up to 28 (16-bit-everywhere not shown).
- D117 (PR #20): lbp-gooey UI layer + --ui-demo (Integrator added --qa-capture for it);
  adds Bevy UI/text deps and Bevy's default font (OFL; needs a notice if distributed).
- D118 (PR #21): skinned preview; Sackboy 0000043f + walk_medium 0001a697: 43/43 bones,
  29/29 morphs, finite; GPU vs CPU frame 8 differ in 8 of 3,072,800 bytes.
- D123 (PR #22): slot/pin/palette/pack readers; 15/16 files complete, 1 slot list
  wrapper_error; Toolkit spot-check not done.
- Animation bank/set/map (ABK/AST/AMP): 0 on 01.33.
**+0 newly viewable / +0 newly Complete.** tutorial_23 now shows its workshop backdrop.

Commands: fmt, workspace Clippy -D warnings, tests for 10 crates, both viewer builds;
census/probe runs (d115/d116/d123 censuses, skinned_clip_probe); captures on non-primary
monitor 213v0: b122-tutorial23-{nolight,lit2}.png, d118-sackboy-walk-f8-{gpu,cpu}.png,
d117-ui-demo.png (shared privately, not committed).

Failed/notes: disk filled (agent build dirs, ~250 GB) and was cleared; many merge
conflicts in level_scene/viewer resolved (LevelSceneOptions now has shape_bevels and
placement; one LevelSettings). UI demo rebuilds every frame (perf, later).
Next: level visual QA vs game; joint skinning; bevel/UV calibration; more levels.

## D124 — U4/U6 follow-ups, PR #23 (2026-10-08)

- U4: gamedata/scratch/testrnp.slt (000056dc, not slot-list magic) is an expected
  `excluded_text_form` exclusion. ui_data_census -> d124-ui-data-census-0133.json:
  16 files, 15 complete (slot_list 3, palette 6, packs 5, pins 1), 1 excluded.
- U6: --ui-demo rebuilds only on change; capture d124-ui-demo.png on non-primary monitor
  201v0 looks the same as D117.
- Checks: fmt, tests and Clippy -D warnings for lbp-level, lbp-gooey, lbp-bevy-viewer.
**+0 newly viewable / +0 newly Complete.**

## D125, D126 — runtime decisions doc, RScript reader (2026-10-08)

- D125 (PR #24, docs only): AGENTS.md scope amendment (gameplay runtime crates, RScript
  interpreter, RPCS3 measurement approved by the owner 2026-10-08), runtime plan Rp-Rt,
  docs/rpcs3-measurement.md. RPCS3 setup waits for the owner's direct OK to download
  and run RPCS3 and the PS3 firmware.
- D126 (U10, PR #25): `lbp_level::script` RScript reader + disassembly listing; synthetic
  tests pass. script_census on 01.33 -> d126-script-census-0133.json: 0 scripts. No
  compiled script bytes are present in the selected dirs (U0: 427 .ff map rows, likely
  inside the encrypted patch.edat). Gate checks and the Toolkit spot-check could not run;
  G10 (native-call census) is blocked on script bytes.
- Checks: fmt, tests and Clippy -D warnings for lbp-level.
**+0 newly viewable / +0 newly Complete.**

## D127, D128 — item details (U5), script VM (Rq) (2026-10-08)

- D127 (U5, PR #26): `lbp_level::item_details` (InventoryItemDetails + RPlan details tail).
  plan_details_census -> d127-plan-details-census-0133.json (unit distinct content hash):
  12,245 plans: 11,936 complete, 225 stopped (heads 0x19f..0x1fe: older layouts), 84
  wrapper_error. Icons 10,497 (10,493 resolved in catalog); title keys 9,584; creators
  9,641. Types: readymade 2,936, costume 2,527, decoration 597, primitive_material 449,
  costume_material 327, music 132, ... D33 prefix cross-check and Toolkit spot-check not run.
- D128 (Rq, PR #27): new crate lbp-script-vm (register VM; synthetic bytecode only), added
  to CI. Real-script semantics check waits for script bytes (patch.edat).
- Checks: fmt, tests (lbp-level, lbp-script-vm), workspace Clippy -D warnings.
**+0 newly viewable / +0 newly Complete.**

## D127 verification (2026-10-08)

The two U5 checks skipped at merge, run afterwards (outputs local-analysis/d127v-*):
- D33 cross-check: 11,313 plans read by both readers, 9 prefix fields (date bits, slot
  type/number, highlight sound, colour, type bits, subType, title/description keys): 0
  mismatches. 932 not read by D33 (848 below its 893 floor, 84 DOUBLE11).
- Toolkit spot check (full RPlan load @ d6a2ef80): 4 plans (material, sticker, readymade,
  head 0x26e) equal on icon, title key, type, location, category, creator.
- 84 wrapper_error rows: all `Unsupported("DOUBLE11 branch payload")` (Vita; by design).
- Correction: the 225 stopped rows are 106 branch-0 plans at heads 0x19f..0x232 (early
  InventoryItemDetails layout) and 119 LEERDAMMER 0x4c44 plans at head 0x272 (refused by
  check_band). Toolkit reads both (InventoryItemDetails.serialize head<0x233 branch and
  LD_LAMS_KEYS middle layout), so both are reader gaps; added to the backlog.

## D129, D130, RPCS3 setup (2026-10-08)

- D129 (U5b, PR #29): early InventoryItemDetails layout (heads 0x197..0x232) and LEERDAMMER
  0x4c44 middle layout. plan_details_census -> d129-plan-details-census-0133.json (unit
  distinct content hash): 12,161/12,245 complete (was 11,936), 0 stopped (was 225), 84
  wrapper_error (DOUBLE11 Vita branch, by design).
- D130 (U7, PR #28): --slot-browser. --slot-browser-report on 01.33: slot lists 3/4 (testrnp
  text form), 164 slots, names 163/164, icons 164/164, root levels 149/149, 122 locked at
  start. Capture d130-slot-browser-0133.png (non-primary monitor) shows names/icons/locks.
- Checks: fmt; tests lbp-level, lbp-preview, lbp-bevy-viewer; workspace Clippy -D warnings;
  viewer build.
- RPCS3 v0.0.43-20247: firmware 4.93 installed (--installfw, headless). 32 update pkgs
  installed into RPCS3 dev_hdd0 (PARAM.SFO 01.33). `--decrypt` on a copy of patch.edat:
  "Invalid klicensee" (needs the game's key, loaded only while the game runs). Boot from
  game-data/a1-disc fails: "Failed to mount disc directory" (also with /dev_bdvd mapped
  in vfs.yml: "Failed to use custom BDVD directory"). Blocked; see Next.
**+0 newly viewable / +0 newly Complete.**

## D131 — U8 inventory browser, PR #30 (2026-10-08)

- `--popit` browser over disc plans. `--popit-report --popit-decode-icons` on 01.33 (unit
  distinct content hash): items 12,161/12,245 (84 DOUBLE11 not read), 27 type pages;
  titles translated 9,785 (path fallback 2,315); icons resolved 10,662/10,705; icons
  decoded 9,971/9,971 distinct; placement keys translated 8,884/11,557 (inferred).
- Capture d131-popit-browser-0133.png (non-primary monitor): type tabs, names and icons
  show. One glyph gap: "DC Comics" title shows a box for a missing character (font).
- Checks: fmt; tests lbp-level, lbp-preview, lbp-bevy-viewer; workspace Clippy -D
  warnings; viewer build.
**+0 newly viewable / +0 newly Complete.**

## D132 — U3 strings and fonts browser, PR #31 (2026-10-08)

- `--text-browser` plus OS font fallback for UI text (bevy `system_font_discovery`). New
  crates: objc2-core-text 0.3.2 (Zlib OR Apache-2.0 OR MIT), roxmltree 0.21.1 (MIT OR
  Apache-2.0), yeslogic-fontconfig-sys 6.0.1 (MIT); the objc2/fontconfig ones are
  non-Windows targets.
- `--text-browser-report` on 01.33: tables 21/21 parsed (unit distinct .trans content hash;
  1 empty: patch american.trans), 583,648 keys summed; fonts with a glyph sheet 1/1,
  glyphs 4/4 decoded (icon_patch_font.fnt).
- Captures (non-primary monitor): d132-text-browser-0133.png (tables, search, rows);
  d132-popit-recheck-0133.png: the "DC Comics™" title now renders ™ (D131 glyph gap fixed).
- Checks: fmt; full workspace build (windows-gnu, viewer included); tests lbp-preview,
  lbp-bevy-viewer; workspace Clippy -D warnings.
**+0 newly viewable / +0 newly Complete.**

## D133 — patch.edat decrypted; U10/G10/check() on real scripts (2026-10-08)

Detail: [docs/d133-patch-scripts.md](docs/d133-patch-scripts.md).
- RPCS3 boots the user's ISO copy (%USERPROFILE%\Emulators\lbp2-iso, read-only) with
  update 01.33; the extracted disc folder does not boot. RPCS3's Decrypt tool, run while
  the game holds the key, decrypted a copy of patch.edat: a FARC, 522 entries, 426 FSHb.
- U10 census d133 (unit distinct content hash, scope decrypted patch.edat): 426/426
  complete (was 0); 6,518 functions, 160,539 instructions, 0 unknown opcodes; gates clean;
  name_index_out_of_range 1,966, all type rows with 0xFFFFFFFF (an "unnamed" sentinel).
- Toolkit spot check (new scripts/run-script-oracle.ps1): 3/3 equal.
- G10 (d133-g10): 15,931 native calls of 23,350; 1,964 of 2,538 declared natives called.
- check() (new lbp-script-vm example script_check): 15,866 branches all in range from the
  current instruction (15,399 from next) -> VM default confirmed; 0 functions without a
  terminal.
- Checks: fmt; lbp-script-vm tests and Clippy -D warnings.
- Not tested: script-vm PC check 2 (argument layouts); running real scripts. X1/S9 need a
  human at the controls (RPCS3 is not a computer-use target).
**+0 newly viewable / +0 newly Complete.**

## D134 — U10 unnamed-type sentinel, PR #32 (2026-10-08)

- Type-row name index 0xFFFFFFFF = unnamed. script_census d134 (decrypted 01.33 scripts,
  unit distinct content hash): 426/426 complete; name_index_out_of_range 0 (was 1,966),
  unnamed_type_refs 1,966; other listing checks 0; heads/layouts unchanged.
- Checks: fmt; tests lbp-level, lbp-script-vm; workspace Clippy -D warnings.
**+0 newly viewable / +0 newly Complete.**

## D135 — Rs first native bindings, PR #33 (2026-10-08)

- 40 native bindings, System.Yield coroutines, example native_coverage. On the 426
  decrypted 01.33 scripts (d135-native-coverage-0133.json; unit call instruction): native
  calls 17,704, bound 2,347 (34 distinct); gameplay scripts 255: native calls 2,472,
  bound 690. Unparsed names 0. Unknown signature codes: `j` 32 (e.g. CheckEqual__jj...,
  ShiftL__ji), `[` 2 (array params, e.g. GatherFriendData__[Q13PodFriendData).
- Gap: System.Yield__ is handled inside the VM but native_coverage lists it as unbound
  (66 gameplay calls). The PR's "953 of 1,356 G10 top-100 calls" uses a different
  gameplay set; not reproduced here.
- Checks: fmt; lbp-script-vm tests; workspace Clippy -D warnings.
- PR #34 (U11/U12 Gooey trace) not merged: PodStateMainMenu UpdateUI stops after 48 of
  1,201 instructions, translated 0/0, outline 2 lines (no English labels); 7/15 functions
  fault (Type on member ops). Gooey 9/45 faulted, PodStateStoriesMenu 5/13.
**+0 newly viewable / +0 newly Complete.**

## D137 — Rs part 2, PR #35; PR #34 still held (2026-10-08)

- native_coverage d137 (426 decrypted 01.33 scripts; unit call instruction): native calls
  17,704, bound 2,970 (was 2,347); gameplay 254 scripts: 1,935 calls, 756 bound; test
  script 1: 537 calls, 524 bound; unknown signature codes none (j = s64, [ = array).
- script_tests d137 (the game's own test script): 47 functions; outcomes done 24, Type 15,
  Native 2, Register 2, Index 1, unbound_native 3 (Test.Succeed still unbound); checks
  passed 532, failed 3 (NeverEvaluated test.cod:438; TestS64 test.cod:1551, 1562).
- Checks: fmt; lbp-script-vm tests; workspace Clippy -D warnings.
- PR #34 at 86684dd (D136 reserved): faults 7 -> 3 of 15 PodStateMainMenu functions, but
  UpdateUI still returns after 48 instructions with translated 0/0 and a 2-line outline;
  not merged (its step 3 needs English labels).
- RPCS3 movement run: PINE IPC on 127.0.0.1:28012 reads guest memory (title, 01.33);
  game at the first-run TV-size screen. Sending keystrokes was denied by the safety
  check (approval came through the coordinator); needs the user's direct OK.
**+0 newly viewable / +0 newly Complete.**

## D138 — Rs part 3, PR #36; PR #34 still held (2026-10-08)

- script_tests d138 (the game's test script): 47 functions, done 37 (was 24); stops: Native
  5, Register 2, Type 2, Index 1; checks passed 589 (was 532), failed 11 (was 3, more test
  code now runs): NeverEvaluated 1, TestMemberInitialisation 4, TestProperties 2,
  TestFunctionWithSideEffect 2, TestS64 2. Failed S64 checks follow ShiftL__ji and
  BIT_ANDs64; NeverEvaluated suggests a short-circuit evaluation rule.
- Checks: fmt; lbp-script-vm tests; workspace Clippy -D warnings.
- PR #34 at 83652c6: 426 GUIDs, superclass not loaded 0; PodStateMainMenu faults 3 of 15
  (OnActivateImpl Register 65535 at pc 395, PerFrameUpdate Register 65535 at pc 204, Draw
  NullObject at pc 153; these pcs are outside the faulting functions' own code ranges, so
  inside callees). UpdateUI still 48 instructions, translated 0/0; not merged.
**+0 newly viewable / +0 newly Complete.**

## D139 — VM no-register operand, default locals, fault site (PR #37) (2026-10-08)

- script_tests d139 (the game's test script): 47 functions, 44 done (was 37), 3 Index stops
  (TestArray +14 element, reached also via main__; TestConstructorChaining +4 object
  field). Checks passed 826 (was 589), failed 27 (16 distinct lines; main__ repeats 11).
- Evidence (masked listings, sent to the Sackboy thread): failing member/property checks
  read fields that only the class's .init__ sets (fresh test instances skip .init__);
  ShiftL__ji is script code (s64 add loop), not a native; NeverEvaluated is reached from
  TestBool's || / && short-circuit branches; the D138 MULm44 and INCi stops are gone.
- Checks: fmt; lbp-script-vm tests; workspace Clippy -D warnings.
**+0 newly viewable / +0 newly Complete.**

## D136 — U11/U12 headless Gooey trace, PR #34 (2026-10-08)

- New crate lbp-ui-script (headless Gooey host) and example ui_trace (catalog input:
  d133 snapshot + roots incl. the decrypted patch.edat overlay; English tables d109).
- PodStateMainMenu (d136d): 426 GUIDs, superclass not loaded 0; 15 functions, 3 faulted
  (NullObject in callees: PartyManagement.GatherFriendData+2, PodComputerState.
  SetupFrameMatrix+3, UpdateUI+888). UpdateUI runs 449 instructions, 68 native calls,
  translated 5/5; its outline draws the main menu: Story, Community, Recent Activity,
  Store, Me buttons and nested frames. Step 3 of docs/u12-gooey-trace.md passes.
- Checks: fmt; tests lbp-ui-script, lbp-script-vm; workspace Clippy -D warnings; workspace
  build. No new third-party crates.
**+0 newly viewable / +0 newly Complete.**

## D140 — U12b script menu in the viewer, PR #39 (2026-10-08)

- `--ui-script CLASS` draws a UI script's frame tree; `--ui-script-report` prints it. HUD
  screen queries answered (XRes 1280, YRes 720, edge crop 0, aspect 16:9).
- PodStateMainMenu: 449 instructions (NullObject at UpdateUI+888 after layout), 68 native
  calls, translated 5/5; outer Sizing 2400 x 1250 (was NaN). Capture
  d141b-u12b-mainmenu.png (non-primary monitor): Story, Community, Recent Activity, Store,
  Me as buttons in a narrow placeholder column (frame property meanings unknown).
- PodStateStoriesMenu: 208 instructions, returned; translated 1/1; Sizing 2400 x 1250.
- Checks: fmt; tests lbp-ui-script, lbp-bevy-viewer; workspace Clippy -D warnings; viewer
  builds. PR #38 (VM) held: 804 passed / 53 failed vs D139 826 / 27 (LCb rule change).
**+0 newly viewable / +0 newly Complete.**

## D141 — VM: .init__ on NEW_OBJECT, superclass by GUID, TestBase counters (PR #38) (2026-10-08)

- script_tests now counts the game's TestBase pass/fail counters (Check* are script methods
  of TestBase, not natives). d142, LCb rule bit 63 (default): 47 functions, 45 done, 2
  Index stops (TestArray +17, also via main__); checks passed 850 (D139 826), failed 7, no
  new failing lines. LCb "any bit" 804/53; "bit 32" 746/113 -> bit 63 confirmed.
- Remaining failures: TestLocals 417, NeverEvaluated 438 (the runner calls it directly),
  TestMatrix44 662, TestS64 1551 and 1562 (s64 shift loop, s64 AND).
- Checks: fmt; lbp-script-vm, lbp-ui-script tests; workspace Clippy -D warnings.
**+0 newly viewable / +0 newly Complete.**

## D142 — script_tests counts Test* functions only (PR #40) (2026-10-08)

- d143: Test* totals 476 passed, 4 failed (TestLocals 417, TestMatrix44 662, TestS64 1551,
  1562); main__ repeats and NeverEvaluated no longer counted. Checks: fmt; lbp-script-vm
  tests and Clippy.
- Finding (CONFIRMED_FROM_LOCAL_DATA, consistent on all 13 entries): the RScript s64
  constant table is zigzag-encoded. Decoding ((n>>1) ^ -(n&1)) turns the TestS64 failures
  into passes by hand: 1551 expects i64::MIN = ShiftL(1, 63); 1562 expects
  0x1111..11 & 0xff00ff00ff00ff00 = 0x1100110011001100. Reader fix pending (U10).
- AGENTS.md: dated standing permission to drive RPCS3 for measurements.
**+0 newly viewable / +0 newly Complete.**

## D143, D144 — X1 doc (PR #43); zigzag s64 + m44 zero defaults (PR #42) (2026-10-08)

- X1 (RPCS3, game 01.33, offline, full speed): 30 game ticks per second, one tick per 2
  vblanks (599/598/599 vs 1,197 over 20 s; x1-tick-counters-0133-run1.csv).
  CONFIRMED_FROM_LOCAL_DATA (emulated). D143 records it in docs/rpcs3-measurement.md.
- D144: script_tests d145 Test* totals 479 passed, 1 failed (TestLocals 417); TestS64 and
  TestMatrix44 pass; TestArray stops at +17 (Index). Evidence: TestArray lines 873-875 fit
  "insert index clamped to [0, length]" (-1 -> front, 100 -> end). TestLocals 417 reads a
  block local (`int a;` emits no code) after the previous iteration set it to 1.
- S9 blocked: the new profile must finish the Introduction level; its last room (desk/TV)
  did not end under keyboard play. Asked the user.
- Checks: fmt; tests lbp-level, lbp-script-vm, lbp-ui-script; workspace Clippy.
**+0 newly viewable / +0 newly Complete.**

## D145 — VM array insert clamp and typed defaults (PR #44); S9 status (2026-10-08)

- script_tests d147: all 47 test functions finish; Test* checks 512 passed, 1 failed
  (TestLocals 417, a block local with no initialising instruction).
- S9: Sackboy position found at guest 0xe2b83c (x) / 0xe2b840 (y) (RPCS3 guest base
  0x300000000; read-only ReadProcessMemory). One provisional story-level walk sample
  (s9-walk-right-rookie-0133-run1.csv, Rookie Test start) recorded but not fitted.
  My Moon is locked for the new profile (Pod menu stays on Story); waiting for the user.
- Checks: fmt; lbp-script-vm, lbp-ui-script tests; workspace Clippy -D warnings.
**+0 newly viewable / +0 newly Complete.**

## D146 — UI objects constructed (PR #41) (2026-10-08)

- --ui-script PodStateMainMenu: UpdateUI returns after 581 instructions (no fault; was a
  NullObject at +888), 78 native calls, translated 5/5, frames balanced; supplied objects 5
  (podDemoThread, ClapperBoardIcon, _StartedVO, Puter, TweakItemSelectTexture).
  PodStateStoriesMenu: 303 instructions, translated 1/1, supplied 4. Capture
  d146-u12c-mainmenu.png (non-primary monitor).
- Checks: fmt; tests lbp-ui-script, lbp-script-vm, lbp-bevy-viewer; workspace Clippy; builds.
**+0 newly viewable / +0 newly Complete.**

## D150 — Rt part 1: lbp-runtime LevelWorld and Runner (PR #45) (2026-10-08)

- level_scripts --all (d150): 426 scripts; 378 levels (20 not read); 13,547 scripted
  things, all resolved (49 classes); fields matched 153,972, missing in class 312;
  unconverted values 15,879. Top classes: PostItNote 3,079, Launcher 2,875,
  Trigger_Speech_Bubble 1,469, TriggerGlobalSettings 1,180, ScoreTweak 1,106.
- Rookie Test, --entry=OnCreate__ --entry=Update__ --ticks=30: OnCreate done 170, fault 42,
  no function 9; Update fault 22, no function 199. First faults: players not provided,
  Thing.SpriteLight member, unbound Audio.GetDialogueList__, Score_Board.
  GetShapeEditorColour__, Game.GetPlayOrEditMode__, Trigger_Speech_Bubble.IsTweaking missing.
- Level census d150 byte-identical to b108. Integrator fix: clippy as_chunks in serial.rs.
- Checks: fmt; tests lbp-level, lbp-runtime, lbp-script-vm; workspace Clippy -D warnings.
**+0 newly viewable / +0 newly Complete.**

## D147-D149 — level polish batch (local subagents) (2026-10-08)

- D147 (docs/d147-joint-skinning.md): Rookie Test span joints (piston 33, string 25,
  chain 4) skinned with rigid end groups and a stretched bridge (INFERRED from mesh
  weights; no oracle): 62/62, 0 kept stretched; pivots 60 unchanged; 455/455 placed.
  New --level-focus-thing N. Capture d147-rookie-joints.png.
- D148 (docs/d148-camera-sackboy.md): game meshes are Z-up, front -Y; single meshes now
  shown upright (--model-up z default, y = raw); eye-level default camera. Sackboy
  0000043f + walk 0001a697 f8 faces the camera; GPU/CPU skinning match. Captures
  d148-sackboy-front(.png, -cpu.png), d148-tutorial23-camera.png.
- D149 (docs/d149-davinci-levels.md): --slot-browser-slots lists slots. Da Vinci's Hideout
  (group 22) levels: Rookie Test 0001e786 3,247 Things, 455/455 meshes, 599/857 shapes;
  Grab and Swing 0001e793 2,163, 439/440, 589/1,061; Hedge Hopping 0001e7f4 875, 163/163,
  171/318. Capture d149-davinci-rookie-test-unlit.png (lit view fogged out: stored fog
  range vs overview camera).
- Checks: fmt; tests lbp-mesh, lbp-preview, lbp-bevy-viewer (26); workspace Clippy; builds.
**+0 newly viewable / +0 newly Complete.**

## D151-D159 — menus, runtime, level polish, Pod (2026-10-08)

- D151 (PR #46): script frame sizes as minimums; --ui-script menu fills the window.
- D152 (Rt part 2): class field defaults, part handles, inherited natives; Rookie Test
  OnCreate 189 done / 23 fault (D150 170/42), Update 21/1 (0/22); 57 natives bound.
- D153: undrawn level shapes classified, all hidden by data (hulls, tree colliders,
  invisible colliders, edit-only); report split, counts unchanged.
- D154-D157 (PRs #47, #48, #50): --puter-screen debug view; --pod-menu globe menu with
  pills from the RPCS3 reference; preview budgets 500k tris / 1M verts (globe draws);
  --pod-camera solved from the globe. Pod levels: 000027b9 pod.bin (shell, controller,
  spawn), 00018fdb pod_planets.bin (globe, moon, room plan 0001d054). Capture
  d163-pod-camera-menu.png: globe 0.32w/0.39h r 0.21h vs RPCS3 0.35/0.40/0.22.
  Notes local-analysis/d155-pod-assets.md; RPCS3 references d156-rpcs3-pod-*.png (local).
- D158: level decorations drawn: Rookie 51/51, Grab and Swing 84/84, Hedge Hopping 15/15
  (inferred placement); sticker decals 910/444/118 counted, not drawn.
- D159 (PR #49): per-tick LevelScripts; Rookie --play=300: create 189/23, no tick faults.
- Checks per merge: fmt, tests, workspace Clippy -D warnings, both viewer builds.
**+150 newly viewable (decorations drawn, unit DECORATIONS entries, scope three Da Vinci
levels, baseline D149 0) / +0 newly Complete.**

## D160-D175 — Pod vs RPCS3, script threads, physics merged (2026-10-08/09)

- D160-D168 (PRs #51-#57, #60): runtime textures, globe sea, sticker decals (1,003 drawn), Pod
  sampler UV and tent; RPCS3 comparison in docs/pod-rpcs3-comparison.md.
- D169 (PR #61): pod-in-use camera. level_scripts pod_planets --play=300 --flags=2
  --puter-in-use=150: before (0,-250,1130)->(0,100,-1500), in use (0,-300,900)->(0,500,-1500);
  both presets found in RPCS3 memory (table at 0x344cb780), live Story view direction matches.
- D170 (PR #62): closed window 0.34-0.65 w x 0.28-0.67 h (RPCS3 0.35-0.67 x 0.28-0.69; D168
  0.33-0.67 x 0.21-0.64); --pod-t mid-dolly vs RPCS3 opening frame (d177-*).
- D171 (PR #59): script threads; pod_planets --play=300 no flags: threads_started 1, intro thread
  stops at unbound Video.StartVideo__Q5Thinggb; 0 tick faults.
- D172 (PR #56): viewer --level-scripts: Pod planets flags 2 649 ticks 0 faults; Rookie create
  189/23/9 (d185-*).
- D173 (PR #63): Pod side panels default to room wall art 0001f3b9 (inferred; window_glassy slot
  0 is a bump map; --glass experiment had no visible effect and was reverted). d184/d187.
- D174 (PR #64): MULm44 is column-major A*B; World matrix zero now at [1][0] like RPCS3;
  script_tests 512/1 unchanged.
- D175 (PRs #54+#58): lbp-sim avian2d physics, joints, frozen bodies asleep, pick-ups and
  infinite-mass fixed, hologram not a collider. Rookie --ticks=90: moved >1 0 (d189). --all on
  7bc777f: 378 levels, 0 panicked, moved >100 12,462 of 72,709 bodies (d183).
- RPCS3 findings: FOV hfov 88.3 deg (1/tan 1.0296, aspect 16:9, near 10, far 100000) in Pod main
  menu; real menu World matrix is PlanetsView view 1 (pitch 0.35, yaw -0.07, scale 2); layer
  switch W = -z, lanes 200 apart, linear 35 u/tick (Rookie x -13036, lanes 0/-200).
- Failed: --glass translucency over nothing (D181); first --all stack overflow (fixed by 512 MB
  thread); live camera address not stable in memory.
- Checks per merge: fmt, workspace Clippy -D warnings, crate tests, viewer builds.
- Not tested: three-lane switch, airborne switch, Pod closed-view FOV, main-menu view in our VM.
- Next: drive PodStateMainMenu planets view (view 1) in the VM; Earth/Moon bounds; physics
  movers (frozen=1 fallers); Sackboy controller measurements.
**+0 newly viewable / +0 newly Complete (Pod/runtime fidelity; sticker decals counted in D167).**

## D188-D190 — preview defaults, mixed-UV graphs, small material fixes (PRs #89-#91) (2026-10-09)
- D188 (PR #91): animation base values and a white instance colour (when none is supplied) are
  the single-mesh preview defaults; --no-animation-base / --no-instance-default restore the old
  behaviour. Material-less groups are excluded from the all-diffuse test (counted separately).
- D189 (PR #90): mixed UV0/UV1 BLEND/MIX graphs evaluated (indianrubble, summergames costumes,
  knightsofold2 dragon; docs/d189-mixed-uv-graphs.md).
- D190 (PR #89): small material fixes.
- PC census on combined head a47631d (= 983e05e content), d68-inventory-0133-verified, a1-disc +
  a1-update-0133: default --blockers 4,261 GUIDs, 4,259 viewable, 4,239 all-diffuse, 59
  material-less groups excluded, 0 failed, 2 no-geometry (b177-default.json). Legacy flags
  (--no-instance-default --no-animation-base --materialless-blocking): 3,814 (b177-legacy.json).
  Versus b176-blockers-default (3,798): 0 GUIDs lost; default +441, legacy +16.
- Remaining 22 not all-diffuse: 24 blocking material keys; largest causes Unresolved GUID (5
  sole-blocked), BORDER alpha MIX (2), animated UV samplers (2), nonfinite mesh UV (2).
- Level scenes do not apply the new preview defaults yet (Bravery Test: 195 Things neutral).
- Captures: local-analysis/b177-{cosmos-room,vin-crane,gymnast-hr,tennisgirl-hd}[-crop].png,
  b177-davinci-level.png; screenshot page v21 (Assets section).
- Verification comment: https://github.com/chilladept/lbp2-asset-explorer/pull/91#issuecomment-6088230961
**+0 newly viewable / +0 newly Complete; all-diffuse 3,798 -> 4,239 by default.**

## D191 — level scenes use the asset defaults (PR #93) (2026-10-09)
- Level scenes now apply the D188 preview defaults (animation base values, white instance colour)
  and tint each Thing by its editorColor as THING_COLOR (mapping inferred).
- PC check at 527f6a8 (rustc 1.99): fmt, lbp-preview tests (114 + 6), Clippy -D warnings clean.
- --level-report, new vs old (--no-instance-default --no-animation-base): Bravery Test neutral
  Things 195 -> 35, 9 editorColor Things (4 mesh+colour variants), meshes 61 -> 65 loaded, 0
  failed, backdrop flat-fallback primitives 4 -> 0; tutorial_23 (0001db4f) neutral 2 -> 1.
  No error, memory or budget lines.
- Captures local-analysis/b178-bravery-{before,after}[-detail].png: overview differs in 615
  pixels, mean luma unchanged, no dark or black tints seen. editorColor tint values are not
  visually proven (most changed Things are off-frame or small at the default camera).
- Verification: https://github.com/chilladept/lbp2-asset-explorer/pull/93#issuecomment-6088926155
**+0 newly viewable / +0 newly Complete (level fidelity: 160 fewer neutral Things in Bravery Test).**

## D192 — material-less primitives hidden (PR #95) (2026-10-09)
- PC Ghidra (01.33 EBOOT): the per-primitive draw loop skips a null-material primitive (no
  bind, no draw; 0x33c814, test 0x33ca88), CONFIRMED. A whole-mesh depth/shadow use is
  INFERRED. docs/d192-materialless-hidden.md.
- Viewer no longer draws material-less groups (single mesh and levels); level neutral count
  ignores them. --level-report lists the binding error per neutral group.
- PC report at ea55c1e (before): Bravery Test 35 neutral Things, every one with a
  material-less group; 00019e98 (22) also has a folded-sampler nested-tint group.
- Folds in #94's D191 STATUS/MODLOG entries.
- Cloud: lbp-preview lib tests 116 pass, lbp-catalog tests pass, fmt clean. No game data.
- Not tested here: level report after the change, close-camera capture (PC).
**+0 newly viewable / +0 newly Complete.**

## B179 — PC verification of D192 (PR #95) (2026-10-09)
- Head c1e201e vs main 9c69018, rustc 1.99.0: fmt clean; CI-crate tests 430 pass; Clippy
  -D warnings pass (CI crates plus both viewers).
- Bravery Test (main_04_bravery_test.bin; GUID 0001e795): neutral Things 35 -> 22, all
  00019e98 (material-less + folded-sampler nested-tint group); draw entities 2112 -> 2093;
  meshes 65/0 failed, placements unchanged. tutorial_23 (0001db4f): 1 neutral, unchanged.
- Corpus mesh census (default): 4,261 / 4,259 viewable / 4,239 all-diffuse, identical to
  b177 per GUID (0 lost). local-analysis/pr95/.
- Gameplay-distance captures (eye target + (296,168,1218), vfov 60; local env-var camera
  patch, not in git): crane 000150dc slabs and 00018d2b frames gone, no holes. A grey square
  on the central beam still draws; likely shape batches gfx 00012ba7/00018401 (INFERRED).
- Not done: Pod planets 0001d050 vs RPCS3 (no frame on hand).
**+0 newly viewable / +0 newly Complete.**

## D193 — sampler port folds under the nested instance tint (2026-10-09)
- `select_instance_diffuse` drops D96 rule 5: port folds are baked into the derived
  sampler and the nested-tint rewrite copies that box, so the two-sample product is exact
  (INFERRED composition of two oracle-checked rewrites). docs/d193-folded-ports-nested-tint.md.
- Target: Bravery Test (0001e795) 22 neutral Things, all 00019e98 with that reason.
- Cloud (rustc 1.97): lbp-preview tests 117 pass (1 new), Clippy -D warnings pass, fmt clean.
- Not tested here: level report, corpus census, gameplay-distance capture (PC).
**+0 newly viewable / +0 newly Complete (expected; PC census decides).**

## D194 — level shapes take their own PShape.color (2026-10-09)
- PC report at b8331fd (reason line only): Bravery Test (0001e795) mesh Things neutral 0;
  shape batches gfx 00012ba7 (9 Things) and 00018401 (59) neutral with "diffuse multiply
  requires two direct textures or one texture and static COLOR"; 68 shapes flat fallback.
- Shape batches key on (gfx material, PShape.color); under the level defaults each loads
  with its PShape.color as the instance colour (link INFERRED). docs/d194-shape-color.md.
- --level-report: neutral reason for shape/backdrop batches, shape Things per batch.
- PC at 6111561: gfx 00012ba7 fixed (68 -> 59 flat fallback); gfx 00018401 (59 shapes,
  beam square Thing 3316) refused "several animated samplers under a folded or
  alpha-subtracted instance input". Graph dump: (s0 x THING) x (s1 x s2), 3 animated
  samplers. Fix: nested tint over 2-3 animated samplers selects and loads (alpha
  subtraction and a fourth sample stay refused); material_graph_dump example.
- Cloud (rustc 1.99): lbp-preview lib tests 120 pass (4 new), workspace Clippy -D warnings
  pass, fmt clean. No game data.
- Not tested here: level report, corpus census, gameplay-distance capture (PC).
**+0 newly viewable / +0 newly Complete (expected; PC census decides).**

## P3m — level lighting from presets blended by PWorld lighting (2026-10-10)
- Live reads (tutorial_23, Bravery Test, Moon) fit: presets spread over PWorld lighting (position
  lighting x (n-1)), blended; darkness scales exposure and fog colour. Replaces preset 0 and the main block;
  fixes the all-blue levels under --preview-lighting (main block fog 1000..4000). docs/p3m-level-lighting.md.
- Tests: lbp-level thing test level_lighting_blends_presets_by_world_lighting_and_darkness.
- Not settled: 4-5 preset spacing, darkness split, Moon/Pod fog colour (script driven).
**+0 newly viewable / +0 newly Complete.**

## D231 - Sackboy plays his own clips in play mode (2026-10-11)
- Animation plan slices 1+2 (local-analysis/results/2026-10-11-sackboy-animation-plan.md).
  --play-sackboy draws mesh 0000043f by default (no --select-guid) and picks stand, walk
  slow/medium/fast, walk_z fwd/back, jump_stand/jump_walk, stretch_crouch per tick from the
  controller (lbp-sim sackboy_anim::pick_clip, Bevy-free). Clip bank: mesh read once, all 9
  clips bound to one rig (lbp-preview load_clip_bank, Rig::rebind, SkinnedClip::swap);
  hard cuts. Replaces the Sackbot walk 0001a697. Every state->clip rule INFERRED.
  docs/d231-sackboy-animations.md.
- cargo test -q -p lbp-sim -p lbp-bevy-viewer -p lbp-preview: pass (9 new tests).
- cargo clippy -q (same crates) --all-targets -D warnings: pass; fmt clean.
- Rookie Test play run (d68 0133 snapshot): clip bank 9/9 bound; scripted log shows
  stand/walk bands/jump_walk/squash/ledge fall/lane/jump_stand in order. QA capture
  local-analysis/d231-sackboy-anim-rookie.png (gameplay distance, stand; not committed).
- Not tested: RPCS3 comparison of thresholds, pacing, loop/hold, fall entry frame.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: slice 3 (Ghidra: readers of the anim fields r30+0x290..0x6c0, BlendFrames).
## D232 - Sackboy animation from the EBOOT rules (2026-10-11)
- Plan slices 3-6 from the Ghidra rules note (local-analysis/results/2026-10-11-ghidra-sackboy-anim-rules.md).
  lbp-sim SackboyAnim::tick replaces pick_clip: weighted layers (walk slow/medium/fast over
  stand, one 13-tick phase), facing eased 0.4/tick (0 / +-90 / +-72 lane), vy-driven jump
  frame, landing = jump tail with fade, stop/turn 5-frame and lane 3-frame ramps, idle LCG
  chains, additive blink. lbp-preview BlendPose (per-bone lerp/nlerp, additive, sub-frame).
  Viewer blends the 21-clip bank per tick; facing interpolated, no snap. docs/d232-sackboy-anim-rules.md.
- OPEN: stretch_crouch spring input (TODO), mirror flag not applied, LCG multiplier 250 vs range 999.
- cargo test -q -p lbp-sim -p lbp-preview -p lbp-bevy-viewer: pass (13 picker + 4 blend tests).
- cargo clippy -q (same crates) --all-targets -D warnings: pass; fmt clean; python -m unittest: 36 OK (1 skipped).
- Rookie Test scripted play run (hidden): clip bank 21/21 bound; log shows walk mix, yaw
  24->90 deg, jump_walk ramp-in, apex frame 8, landing tail fade, stop, lane 72 deg, jump_stand.
- Not tested: RPCS3 comparison of poses/timing; QA capture.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stretch_crouch spring input (0x6c56c) and bone mirror map; then emotes/head look (plan 7-8).

## 2026-10-11 — D232 QA fix pass (PR 127)

- Facing: clip root rotation held at bind (BlendPose::hold_root_rotation); mirror flag
  applied (Rig::mirrored_trs, skeleton mirror table); stop/turn hold +-72 deg (0x2002ac).
- Landing tail starts at 13-14 (frame no longer drops to the apex on the contact tick).
- Blank level: spawn lifted out of the floor slab; back lane works (z -200 -> -400).
- --qa-record cherry-picked (QA frame sequences).
- fmt clean; tests lbp-sim/lbp-preview/lbp-bevy-viewer pass; clippy -D warnings clean;
  python unittest OK.
- Captures: local-analysis/sackboy-clips-2026-10-11b/ (5 clips, hidden, no focus change).
- OPEN: running-landing fade exponent, camera lead when walking left, mirror flip types.
- Details: docs/d232-sackboy-anim-rules.md (QA fix pass).
## 2026-10-11 - D232 open items, Ghidra pass 2

- Applied the FOUND results of local-analysis/results/2026-10-11-ghidra-sackboy-anim-open-items.md
  in lbp-sim sackboy_anim: idle roll 0..249 (mulli 0xfa, was 0..999), layer order walk ->
  stop/turn -> jump/landing -> lane (lane was before stop), idle allowed after > 300 ticks
  with no input (0x1fb4a8/0x75c; sit stand-up resets it), seed 42, first blink at tick 0,
  landing k = 1 and walk_z dz > 0 = forwards (comments only).
- Traced, not implemented: crouch spring input (overlap depth / 350, ground only), emotes,
  mood stands, head look. Still OPEN: mirror maths (SPU job), camera lead, GroundTurnSpeed /
  FootAnimationSpeed readers, arms, face morphs. docs/d232-sackboy-anim-rules.md.
- cargo test -q -p lbp-sim -p lbp-preview -p lbp-bevy-viewer: pass (2 new picker tests).
- cargo clippy -q (same crates) --all-targets -D warnings: pass; fmt clean; python -m unittest: 36 OK (1 skipped).
- Not tested: play run or RPCS3 comparison of idle timing.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: camera lead via a live memory diff; emotes/head look (plan 7-8).
## 2026-10-11 - D233 Sackboy moods, emotes and head look (NOT compiled yet)

- Rules from Ghidra pass 3 (local-analysis/results/2026-10-11-ghidra-sackboy-moods-headlook.md)
  and pass 2 section 4. lbp-sim: Mood + Input::mood (d-pad direction held); SackboyAnim takes
  the press edge: same mood cycles 1, 2, 3, off, the opposite (i ^ 1) cancels through
  to_neutral, another replaces at level 1, no timeout; anticipation -> emote (last frame
  holds, sad 2/3 loop) -> to_neutral; weights ease 0.1 per tick; stand_<mood> lerped on by
  weight once the emote holds; idle chain blocked. Three additive look layers close the
  stack (neutral: right stick, c20 target and tilt term are 0 here). 33 new clips (54).
- Viewer: I/K/L/J = mood up/down/right/left; --play-sackboy-input letters U D R L; log line
  shows `mood <name> <level> <state>`. Details: docs/d233-sackboy-moods-headlook.md.
- BLOCKER: the session's permission gate refused every cargo, python and git add command,
  so nothing was compiled or committed (uncommitted in the lbp2-wt-moods worktree). cargo fmt --all --check: not run. cargo test -q -p lbp-sim
  -p lbp-preview -p lbp-bevy-viewer: not run (10 new picker tests written). cargo clippy
  (same crates) -D warnings: not run. Play run: not run.
- Not tested: everything above; RPCS3 comparison of the d-pad mapping and emote timing.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: run fmt / clippy / tests on this branch and fix what they report, then the scripted
play run in the D233 doc; after that camera lead (live memory diff) and the look inputs.
## 2026-10-12 - D247 landing timing and dip; stop pose measured (no change)

- Stop pose: MEASURED tick-aligned against the D234 walk capture (`local-analysis/d247-stop-turn/`,
  `d248-lean-stretch/cmp-stop.txt`): fit mean 1.56 against 1.51 for the steady walk around it, which is the
  floor of the method. It matches the game; no change. The old "fit 3-5" was the fit tool not converging at
  72 deg yaw. The turn has no game capture yet (needs one with a direction reversal).
- Landing timing (cause): `SackboyBody::grounded` was the floor probe taken before the tick's move, so the
  tick a fall ended on the floor still read as airborne and the landing tail began one tick late. It is now
  the contact after the move (`lbp-sim/src/character.rs`); the controller step still gets the probe, so
  jumps, lane changes, the dip counter and ledge walk-offs are timed as before.
- Landing dip: `LANDING_DIP` = -25.9, -15.9, 0 (was -21.2, -23.5, -10.7, 0), re-derived after the timing fix
  as the game's lowest point minus our pose's. D241's first value came from a half-written buffer read.
- MEASURED, d234 `jump.bin` (standing 6-tick jump) against a release viewer run, tick-aligned, k = ticks
  from touchdown (height = lowest to highest vertex; lowest point relative to rest):
  k -2..+3 height: game 199.7, 201.8, 201.6, 206.0, 202.9, 194.9; ours 199.9, 201.9, 201.9, 206.3, 202.6,
  202.6 (before 199.9, 201.9, 201.9, 201.9, 206.3, 202.6).
  k -2..+3 lowest: game 62.5, 28.8, 0.4, -29.0, -16.2, 0.0; ours 61.7, 28.1, 1.0, -29.0, -16.1, -1.0
  (before 61.7, 28.1, 1.0, -20.2, -26.6, -10.9).
  Flight ticks 1..27 unchanged: height max 2.3 / rms 1.2, take-off max 1.2, fall max 1.0, lowest max 1.1.
- Not fixed, numbers after the change: post-landing leg squash, k +3..+6 height game 194.9, 194.9, 195.3,
  196.5, ours 202.6, 201.0, 199.5, 199.3 (unattributed). After a landing the body rests 1.6 above the
  spawn height (y 1726.9 against 1725.3: floor cast skin); the figure is drawn that much higher.
- Commands: `local-analysis/d249-landing/run_ours.ps1 -name b2 -script "-:150,r:60,-:60,l:70,-:60,j:6,-:200"`
  (release viewer, `--no-activate`), `python local-analysis/d249-landing/land.py b2 "-25.9,-15.9,0"`.
- cargo fmt --all --check: clean. cargo test -q --locked (CI crate list): pass (2 new: touchdown-tick
  contact in tests/character.rs, tail start in sackboy_anim). cargo clippy -q --locked (CI crate list)
  --all-targets -- -D warnings: pass. cargo check of both viewers --all-targets: pass.
- Not tested: landing from a ledge fall or a running jump against the game (no capture); the dip was fitted
  on one standing 6-tick jump; python unittest not run (no Python change).
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: turn pose (game capture with a direction reversal); post-landing squash; jump air lean (+4 deg).
## 2026-10-12 - D248 lean and height measured; crouch spring is not the landing squash (no visible change)

- D248 measurements that need no change (tick-aligned, D234 captures, `local-analysis/d248-lean-stretch/`):
  idle, steady walk and stop match the game within 0.4 deg of lean and 0.4 units of height.
- Ghidra (read-only, notes `local-analysis/d249-crouch-spring/notes.txt`), extra in-air body layer: none
  found. phys+0x41 is the wall-cling state (set by a wall probe, cleared by the wall jump), creature
  +0x874/+0x878/+0x880 its head look, clip-mix weight 0x80c is grab only. Open candidates for the +4 deg
  air lean: the head-look tilt 1.3 x (2 x body angle - target angle) / (pi/4) from the physical body's
  rotation (0x200c6c); the 0x20a5f4 peak follower (runs only while fewer than 2 legs touch) and its
  landing branch (moved 15 or less this tick and 2 legs in contact); the foot pass 0x210dd8.
- Crouch spring FOUND (0x2011d4, input 0x6c56c = deepest contact depth / 350): implemented as
  `CrouchSpring` + `SackboyAnim::set_contact_depth` in `lbp-sim/src/sackboy_anim.rs`, layered additively
  after the lane layer. Nothing feeds it (no contact depth in our sim), so no pose changed.
- Failed experiment (do not repeat without new evidence): feeding the spring a landing depth k x fall speed.
  Start tick k +0..+3, one or two ticks, k fitted to the game's height and pelvis height (d234 `jump.bin`,
  k 0..+14): best rms 1.60 (start +2, two ticks, depth 31.0, k 0.97; max 4.6) and best max 3.9 (start +2,
  one tick, depth 56.4, k 1.77; rms 1.69), against 2.95 / 7.6 with no spring. The game's squash is full on
  its first tick (k +3) and decays; the spring rises for two ticks. Every fit shortens head-to-pelvis by
  3 to 4 where the game's is within 0.4 of ours. Predicted rebound (taller at 6 to 10 ticks after the
  kick): not resolvable at the +-1 level (game 0.9..1.1 lower against one run, 0.2..1.0 higher against
  another, see the QA point); a rebound in proportion to a 7-unit squash is absent.
- Squash, INFERRED: the game's pelvis keeps rising for ten ticks (height above the floor line k +1..+8:
  18.8, 31.1, 39.3, 40.8, 42.5, 45.5, 43.7, 42.1; rest 48) while the lowest vertex is on the floor from
  k +3: feet held down under a body still below rest (foot pass 0x210dd8, active only when the body moved
  15 or less in the tick). Needs a leg solver in the blender plus the dip continued; not done.
- `LANDING_DIP` re-checked: unchanged (-25.9, -15.9, 0); lowest point k +1/+2 game -29.0/-16.2, ours
  -29.0/-16.2. Height k +3..+6 still game 194.9, 194.9, 195.3, 196.5, ours 202.9, 201.3, 200.0, 199.7.
  Flight ticks: height max 2.3, lowest max 1.1 (unchanged). Idle 200.8..206.1 (unchanged).
- QA: `LBP_DUMP_VERTS` now prints `sackboy pose N` per dumped pose (pose = the `sackboy tick` line above
  it); pose counting drifted when two ticks shared a rendered frame. The picker starts when the figure has
  loaded, so the stand phase at a script tick varies between runs (+-1 height after a landing).
- Commands: `local-analysis/d250-crouch/run.ps1 -name fin -secs 38` (release viewer, `--no-activate`,
  script `-:400,j:6,-:120`), `python local-analysis/d250-crouch/final.py fin`, fits `fit.py`.
- cargo fmt --all --check: clean. cargo test -q --locked (CI crate list): pass, 487 tests (2 new:
  `crouch_spring_follows_the_found_recurrence`, `crouch_spring_gives_nothing_without_input`). cargo clippy
  -q --locked (CI crate list) --all-targets -- -D warnings: pass. cargo check -q --locked -p lbp-viewer
  -p lbp-bevy-viewer --all-targets: pass.
- Not tested: walk and stop re-run in the viewer (no layer can appear: the spring's output is exactly 0
  without input, unit-tested); python unittest (no tracked Python change); n for the stretch_crouch
  frames (taken as the last frame index, as for the look clips).
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: leg solver for the post-landing squash (feet on the floor, body dip continued); turn pose capture;
jump air lean.

## D249: short legs after a landing (2026-10-12)
- Changed: `lbp-preview` `BlendPose::move_chain_end` (two-bone solver) and `Rig::bone_named`; `lbp-sim`
  `LANDING_SQUASH`, `Sackboy::landing_squash`, `LANDING_DIP` twelve entries; the Bevy viewer's `blend_frame`
  raises both ankles by the squash after all layers.
- MEASURED (d234 `jump.bin`, release viewer `-:400,j:6,-:120`): touchdown +3..+8 height 194.8, 194.7, 195.0,
  196.3, 199.0, 201.7 (game 194.9, 194.9, 195.3, 196.5, 199.1, 201.3); pelvis and lowest vertex within 0.4;
  flight and idle unchanged. Table in `docs/d241-landing-dip.md` (D249).
- Commands: `local-analysis/d250-crouch/run.ps1 -name leg1 -secs 70`, `python final2.py leg1` (same folder).
- Tests: `move_chain_end_reaches_the_target_and_keeps_the_end_rotation` (new),
  `air_drag_is_tiny_and_landing_plays_the_measured_dip` (extended). fmt, tests, clippy: see the PR checks.
- Not tested: ledge fall, running jump, walking straight out of a landing (no game capture); GPU skinning
  path by eye (same pose data as the measured CPU dump).
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: jump air lean; turn pose against the new game capture.

## D250 / D251: jump air lean, turn facing and walk restart (2026-10-12)
- Changed: `lbp-sim` `sackboy_anim.rs` (`AIR_EASE`, `Blend::UpperBody`, the upper-body layer; the 72-degree
  yaw hold is for the stop and lane change only; the turn's end leaves the walk phase at 1); `lbp-preview`
  `BlendPose::lerp_masked`, `Rig::subtree_mask`; the Bevy viewer's `blend_frame` and layer log (`^clip`).
- MEASURED: jump (d234 `jump.bin`), Spine1 pitch against Spine ours minus game max 5.5 -> 0.05 degrees over
  the 27 flight ticks, upper arms 14.7 -> 2.9. Turn (`local-analysis/d252-turn/turnRL2`), vertex rms at our
  yaw N = 0..18 48..98 -> 1.9..3.6, N = 19..23 6..16 -> 3.5..3.8. Tables in `docs/d250-air-lean-turn.md`.
- Commands: `local-analysis/d250-crouch/run.ps1 -name lean1 -secs 60`, `python
  local-analysis/d251-air-lean/lean.py leg1 lean1`; `local-analysis/d253-turn-cmp/runall.ps1 rl lr`,
  `python local-analysis/d253-turn-cmp/cmp.py turnRL2 rl`.
- Tests: `upper_body_eases_back_toward_stand_in_the_air`, `masked_lerp_moves_only_the_flagged_subtree` (new);
  `stop_holds_72_degrees_and_turn_follows_the_speed`, `turn_plays_19_frames_then_resets_the_walk_phase`
  (changed). fmt, tests, clippy: see the PR checks.
- Not tested: left-to-right turn and stop re-measured after the change (same rule, mirrored); running jump
  and ledge fall for the air ease (no capture); a mood stand as the ease target.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: mirrored-pose fit (front torso bone in the left-to-right turn); walk start; arms.

## D254: mirrored poses from the game's flip types (2026-10-12)
- Changed: `lbp-preview` `skinned.rs` (`flip_local`, `Rig::with_flip_types`, `Rig::mirrored_trs` takes the
  partner bone's local transform through the skeleton's mirror flip type; the old skin-matrix reflection
  stays for rigs without the table). `Rig::from_skeleton` passes `Skeleton::mirror_flip_types`.
- FOUND: EBOOT pose mirror `0x1e42b4` (from `0x1ebb48`), six rules by flip type on local position and
  quaternion, scale copied, types 6 and up copy. Sackboy (mesh 0000043f): Biped bones type 3, root 1, pelvis
  2, jaw and eye positions 4, tongue 5 and 0, both zip-pull bones 6.
- MEASURED (`local-analysis/d252-turn/turnLR` against the release viewer, vertex rms, free yaw): turn frames
  N = 5..14 2.3..4.2 -> 0.5..1.4; blended frames N = 0..4 and 15..18 1.2..1.3 and 0.4..0.5; mirrored steady
  walk 0.7..1.6; zip-pull joint at N = 9 15.1 -> under 1.7. Right-to-left run unchanged (0.6..1.8).
- Commands: `local-analysis/d253-turn-cmp/runall.ps1 lr rl`, `python local-analysis/d253-turn-cmp/cmp.py
  turnLR lr`, `python local-analysis/d253-turn-cmp/perjoint.py`.
- Walk start, MEASURED, no change: phase at the first moving tick in five game captures 0, 0, 3, 4, 4
  (free-running counter, as ours); at the game's phase our start fits `stopR` at 0.9..1.7 on ticks 1..15
  (`local-analysis/d253-turn-cmp/wstart.py`, `wstart2.py`, run `ws`).
- Tests: `mirrored_pose_follows_the_flip_types` (new). fmt, tests, clippy: see the PR checks.
- Not tested: the game's job-queue path of the mirror (only the direct routine was read); costume meshes
  with other tables; mirrored morphs (`mirror_morphs`, not applied).
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: walk start; air ease arm terms; small items.

## D255: Sackboy stick response, shortest jump, mirrored morphs (2026-10-12)

- FOUND (EBOOT, read-only): left stick x = (byte - 128) / 128, no clamp (0x38a578); response per axis
  max(|x| - d, 0) / (1 - d), squared, signed (0x38aa10); the walk and air caps are 30 and 22.5 times it.
  d is read at run time (INFERRED 0.3). Full right (byte 255): 0.9778 -> 29.334 and 22.0006.
- MEASURED (`local-analysis/d211-moon/walk-*.csv`, `jump-air-*.csv`): right 29.34 / 22.00, left 30.00 /
  22.50; ramp 3 per tick both ways. `jump-tap-*.csv`: per-tick rise 12.0, 9.3, 6.6 (peak 33.1, two files)
  and 12.0, 19.3, 16.6 (peak 90.4); `jump-hold-100ms-a` 143.3.
- Change: `stick_response`, `STICK_DEAD_ZONE`, `STICK_FULL_RIGHT`, `JUMP_MIN_TICKS` 1 (`lbp-sim`
  `sackboy.rs`); viewer keys, script `r` and the pad stick pass the stick value; `Rig::mirrored_morphs`,
  `Rig::with_mirror_morphs` (`lbp-preview` `skinned.rs`; FOUND 0x1e42b4, dst morph i = src morph table[i]).
- FOUND, no change: arm terms +0xae0/+0xae4 ease 0.2 per tick toward 1 while an arm button is held
  (0x1fb0e4) or are raised while another creature holds him (0x2097e0); 0 in a plain jump.
- MEASURED, open: standing foot spacing 43.4 (game, d234 Moon and d252 Rookie Test) against 51.9; resting
  height after a landing 0.5 to 1.5 above spawn in a scripted run (five jumps).
- Tests: `full_right_is_slower_than_full_left`, `mirrored_pose_swaps_morph_weights` (new),
  `jump_heights_match_the_measured_tap_and_holds` (one- and two-tick presses added). fmt, tests, clippy:
  see the PR checks.
- Not tested: analogue stick values between the dead zone and full (rule read, no capture); the dead zone
  value itself; a clip whose mirrored morphs differ (none checked); ledge fall (no capture).
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: foot placement while standing; ledge-fall capture.

## D256 standing foot placement (2026-10-12)
- FOUND (EBOOT 0x210dd8, 0x82f20, 0x86e18, 0x6c15b8, 0x6c1528): on the ground with ground-relative speed
  under 1, each Foot joint goes to the collision point nearest its leg anchor (LimbAnchorPos x -25 / +25) by
  a two-bone solve; ease in count/3 over 3 ticks, out 1 - k/5 over 5.
- MEASURED: stand clip Foot joints at x +29.88 / -28.70, depth +2.77 / -0.31 (`pose_fit` env `FEET`).
- Change: `FOOT_PLANT_X`, `FOOT_PLANT_SPEED`, `FOOT_PLANT_IN`, `FOOT_PLANT_OUT`, `Sackboy::foot_plant`
  (`lbp-sim` `sackboy.rs`); `blend_frame` moves both Foot joints by that weight (`lbp-bevy-viewer`
  `sackboy_play.rs`).
- MEASURED after (scripted stand, game `stopR` standing pose): foot spacing 43.6 (game 43.4, was 51.9);
  whole-mesh rms 0.79 (was 1.30), legs 1.52 (was 4.63); per-foot offset within 0.4 in x and depth.
- INFERRED: the depth target is the body's depth line (fits the measured feet; the game adds (0,0,3) to a
  target in a frame not traced); the lane change is kept out of the gate; the foot keeps the clip's rotation
  (the game builds one from the contact normal, the same on flat ground).
- Tests: `foot_plant_eases_in_standing_and_out_when_he_moves` (new). fmt, tests, clippy: see the PR checks.
- Not tested: slopes and steps (the target is a fixed model-space spot, not the contact point); the ease
  against game ticks (rule read, no per-tick comparison); both feet stay 0.6 higher than the game's.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: ledge-fall capture.

## D259 trigger events (2026-10-12)
- RScript plan stage 2 (`docs/rscript-plan.md`); detail in `docs/level-scripts.md` part 10.
- FOUND (EBOOT trigger pass 0xe3bc0, per trigger 0x1364f0, test 0x135fe0, add 0x135a18, remove 0x1346fc):
  the player's position is taken into the trigger Thing's space; type 0 circle, 1 box, 4 sphere of the
  radius multiplier, times the hysteresis multiplier while the in-list is not empty; without the
  all-layers flag the trigger's z plus z offset must lie in the z span of the Sackboy's body; Enter on
  joining the in-list, Leave on removal, Inside for each Thing in it; Enter and Leave go to the script of
  the trigger's Thing or its parent's, Inside to the parent's; ordinary triggers test players only.
- Change: `lbp-level` keeps PTrigger (`PartData::trigger`); `lbp-runtime` `TriggerState`,
  `LevelWorld::add_player`, `move_player`, `update_triggers`, `LevelScripts::trigger_events`, `tick_level`;
  `lbp-script-vm` binds `Native.GetNumInThings__Q5Thing` (FOUND 0x648770) and
  `PartList.HasPYellowHead__Q5Thing` (INFERRED), 118 bound; viewer `level_play.rs` feeds the play-mode
  Sackboy; `level_scripts` writes a trigger census and takes `--visit-triggers`.
- MEASURED (378 levels, 60 ticks, then a visit to each of 21,755 triggers with a POS): 23,990 PTrigger
  Things (type 0 23,715, type 1 275; script on the Thing 2,388, on the parent 2,274, none 19,328); events
  raised 39,212 Enter / 101,918 Inside / 39,178 Leave; run by a script 11,430 / 895 / 7,357; Things
  stopped by a fault 1,722 with and without the visit; unbound natives reached 65 -> 94.
- INFERRED: the player's point is his Thing position; triggers run in Thing order after the updates;
  `PYellowHead.PlayerNumber` is the player index. PLACEHOLDER: the viewer Sackboy's z span (lane +-100).
- Not modelled: the engine object kinds with their own trigger handling or wider candidate lists, the
  Sackboy states that never count, the world-bounds cull.
- Tests: `stage2_trigger_holds_the_player_by_shape_hysteresis_and_layer`,
  `stage2_trigger_events_call_the_parent_script` (new). fmt, clippy, tests: see the PR checks.
- Not tested: the viewer window with a walking Sackboy; any trigger against the game in RPCS3.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stage 3, switch events and inputs.

## D260 switch inputs (2026-10-13)
- RScript plan stage 3a (`docs/rscript-plan.md`); detail in `docs/level-scripts.md` part 11.
- FOUND (EBOOT, native then reader): a Thing has an input table, one entry per port, each empty or the
  SwitchOutput wired to it; the output holds its signal (activation, ternary, player) and its PSwitch.
  `IsSwitchTriggered__i` (0x64228c, 0x7541e0) is true when an output is wired; Binary (0x642228, 0x754308)
  is wired and ternary not 0; Analogue (0x6421c4, 0x7542a4) the activation, else 0.0; Ternary (0x642160,
  0x75437c) the ternary as a float, else 0.0; Player (0x6420fc, 0x754ccc) the signal's player when not -1;
  `GetInput__Q5Thingi` (0x643f2c, 0x754244) the Thing of the wired output's PSwitch; `GetOutput__Q5Thingii`
  (0x643ea0, 0x754af8) a target Thing of an output of the Thing's own PSwitch; `Native.GetInThing__Q5Thingi`
  (0x6486ec, 0x6d558c) an entry of the PTrigger in-list; `BaseWorld.IsScoreBoard__Q5Thing` (0x64bae0,
  0x7569ec, 0x622658, 0x164454) the Thing's script is script 11599 or derives from it.
- INFERRED: the input table is built from the outputs' stored targets (Thing, port); first output in Thing
  order wins a port.
- Change: `lbp-level` keeps PSwitch (`PartData::switch`); `lbp-runtime` `SwitchState`, `InputWire`,
  `LevelWorld::wire_inputs` and the World methods; `lbp-script-vm` reshapes `SwitchInput` to the wired
  signal and binds 5 natives (123 bound), `Vm::class_chain`; `level_scripts` writes a switch census.
- MEASURED (378 levels, 60 ticks): 141,572 PSwitch Things, 144,608 outputs (none null), 177,800 targets
  (none with a null Thing), 136,337 Things with an input table, 177,800 wired ports, 0 port collisions,
  3,732 scripted Things with a wired port; `IsScoreBoard` true for 102 Things. Things stopped by a fault
  1,722 -> 582; native calls bound 51,030 -> 186,679 (with the trigger visit 215,292 -> 1,069,101);
  unbound natives reached 65 -> 66 (visit 94 -> 91).
- Not modelled: the logic network (no sensor, gate or timer runs, signals stay as stored);
  `SwitchPerformReaction__b`; the owner-player fallback of the Player form; the Thing flag at +0x7f that
  makes four forms read "nothing wired"; the game's missing bounds checks (out of range is null here).
- Tests: `stage3_input_tables_are_built_from_the_switch_outputs`,
  `stage3_switch_input_natives_follow_the_wired_output`, `stage3_outputs_in_things_and_score_boards` (new).
  fmt, clippy, tests on the non-viewer crates, `cargo check -p lbp-bevy-viewer` pass.
- Not tested: the viewer window; any switch against the game in RPCS3.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stage 3b, evaluate the logic network and call the switch reactions.

## D261 switch network and switch reactions (2026-10-13)
- RScript plan stage 3b (`docs/rscript-plan.md`). New module `crates/lbp-runtime/src/switches.rs`;
  `tick_level` runs the script tick, the trigger events, then `LevelWorld::update_switches`.
- FOUND in the EBOOT: per-switch update 0x1244a4 (edge rule: on = new ternary not 0 and (ternary changed or
  forced); targets visited on an on/off change, a player change or `on`); apply 0x11ddf0 (reaction by the
  target's object kind; state changes of toggle, counter, selector and timer on input edges); output
  compute 0x11bd58 through jump table 0x11be00, inverted table 0x11bfd4; kind classifier 0x3ed2c (order of
  tests, which switch types take a kind before the script GUID is looked at); microchip off 0x409dc.
- Computed types: 36 microchip, 37 circuit board, 23/24/40 NOT / pass / node, 14 AND, 21 OR, 22 XOR,
  28 battery, 20 direction, 27 sign split, 17 toggle, 35 selector, 15 counter, 16 timer. Stored signal kept:
  every sensor type, 19 randomiser, a board owned by a sequencer. Constant {0, 0, -1}: the types the game
  gives no handler.
- INFERRED: the type names (order of the `SWITCH_TYPE_*` strings); 30 ticks per second and timer length in
  ticks; the first pass compares against the stored signals, so power-up edges fire on tick 1; the off
  flags are computed once before the first pass; cycle breaking by lowest Thing index.
- MEASURED census (378 levels, 141,572 PSwitch): the AND / OR mode word is 0 or 1 (AND 5,671 / 56, OR
  4,084 / 5); 5,686 NOT gates are inverted and 108 are not; 28,278 microchips, each with a type-37 board,
  5,420 with port 0 wired; 4,298 of 4,599 battery outputs are stored off while the battery is on.
- MEASURED runs (378 levels, release):
  - plain, 60 ticks: 75,140 target visits, 204 `__b` and 36 `__bf` calls, 24,612 outputs differ from the
    stored value at the end, 18,100 Things off under a microchip; stopped Things 582 -> 597.
  - with the trigger visit: 222 `__b`, 37 `__bf`; stopped 582 -> 598.
  - with every output pulsed on and off: 3,052 `__b`, 1,345 `__bf`; stopped 1,794 (new: `PWorld.Frame` in
    ScoreTweak 925 Things, `Thing.Pos` in Trigger_Speech_Bubble 287).
  - plain, 300 ticks: 226 `__b`, 37 `__bf`; stopped 598.
  - 19 more unbound natives reached in the plain run (global settings, speech bubble, music, Gooey frame).
- Not modelled: sensors (tag, player, impact, grab ...), the randomiser, dirty flags (every switch runs
  each tick), sequencer boards and cursor chips, the three extra gates of `__bf`, object kinds 0x49, 0x53,
  0x54, the mesh / trigger / gameplay-data kind tests, the activating-player record, the force flag.
- Tests: 21 unit tests in `switches.rs` (edge rule, order with cycles, each type, inverted forms, timer and
  counter over ticks, toggle, selector, microchip off) and two world tests
  (`stage3b_switch_reactions_go_to_listed_scripts_only`, the AND -> NOT -> script chain). fmt, clippy,
  the CI test set pass.
- Not tested: the viewer window; any switch against the game in RPCS3.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stage 4, player and game state.

## D262 world, position, player and game state (2026-10-13)
- RScript plan stage 4 (`docs/rscript-plan.md`), in two halves on one branch. The scripts now get answers
  about the level itself; before, the first such question stopped the Thing.
- World and position half:
  - Bug fix: `part_name` dropped the leading P of every name, so `thing.Pos` was never found (also
    `PhysicsTweak`, `PoppetPowerup`, `PocketItem`, `PowerUp`); `MetaData`, `MicroChip`, `WormHole` and
    `TagSynchroniser` did not match the reader's names.
  - `lbp-level` keeps PWorld `frame`, `simFrame`, `frameLevelStarted`, `gameMode`, `isPaused`, `waterLevel`
    and the two global-setting arrays (FOUND: the serializer 0xb3f3c0..0xb3fd20 names them).
  - Members: `PWorld.GameMode`, `Frame`, `WaterLevel`; `PPos.WorldPosition` get and set; `PTrigger`
    `RadiusMultiplier`, `AllZLayers`, `HysteresisMultiplier` (tied to the trigger test).
  - Transforms (FOUND 0xf2e80, 0xf26bc, 0xf1d58, 0x755874, 0x755a98): local matrices derived at load;
    `SetLocalPosition__m` and `SetWorldPosition__m` recompute the Thing and every descendant;
    `GetWorldPosition__` returns the matrix (it returned a vector); `IsInactive__` reads the deactivated
    flag.
  - Natives newly bound (123 -> 127): `Thing.GetLocalPosition__`, `Thing.GetMicroChipProxyFromThing__Q5Thing`,
    `Thing.GetWorldPositionForSensor__` (the branch for a Thing that is not on a sequencer),
    `Game.IsOnSequencer__Q5Thing`.
  - VM fix: m44 elements are at column * 4 + row. FOUND in the script tables (use only): `GET_M44_cW` is
    used 115 times as a position; the game's test script still passes 512 of 513 checks.
  - `Audio.PlaySound` requests are counted by sound name.
- Player and game state half (FOUND unless marked):
  - Player records (find 0x156b18): score, multiplier counter, timer, source Thing, visible flag, number,
    team. Natives: `EPlayerNumberFromInt` (identity, 0x5e68bc), `DoesPlayerExist` (0x5e8728),
    `IsPlayerConnected` (0x5e79f4), `GetYellowheadFromPlayerNumber` (0x18ec14), `IsLocalPlayerID`
    (0x5ea758), `PlayerNumberIsLocalPlayer` (0x243b54, shape), `GetPartyLeadersPlayerNumber` (0x240a88,
    shape), `Set/GetPlayerTeamNumber` (0x5e81e4, 0x5e8234), the multiplier counter set / get / increment
    (0x5e858c, 0x5e8eb4, 0x5e8e1c), `GetPlayerScoreMultiplier` (0x5e8178: counter / 5 + 1), the timer get /
    decrement (0x5e8540, 0x5e9118), visible and source (0x5e82f0, 0x5e838c), `GetPlayerScore` (0x5e86ac),
    `SetPlayerScore` (0x3970e4: clamped to 0.0 .. 4,000,000.0, both constants read).
  - Fixed: `GetPlayerNumber__i` gives -1 outside the records and `SetPlayerScoreMultiplierTimer` does
    nothing for a number without a record (both were errors).
  - `Get/SetGlobalSettingUID` (0xce4fc, 0xce554): a request below the slot's priority is dropped, a slot at
    priority 1 is never taken, else the slot takes the UID and the priority (0 for UID 0).
  - `GetLevelSlotID__` / `GetPrimaryLevelSlotID__` (0x16cb18, 0x16ca08): a SlotID object made ahead.
  - `IsVcrPaused__` (0x18cf18), `IsTutorialLevel__` (0x156e44), `IsPSNDemoBuild__` (0x3a9a4),
    `Audio.IsSafeToTriggerMusic__i` (0x2c6a14: true in play), `TriggerLevelLink.FindLockMesh__` (0x1343dc).
  - Recorded only (9): `World.SetPieChartMultiplier`, `Thing.UpdateScale`, `HideRenderMesh`,
    `SetRenderMeshEditorTexture`, `SetRenderMeshEditorColour`, `RemoveFromAnimatingDivergentList`,
    `Trigger_Speech_Bubble.NotifyNpcsOfMouthEnd`, `NoJoinMarker.SetGameNoJoin`, `Game.GotoScoreboard`.
  - 37 natives in this half; 164 in the table.
- INFERRED: `Frame` + 1 at the end of a tick; a `PPos.WorldPosition` write is a raw write of the world
  matrix; the identity for a position getter on a null Thing or one without a PPos; the microchip proxy is
  the outermost chip; one local player, number 0, team 0, whose record is there before his Sackboy; a
  player with a record is local; the first record leads the party; `IsVcrPaused` is false in play;
  `IsTutorialLevel` and `IsPSNDemoBuild` are false; the level's slot is Type 0, SlotNumber 0; one shared
  SlotID object instead of a new one per call.
- MEASURED census (236 levels with a PWorld): game mode 0 in 190, 1 in 14, 2 in 32; IsPaused true in 220;
  the global-setting arrays hold 8 entries, all 0. 1,107 of 6,951 scripted Things on a board sit on an
  inner chip that has no PPos; the outermost chip always has one.
- MEASURED runs (378 levels, 13,547 scripted Things, release, 60 ticks; before stage 4 -> after the first
  half -> now):
  - plain: Things stopped by a fault 597 -> 117 -> 4; fault kinds 11 -> 3 -> 2; reaction calls
    240 -> 240 -> 262; distinct unbound natives called 85 -> 98 -> 83.
  - with the trigger visit: 598 -> 117 -> 4; 11 -> 3 -> 2; 259 -> 260 -> 283; 99 -> 112 -> 99.
  - with every output pulsed: 1,794 -> 117 -> 4; 11 -> 3 -> 2; 4,397 -> 5,611 -> 5,704; 102 -> 116 -> 110.
  - Levels with a fault 221 -> 105 -> 2. Native calls in the pulsed run: 3,650,429; 2,697,610 bound, of
    which 17,206 recorded only; 952,819 defaulted.
  - Global-setting slots held at the end: 77 plain, 121 with the visit, 188 pulsed. Scoreboard requests:
    3, 4 and 39 (in 3, 4 and 23 levels).
  - Faults left: `Thing.GetCOM__` (3 Things, 1 level), a null Thing in the Pod's `SetShapeEditorColour`
    (1 Thing).
  - New unbound natives reached: `Pad.IsJustPressed` (131,404 calls, 220 levels, pulsed run),
    `OffScreenIndicator.GetPlayerIndicatorActivityType` (65,702 / 220), `Game.GetGameFrame` (763 / 121),
    `ModalOverlay.IsPartyLeaderOverlayActive` (33,120 / 103), the level-link prompt, the
    `SetGlobalSettings...To` group.
- Not modelled: where the game steps `Frame`; the Thing flag that forces world = local; the refresh of
  bodies and shapes after a transform write; the sequencer branch of the sensor position; the score
  history and the HUD calls of the score natives; edit mode of `SetPlayerScore` and
  `IsSafeToTriggerMusic`; the level's own slot; which levels are tutorials; the part test in the
  `FindLockMesh` walk; `Thing.IsDead` / `IsInLimbo` (left unbound: a creature's state is not kept); the
  Sackbot and enemy group, Gooey, HUD, messaging, the slot database.
- Tests: world tests on synthetic levels for each item (27 in `level_world.rs`, 10 of them stage 4), 11 unit
  and 13 VM tests in `lbp-script-vm`. fmt, the CI clippy line, the CI test set and `cargo check` of both
  viewers pass.
- Not tested: the viewer window; any value against the game in RPCS3.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stage 5, show what the scripts do.

## D263 the natives the stage 4 runs reached (2026-10-13)
- Between RScript plan stages 4 and 5 (`docs/rscript-plan.md`). The stage 4 runs called 110 natives without
  a binding; the 48 reached in the most levels are bound now (164 -> 212), from research notes that give
  each one a class: real value from kept state, real value from new state, constant, recorded only.
- New state (FOUND unless marked):
  - Pad: `PlayerRecord::pad` is a `PadState` (two mode masks, buttons now, buttons of the frame before).
    `Pad.IsPressed` (0x388e2c), `IsJustPressed` (0x388358), `IsJustReleased` (0x389400): the button
    argument is a mask and all its bits must be down. The Thing's player has no pad until
    `LevelWorld::set_pad_buttons` writes one, so the three are false; `end_pad_frame` (called at the end
    of `tick_level`) makes the buttons the old ones. Only the four d-pad bits have FOUND names; no button
    name is in the code.
  - LAMS: `LevelWorld::translations`, searched in order. `IsInLams` (0x159c1c) is true when a table has
    the key; `Translate` gives the first table's text. `level_scripts` loads the English base and patch
    tables (2 read, 29,477 keys, all in the base table).
  - Global settings: `Blend` (start, target, t; value = start + (target - start) * t) for water level,
    gravity, wave magnitude, murkiness and water bits; one tint value; the lighting group of five values
    under one t; 8 blend factors. `SetGlobalSettingsTo` (0xd9dac), `...GravityTo` (0xcef3c),
    `...WaterLevelTo` (0xce608), `...WaterWaveMagnitudeTo` (0xce654), `...WaterTintTo` (0xd5d68, shape),
    `...WaterMurkinessTo` (0xd51b0), `...WaterBitsTo` (0xce6a0), `...WaterDrainSoundsEnabled` (0x5ec908),
    `SetGlobalSettingsBlendFactor` (0xd623c), the five water getters (0x5ec6f8, 0x5ec4f8, 0x5ec424,
    0x5ec340, 0x5ec25c). The water level getter gives 0.0 unless the level is above 0.0001 (both
    constants read).
  - `lbp-level` keeps the stored wave magnitude, gravity, tint, murkiness, water bits and the drain-sound
    flag.
- From state already kept: `Game.GetGameFrame` (0x19f63c), `GetNumYellowHeadAvatars` (0x5ed9f8),
  `Score_Board.GetGameScoreDataType` (0x16aa4c), `IsSlotIDValidForScoring` (0x229bb8),
  `Messaging_ShouldFilterMessageStrings` (0x736260), `Resource.LoadAnimatedTextureByFilename` (0x617c0c).
- `Slot.GetSlotGroup` (0x640500 -> 0x7477d8 -> 0x1a6cd8, builder 0x50c2cc): a new SlotID from the group
  pair of the slot's record, or (0, 0) when the slot database lacks the slot; never null. The lookup
  finds nothing for the slot (0, 0), so the world hands out a SlotID (0, 0) made ahead for the empty
  slot; any other slot is an error (no slot database).
- Bug fix: a script cast (`CASTsp`) of a Thing tests its script class and superclasses
  (`LevelWorld::thing_is`, names kept by `add_class_defaults`). Every cast passed before.
- Constants: `Poppet.GetTextEntryType` 2 (0x56a838) and `Game.IsTarsierDLCEnabled` true (0x5e68d8) are
  literals in the executable. INFERRED for one local player: `GetPlayerIndicatorActivityType` 0,
  `GetNumKeyNames` 0 and `GetKeyIndex` -1, `IsPartyLeaderOverlayActive` false,
  `GetNumKnownItemsCollectedAtStart` 0.
- Recorded only (16; 29 in `RECORDED_ONLY`): `Messaging_NewTriggerMessage`, `...NoPrompt`,
  `...Subtitles`, `Messaging_NewLevelLinkPrompt`, `Messaging_NewWormHolePrompt`, `Messaging_NewGameEvent`,
  `HUD.RegisterOverlayCallback`, `HUD.PreloadGlyphs`, `Game.TrackSpeechKey`,
  `Trigger_Speech_Bubble.NotifyNpcsOfMouthStart`, `Thing.AddToAnimatingDivergentList`,
  `Thing.SetAnimLoop`, `Gooey.SetLastNavSelectionMode`, `Gooey.PerFrameUpdate`,
  `Gooey.SetStylesheetScalingFactor`, `Game.SetGlobalSettingsAudioTo`. Ten are kept with their arguments
  and the frame in `LevelWorld::requests` (at most 4,096); `SetAnimLoop` is kept on its Thing.
- INFERRED: both pad mode masks are 1 (mode 1 = gameplay); the old buttons are copied at the end of a
  tick; the English tables are the ones loaded in a level; every setting starts at rest on its stored
  value (the stored start, target and factor are not read); the lighting group starts from the stored
  `from`, `target` and blend factor; `SetGlobalSettingsWaterTintTo` stores its argument; the type 2 answer
  of the message filter; the scripts do not change the SlotID objects they are handed.
- MEASURED runs (378 levels, 13,547 scripted Things, release, 60 ticks; before -> now):
  - plain: unbound natives called 83 -> 52; unbound calls 88,892 -> 67,045; recorded-only calls
    3,379 -> 13,526; Things stopped by a fault 4 -> 4; levels with a fault 2 -> 2.
  - with the trigger visit: 99 -> 85; 941,081 -> 758,850; 13,855 -> 76,257; 4 -> 4; 2 -> 2.
  - with every output pulsed: 110 -> 88; 952,819 -> 766,940; 17,206 -> 83,820; 4 -> 4; 2 -> 2.
  - Native calls in the pulsed run: 4,064,366; 3,297,426 bound; 766,940 defaulted.
  - Faults as before: `Thing.GetCOM__` (3), the Pod's null Thing (1). A first run stopped 26 / 35 / 58
    Things: a null object in `TriggerLevelLink.IsSourceSlotLocked` (reached because `IsTarsierDLCEnabled`
    is true; `Slot.GetSlotGroup` was unbound and returned null), and `Race_End.Enter` reading
    `Race_Start.ColorIndex` on the world's current challenge when that Thing is of another class (the
    cast to `Race_Start` passed for any Thing). Both are fixed above.
  - With the trigger visit the scoreboards of 101 levels go on: 19 natives are new there
    (`Score_Board.PostScore`, `SetScoreLocked`, `CreateMagicWall`, `Cam.SetOverrideCamera`,
    `Game.StartAsyncSection` and others).
  - Requests kept, pulsed run: 1,771 speech bubbles and subtitles, 2,058 game events, 216 level-link and
    103 wormhole prompts, 137 overlay callbacks, 297 audio settings, 1,806 speech keys.
  - Most called without a binding (pulsed): `Native.IsEnemyAI`, `Native.SetWalkInput`, `Thing.IsDead`,
    `Thing.IsInLimbo` 114,228 calls in 26 levels each, `Thing.GetBodyRoot` 65,490 in 27.
- Left unbound on purpose: `Slot.GetLevelIcon` (slot database and a default icon),
  `Game.GetNumKnownItemsInLevelAtStart` (0 would be a wrong fact), the 11 audio getters (the reader drops
  the stored audio settings).
- Not modelled: any pad input (the viewer does not call `set_pad_buttons` yet); the button names other
  than the d-pad and the meaning of the mode masks; which tables the game loads, and tables in the viewer;
  the tint colours, the lighting block, the physics gravity and the renderer values behind the global
  settings; the off-screen indicator for a Sackboy off screen; Sackbot names; the second test of the
  scoring slot test; a SlotID a script built itself and the group of any slot but the empty one (errors).
- Tests: 33 world tests in `level_world.rs` (6 new: pads, LAMS tables, global-settings blends, game-state
  queries with the scoring slot test and the slot group, recorded calls with their arguments, the script
  cast), 12 unit and 13 VM tests in
  `lbp-script-vm`, the level reader test for the new PWorld fields. fmt, the CI clippy line, the CI test
  set, the Python tests and `cargo check` of both viewers pass.
- Not tested: the viewer window; any value against the game in RPCS3.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stage 5, show what the scripts do.

## D264 tag sensors and player sensors (2026-10-13)
- Between RScript plan stages 4 and 5 (`docs/rscript-plan.md`). The switch pass of D261 kept every
  sensor's stored signal. The tag sensor (switch type 4) and the player sensor (type 3) are computed now,
  from research notes on the two handlers; the other sensors, the randomiser and the sequencers still keep
  the stored signal.
- Reader: `lbp-level` keeps the PSwitch fields Radius, MinRadius, the key colour index and name,
  RequireAll, AngleRange, KeySensorMode, DetectUnspawnedPlayers and PlayerMode, and the PSwitchKey part's
  colour index and name (`PartData::switch_key`). The bytes read are the same; the 378 levels read as
  before, with the same Thing counts per level.
- Rules (FOUND in the 01.33 EBOOT), in `lbp-runtime/src/sensors.rs`:
  - Tag map: per (colour index, name) the tags with x and y, sorted by x, rebuilt as the first step of
    the switch pass (0xd5c1c); `LevelWorld::refresh_tags`, `LevelWorld::tags`.
  - A sensor or tag is placed by its microchip's Thing when it is on a chip, else by its own Thing;
    distances in x and y only.
  - Tag signal (0x12f6f4): off on a deactivated Thing, else the signal wired to the Thing's input port 0,
    {1.0, 1, -1} with nothing wired.
  - Range: MinRadius^2 <= d^2 <= Radius^2; none with a negative Radius. Closeness: (1 - d / Radius) / 0.75
    with MinRadius 0, else 1 - (d - MinRadius) / (Radius - MinRadius); 1.0 with a negative Radius.
  - Ternary (0x11873c): with s = activation * 3.75, off at s <= 0.25, on at s >= 0.75, in between what
    output 0 holds (read back through the inversion).
  - Tag sensor (0x118d5c): the tags within the Radius in x, by x; a tag that is off is skipped (in
    KeySensorMode 1: one with activation 0); team filter (0xf9a64); a tag on the sensor's own chip is
    skipped unless DetectUnspawnedPlayers is set; range; kept by falling strength, cut to BulletsRequired;
    the output is the weakest kept one, closeness clamped to 0..1, or in mode 1 the tag's own signal.
  - Player sensor (0x119910): the players' Sackboys; PlayerMode 0 counts the leader only; RequireAll
    needs every one that counts, else BulletsRequired; closeness of the farthest one needed; the output's
    player is the nearest.
  - The inversion is applied afterwards, also to "off".
- INFERRED choices, with the sensors they touch (378 levels):
  - No cone test for AngleRange below 180: 1,811 player and 2,975 tag sensors. The test is known, the
    direction is not.
  - No layer test for IncludeTouching 1: 212 player and 489 tag sensors (1,932 and 3,353 with either of
    the two). The formulas are known, the bodies' z spans are not kept.
  - The points are the Things' positions (the game takes body centres for the player sensor): all 5,627
    player sensors.
  - The leader is the first player record: 4,280 player sensors with PlayerMode 0; no effect with one
    player.
  - The sensor's body has no owning player: 8 tag sensors with TeamFilter 1, none in a versus or team
    level, so no effect.
- MEASURED census: 5,627 player sensors (227 inverted, 408 RequireAll, 360 with BulletsRequired above 1,
  47 with a negative Radius, 89 with a MinRadius, 192 with DetectUnspawnedPlayers; PlayerMode 0 / 1 / 2:
  4,280 / 759 / 588); 16,953 tag sensors (1,677 inverted, KeySensorMode 0 / 1: 15,984 / 969, 418 with
  BulletsRequired above 1, 467 with a negative Radius, 130 with a MinRadius, 961 without a matching tag in
  their level); 19,629 tags, 5,316 with input port 0 wired.
- MEASURED runs (378 levels, 13,547 scripted Things, release, 60 ticks; before -> now):
  - plain: sensors unlike the stored signal on some tick 184 player and 5,446 tag (184 and 4,364 by
    ternary); outputs unlike the file at the end 24,612 -> 33,541; reaction calls 262 -> 323; unbound
    natives called 52 -> 52; unbound calls 67,045 -> 73,329; Things stopped by a fault 4 -> 4.
  - with the trigger visit, which now also stands at all 5,627 player sensors: 5,204 and 6,272 (5,203
    and 5,152 by ternary); 24,665 -> 40,244; 283 -> 8,970; 85 -> 88; 758,850 -> 1,002,475; 4 -> 4.
  - with every output pulsed: 9,486 -> 9,486; 5,704 -> 14,346; 88 -> 88; 766,940 -> 1,010,588; 4 -> 4.
  - Fault kinds 2 and levels with a fault 2 in all three runs, as before.
  - New without a binding (visit run): `Game.PlayTutorialMonitorVideo`,
    `Profile.SetLBP2GameProgressionFlag`, `Game.TryToAwardPin`.
- Viewer: the play-mode Sackboy is already the level world's player, so the player sensors follow him;
  no viewer change.
- Not modelled: the cone and the layer test (above); sensors and tags on a cursor; the owning player of
  a body; a tag skipped by a Thing flag in one game state; players out of play, a player sitting in
  something and DetectUnspawnedPlayers on a player sensor; the snapshot copy of the tag map; the other
  sensor types, the randomiser, sequencers.
- Tests: 9 unit tests in `sensors.rs`, one world test (`d264_sensors_read_the_level_parts`), one reader
  test (`switch_and_switch_key_keep_the_sensor_fields`). fmt, the CI clippy line, the CI test set, the
  Python tests and `cargo check` of both viewers pass.
- Not tested: the viewer window; any sensor against the game in RPCS3.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stage 5, show what the scripts do; the cone direction and the layer test.

## D265 the message list (2026-10-13)
- RScript plan stage 5, PR 1 (`docs/rscript-plan.md`). D263 kept the speech-bubble, prompt and subtitle
  requests with their arguments. The level world now runs the game's message list by them, from the
  stage 5 scoping notes and further reads of the manager's code. Nothing is drawn yet.
- Rules (FOUND in the 01.33 EBOOT), in `lbp-runtime/src/messages.rs`:
  - Manager 4 of 5 takes the trigger messages and the two prompts; at most 8 messages (0x3e8dc0).
  - Types: bubble with a prompt 1, without 2, subtitles 3, level-link prompt 0x2b, wormhole prompt 0x59,
    caption 0x5d / 0x5e, think 0x5f / 0x60.
  - A request of a type about a Thing that has a message of the type, not closing, does nothing
    (0x3e4124, asked by every core).
  - With 8 held, the eighth is cleared unless its priority number is below the new type's; all nine
    types have priority 10 (0x3e3248).
  - Types 2, 3, 0x5e and 0x60 (flag bit 0) are written over the first message of the type, keeping its
    id; the others are added.
  - Event 3 reaches the messages of the Thing passed with it and runs action 0x10, which sets the
    closing byte (0x3e8aa0, 0x3e528c); event 9 does the same to subtitles. `Messaging_NewGameEvent` with
    an index above 4 means manager 4 (0x7353ec).
  - No type here has a timed action. A trigger message has no player (both fields -1). The prompt
    bubble carries a prompt string unless prevent close is set (0x7382e0).
  - Actor name (0x737910): a translation tag when the level slot's type is 0 or 8, else as passed.
- INFERRED choices:
  - A closing message leaves the list at once (the game keeps it flagged for a time that was not read).
  - The list keeps the order of adding; the game's sort comparison was not read. This decides which
    message is the eighth: it mattered once in the three runs.
  - A message written over restarts its age (`Message::age`, counted in `PWorld.Frame`).
  - The anchor is the Thing's microchip when it sits on one, else the Thing (the Thing the bubble's
    script measures its off-screen distance at); where the HUD draws was not read.
  - A tag in no table gives no actor name (the game's default string taken as empty); the slot type is
    the INFERRED 0 of D262.
- Read API for the viewer: `LevelWorld::messages.open`, `Messages::of_thing`, `Message` (id, kind, style,
  text, actor, actor_tag, thing, anchor, bubble_pos, triggered_once, prevent_close, has_voice, frame),
  `Message::age`, `message_type`, `has_close_prompt`, `LevelWorld::message_position`, `CLOSE_PROMPT_KEY`.
- Natives: `Messaging_NewTriggerMessageCaption`, `...CaptionNoPrompt`, `...Think`, `...ThinkNoPrompt`
  bound (handlers 0x64ac74, 0x64abc8, 0x64ab1c, 0x64aa70), 212 -> 216. `natives::MESSAGING` lists the
  ten natives that open or close a message; they are no longer in `RECORDED_ONLY` (29 -> 23), because
  the world keeps state by them. `KEPT_REQUESTS` 10 -> 14.
- `level_scripts`: a `messages` block per played level and in `play_all` (requests, opened, written
  over and duplicate by type; text in a loaded table, other, empty; actor names; anchors; closed; open
  at the end; levels).
- MEASURED runs (378 levels, 13,547 scripted Things, release, 60 ticks; plain / visit / pulsed):
  - requests 92 / 1,690 / 2,381; opened 57 / 1,036 / 1,187; written over 35 / 621 / 1,161; dropped as
    duplicate 0 / 33 / 33; closed 0 / 1,000 / 1,152; open at the end 57 / 36 / 34; levels with a
    message 47 / 192 / 216.
  - Pulsed run by type, requests / opened / written over / duplicate: bubble with a prompt 214 / 214 /
    0 / 0; without 1,058 / 306 / 743 / 9; caption without a prompt 59 / 34 / 25 / 0; subtitles 719 /
    326 / 393 / 0; level-link prompt 218 / 194 / 0 / 24; wormhole prompt 113 / 113 / 0 / 0. The caption
    with a prompt and both think forms are never called.
  - Text of bubbles and subtitles that entered the list: from a loaded table 69 / 1,289 / 1,974, other
    0 / 1 / 1, empty 0 / 63 / 66. Actor names 16 / 69 / 218, all tags of the loaded tables. Every
    message has an anchor with a position.
  - Close events 0 / 1,740 / 2,318, of which 740 and 1,166 closed nothing (the message had been written
    over, or was never opened). The list was full once (pulsed run).
  - Things stopped by a fault 4 -> 4 in all three runs; fault kinds 2, levels with a fault 2. Unbound
    natives called 52 -> 51, 88 -> 87, 88 -> 87; recorded-only calls 13,648 -> 13,560, 95,656 -> 92,256,
    102,810 -> 98,170 (the messaging natives left the list). Native calls, trigger events and reaction
    calls as in D264.
- Not modelled: event 2, the player's button (action 0x17 sets a byte and calls a function with a part
  of the Thing unless prevent close is set; action 0x37 not read); the sort order and states; other
  Things' bubbles closed by 0x3e4af0; the subtitle text formatting (0x736f1c); the prompts' text;
  placement; the script's off-screen close (screen natives unbound).
- Tests: 6 unit tests in `messages.rs`, the world test
  `d265_the_messaging_natives_open_and_close_the_worlds_messages`; the D263 recorded-calls test covers
  the four new natives. fmt, the CI clippy line, the CI test set, the Python tests and `cargo check` of
  both viewers pass.
- Not tested: the viewer window; any message against the game in RPCS3.
**+0 newly viewable / +0 newly Complete (runtime feature).**
Next: stage 5 PR 2, the viewer draws the open messages; then global settings, Things, camera.
