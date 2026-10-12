# D27 level/plan reconnaissance

2026-10-06 America/Chicago. Snapshot-only census; no original game payload reads,
new parser, executable investigation, Thing deserialization or rendering this pass.
Build applicability: user's BCUS98245 01.00, existing schema2 A2 verified inventory.
Counts describe the selectively accessible DATA.FARC corpus, not every disc archive.

## Local evidence

CONFIRMED_FROM_LOCAL_DATA via existing inventory/header probes (payload not parsed):

| Observation | Levels | Plans |
|---|---:|---:|
| Confirmed binary locations | 282 | 7,342 |
| Stored bytes | 44,574,615 | 10,428,191 |
| Header cohorts | 45 | 97 |
| Distinct head revisions | 43 (350..951) | 94 (415..951) |
| Parsed dependency entries, including repetitions | 45,946 | 47,672 |
| Matching extension MAP aliases | 282 `.bin` | 7,342 `.plan` |
| Unique hashes among those aliases | 282 | 7,342 |

All confirmed samples have parsed header dependency tables and compressed wrappers.
Both sets contain flags0/7 and branch0/0x4c44. Level branch revisions include0/4/22/23;
plan branch revisions0/20/22/23. These are observed variants, not supported payload
claims. Dependency type histograms and every header cohort are in ignored census.
Extension counts are computed independently from confirmed-type counts. `.bin`
could identify other types; matching counts alone do not establish a general rule.
Dependencies are resource-table entries, not recovered relationships inside Things.

## Pinned reader map

CONFIRMED_FROM_COMMUNITY_CODE at Toolkit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`; local reference checkout verified clean.
These source links identify inspected local files; web fetch of the same pinned
pages returned cache-miss, so local checkout is the actual evidence for this pass.
No code copied, adapted or translated. Independent specification/oracle route only.

| Rule / source | Applicability and conflict | Coverage now |
|---|---|---|
| [RLevel::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RLevel.java) reads cross-play hash array only when head>0x3e6, then worldThing reference | Local heads<=951, so hash-array gate absent. Annotation says min0x3e6 while runtime uses strict greater; actual read path governs any future experiment | Community source and local header ranges; no local payload read |
| [Serializer::reference](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/serializer/Serializer.java) reads integer tag; zero=null; seen tag returns prior object; new tag recursively reads inline structure | Cannot treat nonzero as a flat external resource identifier, object count or byte offset | Community source only; no independent Thing implementation |
| [Thing::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/things/Thing.java) revision marker, UID, parent/group/emitter and parts vary by head/branch | Full level parsing requires recursive object/reference state and part readers; depth/count/cycle/allocation budgets needed before an oracle call | Reader study only; no object semantics inferred |
| [RPlan::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RPlan.java) streaming bool at subVersion>=0xcc, inner revision via input.i32, then serializer.bytearray(thingData) | Branch0 implies subVersion0 in proposed scope; inner revision is ignored by reader and resource revision governs Things | Community source; local inner value UNKNOWN |
| [MemoryInputStream::i32/bytearray](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java), [Serializer::bytearray](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/serializer/Serializer.java) length uses flag-controlled BE32/ULEB, followed by exact array bytes | Prefix byte-array bounds are independently checkable; opaque bytes are not parsed Things | Community source; existing shared integer tests support primitive mechanism, not plan compatibility |
| [Revisions](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/Revisions.java) PLAN_DETAILS0x197; [InventoryItemDetails::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/inventory/InventoryItemDetails.java) head>0x37c modern layout then recursive metadata references | Inventory details follow opaque thing blob, not precede it. Modern date/slot/guid/type/title/description fields may offer a later bounded slice; layouts/allocations require preflight | Community source only; local fields UNKNOWN |

STRONG_INFERENCE: a bounded modern branch-zero plan prefix is a useful first payload
slice: record stored inner revision, opaque Thing byte-array length/start/hash and
remaining inventory-tail length, without recursively interpreting world objects.
An optional next slice could include well-understood inventory scalar fields once
slot/reference/version gates have an oracle. No names/icon/title interpretation yet.

UNKNOWN: local inner revision values, exact Thing array counts/types/parts, inventory
scalar metadata, legacy branch variants, all payload validity, complete level reader
agreement and spatial/visual fidelity. No executable investigation justified: the
community reader already documents this structure.

## Census and bounded samples

Authored `scripts/level_plan_report.py` reads only existing JSON. Deterministic
ordering, confirmed magic/type separated from extension aliases, explicit dependency
statuses and counts. Sample selection is a future oracle input, not a parse result:
clear PLNb ordinary branch0/sub0, head893..951, flags0/7, stored18B..2MiB, DATA.FARC.
One minimum per eligible cohort ordered by prevalence, then maxima up to48samples /
64MiB total. All observed28eligible cohorts represented by48samples /313,643storedB;
no omitted eligible cohort, but budget means not every maximum is represented.
Older/branch variants remain counted in census and outside proposed oracle scope.

```powershell
& 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -m unittest discover -s tests -p test_level_plan_report.py
& 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' scripts/level_plan_report.py local-analysis/a2-inventory-verified.json local-analysis/new-level-plan-census.json local-analysis/new-plan-samples.json
```

Four authored synthetic tests pass: alias/type/dependency separation, input-order
invariance, old/branch/budget/wrong-magic exclusions, count/total budget with explicit
omitted cohorts, private JSON/new-only writer. Repeat census49,366B and manifest7,946B
byte-identical. Initial non-escalated private-root canonicalization hit WinError5;
scoped authorized execution succeeds. No new Rust changes/check claim this pass;
D26 build and126tests remain last verified Rust state.

Outputs: ignored `local-analysis/d27-level-plan-census.json`, `d27-plan-samples.json`
and repeat files. Authored script/test/docs pass default-deny allowlist check.
No original file opened by this census, no extraction or asset fixture generated.

## D28 recommended task

Build an authored bounded external plan-prefix oracle adapter using pinned
MemoryInputStream::i32/bytearray and existing guarded decompression helpers. Preflight
header/revision/branch, dependency counts/ranges, 2MiB stored/16MiB expanded/64MiB run,
48samples, cancellation/time limits, exact SHA1 and wrapper agreement before handing
bytes to community constructors. Nonempty dependency tables are normal here; do not
reuse Animation's zero-dependency gate or silently discard them.

Project only prefix values, opaque-byte hash and tail lengths. Full RPlan reader
should run only for safely preflighted inventory structures or authored synthetic
fixtures; RPlan's opaque thingData does not itself deserialize Things. Full RLevel
reader is deferred because worldThing immediately starts recursive inline parsing.
Compare repeats and authored malformed/flags/revision recipes. Then implement the
independent CPU plan reader only after local oracle evidence is recorded. No blob
export, inventory edits, full Thing parser, rendering, gameplay or protected access.
