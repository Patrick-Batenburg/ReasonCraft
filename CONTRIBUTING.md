# Contributing to ReasonCraft

Thanks for helping. Bug reports and ideas go to the [issue tracker](https://github.com/Patrick-Batenburg/ReasonCraft/issues); security problems go privately, as [SECURITY.md](SECURITY.md) says. For a larger change, open an issue first so we can agree on the approach.

Deliberate decisions, with what they cost, are in [docs/rulings.md](docs/rulings.md): read the rulings of the area you touch, and add one when you make a choice someone could later take for a bug. [docs/dev/](docs/dev/README.md) holds the historical design and process notes.

## Layout

| Crate | What it holds |
|---|---|
| `crates/reasoncraft_core` | Everything without a UI: the document model and its rules (`model`, `rules`), editing and undo (`edit`, `history`), layout and connector geometry (`layout`, `connector`), checks, outline and essay plan, find, Argdown import and export (`argdown`), link safety (`link`) and the `.rcraft` file format (`file`, `package`). Integration tests in `tests/`. |
| `crates/reasoncraft_spell` | Spell checking on the bundled English and Dutch Hunspell dictionaries (`dict/`), through spellbook. |
| `app/reasoncraft` | The egui app. `actions` (every command and the one place that runs it), `commands` (the registry the ribbon, backstage and Search commands read), `canvas` (camera, painting, hit testing, keyboard and mouse), `scene` (the shapes the canvas and the exports draw), `shell` (title bar, tabs, ribbon, panels, backstage, status bar), `export`, `file_ops`, `recovery` (autosave), `spell`, `help`, `i18n`, `instance` (one running copy), `settings`. Strings in `i18n/*.ftl`, help pages in `help/<lang>/`. |
| `xtask` | Developer chores, never shipped (see below). |
| `packaging/` | The installers: WiX MSI and zip (Windows), tar.gz, deb, rpm, AppImage and Flatpak (Linux), the app bundle and DMG (macOS). See [docs/release.md](docs/release.md). |

## How a command flows

Every command the user can give is a variant of `Action` (`app/reasoncraft/src/actions/action.rs`):

1. `Action::traits` in `actions/classify.rs` says what the action leaves alone (the status note, "Move to…", a running edit session). It is one exhaustive match, so a new action does not compile until you decide.
2. `actions/dispatch.rs` (`run`) carries it out: one match without a wildcard, again exhaustive. `actions/enabled.rs` says when it would do anything, which greys out its button.
3. `commands/registry/` gives it a stable id, a label key, a ribbon group and an icon, for the ribbon, the backstage and Search commands (Alt+Q).
4. Its label is a Fluent message in `app/reasoncraft/i18n/en.ftl` and `nl.ftl` (key `action.add_reason` is message `action-add_reason`).
5. A keyboard shortcut goes in the table of `canvas/input/shortcuts.rs`, and on the matching row of README's "Keyboard shortcuts" tables: `commands/readme_tests.rs` fails until README and the tables agree.

To add a **string**, add it to `en.ftl` and `nl.ftl` together; a parity test fails when a key is missing in one. Dutch uses informal "je" and the terms in [docs/glossary-nl.md](docs/glossary-nl.md). To add a **help page**, write `help/en/<topic>.md` and `help/nl/<topic>.md` and add the topic to `TOPICS` in `src/help/mod.rs` (`build.rs` embeds the files).

## Tests

```sh
cargo test --workspace -j 4
```

- Unit tests sit next to the code (`tests.rs` or `*_tests.rs` modules). The app's integration tests are a few binaries under `app/reasoncraft/tests/`: `ui` (egui_kittest tests, one module per area), `ui_language` (tests that switch the interface language, which is one setting per process, so they take turns), `e2e` (end-to-end flows), `recovery` (autosave and restore), `packaging` (the installers' sources) and `dll_search` (it changes the process's DLL search order). Shared helpers are in `tests/common/`.
- UI tests drive the app through actions and `egui_kittest`, never through the real keyboard, mouse or clipboard. After anything that starts background work, wait with `common::settle` (or `common::step_until`) instead of a fixed number of frames; to keep a worker from finishing too early, give it a `jobs::Hold` and release it when the test is ready.
- A test that starts the real executable sets `REASONCRAFT_PROFILE_DIR` (`reasoncraft::profile::PROFILE_DIR_VAR`) to a temporary folder and kills the process on a timeout, so it never touches your settings or recovery files.
- Tests that measure speed are marked `#[ignore = "timing: run in release"]`. Run them with `cargo test --workspace --release -j 4 -- --ignored`.
- `-j 4` keeps the parallel link of the many test binaries within memory on Windows.

To try the app without touching your own settings, start it with `REASONCRAFT_PROFILE_DIR` set to an absolute folder (see README, "Separate profile").

## Chores (`cargo xtask`)

- `cargo xtask version` prints the workspace version; `cargo xtask version set 0.2.0` changes it (Cargo.toml is the single source).
- `cargo xtask icons` renders `app/reasoncraft/assets/icon.svg` to every icon file; commit the results.
- `cargo xtask notices` regenerates the Rust crates' licence texts for the About page (`assets/licences/crates.txt`); run it after changing dependencies (it needs `cargo install cargo-about --version 0.9.3 --locked --features cli`). CI fails when the file is out of date.
- `cargo xtask licence-rtf <out.rtf>` writes the Windows installer's licence page (the packaging script runs it).

## Before you open a pull request

CI runs these on Windows, macOS and Linux for every pull request; run them locally first:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -j 4 -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items -j 4
cargo test --workspace -j 4
cargo check --workspace --release --all-targets -j 4
cargo deny check
```

`cargo deny` (`cargo install cargo-deny`) checks licences, advisories and sources against `deny.toml`; CI also checks that `crates.txt` is current. The release check catches debug-only code (`cfg(debug_assertions)`) used from normal code.

Keep files under about 400 lines: split a module before adding to one that is already that long. Colours come from the theme's tokens (`theme.rs`), never literals. Code comments say what the code must do and why, not which plan or review asked for it.

## Licence of contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in ReasonCraft by you, as defined in the Apache-2.0 licence, shall be dual licensed under the [MIT licence](LICENSE-MIT) and the [Apache License, Version 2.0](LICENSE-APACHE), without any additional terms or conditions.
