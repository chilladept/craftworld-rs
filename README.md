# LBP2 Asset Explorer research foundation

Unofficial, independent fan project, unaffiliated with Media Molecule or Sony.
Prepared with AI assistance. Users supply their own lawful game data; nothing here
includes original game assets or a playable replacement engine.

Local decryption/decompilation research on the user's own copy is permitted when
needed; original inputs stay read-only and research outputs stay private/ignored.
The delivered product contains no original LBP code or assets. Recovered source
must not be copied, adapted or translated into product code. Runtime assets come
from user-owned ISO/game data; direct ISO support remains a later input feature.
Current rules: [AGENTS.md](AGENTS.md).

## What is not in this repository

No game assets are included: no textures, meshes, audio, levels, archives,
executables, keys, saves or emulator captures, and none have ever been committed.
`.gitignore` is default-deny, so only authored source, docs and synthetic test
recipes can be tracked. To use the tools you supply your own disc dump of
LittleBigPlanet 2. Asset names, GUIDs and hashes in the docs are identifiers
only. The authored code is licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option; that license does not cover any
game data. LittleBigPlanet is a trademark of Sony Interactive Entertainment.

Current milestone: **D40 first material-linked textured mesh preview complete**;
B2 metadata explorer supports validated source attachment and hex inspection.
Rust readers inventory SFO, MAP/FARC, resource magic, binary headers and dependencies
with deterministic JSON. Pinned Toolkit oracle parity is verified. Visual previews
support plain 2D GTF and TEX/DDS BC1/BC3/BGRA8 top mips plus ordinary MSH wireframe,
neutral solid and a narrow direct-diffuse textured preview. Start with [STATUS](STATUS.md),
[textured mesh usage and limits](docs/textured-mesh.md) and
[preview usage](docs/texture-preview.md).

D4 establishes a bounded external material-prefix oracle for32samples, with scalar
bits, opaque shader boundaries, texture descriptors and wraps. The independent Rust
prefix reader now matches all32samples (D5). Native Material tab exposes those fields
and texture-slot navigation (D6); [format contract](docs/material-prefix.md) and
[material viewer usage](docs/material-viewer.md).

D7 graph metadata research now corroborates32materials through guarded community
readers; [layout map, conflicts and next independent parser](docs/material-graph.md).

D8 optional independent graph metadata now matches all32oracle samples; native graph
tables remain next. The existing Material tab still uses the verified prefix API.

## Run the metadata explorer

```powershell
. ./scripts/use-local-rust.ps1 # only needed for this workspace's local toolchain
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc
```

Use `--demo` for authored synthetic metadata, or start without arguments and paste
a schema-2 scanner JSON path into the window. Browse archives/MAP rows, filter by
type/status, search paths or exact `guid:8-digit-hex` / `hash:40-digit-hex`, inspect
metadata, and follow outgoing/incoming references. JSON loads in a cancellable
worker; lists are virtualized. Attach an extracted source to validate its SFO/MAP
fingerprints and FARC tables before reading bounded 4 KiB hex pages. Switch to
Archive entries to browse every table entry, including those without MAP aliases.
Original files remain read-only; protected resources are blocked. See
[viewer usage](docs/viewer.md) and [source validation](docs/source-attachment.md).

## Run the asset scanner

With a normal stable Rust installation, run from this project directory:

```powershell
cargo run -p lbp-cli -- --game-dir 'path/to/disc-root' --json local-analysis/inventory-new.json
```

Input can be a disc directory containing `PS3_GAME`, or `PS3_GAME` itself. It must
contain `PARAM.SFO`, `USRDIR/OUTPUT/BLURAYGUIDS.MAP` and at least one `.FARC` under
`USRDIR`. The output parent must exist, be outside the selected game directory,
and the filename must be new. Use `--help` for limits. Original files are opened
read-only; nested symbolic links and Windows reparse points are skipped.
To read an installed update over the disc, pass both directories as one path list,
base first, e.g. `--game-dir 'game-data/a1-disc;game-data/a1-update-0133'` (Windows
`;`). The update's files and `USRDIR/output/brg_patch.map` rows take precedence; see
`docs/update-overlay.md`. Viewers and census tools accept the same list.
The default scan inspects bounded metadata; add `--tables-only` for A1 table reads.
Schema version 2 includes per-location inspections and dependency catalog links.
Exit 2 preserves a partial JSON report for malformed/I/O/budget diagnostics;
unknown, unsupported, encrypted and missing resources remain explicit statistics.

This workspace has an ignored local Rust installation and three selectively copied,
read-only scanner inputs. Activate the toolchain for this shell and scan them:

```powershell
. ./scripts/use-local-rust.ps1
cargo run -p lbp-cli -- --game-dir game-data/a1-disc --json local-analysis/inventory-new.json
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

The minimal directory is not a complete extracted disc. Missing loose candidates
refer to this selected directory only. SHA-1s are index identifiers; payload hashes
are not verified. Extension hints are separate from confirmed resource types.
See [metadata contract and oracle](docs/resource-metadata.md) for scope and reproduction.
`scripts/prepare_scanner_data.py` prepares only the three already readable inputs
from the known local ZIP, refuses an existing destination and verifies metadata
fingerprints. It does not decrypt protected data or extract the whole image.

## Verified

- Readable ISO9660 tree inside the local ZIP, PS3 BCUS98245, APP_VER 01.00.
- MAP revision 0x100: 36,854 rows; FARC: 31,534 entries.
- Bounded resource-header/dependency samples; local PS3 GTF texture headers.
- No whole-disc extraction, game modification, decryption, or executable analysis.

## Run the reconnaissance probes

Python 3.11+; first two commands need only the standard library. The resource probe's
optional TEX oracle needs Pillow and the ignored `lbptoolspy-lite` reference checkout.
Run from this project directory in PowerShell:

```powershell
python scripts/probe_disc.py 'LittleBigPlanet 2 (USA) (EnFrEsPt).zip' --prefix-mib 40
python scripts/probe_lbp_tables.py 'LittleBigPlanet 2 (USA) (EnFrEsPt).zip' --output local-analysis/lbp-tables-new.json
python scripts/probe_resource_samples.py 'LittleBigPlanet 2 (USA) (EnFrEsPt).zip' --output local-analysis/resource-samples-new.json
python -m unittest discover -s tests -v
```

The latter two probes currently use the fixed local reconnaissance inventory/table
filenames described in STATUS. They reject existing output filenames. They are
research helpers, not a general-purpose scanner. The disc probe emits metadata to
stdout; redirect only to `local-analysis/`. Inventory records are directory records,
so multi-extent logical files require consolidation before file-size totals.

## Planned

[Architecture](docs/architecture.md): Rust CLI and UI-independent libraries first.
[Roadmap](docs/roadmap.md): scanner, explorer, textures, then meshes.
[Formats](docs/formats.md): evidence, implementation rules, and unknowns.
[Community projects](docs/community-projects.md): ranked references and reuse limits.
[Local inventory](docs/local-data.md): exact input and container facts.

This project's own authored source is licensed MIT OR Apache-2.0. No third-party
source is vendored into the implementation. See [notices](THIRD-PARTY-NOTICES.md).
## Texture research (C1 complete)

The bounded census found 13,624 clear textures. A pinned Toolkit oracle decodes
22 representative top mips with matching metadata. C2 adds CPU BC1 pixels verified
on 19 local samples and native preview. [Texture evidence](docs/textures.md) records reproduction,
format counts, licensing provenance, unknown mip tails and synthetic recipes.
C3 adds 15 TEX samples with pixel/DDS field parity; see [C3 contract](docs/tex-preview.md).
C4 adds 35 BC3 samples with pixel/metadata parity; see [BC3 contract](docs/bc3-preview.md).
D1 adds verified bounded [mesh prefix diagnostics](docs/mesh-prefix.md).
D2 adds a verified native [mesh wireframe](docs/mesh-geometry.md), with Mesh tab,
orbit/zoom and stored-geometry metadata. No materials, skinning or animation applied.
D3 adds [material reference navigation and header census](docs/material-navigation.md).
Material payload/shading, posed animation and levels remain future extensions.

D9 adds optional [material graph diagnostics](docs/material-graph.md) in the Material
tab: bounded box/wire metadata, embedded-animation summaries and texture-slot
candidate navigation. Prefix inspection remains available if graph inspection
fails. No material/shader evaluation or animation playback. Next is a bounded
community-reader investigation of the remaining material tail.

D10 [material tail research](docs/material-tail.md) accounts for the remaining bytes
in32selected materials through the pinned community reader. Sound and parameter
animation metadata are documented; behavior remains unknown. Next is a small
optional Rust tail reader with oracle parity, followed by GUI integration.

D11 adds optional independent material tail diagnostics with32sample oracle parity,
including unchanged prefix/graph metadata. Raw sound and parameter-animation
summaries are available to parser callers; GUI integration is the next bounded
milestone. See [tail evidence and verification](docs/material-tail.md).

D12 adds explicit native material tail inspection, raw sound and parameter-animation
summaries, preserving prefix/graph metadata if tail inspection fails. Verified empty
and both populated sampled arrays. Next is a bounded material metadata coverage
census using existing readers. See [verification and scope](docs/material-tail.md).

D13 [bounded material coverage census](docs/material-census.md) successfully inspects
5,163unique materials in the available snapshot. This measures parser acceptance;
40additional representative oracle samples are selected for independent validation.
No sound/animation behavior or full-game compatibility claim.

D14 validates40additional material samples against the pinned community oracle;
complete prefix/graph/tail fields match. Combined72samples cover the observed
revision/sound/array-count cohorts. Remaining graph-shape and component-array
boundary samples are the next bounded validation task; see material-census.md.


D15 validates the remaining count-shape and largest animation-array samples.
Combined 120 samples cover all 76 observed box/wire count pairs and the 1,143-value
boundary. Next native diagnostic graph diagram; shader/animation behavior remains
unknown. See [coverage evidence](docs/material-census.md).


D16 adds a [native material graph diagram](docs/material-diagram.md) with linked
selection, pan/zoom/Fit and explicit invalid endpoints. Raw shader graph metadata
is visualized without evaluating it. Next: private cached diagnostic JSON export.


D17 adds [cached material diagnostic JSON export](docs/material-export.md): choose a
new .json file under ignored local-analysis in the Material inspector. Cached stage
metadata is exported without rereading assets. Next: cached texture diagnostics.


D18 adds [cached texture diagnostic JSON export](docs/texture-export.md) beside the
preview. It exports header/count metadata, excludes pixels, and shares Material's
private new-file safeguards. Next: cached mesh metadata summaries.


D19 adds [cached mesh diagnostic summaries](docs/mesh-export.md), including topology,
counts, bounds and material references while excluding geometry arrays. Next:
bounded Back/Forward navigation across resources, archive entries and overview.

D20 adds [bounded Back/Forward navigation](docs/navigation.md) across MAP rows,
archive entries and Overview, restoring the inspection tab and making the restored
row visible. History resets on snapshot replacement. Next: keyboard route shortcuts.

D21 adds [keyboard history shortcuts](docs/navigation-shortcuts.md): Alt+Left and
Alt+Right, with visible help. Ordinary editing arrows remain available to fields.
Next: independently verified32-bit TEX/DDS top-mip previews.

D22 adds [standard-mask BGRA8 TEX/DDS previews](docs/bgra-preview.md), checked
against the pinned reader. Existing channels, cached metadata export and bounds
apply. Next: evidence-based animation metadata inspection.


D23 adds [animation metadata evidence and an external oracle](docs/animation-prefix.md).
Modern scalar/count fields are corroborated before implementation; old layouts and
locator/playback work remain explicit boundaries. Next: independent prefix reader.


D24 adds the [independent animation metadata parser](docs/animation-parser.md),
matching33pinned-reader prefixes. Raw fields and opaque data lengths are inspectable
by the diagnostic example; native Animation inspector is next. No playback yet.


D25 adds a [native Animation metadata tab](docs/animation-inspector.md) for MAP rows
and archive entries. It displays raw prefix fields/counts with bounded reads and
explicit unsupported layouts. No playback. Next: cached Animation JSON diagnostics.


D26: Animation tab can save cached metadata-only JSON under ignored local-analysis;
raw prefix fields and unsupported outcomes, no animation payload or extra reads.
See [export contract](docs/animation-export.md). Full suite126tests and native
oracle/schema/repeat checks pass. Next bounded level/plan reconnaissance.


D29 adds independently verified CPU plan-prefix diagnostics (raw inner revision,
opaque Thing-data digest/bounds and unparsed inventorytail).48externaloracle records
match;130Rust tests pass. [Contract](docs/plan-parser.md). Native Plan tab next.


D30: Plan tab browses verified rawprefix/hash/bounds/tail diagnostics on MAPresources
and archiveentries.134Rust tests and nativeoraclechecks pass; oldlayouts explicit
Unsupported. [Inspector](docs/plan-inspector.md). Cached Plan JSON export next.


D31: Plan inspector saves cachedmetadata-only JSON under ignoredlocal-analysis;
rawinner/hash/bounds/tail and explicit unsupportedoutcomes, no payloadornewreads.
[Export contract](docs/plan-export.md).136Rusttests/nativeoracle/schema/repeat pass.


D33 adds independentlyverified optionalCPUplan inventoryscalar diagnostics. Encoded
64bitdate/raw32fields andextentcounts, no recursivemetadata orsemanticnormalization.
140tests/48oraclematches; [contract](docs/plan-inventory-parser.md). Nativeoptionnext.


D34: Plan tab optionally inspects raw inventory scalar metadata, preserving the
base prefix on deeper failure. 143 Rust tests and native oracle checks pass.
Version1 export remains base-prefix-only; stage-aware version2 export is next.
[Inspector and limits](docs/plan-inventory-inspector.md).


D35: Plan JSON export now uses version2 with separate base and optional inventory
stages, including partial failures. Historical version1 schema retained.145Rusttests
and nativeoracle/schema/repeat checks pass. [Contract](docs/plan-export-v2.md).
Next: stable resource-list sorting for easier browsing.


D36: MAP resource lists now sort by original MAP row, path, GUID or SHA1, preserving
aliases and selection. CLI: `--sort map|path|guid|hash`.147tests and five snapshot-only
native checks pass. [Ordering contract](docs/resource-sorting.md).


D37: Archive entries sort by table order, offset, size or SHA1, retaining orphans and
aliases. CLI: `--entries --entry-sort table|offset|size|hash`.149tests and five native
checks pass. [Ordering contract](docs/archive-sorting.md). Next folder-prefix browsing.


D38: Browse MAP folders with child counts and Root/Up breadcrumbs, combined with
search/sorting. CLI: `--folder gamedata`. Metadata only;152tests and four native checks
pass. [Contract](docs/folder-browsing.md). Next optional neutral solid mesh preview.


D39: Mesh tab offers optional Neutral solid surfaces alongside Wireframe.
CLI: `--mesh-view solid`. Generatedtwo-sided flatshading and approximate averagedepth
order;154tests/nativechecks pass, metadataunchanged. [Limits](docs/mesh-surface.md).
Next focus-preview/axis viewing controls.
