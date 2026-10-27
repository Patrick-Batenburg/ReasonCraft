# Third-party components

## spellbook (MPL-2.0)

Pure-Rust Hunspell-compatible spell checker, used unmodified from crates.io and confined to the `reasoncraft_spell` crate.
Source: https://github.com/helix-editor/spellbook. Licence: Mozilla Public License 2.0.

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
