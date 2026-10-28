# Third-party components

## spellbook (MPL-2.0)

Pure-Rust Hunspell-compatible spell checker, used unmodified from crates.io and confined to the `reasoncraft_spell` crate.
Source: https://github.com/helix-editor/spellbook. Licence: Mozilla Public License 2.0.

## WordCraft (MIT OR Apache-2.0; used under Apache-2.0)

The ribbon's group fitting (groups shrink to compact, then collapse into a drop-down, from the right; `app/reasoncraft/src/shell/ribbon/fit.rs`) and its scroll edge cues (`app/reasoncraft/src/shell/ribbon/overflow.rs`) are adapted from WordCraft, and changed. So are, from WordCraft's commit `aadb077`, the workspace version command (`xtask/src/version.rs`), the Windows installer's licence page (`xtask/src/licence.rs`, ported from its `packaging/windows/package.ps1`), and the Windows installer source, its install-policy checks and its build scripts (`packaging/windows/reasoncraft.wxs`, `install-policy.wxi`, `install-policy.json`, `check-install-policy.ps1`, `test-install-policy.ps1`, `package.ps1`, `sign.ps1` and `test-packaging-script.ps1`), the shared packaging setup (`.shellcheckrc` and `packaging/env.sh`) and the Linux packaging (`packaging/linux/package.sh`, `nfpm.yaml`, `postinst.sh` and `nl.reasoncraft.app.metainfo.xml.in`); each file's header says what changed. Source: https://github.com/storytold/wordcraft. Its NOTICE reads:

> WordCraft
> Copyright (c) 2026 ArtCraft Team and the WordCraft contributors.
> Licensed under the MIT License or the Apache License, Version 2.0, at your option
> (LICENSE-MIT, LICENSE-APACHE).

## Fonts

Bundled with egui (the `epaint_default_fonts` crate) and used unmodified. Ubuntu Light is also embedded in PDF exports, and its glyph outlines draw the text of outlined SVG exports.

- Ubuntu Light: Ubuntu Font Licence 1.0 (https://ubuntu.com/legal/font-licence).
- Hack Regular: MIT, with the Bitstream Vera licence for the parts from Bitstream Vera (https://github.com/source-foundry/Hack).
- egui-icons, a five-glyph subset of emoji-icon-font by John Slegers: MIT (https://github.com/jslegers/emoji-icon-font).

## Dictionaries

Bundled in `crates/reasoncraft_spell/dict/<lang>/`, taken unmodified from https://github.com/wooorm/dictionaries
at commit `8cfea406b505e4d7df52d5a19bce525df98c54ab`. Each directory carries its licence as `LICENSE`.

- English (`dict/en`): dictionary-en 4.0.0, licence MIT AND BSD.
- Dutch (`dict/nl`): dictionary-nl 2.0.0 (OpenTaal), dual-licensed BSD-3-Clause OR CC-BY-3.0. We use it under BSD-3-Clause.

## Windows installer

The Windows installer is built with NSIS 3.09 (https://nsis.sourceforge.io), whose installer code and zlib decompressor are under the zlib/libpng licence, through cargo-packager (https://github.com/crabnebula-dev/cargo-packager, MIT OR Apache-2.0). It uses cargo-packager's installer template, changed where marked, and the nsis_tauri_utils plugin (https://github.com/tauri-apps/nsis-tauri-utils, MIT OR Apache-2.0).

## Licence texts

The app shows the full licence texts of these components, and of every Rust crate it is built from, under File > About > Licences.
