# D133: patch.edat decrypted in RPCS3; U10, G10 and script-VM checks on real scripts

Date: 2026-10-08 (America/Chicago). Scope: the user's LBP2 (BCUS98245) with update 01.33.

## Question

Compiled scripts (`.ff`, magic `FSHb`) were not readable on 01.33: the D126 census found
0 scripts. U0 found 427 `.ff` MAP rows whose bytes are not in any clear FARC. The guess
was the encrypted `USRDIR/patches/patch.edat`. G10 (native-call census) and the D128
real-script checks were blocked on those bytes.

## Method and provenance

Approved research method (AGENTS.md: RPCS3 measurement and local decryption, 2026-10-08).
All derived files are under ignored `local-analysis/`; nothing derived is committed, and
no key is written into the repository.

1. RPCS3 v0.0.43-20247-222754bf (user-installed, `%USERPROFILE%\Emulators\rpcs3`),
   firmware 4.93 installed with `--installfw` (dev_flash `version.txt` release 04.9300).
2. The 32 official update packages (A0101..A0133) installed with `--installpkg` into
   RPCS3's own `dev_hdd0/game/BCUS98245` (PARAM.SFO APP_VER 01.33).
3. The extracted disc folder does not boot ("Failed to mount disc directory", also with
   `/dev_bdvd` mapped in vfs.yml). The user's ISO, extracted as a copy from their ZIP to
   `%USERPROFILE%\Emulators\lbp2-iso\` and marked read-only, boots (`--game-screen 1`,
   second monitor). The ZIP is untouched.
4. While the game runs, its NPDRM check for `patches/patch.edat` makes RPCS3 hold the
   game key. RPCS3's own *Utilities -> Decrypt PS3 Binaries* then decrypts a copy:
   `local-analysis/rpcs3-decrypt-0133/patch.edat` -> `patch.edat.unedat` (1,971,662 bytes).
   The headless `--decrypt` route fails without the running game ("Invalid klicensee").
5. The output is a FARC: 522 entries, 426 with magic `FSHb`. A copy is placed as
   `local-analysis/d133-edat-overlay/USRDIR/patches/patch_edat.farc` so the existing
   scanner sees it as a third overlay root.

Evidence label: CONFIRMED_FROM_LOCAL_DATA (the user's own copy, decrypted by RPCS3).

## Results

Unit: distinct content hash. Scope: the 426 `FSHb` entries of the decrypted 01.33
patch.edat (disc FARCs contain no `FSHb`).

- Inventory: `lbp-cli --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --json local-analysis/d133-inventory-0133-edat.json`:
  34 archives (was 33), partial false.
- U10 census (`script_census` with that snapshot and those roots ->
  `d133-script-census-0133.json`, listings `d133-script-listings-0133/`): 426/426 complete;
  6,518 functions (2,665 native), 160,539 instructions; unknown opcodes 0. Heads: 0x3e6 253,
  0x3f6 74, 0x3f8 36, 0x3ee 25, 0x3e7 20, others small; 0x37c 2. Layouts: i32 ranges 2,
  u16 ranges 424. UI-named classes 61.
- Listing checks: constant, member and function-range indexes 0 out of range;
  **name_index_out_of_range 1,966**, all on `type` rows with index 0xFFFFFFFF (424 of 426
  scripts): unnamed builtin types (void, bool, s32, f32, v4, m44, s64) and derived types.
  0xFFFFFFFF is a "no name" sentinel, not an overrun; the U10 check should accept it.
  Field, property, function and reference rows: 0 misses.
- Gates: no script has a head in 0x33b..0x360 or 0x30b. Both 0x37c scripts read with the
  i32 layout.
- Toolkit spot check (`scripts/run-script-oracle.ps1`, Toolkit d6a2ef80, whole RScript
  load; `d133-toolkit-spot/`): 3/3 equal on class, superclass GUID, function count and
  instruction count (a UI pod script 15/316, a small trigger 7/22, the largest 31/7,310).
- G10 native-call census (local script over the census and listings ->
  `d133-g10-native-calls-0133.json`): 23,350 call instructions, 15,931 to functions declared
  native, 397 virtual/unnamed. 1,964 distinct native functions called of 2,538 declared.
  Top: translation lookup (1,206 calls, 142 scripts), then Gooey frame layout calls
  (SetFrameSizing 831, EndFrame 688, SetFrameLayout 412, SetFrameBorders 341, DoBreak 332,
  DoText 321), LoadTextureByFilename 543, GetThingUID 496. Names are matched by mangled
  name only; a name declared in two classes counts once.
- D128 `check()` (`lbp-script-vm` example `script_check` over
  `local-analysis/d133-scripts-0133/` -> `d133-script-check-0133.json`): 426/426 read;
  15,866 branches; in range from the current instruction 15,866, from the next 15,399.
  All 426 scripts are fully in range under "from current", 221 under "from next". The VM
  default (`branch_from_current: true`) is confirmed. Unknown opcodes 0; functions
  without a terminal 0.

Counts: +0 newly viewable, +0 newly Complete (scripts are not assets).

## Not tested

- script-vm.md PC check 2 (member functions' first argument typed as their own class;
  `ARG` addresses equal to callee argument offsets).
- Scripts referenced only by story levels vs all patch scripts: G10 covers all 426.
- Executing any real script in the VM.

## X1/S9 measurements: need a human at the controls

docs/rpcs3-measurement.md section 3 needs Sackboy moved in a level (walk 3+ s, standing
jump, long drop, layer shift) while positions are sampled each frame. Claude's computer
control cannot target RPCS3 (not an installed app it recognises), so these need the user
to play while the PC session records: start a flat story level, then on cue walk right
3 s, jump once on flat ground, drop from a ledge, and shift layer once. Sampling needs a
memory scanner or RPCS3's debugger to find Sackboy's position (big-endian floats).

## Files

Committed: `scripts/ScriptOracle.java`, `scripts/run-script-oracle.ps1`,
`crates/lbp-script-vm/examples/script_check.rs` (generic tools, no game data).
