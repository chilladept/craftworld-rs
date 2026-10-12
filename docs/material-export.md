# D17 cached material diagnostic JSON export

Completed 2026-10-05 (America/Chicago). The Material inspector has an explicit
"Export cached diagnostic JSON" section with a destination field and Save button.
Exports use the already loaded prefix, graph and tail metadata. Saving never
calls the source reader, parser or renderer. No payload/shader bytes, animation
keys, image data, absolute paths or timestamps are exported.

## Envelope and stage semantics

Version 1 envelope: `schemas/material-diagnostic.schema.json`. Scope:
`cached_material_diagnostic_metadata_not_asset_export`. It includes the selected
relative source path/offset/size, last request outcome and three stage records.
Each stage includes availability, current-mode request flag, stable error category
and metadata or null. Available cached earlier stages survive later errors and
cancellation. A stage can remain available after switching to prefix mode; its
current-mode request flag then becomes false. Failed/unrequested/not-available
stages are distinct. Null does not mean empty. Busy, unselected and uncompleted
inspections cannot export. Protected/unsupported failures remain explicit.

The export describes a prior validated read. It explicitly says
`validated_at_read_not_rechecked_for_export`; it does not assert current source
stability or perform additional reads. GUI failure messages remain available, but
the JSON stores stable categories rather than arbitrary OS strings/host paths.
The schema validates the envelope/stage state, not every resource format field.

The writer accepts an existing parent under ignored workspace local-analysis,
canonicalizes containment, rejects a redirected local-analysis root, requires
`.json`, and refuses existing files. Exclusive create also prevents a raced
existing leaf file/link from being overwritten. Serialization has an 8 MiB byte
budget before opening output. A later disk-write failure may leave a partial file;
the error is reported and future writes still refuse overwrite. No claim of
protection against an adversarial concurrent replacement of directory ancestors.

## Provenance

No new format rule. Existing independent prefix/graph/tail API structures are
serialized without transformation. CONFIRMED_FROM_COMMUNITY_CODE rules retain
Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`, per-field sources and head
920-951 branch-zero applicability in material-prefix.md, material-graph.md and
material-tail.md. CONFIRMED_FROM_LOCAL_DATA: native export of GUID 00017049,
cosmos_racing_game_top_02.gmat (head 949, 8 boxes/10 wires, raw sound 0, one
241-component-value parameter animation), matches all existing pinned oracle
prefix/graph/tail fields, compression flags and expanded size. SHA1 identifier
`962947a887b0f53b5528a17fbff2cc1cdac219ed` is checked against the original private
sample manifest's exact relative location. Sound, shader and animation semantics
remain UNKNOWN. This validates cached export equality, not fresh full-corpus parsing.

## Verification and reproduction

Four new authored Rust cases cover stage availability/current mode/fallback,
cached cancellation and busy/reset refusal; full tail field preservation independent
of selected UI rows; private containment, traversal/extensions/missing parents,
exclusive overwrite refusal and repeated byte equality; output-budget boundary and
host-path exclusion. All 99 workspace tests (25 viewer tests), fmt, Clippy and
native build passed. Log: ignored local-analysis/d17-workspace-tests.log.

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
$qaProcess = Start-Process -FilePath target/debug/lbp-viewer.exe -ArgumentList @('--snapshot','local-analysis/a2-inventory-verified.json','--game-dir','game-data/a1-disc','--select-guid','00017049','--qa-tab','material-tail','--qa-material-json','local-analysis/d17-material-diagnostic.json','--qa-capture','local-analysis/d17-material-export.png') -WindowStyle Hidden -PassThru -RedirectStandardOutput local-analysis/d17-material-export.stdout.log -RedirectStandardError local-analysis/d17-material-export.stderr.log
$qaProcess.WaitForExit(30000)
```

Repeated native run used new `-repeat` JSON/image/log paths; both runs exited zero.
The QA flag calls the same cached export function as the GUI button, then captures
the resulting status. RTX3080/Vulkan screenshot inspected: success and existing
tail metadata fit the pane. Repeated JSON is byte-identical: 19,153 bytes, SHA256
`e1cfe4f258d0f31626f3f56381ff79f2b0a0c2c8d8fc538bf16daccf5a398016`.

Bundled Python lacked jsonschema. Installed validator `jsonschema==4.23.0` only
under ignored local-analysis/schema-tools; product dependencies are unchanged:

```powershell
$pythonRuntime = 'C:/Users/<you>/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
& $pythonRuntime -m pip install --target local-analysis/schema-tools 'jsonschema==4.23.0'
```

With that directory on Python sys.path, Draft202012Validator.check_schema and
validation of the real report passed. Four mutated envelopes (wrong version,
available/null metadata, unknown stage status, extra asset_bytes field) were
rejected. Independent full-field oracle comparison passed. No failed format
experiment or weakened guard. No manual destination editing/Save click, native
error-dialog interaction, symlink-race experiment, other GPU/platform or full-game
export tested. Synthetic refusal and shared native export entry point are the
specific evidence. Python source/tests unchanged; their prior 27-test suite was
not repeated. Original SFO/MAP hashes, FARC size/ReadOnly, clean pinned reference,
root outside Git and default-deny audit remain unchanged. No commits/publication,
license decision or approval-review rejection.

## Next useful task: D18 cached texture diagnostic export

Inspect the actual Texture preview cache and supported metadata shapes. Extend
the same private-output safeguards to a texture metadata export without pixel
arrays or asset bytes, preserving unsupported and unrequested outcomes. Factor
the shared output guard/writer when it gains this second consumer, keeping resource
parsers independent of UI. Use a versioned texture envelope, synthetic stage/output
checks and a native exported sample compared to the existing pinned texture oracle.
No new texture codec, bulk extraction or shader/material rendering required.
