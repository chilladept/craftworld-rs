# craftworld-rs

An independent, experimental asset explorer and runtime for *LittleBigPlanet 2*,
written in Rust with [Bevy](https://bevyengine.org). It reads **your own** game
copy, browses its resources, previews them, loads levels, and can run a playable
Sackboy and the game's scripts on a reimplemented runtime.

This is an unofficial fan project, unaffiliated with Media Molecule or Sony.
Despite the similar name, it is also not part of or affiliated with
[Craftworld Toolkit / cwlib](https://github.com/ennuo/toolkit), the community's
long-standing LittleBigPlanet modding toolkit, whose format work this project
gratefully builds on as prior art. It was prepared with AI assistance.

> **Status: experimental research software.** Many formats are only partly
> understood, rendering and physics are approximations, and nothing here is a
> complete or compatible replacement for the game. Expect gaps and breaking changes.

## What it does today

- **Asset scanner** (`lbp-cli`): indexes a game directory (disc plus an optional
  installed update) into deterministic JSON: archives, MAP rows, resource headers
  and dependencies.
- **Asset explorer** (`lbp-bevy-viewer`, plus a legacy `lbp-viewer`): browse and
  search resources by path, GUID or hash, follow references, view hex, and preview
  textures (DDS/GTF BC1/BC3/BGRA8) and meshes with their materials and animations.
- **Level viewing**: loads level plans and draws their things, shapes, decorations,
  backdrops and lighting.
- **Playable Sackboy**: `--play-sackboy` puts a controllable Sackboy with his own
  animations, moods and a Popit overlay into a loaded level.
  Movement is measured against the original running under an emulator, and is close
  but not exact.
- **RScript VM** (`lbp-script-vm`, `lbp-runtime`, `lbp-sim`): an interpreter for the
  game's compiled scripts, with a native-function host and a small physics simulation.

Not done: complete material and shader evaluation, full level scripting, audio,
online features, save games, and a standalone distribution. See
[STATUS-CURRENT.md](STATUS-CURRENT.md) for the exact current state and open blockers.

## You must bring your own game data

No game assets are included or ever committed: no textures, meshes, audio, levels,
archives, executables, keys, saves or emulator captures. `.gitignore` is default-deny.
You need your own lawful dump of LittleBigPlanet 2 (developed against the US disc,
BCUS98245, with update 01.33), extracted to a directory containing `PS3_GAME`.
Protected or encrypted resources are not decrypted by this project.

## Build and run

Requires a recent stable Rust toolchain (CI pins the version in
`.github/workflows/ci.yml`). Windows is the primary development platform.

```sh
cargo build --release -p lbp-cli -p lbp-bevy-viewer

# 1. Scan your game directory into an inventory (output must be a new file
#    outside the game directory; add the update directory after a ';' to overlay it)
cargo run --release -p lbp-cli -- --game-dir path/to/disc-root --json inventory.json

# 2. Explore it
cargo run --release -p lbp-bevy-viewer -- --snapshot inventory.json --game-dir path/to/disc-root
```

Useful viewer options: `--select-guid HEX` to open a resource, `--level-guid HEX`
to load a level, `--play-sackboy` to play in it, `--help` for the full list.
Controls: left-drag orbit, right/middle-drag pan, wheel zoom, `R` reset, `F` frame.
More detail: [viewer usage](docs/viewer.md), [Bevy preview](docs/bevy-preview.md),
[update overlay](docs/update-overlay.md).

Checks used by CI: `cargo fmt --all --check`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`. To contribute, see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Layout

| Path | Contents |
|---|---|
| `crates/lbp-core`, `lbp-io`, `lbp-archive`, `lbp-resource` | readers: containers, archives, resource headers |
| `crates/lbp-texture`, `lbp-mesh`, `lbp-preview`, `lbp-level` | texture, mesh, material, animation and level parsing and preparation |
| `crates/lbp-script-vm`, `lbp-runtime`, `lbp-sim`, `lbp-ui-script`, `lbp-gooey` | script VM, gameplay runtime, simulation, UI scripts |
| `crates/lbp-cli`, `lbp-catalog`, `lbp-viewer`, `lbp-bevy-viewer` | scanner, catalog, and the two viewers |
| `docs/` | format evidence and per-milestone notes ([architecture](docs/architecture.md), [formats](docs/formats.md), [roadmap](docs/roadmap.md)) |

Parsing crates are independent of any UI or renderer. Earlier research milestones
are logged in [docs/readme-history.md](docs/readme-history.md).

## Legal

- The authored source in this repository is licensed under either of
  [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. That license
  does not cover any game data.
- The implementation is written from independent analysis and published community
  format documentation. Third-party tools are used only as external references
  (see [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) and
  [community projects](docs/community-projects.md)); no recovered or third-party
  game code is copied into the product.
- LittleBigPlanet is a trademark of Sony Interactive Entertainment. This project
  is not endorsed by or affiliated with Sony or Media Molecule.
- Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md);
  never submit game assets or decompiled code.
