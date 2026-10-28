# Third-party components

## spellbook (MPL-2.0)

Pure-Rust Hunspell-compatible spell checker, used unmodified from crates.io and confined to the `reasoncraft_spell` crate.
Source: https://github.com/helix-editor/spellbook. Licence: Mozilla Public License 2.0.

## WordCraft (MIT OR Apache-2.0; used under Apache-2.0)

The ribbon's group fitting (groups shrink to compact, then collapse into a drop-down, from the right; `app/reasoncraft/src/shell/ribbon/fit.rs`) and its scroll edge cues (`app/reasoncraft/src/shell/ribbon/overflow.rs`) are adapted from WordCraft, and changed. So are, from WordCraft's commit `aadb077`, the workspace version command (`xtask/src/version.rs`), the Windows installer's licence page (`xtask/src/licence.rs`, ported from its `packaging/windows/package.ps1`), and the Windows installer source, its install-policy checks and its build scripts (`packaging/windows/reasoncraft.wxs`, `install-policy.wxi`, `install-policy.json`, `check-install-policy.ps1`, `test-install-policy.ps1`, `package.ps1`, `sign.ps1` and `test-packaging-script.ps1`), the shared packaging setup (`.shellcheckrc` and `packaging/env.sh`), the Linux packaging (`packaging/linux/package.sh`, `nfpm.yaml`, `postinst.sh`, `nl.reasoncraft.app.metainfo.xml.in`, `flatpak-bundle.sh` and `flatpak/nl.reasoncraft.app.bundle.yml`) the macOS packaging (`packaging/macos/Info.plist.in`, `entitlements.plist`, `import-cert.sh` and `package.sh`), and the install-policy job of `.github/workflows/packaging-lint.yml`; each file's header says what changed. Source: https://github.com/storytold/wordcraft. Its NOTICE reads:

> WordCraft
> Copyright (c) 2026 ArtCraft Team and the WordCraft contributors.
> Licensed under the MIT License or the Apache License, Version 2.0, at your option
> (LICENSE-MIT, LICENSE-APACHE).

## craft-launcher (MIT OR Apache-2.0; used under Apache-2.0)

The macOS release check (`packaging/macos/verify.sh`) is adapted from craft-launcher's `packaging/macos/verify.sh` at commit `deccd0a`, and changed; its header says how. Source: https://github.com/storytold/craft-launcher. Its NOTICE reads:

> ArtCraft Launcher
> Copyright (c) 2026 ArtCraft Team and the ArtCraft Launcher contributors.
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

The MSI is built with the WiX Toolset 5.0.2 (https://github.com/wixtoolset/wix), under the Microsoft Reciprocal License (MS-RL). The MSI carries WiX's code and data: the WixUI dialogs and bitmaps (`WixToolset.UI.wixext`) and the WixUtilCA custom action that starts ReasonCraft from the finish page (`WixToolset.Util.wixext`). Neither is changed; their source is at WiX's `v5.0.2` tag. The licence text is in `app/reasoncraft/assets/licences/wix.txt`.

## Linux AppImage

The AppImage starts through the AppImage type2-runtime, release `20251108` (https://github.com/AppImage/type2-runtime), under the MIT licence. The runtime links musl libc (MIT), libfuse (LGPL-2.1), squashfuse (BSD-2-Clause), libzstd (BSD-3-Clause) and zlib (zlib licence) statically; their source, the LGPL-2.1 libfuse included, is reachable from the runtime's release tag. The licence text is in `app/reasoncraft/assets/licences/appimage-runtime.txt`.

## Licence texts

The app shows the full licence texts of these components, and of every Rust crate it is built from, under File > About > Licences.
