# Lane state: friends' launcher (2026-10-19)

## Read first
1. docs/d600-launcher.md (D600, D601)
2. crates/lbp-bevy-viewer/src/launcher.rs (MODES, disc_input, archive_problem, start_setup)
3. README.md "Play it" and "If something goes wrong"

## State
- PR 139 (D600) merged; Story button on (PR 153).
- D601 shipped: PR 195 merged as 9ce8441 (plain-words setup errors, disc folder accepted,
  FARC-tag check for encrypted dumps, "didn't load" window in game modes, README guide).
- Friends zip rebuilt from 9ce8441: craftworld-rs-preview-2026-10-19.zip (40.5 MB).
- GitHub release posted 2026-10-19 on the owner's word: tag `preview-2026-10-19`
  (pre-release, target 9ce8441, zip attached):
  https://github.com/chilladept/craftworld-rs/releases/tag/preview-2026-10-19
  `gh release create --target` needs the full commit SHA (a short one gives HTTP 422).
- Decision numbers D600-D699; next free D602.

## Facts
- Owner rule (2026-10-18, standing): the app needs only the user's fully updated USA ISO
  (BCUS98245) plus the 01.33 update RPCS3 installed. Never require a decrypted copy, never
  add keys; encrypted-only data (patch.edat scripts) gets built-in fallbacks.
- Launcher: no args or `--launcher` opens it; `--launcher-setup ISO UPDATE` is headless.
  Data in %LOCALAPPDATA%/craftworld-rs (disc/ copy 1.2 GB, inventory.json, launcher.txt).
  A disc folder (holding PS3_GAME) is read in place, not copied.
- Every README error-table row matches a launcher/viewer message; change both together.
- Zip build: `cargo build --release -p lbp-bevy-viewer` with
  CARGO_PROFILE_RELEASE_STRIP=true; zip with Windows `tar.exe -a` (forward-slash entries,
  not PS 5.1 Compress-Archive). Contents: one folder holding lbp-bevy-viewer.exe (the name
  the README uses), LICENSE-MIT, LICENSE-APACHE, THIRD-PARTY-NOTICES.md, README.txt. Lives
  in ignored local-analysis/friends-build/; never committed.
- 2026-10-19 smoke test: `--launcher-setup` with the real ISO + update ended "Ready in 17s";
  the update folder given as the ISO gives the D601 "goes in the second box" message, exit 1.
  The exe has no user paths or email strings.

## Next
1. On the PC: run Set up from the GUI against the real ISO once (headless already passes).
