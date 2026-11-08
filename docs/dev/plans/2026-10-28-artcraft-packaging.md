# Packaging the ArtCraft way: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace cargo-packager and its vendored NSIS template with the packaging the ArtCraft "Craft apps" use (WordCraft as the reference). Every release artifact must be built by scripts that keep to the interface of `storytold/craft-common-actions`' reusable release workflow, while ReasonCraft's own workflows stay self-contained.

**Architecture:**
- One `packaging/` folder at the repository root, laid out like WordCraft's:
  - `env.sh`, shared by the shell scripts;
  - `windows/`: a WiX 5 MSI and a portable zip;
  - `macos/`: a universal `.app` on a DMG;
  - `linux/`: tar.gz, deb and rpm through nfpm, AppImage with zsync, and a Flatpak bundle.
- The version lives only in `[workspace.package] version`. `cargo xtask version [set X]` reads and writes it, and every script reads it from there, or from `REASONCRAFT_VERSION`.
- Each script writes `dist/release/reasoncraft-<version>-<os>-<arch>.<ext>`.
- `.github/workflows/release.yml` runs these scripts on a push to the `release` branch or by hand, and drafts a GitHub Release with `SHA256SUMS.txt`.
- Later, the workflow can be swapped for a ~20-line caller of the reusable workflow without changing any script.

**Tech stack:**
- WiX Toolset 5.0.2 (`dotnet tool`) with its UI and Util extensions.
- nfpm 2.47.0, appimagetool 1.9.1 and AppImage type2-runtime 20251108 (both pinned by SHA-256), and flatpak-builder with the freedesktop 25.08 runtime.
- On macOS: codesign, notarytool and hdiutil.
- PowerShell 7, bash, and the existing `xtask` crate.

**Spec:** the owner's request of 2026-10-28 ("follow ArtCraft's way" for installers), with these references:
- [storytold/wordcraft@aadb077](https://github.com/storytold/wordcraft/tree/aadb077e072b7de05f724e359d8db82e151f125c): `packaging/`, `.github/workflows/{release,packaging-lint}.yml`, `xtask/src/version.rs`, `docs/releasing.md`;
- craft-launcher@deccd0a: `packaging/macos/verify.sh`;
- the contract of craft-common-actions@fa5ef18: `.github/workflows/release.yml` and `actions/*`.

Also read [docs/release.md](../../release.md), [docs/rulings.md](../../rulings.md) (Step 6: L3, M3, S6-T11-3/4, S6-F2/F3) and [docs/dev/lessons.md](../lessons.md).

## Scope and why

| In | Out (and why) |
|---|---|
| Windows x64, x86 and arm64: one dual-scope MSI per architecture, per user by default into `%LOCALAPPDATA%\Programs\ReasonCraft` and per machine into `Program Files\ReasonCraft`. Also a portable zip, static CRT, PE header and CRT checks, a signing script that skips without secrets, the licence RTF, the notices, the `.rcraft` ProgID `ReasonCraft.rcraft`, and the install-policy checks | Desktop shortcut: WordCraft has none, and GridCraft's opt-in dialog is about 150 lines of custom MSI UI |
| macOS: a universal `.app` on a plain DMG; ad hoc signing locally, Developer ID signing and notarization in CI when the secrets exist; `verify.sh` | The DMG background art: it needs a design and `generate.py`, and there is no Mac to look at the result. The Apple Event "open" for double-clicked files stays a carry-over (ruling S6-T11-2) |
| Linux x86_64 and aarch64: tar.gz, deb, rpm (nfpm), AppImage with `.zsync`, and the Flatpak bundle | Arch: the reusable workflow builds it with an unlicensed action, it can be switched off there (`has-arch: false`), and there is little demand. Flathub's from-source manifest: it can come once there is a public repository |
| `SHA256SUMS.txt` and a draft GitHub Release on the `release` branch; `workflow_dispatch` dry runs | FreeBSD (needs a VM job; nobody asked for it), web/wasm (rfd, file IO and the clipboard would need a wasm port), riscv64 (a cross toolchain for little value), the headless server, the SBOM and attestation (`supply-chain.yml` needs a known repository and its visibility). All of them can be switched off in the reusable workflow |
| Packaging lint (actionlint, shellcheck, PowerShell parse, XML, desktop and AppStream validation) and the MSI policy fixtures | Using `gate.yml` (`cargo xtask ci`): CI keeps its own `ci.yml`. It is noted under compatibility |

The Flatpak bundle stays in scope because the reusable workflow's `flatpak` job always runs. Without `packaging/linux/flatpak-bundle.sh` a later switch would fail every release. It costs one script and one manifest, since it only repackages the tarball.

## Global Constraints

- **Git.** Work on a branch `artcraft-packaging`, with local commits only. Never push or add a remote.
- **Memory.** Every cargo build uses `-j 4`. The scripts default to `CARGO_BUILD_JOBS=4` and honour an existing value.
- **Toolchain.** The toolchain is the one pinned in `rust-toolchain.toml` (1.99.1). The scripts run `rustup target add <triple>` themselves, because a caller may only have installed targets for `stable`.
- **File size.** Files stay at about 400 lines or fewer, and that includes scripts.
- **Sandboxing.**
  - Tests and scripts never touch real user data.
  - Real installers are installed only inside the Hyper-V VM "Windows 11 Sandbox", never on the host. The VM routine is: checkpoint → verify → restore → remove the checkpoint → leave the VM Off. The credential is in `%USERPROFILE%\.vm-sandbox-cred.xml`.
  - The `*-NOT-INSTALLABLE.msi` fixtures are never installed anywhere.
  - No synthetic keyboard or mouse input.
- **Branding.**
  - Product `ReasonCraft`; Manufacturer/publisher/vendor `ReasonCraft contributors` (ruling S6-F4); app ID and bundle ID `nl.reasoncraft.app`; ProgID `ReasonCraft.rcraft`, described as "ReasonCraft map"; MIME type `application/x-reasoncraft`; `.rcraft` (zip-based).
  - Leave out every ArtCraft URL, Discord link, "Learning Machines LLC" and the ArtCraft name itself (a trademark per WordCraft's NOTICE): the install folder is `ReasonCraft` directly under `Programs` / `Program Files` (owner's decision 2026-10-28).
- **Self-contained workflows.** Nothing in `.github/` references `storytold/*`, craft-fonts or craft-i18n. ReasonCraft bundles its own fonts and i18n, and the scripts ignore `CRAFT_FONTS_DIR` and `CRAFT_I18N_DIR`.
- **Licences of copied code.**
  - craft-common-actions has **no licence**, so nothing may be copied from it: only its interface (paths, arguments, artifact names, environment variables) is matched, and our own code implements it.
  - WordCraft and craft-launcher are "MIT OR Apache-2.0" by their NOTICE files, and are used under Apache-2.0 as for the ribbon. Each adapted file starts with a header comment, `Adapted from WordCraft (https://github.com/storytold/wordcraft, <path> at aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: <what>.`, and `THIRD-PARTY.md` lists it, quoting the NOTICE.
- **Pinned tools.**
  - WiX is exactly 5.0.2: it is MS-RL, and the v6+ binaries come with the OSMF EULA.
  - Actions are pinned to commit SHAs with the tag in a comment. Look the SHAs up with `gh api repos/<o>/<r>/commits/<tag> --jq .sha`.
  - Downloaded tools are checked against a SHA-256 taken from their upstream release.
- **Unsigned builds.** Signing stays optional (ruling L3, spec §12). With no secrets, a build warns and ships unsigned, also on the `release` branch.

## Review Focus

1. **A second copy in the other scope.** Installing per machine over an existing per-user copy, or the reverse, must be refused with the "already installed for…" message. It must never produce two registered copies. Pinned by `check-install-policy.ps1` (Task 2) and by the VM's cross-scope step (Task 4).
2. **Document paths with spaces and non-ASCII letters, opened through the association.** They must arrive as one quoted argument. Pinned by the registry contract `"[#ReasonCraftExe]" "%1"` (Task 2) and by the VM opening `Één kaart.rcraft` (Task 4).
3. **Pre-release versions such as `0.2.0-rc.1`.**
   - The MSI gets `0.2.0` and still replaces an earlier 0.2.0 build (same-version upgrade).
   - deb and rpm get `0.2.0~rc.1`.
   - `CFBundleShortVersionString` gets `0.2.0`.
   - `xtask version set` refuses `v1.0`, `1.0` and `1.0.0+meta`.
   - Pinned by the xtask tests (Task 1), `test-packaging-script.ps1` (Task 3), and nfpm's `version_schema: semver` (Task 5).
4. **Missing or partial signing secrets.**
   - Without secrets, `sign.ps1` and `import-cert.sh` warn and exit 0.
   - Partial Azure variables count as missing.
   - Apple identity without notarization credentials: signed, not notarized, with a warning.
   - Pinned in Task 3 (`sign.ps1` with an empty environment) and Task 7.
5. **A wrong-architecture or dynamic-CRT exe.** The build must fail before packaging if the exe's PE machine is not `-Arch`, if its subsystem is not GUI (2), or if it imports `VCRUNTIME140*.dll` or `api-ms-win-crt-*`. Pinned by `test-packaging-script.ps1` against the release exe (passes) and the debug exe (fails the CRT check) (Task 3).

---

### Task 1: One version source, `--version`, and the licence RTF in xtask

**Files:**
- Create: `.cargo/config.toml` (only `[alias] xtask = "run --package xtask --"`; no `[source]`, `[patch]` or `paths`, which the reusable workflow's git-sources gate rejects), `xtask/src/version.rs`, `xtask/src/licence.rs`, `app/reasoncraft/src/cli.rs`.
- Modify: `xtask/src/main.rs` (dispatch `version`, `version set <v>` and `licence-rtf <out>`; move `licence()` into `licence.rs`), `app/reasoncraft/src/lib.rs` (`pub mod cli;`), `app/reasoncraft/src/main.rs`.
- Test: unit tests in `xtask/src/version.rs` and `xtask/src/licence.rs`, and `app/reasoncraft/tests/ui/packaging.rs::version_flag_prints_version`.

**Interfaces:**
- Produces:
  - `version::read(manifest: &str) -> Result<String, String>`;
  - `version::replace(manifest: &str, new: &str) -> Result<String, String>` (changes only that line and keeps CRLF);
  - `version::validate(v: &str) -> Result<(), String>` (`X.Y.Z[-pre]`: no leading zeros, no `+build`);
  - `cargo xtask version`, which prints the version;
  - `cargo xtask version set <v>`, which writes the manifest atomically through `target/`, then runs `cargo update --workspace --offline`, falling back to online.
  - `licence::rtf(mit: &str, apache: &str) -> String` and `cargo xtask licence-rtf <out.rtf>`.
  - `reasoncraft::cli::wants_version(args: &[OsString]) -> bool`: true only when the sole argument is `--version`. `main` then prints `ReasonCraft <CARGO_PKG_VERSION>` and exits 0 before any window opens.

- [ ] **Step 1: Write the failing tests.**
  - Adapt WordCraft's `version.rs` tests (header comment as in the Global Constraints): `reads_the_workspace_version`, `replaces_only_the_workspace_version`, `keeps_crlf_line_endings`, `validates_versions` (with WordCraft's ok/bad lists), and `the_real_manifest_has_a_valid_version`.
  - `licence.rs`:
    - `rtf_escapes_braces_backslashes_and_unicode`: `rtf_escape("a{b}\\ é€")` equals `"a\\{b\\}\\\\ \\u233?\\u8364?"`, and a code point above 32767 is written negative.
    - `rtf_holds_both_licences`: the output starts with `{\rtf1\ansi`, contains `\b MIT License\b0` and `Apache License`, and has no bare `{` or `}` inside a paragraph.
  - `version_flag_prints_version`: run `env!("CARGO_BIN_EXE_reasoncraft")` with `--version`, wait for the status, and assert stdout is `ReasonCraft 0.1.0\n` (read the version from `CARGO_PKG_VERSION`) and the status is success.
- [ ] **Step 2:** Run `cargo test -p xtask -j 4` and `cargo test -p reasoncraft --test ui -j 4 version_flag`. Expected: they FAIL, because the items are missing.
- [ ] **Step 3: Implement.**
  - The RTF follows WordCraft's `ConvertTo-RtfText`, ported to Rust:
    - the header `{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fswiss\fcharset0 Tahoma;}}\viewkind4\uc1\pard\sa120\f0\fs16`;
    - a bold title `ReasonCraft` and the sentence "ReasonCraft is licensed under the MIT License or the Apache License, Version 2.0, at your option. Both licence texts follow. Bundled third-party material keeps its own licence, listed in THIRD-PARTY.md.";
    - the hard-wrapped paragraphs joined, and each file's first paragraph in bold.
  - Keep `xtask licence` (it writes `packaging/LICENSE.txt`) until Task 8 removes cargo-packager.
- [ ] **Step 4:** Run the quality gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -j 4 -- -D warnings`, `cargo test --workspace -j 4`, then `cargo xtask version` (prints `0.1.0`) and `cargo xtask licence-rtf target/licence.rtf` (opens in WordPad). Expected: all PASS.
- [ ] **Step 5:** Commit `feat(xtask): workspace version command, licence RTF; reasoncraft --version`.

### Task 2: WiX source and the install-policy checks (no app build)

**Files:**
- Create (all adapted from WordCraft, with headers):
  - `packaging/windows/reasoncraft.wxs`;
  - `packaging/windows/install-policy.wxi`, with `Software\WordCraft` → `Software\ReasonCraft` and `wordcraft.exe` → `reasoncraft.exe`;
  - `packaging/windows/install-policy.json`;
  - `packaging/windows/check-install-policy.ps1`, without the launcher `perUser` branch and the deckcraft/filmcraft/soundcraft route;
  - `packaging/windows/test-install-policy.ps1`, with `-d` variables trimmed to ours.
- Modify: `THIRD-PARTY.md`. The WordCraft entry gains the list of adapted packaging files.
- Test: `packaging/windows/test-install-policy.ps1` and `app/reasoncraft/tests/ui/packaging_windows.rs` (new module in `tests/ui/main.rs`).

**Interfaces:**
- WiX preprocessor variables: `Version` (numeric `X.Y.Z`), `BinDir`, `IconPath`, `LicenseRtf`, `AppNoticesDir`. Build with `-arch x64|x86|arm64 -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext`.
- Package attributes: `Name="ReasonCraft" Manufacturer="ReasonCraft contributors" Scope="perUserOrMachine" UpgradeCode="<new GUID>" Language="1033"`.
  - Generate the UpgradeCode and the two component GUIDs once with `[guid]::NewGuid()`, and record them in the wxs and in `install-policy.json`. They never change afterwards.
  - Also: `MajorUpgrade AllowSameVersionUpgrades="yes"`, `MediaTemplate EmbedCab="yes"`, and the folders `ProgramFiles6432Folder\ReasonCraft(INSTALLFOLDER)` (no suite folder).
  - UI: `WixUI_Minimal` with the licence RTF and a ticked "Launch ReasonCraft" box on the finish page (WixShellExec).
  - No ARP URLs.
- IDs: icon `ReasonCraftIcon.ico`; file `ReasonCraftExe` with the advertised Start menu shortcut `ReasonCraft`; components `ReasonCraftApp` and `ReasonCraftFileTypes`; component group `AppNotices`, which takes `$(var.AppNoticesDir)\**`.
- The registry contract: these `install-policy.json` rows, all with root `-1` (HKMU), all exactly once in the compiled MSI. `E` stands for `"[#ReasonCraftExe]"`.
  - `Software\Microsoft\Windows\CurrentVersion\App Paths\reasoncraft.exe` | `` | `[#ReasonCraftExe]`
  - `Software\ReasonCraft` | `InstallDir` | `[INSTALLFOLDER]`
  - `Software\RegisteredApplications` | `ReasonCraft` | `Software\ReasonCraft\Capabilities`
  - `Software\ReasonCraft\Capabilities` | `ApplicationName` = `ReasonCraft`, `ApplicationDescription` = `Map out arguments: claims, reasons and objections.`, `ApplicationIcon` = `E,0`
  - `Software\ReasonCraft\Capabilities\FileAssociations` | `.rcraft` | `ReasonCraft.rcraft`
  - `Software\Classes\Applications\reasoncraft.exe` | `FriendlyAppName` | `ReasonCraft`; `...\shell\open\command` | `` | `E "%1"`; `...\SupportedTypes` | `.rcraft` | ``
  - `Software\Classes\ReasonCraft.rcraft\shell\open\command` | `` | `E "%1"`, plus `<ProgId Id="ReasonCraft.rcraft" Description="ReasonCraft map" Icon="ReasonCraftExe" IconIndex="0"/>`
  - `Software\Classes\.rcraft` | `` | `ReasonCraft.rcraft`, and `Content Type` = `application/x-reasoncraft`. Unlike WordCraft, we set the default, because `.rcraft` is our own type.
  - `Software\Classes\.rcraft\OpenWithProgids` | `ReasonCraft.rcraft` | ``
- `install-policy.json`: `{"id":"reasoncraft","name":"ReasonCraft","upgradeCode":…,"sources":["reasoncraft.wxs"],"icon":"app/reasoncraft/assets/icon.ico","localization":[],"extensions":["UI","Util"],"payloads":["reasoncraft.exe"],"registry":[…],"mode":"dualScope"}`. The `icon` path is relative to the repository root.

- [ ] **Step 1: Write the failing Rust test `packaging_windows.rs`.** It parses the wxs with `roxmltree` and `install-policy.json` with `serde_json`.
  - `wix_package_identity`: the Name, Manufacturer and Scope above; the UpgradeCode equals the JSON's; INSTALLFOLDER's `Name="ReasonCraft"` sits directly under ProgramFiles6432Folder.
  - `every_policy_row_is_authored_once`: each JSON registry row matches exactly one `RegistryValue` with `Root="HKMU"`, its Key, Name (empty means none) and Value, after `&quot;` decoding.
  - `association_opens_quoted_path`: the ProgId has the description "ReasonCraft map", and its open command equals `"[#ReasonCraftExe]" "%1"`.
- [ ] **Step 2:** Run `cargo test -p reasoncraft --test ui -j 4 packaging_windows`. Expected: it FAILS, because the files are missing.
- [ ] **Step 3: Write the WiX sources and scripts.**
  - **Prerequisites on this machine.** There is a dotnet host but no SDK: run `winget install Microsoft.DotNet.SDK.8`. Then `dotnet tool install --global wix --version 5.0.2`, followed by `wix extension add -g WixToolset.UI.wixext/5.0.2 WixToolset.Util.wixext/5.0.2`.
- [ ] **Step 4: Verify.**
  - Run `pwsh packaging/windows/test-install-policy.ps1 -OutputDirectory target/install-policy-tests`. Expected: `PASS ReasonCraft: 24 scope cases…`, then `PASS <arch> metadata-only package` for x64, x86 and arm64.
  - Then run the Rust test again. Expected: PASS.
- [ ] **Step 5:** Commit `feat(packaging): WiX 5 dual-scope MSI source and install-policy checks`.

### Task 3: Windows packaging script, PE checks and signing

**Files:**
- Create:
  - `packaging/windows/package.ps1`, adapted from WordCraft: no CLI, no craft-fonts, no smoke run of a CLI;
  - `packaging/windows/pe-checks.ps1`, new and dot-sourced;
  - `packaging/windows/sign.ps1`, adapted: `/d ReasonCraft`, with `/du` only when `REASONCRAFT_SIGN_URL` is set;
  - `packaging/windows/test-packaging-script.ps1`, adapted, plus the cases below.
- Modify: `.gitignore` (add `dist/`).

**Interfaces:**
- Consumes: `cargo xtask licence-rtf` (Task 1) and the wxs variables (Task 2).
- `package.ps1 [-Arch x64|x86|arm64] [-SkipBuild]` reads `REASONCRAFT_VERSION` (else Cargo.toml), `DIST` (default `dist\release`), `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS` (default 4).
  1. `rustup target add <triple>`, then `cargo build --release --locked -p reasoncraft --target <triple>`, with `CARGO_TARGET_<TRIPLE>_RUSTFLAGS=-C target-feature=+crt-static`.
  2. `Assert-ReleaseExe`.
  3. Stage the exe and sign it.
  4. `AppNoticesDir` = `LICENSE-MIT`, `LICENSE-APACHE` and `THIRD-PARTY.md`.
  5. Write the licence RTF.
  6. `wix build`.
  7. `check-install-policy.ps1 -MsiPath $Msi`.
  8. Delete the `.wixpdb`.
  9. `sign.ps1 $Msi`.
  10. Write the portable zip: `reasoncraft.exe`, `README.md`, the licences and `THIRD-PARTY.md`.
- It writes `dist/release/reasoncraft-<v>-windows-<arch>.msi` and `dist/release/reasoncraft-<v>-windows-<arch>-portable.zip`. These names are part of the contract.
- `pe-checks.ps1`:
  - `Get-PeHeader([string]$Path) -> @{ Machine; Subsystem }`;
  - `Get-PeImports([string]$Path) -> string[]`: the DLL names from the import directory, read by walking the PE headers rather than searching for strings;
  - `Assert-ReleaseExe([string]$Path, [string]$Arch)` throws when the machine is not 0x8664/0x14C/0xAA64 for the arch, when the subsystem is not 2, or when an import matches `^(vcruntime|msvcp)\d+.*\.dll$|^api-ms-win-crt-`.

- [ ] **Step 1: Write the failing cases in `test-packaging-script.ps1`.**
  - WordCraft's AST checks: one policy check on `$Msi`, before the single `sign.ps1 $Msi`.
  - `Assert-ReleaseExe target\x86_64-pc-windows-msvc\release\reasoncraft.exe x64` passes, and the same exe with `arm64` throws "machine".
  - `target\debug\reasoncraft.exe` with `x64` throws "C runtime".
  - `sign.ps1` on a copy of the exe, with every signing variable removed and `AZURE_TENANT_ID` alone set, exits 0 with "leaving unsigned".
  - With `REASONCRAFT_VERSION=0.2.0-rc.1` and `-SkipBuild`, the MSI's ProductVersion (read through `WindowsInstaller.Installer`, as `check-install-policy.ps1` does) is `0.2.0`.
- [ ] **Step 2:** Run `pwsh packaging/windows/test-packaging-script.ps1`. Expected: it FAILS, because `package.ps1` and `pe-checks.ps1` are missing.
- [ ] **Step 3:** Implement the scripts.
- [ ] **Step 4: Verify.**
  - Run `pwsh packaging/windows/package.ps1 -Arch x64` and `-Arch x86`. Expected: two MSIs and two zips in `dist/release`, and `PASS ReasonCraft…` from the policy check.
  - arm64 needs the "MSVC v143 ARM64 build tools", which are not installed here. It is built in CI only, where the PE check guards its label.
  - Then run `test-packaging-script.ps1`. Expected: all PASS.
- [ ] **Step 5:** Commit `feat(packaging): Windows MSI and portable zip with static CRT, PE and signing checks`.

### Task 4: Check the MSI in the VM

**Files:**
- Create: `packaging/windows/verify-in-vm.ps1` (orchestration, ≤ 400 lines) and `packaging/windows/vm-checks.ps1` (dot-sourced helpers).
- Delete: `app/reasoncraft/packaging/windows/verify-in-vm.ps1`, after moving its connect, sign-in, scheduled-task and screen-capture code over.

**Interfaces:**
- Consumes: the MSIs from Task 3.
- `verify-in-vm.ps1 -Msi <new.msi> -OlderMsi <old.msi> -Sample <map.rcraft> -OutDir <dir> [-VMName 'Windows 11 Sandbox'] [-Checkpoint 'before-reasoncraft']`.
- Make the older MSI with `$env:REASONCRAFT_VERSION='0.0.1'; $env:DIST='target\vm-old'; package.ps1 -Arch x64 -SkipBuild`.
- `vm-checks.ps1`:
  - `Invoke-Msi($Session, [string]$Path, [string[]]$Arguments) -> int`, which runs `msiexec /i|/x … /qn /l*v C:\rc-test\<name>.log` and waits;
  - `Get-InstallState($Session, [ValidateSet('user','machine')]$Scope) -> pscustomobject`, with the fields Dir, Exe, Notices, ArpCount, ArpVersion, InstallDirKey, AppPath, ProgId, OpenCommand, ExtDefault, ContentType, HkcrResolves, StartMenu, Desktop;
  - `Test-OpenByAssociation($Session, [string]$File)`;
  - `Test-RendererFallback($Session, [string]$Exe)`;
  - `Save-VMScreen([string]$Path)`.

- [ ] **Step 1: Write the check list into the script** (a `Check` call for each). Copy the sample in twice, as `Sample map.rcraft` and `Één kaart.rcraft`. Then:
  1. **Per user.** Install `-OlderMsi` with `/qn`, then `-Msi`, which is a major upgrade.
     - Both exit with 0.
     - There is one ARP entry under HKCU, and its DisplayVersion is the new one.
     - The exe and `LICENSE-MIT`, `LICENSE-APACHE` and `THIRD-PARTY.md` are in `%LOCALAPPDATA%\Programs\ReasonCraft`.
     - `HKCU\Software\ReasonCraft\InstallDir` is that folder, and App Paths is set.
     - `HKCU\Software\Classes\.rcraft` is `ReasonCraft.rcraft`, with the Content Type.
     - The ProgID is described as "ReasonCraft map", and its command matches `'"*\reasoncraft.exe" "%1"'`.
     - `HKCR\.rcraft` resolves to the ProgID.
     - `ReasonCraft.lnk` is in the user's Start menu, and there is no desktop shortcut.
  2. **The other scope is refused.** `msiexec /i -Msi ALLUSERS=1 /qn` exits 1603, its log contains "already installed for your account", and nothing is added under HKLM.
  3. **Open by association.** Opening `Één kaart.rcraft` starts `reasoncraft.exe` with that path as one quoted argument, and the app stays up. Save the screen, then run the renderer fallback check as before.
  4. **Uninstall per user.** `msiexec /x -Msi /qn` exits 0. Every item from 1 is gone. `Documents\Keep me.rcraft` and `%LOCALAPPDATA%\ReasonCraft\recovery` survive.
  5. **Per machine.** First check that the session is elevated (fail if not). `ALLUSERS=1 /qn` exits 0. The install is in `%ProgramW6432%\ReasonCraft`. The association is under `HKLM\Software\Classes`. `ReasonCraft.lnk` is in `%ProgramData%\Microsoft\Windows\Start Menu\Programs`. Opening by association starts the app.
  6. **Uninstall per machine.** Everything from 5 is gone.
  7. **`finally`.**
     1. `Stop-VM -TurnOff`, then `Restore-VMSnapshot`, then `Remove-VMSnapshot`.
     2. If the VM is not `Off`, run `Remove-VMSavedState` or `Stop-VM -TurnOff`.
     3. Print the VM state and the number of checkpoints left, which must be 0.
     4. Stop the transcript (`msi-vm-transcript.txt`).
- [ ] **Step 2:** Run it before building the new MSIs, with a missing `-Msi`. Expected: it fails fast with "Cannot find path", having taken no checkpoint.
- [ ] **Step 3:** Run it for real: `cargo xtask sample-map target\sample.rcraft`, then `.\packaging\windows\verify-in-vm.ps1 -Msi dist\release\reasoncraft-0.1.0-windows-x64.msi -OlderMsi target\vm-old\reasoncraft-0.0.1-windows-x64.msi -Sample target\sample.rcraft -OutDir target\vm-check`. Expected: `0 failure(s)`, the VM is `Off`, and no `before-reasoncraft` checkpoint is left. Look at both PNGs.
- [ ] **Step 4:** Commit `test(packaging): verify the MSI per user, per machine, upgrade and uninstall in the VM`.

### Task 5: Linux: shared env, tar.gz, deb, rpm and AppImage

**Files:**
- Create:
  - `.shellcheckrc`, copied from WordCraft;
  - `packaging/env.sh`, adapted: `REASONCRAFT_VERSION`; exports `ROOT VERSION DIST CARGO_TARGET_DIR CARGO_BUILD_JOBS`; functions `warn`, `copy_docs` (README.md, LICENSE-MIT, LICENSE-APACHE, THIRD-PARTY.md) and `sha256`;
  - `packaging/linux/package.sh`, adapted: one binary, icons staged from `app/reasoncraft/assets`;
  - `packaging/linux/nfpm.yaml`, `packaging/linux/postinst.sh`, `packaging/linux/nl.reasoncraft.app.metainfo.xml.in`;
  - `packaging/linux/Dockerfile`: Ubuntu 22.04 (glibc 2.35, as on CI), rustup with the pinned toolchain, `pkg-config libxkbcommon-dev libgtk-3-dev libxcb-*-dev libssl-dev desktop-file-utils appstream zsync file curl`, and nfpm 2.47.0 checked against the SHA-256 from its release's `checksums.txt`.
- Move: `app/reasoncraft/packaging/linux/reasoncraft.desktop` to `packaging/linux/nl.reasoncraft.app.desktop`, and `reasoncraft-mime.xml` to `packaging/linux/nl.reasoncraft.app.mime.xml`. The old cargo-packager files keep working until Task 8, so copy now and delete in Task 8.
- Test: `app/reasoncraft/tests/ui/packaging_linux.rs`.

**Interfaces:**
- `package.sh [--skip-build] [--formats "appimage deb rpm tar"]`, with the arch from `uname -m` (x86_64 or aarch64). It writes into `$DIST`:
  - `reasoncraft-<v>-linux-<arch>.tar.gz`, with top folder `reasoncraft-<v>-linux-<arch>/` and `bin/reasoncraft` and `share/…` inside;
  - `.deb` and `.rpm`;
  - `.AppImage`;
  - `.AppImage.zsync`, only when `GITHUB_REPOSITORY` is set. The update info is then `gh-releases-zsync|<owner>|<repo>|latest|reasoncraft-*-linux-<arch>.AppImage.zsync`; local builds have none.
- Staged FHS tree:
  - `usr/bin/reasoncraft` (stripped);
  - `usr/share/applications/nl.reasoncraft.app.desktop`;
  - `usr/share/mime/packages/nl.reasoncraft.app.xml`;
  - `usr/share/metainfo/nl.reasoncraft.app.metainfo.xml`;
  - `usr/share/icons/hicolor/<s>x<s>/apps/nl.reasoncraft.app.png` for s in 16 24 32 48 64 128 256 512, from `assets/icons/icon-<s>.png`, plus `scalable/apps/nl.reasoncraft.app.svg` from `assets/icon.svg`;
  - `usr/share/doc/reasoncraft/` with the documents from `copy_docs`.
- Desktop entry: `Exec=reasoncraft %F`, `TryExec=reasoncraft`, `Icon=nl.reasoncraft.app`, `StartupWMClass=nl.reasoncraft.app` (equal to `with_app_id` in `main.rs`), `Categories=Education;Office;`, `MimeType=application/x-reasoncraft;`.
- nfpm: `name: reasoncraft`, `version_schema: semver`, `maintainer: ${REASONCRAFT_MAINTAINER}` (default `ReasonCraft contributors`, with no address; which address to publish is an open question in `docs/dev/todo-history.md`), `vendor: ReasonCraft contributors`, `license: MIT OR Apache-2.0`, `section: education`.
  - deb `depends`: exactly today's list, `libc6 libgcc-s1 libxkbcommon0 libxkbcommon-x11-0 libx11-6 libx11-xcb1 libxcursor1 libxi6 libwayland-client0 libwayland-egl1 libegl1 libgl1` (ruling S6-T13-3: no libxrandr2). Recommends `xdg-desktop-portal`, since rfd 0.17 uses the portal; there is no gtk-sys in Cargo.lock.
  - rpm `depends`: `libxkbcommon`, `libxkbcommon-x11`, `(libglvnd-egl or mesa-libEGL)`. Recommends `libX11 libX11-xcb libXcursor libXi libwayland-client libwayland-egl xdg-desktop-portal`.
- AppImage: appimagetool 1.9.1 and type2-runtime 20251108, with the SHA-256s from WordCraft's `package.sh`. Re-check them against the upstream release downloads before committing.

- [ ] **Step 1: Write the failing Rust tests in `packaging_linux.rs`.**
  - `desktop_entry_matches_app_id`: the `StartupWMClass` value appears in `src/main.rs` as `with_app_id("…")`; `Exec=reasoncraft %F`; and the MimeType.
  - `mime_and_metainfo_declare_rcraft`: the MIME XML has the type and the `*.rcraft` glob; the metainfo's `<id>` is `nl.reasoncraft.app` and it provides `<mediatype>application/x-reasoncraft</mediatype>`.
  - `deb_names_its_runtime_libraries`: moved from `packaging.rs`. It reads the lines of nfpm.yaml between `  deb:` and `  rpm:`.
  - `prerelease_versions_sort_before_release`: nfpm.yaml has `version_schema: semver`.
- [ ] **Step 2:** Run `cargo test -p reasoncraft --test ui -j 4 packaging_linux`. Expected: it FAILS.
- [ ] **Step 3:** Implement the files, and run `git update-index --chmod=+x packaging/env.sh packaging/linux/*.sh`.
- [ ] **Step 4: Verify in Docker.** Use the read-only copy routine from `docs/release.md`, with `-m 16g`:
  - `docker build -f packaging/linux/Dockerfile -t reasoncraft-linux .`
  - `docker run --rm -m 16g -v "$PWD:/repo:ro" -v rc-target:/src/target -v "$PWD/target/linux-dist:/src/dist" reasoncraft-linux bash -c '<copy files>; cargo test --workspace -j 4 && packaging/linux/package.sh && dpkg-deb --info dist/release/*.deb && rpm -qip dist/release/*.rpm && APPIMAGE_EXTRACT_AND_RUN=1 dist/release/*.AppImage --version'`
  - Expected: `ReasonCraft 0.1.0`, and desktop-file-validate and appstreamcli report no errors.
  - Also install the deb in a clean `ubuntu:22.04` container (`apt install ./…deb`, then `reasoncraft --version`).
- [ ] **Step 5:** Commit `feat(packaging): Linux tar.gz, deb, rpm and AppImage via nfpm and appimagetool`.

### Task 6: Flatpak bundle

**Files:**
- Create: `packaging/linux/flatpak-bundle.sh` and `packaging/linux/flatpak/nl.reasoncraft.app.bundle.yml`, both adapted.
  - The manifest uses `runtime: org.freedesktop.Platform`, `runtime-version: '25.08'`, and `command: reasoncraft`.
  - finish-args: `--socket=wayland`, `--socket=fallback-x11`, `--share=ipc`, `--device=dri`, `--filesystem=xdg-documents`. No network, because the app is offline.

**Interfaces:**
- `flatpak-bundle.sh [--no-test] [TARBALL]` → `$DIST/reasoncraft-<v>-linux-<arch>.flatpak`. The reusable workflow calls it with exactly one tarball argument. The smoke test is `flatpak run nl.reasoncraft.app --version`.

- [ ] **Step 1: Verify in CI only.** flatpak-builder needs user namespaces, which a Docker Desktop container lacks. So the check is `shellcheck`, plus `flatpak-builder --show-manifest` in the Linux image. The real build runs in the release workflow's `flatpak` job (Task 9) on the first dispatch once a remote exists. Record that in `docs/dev/todo-history.md`.
- [ ] **Step 2:** Commit `feat(packaging): Flatpak bundle from the release tarball`.

### Task 7: macOS: universal app, DMG, signing and verification

**Files:**
- Create:
  - `packaging/macos/Info.plist.in`:
    - `CFBundleIdentifier` `nl.reasoncraft.app`, executable and icon `ReasonCraft`, `LSMinimumSystemVersion` 11.0, `NSHumanReadableCopyright` "Copyright © Patrick van Batenburg and ReasonCraft contributors. MIT OR Apache-2.0.", category `public.app-category.education`;
    - `UTExportedTypeDeclarations` for `nl.reasoncraft.map`: conforms to `public.data` and `public.zip-archive`, extension `rcraft`, MIME type `application/x-reasoncraft`;
    - `CFBundleDocumentTypes`: role `Editor`, rank `Owner`, content type `nl.reasoncraft.map`.
  - `packaging/macos/entitlements.plist` (empty) and `packaging/macos/import-cert.sh`, both adapted.
  - `packaging/macos/package.sh`, adapted: no CLI; a plain DMG of `ReasonCraft.app` with an `Applications` link (no background or `.DS_Store`); everything else as WordCraft, including `xattr -cr`, signing inside-out, `verify_dmg_app`, notarization, and the stapler and hdiutil retries.
  - `packaging/macos/verify.sh`, adapted from craft-launcher: `PKG=reasoncraft`, `APP_NAME=ReasonCraft`, without the `LSUIElement` check.
- Modify: `THIRD-PARTY.md`, adding the craft-launcher entry with the quote from its NOTICE.
- Test: `app/reasoncraft/tests/ui/packaging_macos.rs`.

**Interfaces:**
- `package.sh [--arch universal|aarch64|x86_64] [--skip-build]` → `$DIST/reasoncraft-<v>-macos-<arch>.dmg`.
  - It builds with `--target aarch64-apple-darwin --target x86_64-apple-darwin`, after `rustup target add` for both, and runs `lipo`.
  - It sets `MACOSX_DEPLOYMENT_TARGET=11.0`.
  - Environment: `MACOS_SIGN_IDENTITY` (default `-`, ad hoc), `MACOS_KEYCHAIN`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`.
- `import-cert.sh` reads `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` and `KEYCHAIN_PASSWORD`, and appends `MACOS_SIGN_IDENTITY` and `MACOS_KEYCHAIN` to `$GITHUB_ENV`.
- `verify.sh --arch universal` is strict only when the identity and notarization credentials are all set.

- [ ] **Step 1: Write the failing test `plist_declares_rcraft`.**
  - Substitute `@VERSION@` and `@SHORT_VERSION@`, then parse with `roxmltree`.
  - Assert: the identifier; the exported UTI with the `rcraft` extension and the MIME type; the document type `Editor` pointing to that UTI; and `LSMinimumSystemVersion` equal to the `MACOSX_DEPLOYMENT_TARGET` in `package.sh`.
- [ ] **Step 2:** Run `cargo test -p reasoncraft --test ui -j 4 packaging_macos`. Expected: it FAILS, then PASSES after the implementation.
- [ ] **Step 3: Verify without a Mac.**
  - Run `shellcheck` (from the Docker image `koalaman/shellcheck:v0.10.0`) on `packaging/**/*.sh`, then `xmllint --noout packaging/macos/*.plist*`.
  - The real build is checked by the release workflow's `macos` job and on a real Mac (release checklist).
- [ ] **Step 4:** Commit `feat(packaging): macOS universal app on a DMG, signing, notarization and verification`.

### Task 8: Remove cargo-packager and NSIS; notices and supported targets

**Files:**
- Delete:
  - `app/reasoncraft/packaging/`: `installer.nsi`, `LICENSE.txt`, the old `linux/*`, `Dockerfile` and `Dockerfile.dockerignore`. Move the dockerignore to `packaging/linux/Dockerfile.dockerignore` if the build context needs it.
  - `app/reasoncraft/assets/licences/nsis.txt`;
  - the `[package.metadata.packager*]` tables in `app/reasoncraft/Cargo.toml`, the `toml` dev-dependency, and `xtask licence` (`licence-rtf` replaces it).
- Create:
  - `app/reasoncraft/assets/licences/wix.txt`: WiX v5.0.2's `LICENSE.TXT` verbatim (MS-RL);
  - `app/reasoncraft/assets/licences/appimage-runtime.txt`: type2-runtime's `LICENSE` verbatim (MIT, with the notes on musl and libfuse LGPL-2.1).
- Modify:
  - `app/reasoncraft/src/shell/about/licences.rs`: `BUNDLED` becomes `[_; 8]`, and `about.licences.nsis` becomes `about.licences.wix` and `about.licences.appimage`;
  - `app/reasoncraft/i18n/{en,nl}.ftl`: `about-licences-wix = Windows installer (WiX Toolset): Microsoft Reciprocal License` / `Windows-installatieprogramma (WiX Toolset): Microsoft Reciprocal License`, and `about-licences-appimage = Linux AppImage runtime: MIT, with musl (MIT) and libfuse (LGPL-2.1)` / `Linux-AppImage-runtime: MIT, met musl (MIT) en libfuse (LGPL-2.1)`;
  - `THIRD-PARTY.md`: replace "Windows installer" with WiX 5.0.2 (MS-RL: the WixUI dialogs and bitmaps and the WixUtilCA custom action in the MSI), and add "Linux AppImage" (type2-runtime 20251108, MIT; it links musl and libfuse statically, whose LGPL-2.1 source is at the runtime's release tag);
  - `deny.toml` and `xtask/about.toml`: add `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc` and `aarch64-unknown-linux-gnu` to the targets, then regenerate `crates.txt` with `cargo xtask notices`;
  - `app/reasoncraft/tests/ui/packaging.rs`: keep `icon_assets_present_and_sized`, `cli_path_opens_file` and `version_flag_prints_version`; drop `packager_config_has_association`.

- [ ] **Step 1: Update the tests first.** In the `licences.rs` test, `text("about.licences.wix")` contains "Microsoft Reciprocal License", and `text("about.licences.appimage")` contains "libfuse". Add `no_cargo_packager_left`: the app's Cargo.toml has no `metadata.packager`, and `app/reasoncraft/packaging` does not exist.
- [ ] **Step 2:** Run `cargo test --workspace -j 4`. Expected: it FAILS.
- [ ] **Step 3: Implement.** Then run `rg -i "packager|nsis|setup\.exe|installer\.nsi" --glob '!docs/dev/plans/**' --glob '!docs/dev/specs/**' --glob '!target/**'`. Expected: hits only in `docs/rulings.md`'s history and `docs/dev/todo-history.md`'s history.
- [ ] **Step 4:** Run the full gate, plus `cargo check --workspace --release --all-targets -j 4`, `cargo deny check` and `cargo xtask notices && git diff --exit-code app/reasoncraft/assets/licences/crates.txt` (after committing the regenerated file). Expected: all PASS.
- [ ] **Step 5:** Commit `refactor(packaging): drop cargo-packager and NSIS; WiX and AppImage notices; all release targets in deny/about`.

### Task 9: Workflows: release, packaging lint and CI

**Files:**
- Rewrite: `.github/workflows/release.yml`.
- Create: `.github/workflows/packaging-lint.yml`.
- Modify: `.github/workflows/ci.yml`. Its comments point to `packaging/linux/Dockerfile`, and nothing else changes.

**Interfaces:**
- `release.yml` triggers on `push: branches: [release]` and on `workflow_dispatch` with an input `version` (default empty). Permissions are `contents: read`, and `contents: write` only in `draft`.
- Jobs:
  - `version` (ubuntu-latest): source `packaging/env.sh`, strip a leading `v` from the input, refuse CR/LF, and check the version against `^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$`. Outputs `version` and `prerelease`.
  - `windows` (matrix x64, x86 and arm64 on windows-latest):
    1. checkout and `rustup toolchain install`;
    2. if an input version is set, `cargo xtask version set`;
    3. `dotnet tool install --global wix --version 5.0.2` and the extensions;
    4. `pwsh packaging/windows/test-packaging-script.ps1` (x64 only);
    5. `./packaging/windows/package.ps1 -Arch <arch>`, with the `WINDOWS_*` and `AZURE_*` secrets;
    6. upload `release-windows-<arch>`, `dist/release/*`.
  - `macos` (macos-latest): `import-cert.sh`, then `package.sh --arch universal`, then `verify.sh --arch universal`; upload `release-macos`; remove the keychain under `always()`.
  - `linux` (matrix: x86_64 on ubuntu-22.04 with deb amd64, aarch64 on ubuntu-22.04-arm with deb arm64): the apt packages from the Dockerfile and `zsync`, then nfpm 2.47.0 with its SHA-256 checked, then `packaging/linux/package.sh`; upload `release-linux-<arch>`.
  - `flatpak` (matrix on ubuntu-24.04 and ubuntu-24.04-arm; needs `linux`): download the tarball, `apt install flatpak flatpak-builder librsvg2-common`, `sysctl kernel.apparmor_restrict_unprivileged_userns=0`, then `flatpak-bundle.sh <tarball>`; upload `release-flatpak-<arch>`.
  - `draft` (needs all; `if: !cancelled() && !contains(needs.*.result, 'failure')`):
    - download the `release-*` artifacts, merged;
    - write `sha256sum -- * > SHA256SUMS.txt` and upload it as `release-checksums`;
    - only on `refs/heads/release`, create or update the draft `v<version>`, titled "ReasonCraft v<version>", with `gh release`. Refuse if that tag's release is already published.
- The signing jobs set `environment: ${{ github.ref == 'refs/heads/release' && 'release' || '' }}`. Missing secrets give a warning, not a failure (ruling L3).
- `packaging-lint.yml` triggers on `paths: [.github/workflows/**, packaging/**, app/reasoncraft/assets/icon*]` and on dispatch.
  - `lint` (ubuntu-latest): actionlint 1.7.12 (tarball SHA-256 from its `checksums.txt`), `shellcheck packaging/**/*.sh`, `xmllint --noout` over the wxs, wxi, plist and Linux XML, `desktop-file-validate`, `appstreamcli validate --no-net` on the substituted metainfo, a PowerShell parse of `packaging/windows/*.ps1`, and the duplicate-GUID check.
  - `windows-install-policy` (windows-latest): WiX 5.0.2 into `$RUNNER_TEMP`, then `test-install-policy.ps1`. Adapted from WordCraft's `packaging-lint.yml`.

- [ ] **Step 1: Validate.**
  - `docker run --rm -v "${PWD}:/repo" -w /repo rhysd/actionlint:1.7.12 -color`. Expected: no output, exit 0.
  - `rg -n "storytold|craft-fonts|craft-i18n" .github/`. Expected: no matches.
  - `rg -n "uses: [^ ]+@[^0-9a-f]" .github/`. Expected: no unpinned actions.
- [ ] **Step 2: Rehearse the lint job locally.** In the Linux image, run its shell lines. On the host, run the `windows-install-policy` lines. Expected: PASS.
- [ ] **Step 3:** Commit `ci: self-contained release workflow (MSI, DMG, Linux, Flatpak, SHA256SUMS) and packaging lint`.

### Task 10: Docs, rulings and the release checklist

**Files:**
- Modify:
  - `docs/release.md`: rewrite for:
    - the version (`cargo xtask version set`);
    - the local prerequisites (.NET SDK 8, WiX 5.0.2 and its extensions, the Windows SDK only for signing, the MSVC ARM64 tools only for arm64, Docker, nfpm in the image);
    - each platform's script and outputs;
    - the install-policy checks (the meaning of WordCraft's INSTALLATION.md in a few lines);
    - the VM check (Task 4);
    - signing secrets and the `release` environment;
    - the release flow (bump on `main`, merge into `release`, review the draft, publish);
    - CI;
    - a section "Compatibility with the Craft suite's reusable workflow" (below);
    - the checklist, updated for MSI scope choice, the DMG, rpm, the Flatpak, and an AppImage on a second distribution.
  - `README.md` (Install), and the CHANGELOG 0.1.0 "Installing" section. 0.1.0 was never tagged or published, so it is edited in place, and no NSIS-to-MSI migration is needed.
  - `docs/rulings.md`: append "Packaging the ArtCraft way" with rulings PK-1…PK-8 (below). Add "(superseded by PK-n)" to L3 (format part only), S6-T11-3, S6-T11-4, S6-F2 and S6-F3's NSIS mention.
  - `docs/dev/todo-history.md`: tick the items, add the review section, and the carry-overs: the first CI run for macOS, Flatpak and arm64, the Mac check, the Apple Event, the DMG background, and an arm64 install smoke test on `windows-11-arm`.

Compatibility section, to state in `docs/release.md`. To switch to `storytold/craft-common-actions/.github/workflows/release.yml`:
- the caller passes `app-name: reasoncraft`, `app-title: ReasonCraft`, `craft-fonts-ref: ''`, and `has-web`, `has-riscv64`, `has-freebsd` and `has-arch` all `false`;
- every script path, argument and artifact name above already matches;
- `cargo xtask version set` and `packaging/env.sh` serve its `version` job;
- there is no `REASONCRAFT_*` environment entry upstream, which is not needed because the version comes from Cargo.toml after `version set`;
- its `git-sources` gate passes (no git dependencies, and `.cargo/config.toml` holds only an alias);
- its Linux job installs no `libgtk-3-dev`, which ReasonCraft does not need (rfd 0.17 uses the portal), but this has to be confirmed by building in a container without it;
- it always checks out craft-i18n, which is harmless;
- it **requires** signing secrets on the `release` branch, while ours only warns;
- ArtCraft's CI uses `gate.yml` with `cargo xtask ci`, which ReasonCraft would have to add to use it.

Rulings:
- **PK-1:** installers follow WordCraft's packaging. cargo-packager and NSIS are removed. Cost: more scripts to maintain (about 10 files), and no single tool.
- **PK-2:**
  - one dual-scope MSI per architecture (x64, x86, arm64), per user by default into `%LOCALAPPDATA%\Programs\ReasonCraft`, with "Everyone" (per machine) on request;
  - a cross-scope copy is refused;
  - Start menu shortcut only, no desktop shortcut;
  - English installer UI.
  
  Cost: the MSI's dialogs are in English only.
- **PK-3:** `.rcraft` keeps the ProgID `ReasonCraft.rcraft` and is made the default for `.rcraft` (our own type); it is also registered under Capabilities. The NSIS backup and restore of a previous handler is gone. Cost: none (Windows keeps the user's choice).
- **PK-4:** static CRT, and the packaging fails on a wrong PE machine, a non-GUI subsystem or a CRT import. Cost: a slightly larger exe.
- **PK-5:** the version lives only in `[workspace.package]`; `cargo xtask version set` changes it. Cost: none.
- **PK-6:**
  - Linux ships tar.gz, deb, rpm, AppImage with zsync, and a Flatpak bundle, for x86_64 and aarch64;
  - macOS ships a universal app on a plain DMG;
  - Arch, FreeBSD, web and riscv64 are out.
  
  Cost: no DMG background, and fewer platforms than the suite.
- **PK-7:**
  - signing runs when secrets exist and warns otherwise;
  - releases come from the `release` branch as a draft with `SHA256SUMS.txt`;
  - the workflows are self-contained, with nothing copied from the unlicensed craft-common-actions.
  
  Cost: a later switch means changing the caller.
- **PK-8:** WiX 5.0.2 (MS-RL) and the AppImage runtime (MIT, with libfuse LGPL-2.1) are listed in About › Licences and in THIRD-PARTY.md. Cost: two more licence entries.

- [ ] **Step 1:** Write the docs. Then run `rg -n "cargo packager|cargo-packager|NSIS|setup\.exe" README.md CHANGELOG.md docs/release.md docs/user-guide.md`. Expected: no matches.
- [ ] **Step 2:** Run the full gate once more. Expected: all PASS. Then commit `docs: release guide, rulings and checklist for the ArtCraft packaging`.
- [ ] **Step 3:** Ask for the whole-branch review (superpowers:requesting-code-review), then merge into `main` locally. Never push.

## Owner decisions (2026-10-28)
- Install folder: `Programs\ReasonCraft` (per user) and `Program Files\ReasonCraft` (per machine); no "ArtCraft" name anywhere (trademark).
- Maintainer: "ReasonCraft contributors", no email; no homepage URL. Where a tool demands a URL, leave it out and accept/suppress the warning with a comment.
- .NET 8 SDK (winget) and WiX 5.0.2 dotnet tool may be installed on this machine.
