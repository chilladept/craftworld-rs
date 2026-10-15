# Contributing

Thanks for your interest. This is an experimental fan project, so start small:
a focused fix, a format finding with evidence, or a test is easier to review than
a large rewrite. If you plan something bigger, open an issue first so we can agree
on the approach.

## The one hard rule: no game data, no decompiled code

The repository contains only authored source, docs and synthetic test recipes.
Never commit, attach or paste into a PR:

- game assets of any kind: textures, meshes, audio, levels, plans, archives
  (`.farc`, `.map`), executables, keys, saves or byte dumps from the game;
- screenshots, RPCS3 frames, RSX captures or videos of the game;
- decompiler or disassembler output, or code translated from it;
- binaries or bundled files from other community projects.

`.gitignore` is default-deny, and you should never `git add -f` around it. If a
test needs game bytes, it must read them from the contributor's own game
directory at run time and skip cleanly when that directory is absent. Format
findings go in the docs as your own description, with the evidence labels from
[docs/formats.md](docs/formats.md). [AGENTS.md](AGENTS.md) has the full project
rules.

## You need your own copy of the game

Nothing in the repo lets you run the explorer without game data. You need your
own lawful dump of LittleBigPlanet 2, extracted to a directory containing
`PS3_GAME` (development uses the US disc, BCUS98245, with update 01.33). Most
unit tests use synthetic data and run without it; tests that need real data skip
themselves.

## Building

Install a stable Rust toolchain. CI pins the exact version in
[.github/workflows/ci.yml](.github/workflows/ci.yml); using the same one avoids
surprise Clippy lints.

```sh
cargo build --release -p lbp-cli -p lbp-bevy-viewer
```

The workspace builds with plain `cargo` on Linux and macOS as well as Windows.
CI runs on Linux; macOS is not covered by CI, so please mention it in your PR if
you built there. On Linux the viewers need the usual Bevy system libraries, for
example on Debian or Ubuntu:

```sh
sudo apt-get install libfontconfig1-dev libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
```

The PowerShell scripts under `scripts/` (for example `launch-viewer.ps1` and
`use-local-rust.ps1`) are conveniences for the maintainer's Windows setup. You
never need them to build or test; the equivalent `cargo` commands are in the
[README](README.md#build-and-run).

## Checks

CI runs these on every pull request, so run them locally first:

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python3 -m unittest discover -s tests
```

CI tests the non-GUI crates on every PR and compile-checks the two viewers after
merges to `main`. If you changed `lbp-bevy-viewer` or `lbp-viewer`, run
`cargo check -p lbp-bevy-viewer -p lbp-viewer --all-targets` yourself.

## Pull request flow

1. Fork the repository and clone your fork.
2. Create a branch from the latest `main` (`git switch -c fix/short-name`).
3. Make your change with tests, and keep unrelated reformatting out of it.
4. Push the branch to your fork and open a pull request against `main`. The PR
   template asks what changed, how you checked it, and confirms no game data is
   included.
5. CI does not run automatically on a first-time contributor's PR: a maintainer
   has to approve the workflow run first, so a PR showing "waiting for approval"
   is expected and not something you need to fix.
6. Address review comments with new commits; the maintainer squash-merges.

Parsing crates (`lbp-core`, `lbp-io`, `lbp-archive`, `lbp-resource` and the
format crates) must not depend on UI, rendering, physics or gameplay crates.
Treat every length, count and offset read from a file as untrusted and use checked
arithmetic.

## License

By contributing you agree that your contribution is licensed under either of
[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at the licensee's option, the same
as the rest of the project.
