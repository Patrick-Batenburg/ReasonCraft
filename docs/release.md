# Building a release

Every release artifact is built by a script in `packaging/`, one folder per platform:

| Platform | Script | Writes into `dist/release/` |
|---|---|---|
| Windows x64, x86, arm64 | `packaging/windows/package.ps1 -Arch <arch>` | `reasoncraft-<v>-windows-<arch>.msi`, `reasoncraft-<v>-windows-<arch>-portable.zip` |
| macOS (universal) | `packaging/macos/package.sh --arch universal` | `reasoncraft-<v>-macos-universal.dmg` |
| Linux x86_64, aarch64 | `packaging/linux/package.sh` | `reasoncraft-<v>-linux-<arch>.tar.gz`, `.deb`, `.rpm`, `.AppImage` (and `.AppImage.zsync` in GitHub Actions) |
| Linux Flatpak | `packaging/linux/flatpak-bundle.sh <tarball>` | `reasoncraft-<v>-linux-<arch>.flatpak` |

Each platform's artifacts are built on that platform (the Windows script cross-compiles x86 and arm64 on an x64 machine).
The scripts read the version from the workspace, build with `cargo build --release --locked`, honour `DIST`, `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS` (default 4: each parallel link needs a few gigabytes of memory), and add the Rust target they need with `rustup target add`.
The shell scripts share `packaging/env.sh`.
Several are adapted from WordCraft; each file's header says from where and what changed, and `THIRD-PARTY.md` lists them.

## The version

The version lives in one place: `version` under `[workspace.package]` in the root `Cargo.toml`.
Every crate, the exe's version information, `reasoncraft --version` and every script take it from there.

```sh
cargo xtask version                 # prints it
cargo xtask version set 0.2.0-rc.1  # changes it, then updates Cargo.lock
```

`version set` takes `X.Y.Z` or `X.Y.Z-pre` (no leading zeros, no `+build`), and refuses `v1.0`, `1.0` and `1.0.0+meta`.
A script also takes `REASONCRAFT_VERSION` from the environment, to package an existing build under another version (the VM check's older MSI does that).
Formats that only know numbers get the numeric part: the MSI's ProductVersion and macOS's `CFBundleShortVersionString` of `0.2.0-rc.1` are `0.2.0`, and the MSI allows a same-version upgrade, so one release candidate replaces the previous one.
That works in both directions: every pre-release of `0.2.0`, and `0.2.0` itself, is MSI version `0.2.0`, so running `rc.1`'s MSI over an installed `rc.2` installs the older build (ruling PK-18); install release candidates in order.
Windows Installer compares only major and minor (at most 255) and patch (at most 65535), so `package.ps1` refuses a version beyond those before it builds anything.
deb and rpm write it as `0.2.0~rc.1`, which sorts before `0.2.0` (nfpm's `version_schema: semver`); `packaging/linux/check-version-order.sh` checks that on placeholder packages with dpkg's and rpm's own comparison (packaging lint, or in the Docker image).

## Prerequisites on a development machine

- Rust through rustup; the toolchain is pinned in `rust-toolchain.toml`.
- Windows: PowerShell 7, the .NET SDK 8 (`winget install Microsoft.DotNet.SDK.8`) and WiX 5.0.2 with its UI and Util extensions:

  ```powershell
  dotnet tool install --global wix --version 5.0.2
  wix extension add -g WixToolset.UI.wixext/5.0.2 WixToolset.Util.wixext/5.0.2
  ```

  WiX stays at exactly 5.0.2: it is under the MS-RL, while WiX 6 and later come with the Open Source Maintenance Fee EULA.
  The workflows install it with `packaging/windows/install-wix.ps1 -ToolPath <folder>`, which checks the three `.nupkg` files against pinned SHA-256s and installs from that folder alone.
  The Windows SDK (signtool) is needed only for signing; the "MSVC v143 ARM64 build tools" only for `-Arch arm64`.
- Linux: Docker. The image `packaging/linux/Dockerfile` (on `ubuntu:22.04` pinned by digest) holds the build dependencies, the validators, nfpm 2.47.0 (checked against its SHA-256) and rustup, installed by `packaging/linux/install-rustup.sh` from a `rustup-init` checked against its pinned SHA-256 (the release workflow's arm64 Linux runner uses the same script).
  `package.sh` downloads appimagetool 1.9.1 and the AppImage type2-runtime 20251108 itself and checks both against pinned SHA-256s.
- macOS: Xcode's command line tools (codesign, notarytool, stapler, hdiutil, lipo).

## Windows

```powershell
pwsh packaging/windows/package.ps1 -Arch x64     # also x86 and arm64
```

The script builds the exe with a static C runtime (`+crt-static`), checks it, signs it (see [Signing](#signing)), builds the licence page (`cargo xtask licence-rtf`) and the MSI (`wix build`), checks the MSI's install policy, signs the MSI and writes the portable zip (`reasoncraft.exe`, `README.md`, the licences and `THIRD-PARTY.md`).
Before packaging, `pe-checks.ps1` refuses an exe whose PE machine is not the `-Arch` asked for, whose subsystem is not GUI, or that imports the C runtime (`vcruntime*`, `msvcp*`, `api-ms-win-crt-*`), so the zip needs no Visual C++ redistributable.
`-SkipBuild` packages the exe already built.
`packaging/windows/test-packaging-script.ps1` checks the script itself: the policy check comes before the MSI's single signing, the PE checks pass the release exe and refuse the debug exe, `sign.ps1` without secrets warns and exits 0 and with partly configured secrets warns or (on `release`) fails, versions beyond the MSI limits are refused, and a `0.2.0-rc.1` MSI has ProductVersion `0.2.0`.
It needs the x64 release exe (run `package.ps1 -Arch x64` first) and a debug build (`cargo build -p reasoncraft`).

The MSI (`packaging/windows/reasoncraft.wxs`, English UI):
- is one package for both scopes: for the current user by default, without administrator rights, into `%LOCALAPPDATA%\Programs\ReasonCraft`, or for everyone into `Program Files\ReasonCraft` when "Everyone on this computer" is chosen (or `ALLUSERS=1` on the msiexec command line); the app's own data, such as the recovery folder, lives apart in `%LOCALAPPDATA%\ReasonCraft`;
- adds a Start menu shortcut (no desktop shortcut) and an entry in Settings → Apps, published by "ReasonCraft contributors";
- makes `.rcraft` open with the ProgID `ReasonCraft.rcraft` ("ReasonCraft map", `"…\reasoncraft.exe" "%1"`), with the MIME type `application/x-reasoncraft`, and registers ReasonCraft under Default apps (`RegisteredApplications`);
- shows both licence texts on its licence page and offers to start ReasonCraft on its last page;
- runs silently with `msiexec /i <msi> /qn`; `msiexec /x <msi> /qn` uninstalls, and leaves the user's documents and the app's data alone.

### The install policy

The rules for installing in two scopes come from WordCraft's `install-policy.wxi`, and `check-install-policy.ps1` checks them in the compiled MSI, without installing it:
- An update stays where the existing copy is: in its scope and in its folder.
- A second copy in the other scope is refused. A newer build for everyone over a per-user copy (or the reverse) stops with "This app is already installed for your account" ("… for everyone"); the user uninstalls first to change scope. Running the very same package again in the other scope does not install anything either: Windows Installer finds the product already installed and only reconfigures that copy (maintenance mode, exit 0; checked in the VM).
- The scope page appears only for a new interactive install.
- Every registry value of the contract in `packaging/windows/install-policy.json` is written exactly once, under HKMU (HKCU per user, HKLM per machine).
- The UpgradeCode and the component GUIDs in `install-policy.json` never change.

`packaging/windows/test-install-policy.ps1 -OutputDirectory target/install-policy-tests` builds the real `reasoncraft.wxs` for x64, x86 and arm64 with text files in place of the exe (`*-NOT-INSTALLABLE.msi`, never to be installed) and checks each one; it needs WiX but no app build, and the packaging lint workflow runs it.
It also runs Windows Installer's validation (`wix msi validate`, the ICE checks) and fails on any message except the known ones it lists, each with its reason: ICE03 on the x86 package's `[%ProgramFiles(x86)]` default folder, ICE80 on the x86 package's searches of the 64-bit registry view, and the warnings ICE57 (registry under HKMU in a dual-scope package), ICE61 (same-version upgrades) and ICE69 (the file types name the exe of the other component, in the same feature).

### Checking the MSI in the test VM

`packaging/windows/verify-in-vm.ps1` (with its helpers in `vm-checks.ps1`) checks the MSI inside the Hyper-V VM "Windows 11 Sandbox".
It reaches the VM with PowerShell Direct, using the credential saved in `%USERPROFILE%\.vm-sandbox-cred.xml`, and it sends no keyboard or mouse input.
Run it from an elevated PowerShell 7 on the host; it never installs anything on the host.
The upgrade check needs an older MSI built from the same exe:

```powershell
cargo xtask sample-map target\sample.rcraft
$env:REASONCRAFT_VERSION = '0.0.1'; $env:DIST = 'target\vm-old'
pwsh packaging/windows/package.ps1 -Arch x64 -SkipBuild
Remove-Item Env:REASONCRAFT_VERSION, Env:DIST
.\packaging\windows\verify-in-vm.ps1 -Msi dist\release\reasoncraft-0.1.0-windows-x64.msi `
    -OlderMsi target\vm-old\reasoncraft-0.0.1-windows-x64.msi -Sample target\sample.rcraft -OutDir target\vm-check
```

The script:
1. takes the checkpoint `before-reasoncraft`, starts the VM, and turns on automatic sign-in and restarts the guest if nobody is signed in;
2. per user: installs the older MSI, checks that the new MSI for everyone (`ALLUSERS=1`) over it is refused with 1603 and "This app is already installed for your account" and adds nothing under HKLM, then installs the new MSI per user (a major upgrade) and checks the files in `%LOCALAPPDATA%\Programs\ReasonCraft`, the single ARP entry, the registry keys, the `.rcraft` association, the Start menu shortcut and `reasoncraft.exe --version`; the same MSI again with `ALLUSERS=1` only reconfigures the per-user copy (exit 0) and adds no second one;
3. opens `Één kaart.rcraft` through the association (the shell's open, as a double-click), checks that the path arrives as one quoted argument and the app stays up, and saves the VM screen; then starts the app with `WGPU_BACKEND=metal`, so that wgpu cannot start and OpenGL, which the VM lacks, is tried next, and checks that the error dialog shows;
4. uninstalls and checks that nothing is left except a map in Documents and the app's recovery folder;
5. per machine: the same install, upgrade and checks in `%ProgramW6432%\ReasonCraft` and HKLM, the reverse refusal ("already installed for everyone"), opening by association, and the uninstall;
6. always (also after a failure) turns the VM off, restores the checkpoint, deletes it, and leaves the VM off.

Windows Installer writes the ARP entry of a per-user install under HKLM as well, so the script tells the scopes apart by the product's assignment type, not by the hive.

#### x64 and x86 side by side

An x86 package on 64-bit Windows writes its per-machine keys in the 32-bit registry view (WOW6432Node), and a package's registry searches read only its own view, so `install-policy.wxi` repeats every HKLM search in the other view (ruling PK-18).
`packaging/windows/verify-cross-arch-in-vm.ps1 -X64Msi <x64.msi> -X86Msi <x86.msi> -OutDir target/vm-cross-arch` checks that in the same VM, with the same checkpoint routine: the x64 MSI for everyone and then the x86 MSI only for me is refused (and the reverse), with nothing installed per user.
With `-BaseX86Msi` (an x86 MSI built without the fix) it also shows the old behaviour: refused only through the App Paths key, which both views share, and installed as a second copy once that key is gone.
The transcript (`msi-vm-transcript.txt`), the msiexec logs and the screenshots land in `-OutDir`.
The script needs no changes to the VM's settings; the checkpoint undoes everything it changes inside the guest.

## macOS

```sh
packaging/macos/package.sh --arch universal
packaging/macos/verify.sh --arch universal
```

`package.sh` builds `aarch64-apple-darwin` and `x86_64-apple-darwin` (macOS 11 or later), joins them with `lipo` into `ReasonCraft.app` (bundle ID `nl.reasoncraft.app`, with the `.rcraft` document type), signs it inside-out with the hardened runtime, and puts it on a plain DMG with a link to Applications.
Without a signing identity it signs ad hoc, which runs only on the Mac that built it.
With a Developer ID identity and all three Apple credentials it also notarizes the DMG and staples the ticket; with the identity alone it signs and warns that it did not notarize.
`verify.sh` checks the DMG as users download it: strictly when the identity and the notarization credentials are all set, and otherwise only the signature's integrity, with warnings.
`import-cert.sh` (CI only) imports the certificate into a temporary keychain.

There is no Mac to build or test on yet: the release workflow builds on `macos-latest`, and a check on a real Mac is part of the release checklist.
Known gap: macOS hands a double-clicked document to an app through an Apple Event, not on the command line, and the app does not handle that event yet (ruling S6-T11-2).
Double-clicking a `.rcraft` file starts ReasonCraft without opening the file.

## Linux

```sh
packaging/linux/package.sh [--skip-build] [--formats "appimage deb rpm tar"]
```

The script builds for the host's architecture (x86_64 or aarch64), stages one FHS tree and makes every format from it:
- the binary, stripped, as `/usr/bin/reasoncraft`;
- the desktop entry `nl.reasoncraft.app.desktop` (`Exec=reasoncraft %F`, `MimeType=application/x-reasoncraft;`), the shared-mime-info file that makes `*.rcraft` files `application/x-reasoncraft`, the AppStream metainfo, and the icons in `hicolor`;
- the readme, the licences and `THIRD-PARTY.md` in `/usr/share/doc/reasoncraft/`.

`desktop-file-validate` and `appstreamcli validate` check the staged files when they are installed.
The binary links only libc: winit, wgpu and glow open the X11, Wayland, xkbcommon, EGL and OpenGL libraries at run time, so the `.deb` and `.rpm` name them in `packaging/linux/nfpm.yaml`, and a test checks both lists.
The `.deb` names Debian's packages; the `.rpm` names the libraries by soname (`libxkbcommon.so.0()(64bit)`, …), which Fedora and openSUSE both resolve to their own package names (ruling PK-15).
Both require glibc 2.35 or later (`libc6 (>= 2.35)`, `libc.so.6(GLIBC_2.35)(64bit)`): built on Ubuntu 22.04, the binary's newest glibc symbol version is `GLIBC_2.35` (`hypot`; check with `objdump -T reasoncraft | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1` and update both lists and their test when the build image changes).
Distributions with an older glibc refuse the packages instead of installing a binary that does not start: Debian 11, Ubuntu 20.04, RHEL 8 and 9 (and their rebuilds), openSUSE Leap 15 and Fedora 35 or older; the AppImage and the tarball do not run there either, the Flatpak does.
The packages' maintainer is `REASONCRAFT_MAINTAINER` (default "ReasonCraft contributors").
The AppImage embeds the pinned static runtime, so users need no libfuse2; it is built with `APPIMAGE_EXTRACT_AND_RUN=1`, so the build needs no FUSE either.
In GitHub Actions it also carries update information and gets a `.zsync` beside it for delta updates; a local build has neither.

`packaging/linux/flatpak-bundle.sh` repackages the tarball as a Flatpak bundle (runtime `org.freedesktop.Platform` 25.08, no network access) and, unless `--no-test`, installs it per user and runs `reasoncraft --version` inside the sandbox.
It needs flatpak, flatpak-builder and unprivileged user namespaces.

### Building and testing in Docker

Build the image from the repository root, so that it picks up `rust-toolchain.toml` (`Dockerfile.dockerignore` keeps the context to that file):

```sh
docker build -f packaging/linux/Dockerfile -t reasoncraft-linux .
```

Then run the tests and build the packages in a copy of the checked-in files, with the `target` folder in a volume (not in the host's checkout):

```sh
docker run --rm -m 16g -v "$PWD:/repo:ro" -v rc-target:/src/target -v "$PWD/target/linux-dist:/src/dist" reasoncraft-linux bash -c '
  git config --global --add safe.directory /repo &&
  git -C /repo ls-files -z | tar -C /repo --null -T - -cf - | tar -xf - -C /src &&
  cargo test --workspace -j 4 &&
  packaging/linux/package.sh &&
  dpkg-deb --info dist/release/*.deb && rpm -qip dist/release/*.rpm &&
  APPIMAGE_EXTRACT_AND_RUN=1 dist/release/*.AppImage --version'
```

The packages land in `target/linux-dist/release/` on the host.
`-m 16g` caps the container's memory and can be changed to suit the machine; with less memory, also lower `-j 4`.
The tests are headless and need no display.
The clipboard is checked only through the tests' fake clipboard; the real X11 or Wayland clipboard is a manual check (see the release checklist).

flatpak-builder does not run in a plain Docker Desktop container (no user namespaces).
A local Flatpak check needs a privileged `ubuntu:24.04` container with flatpak, flatpak-builder and `librsvg2-common`, run under `dbus-run-session`; the release workflow builds it on every run.

## Signing

Signing is optional (ruling L3): without secrets every script warns and ships unsigned, also on the `release` branch, and SmartScreen and Gatekeeper then warn users.
A set of secrets is used whole or not at all: on the `release` branch the workflow sets `REASONCRAFT_REFUSE_PARTIAL_SIGNING=1`, and a set that is only partly there (five of the six `AZURE_*`, a certificate password without its certificate, or the Apple certificate without all three notarization credentials, or the reverse) fails the job instead of shipping unsigned or signed but not notarized (ruling PK-16).
Elsewhere such a set gives a warning naming the missing secrets (never a value).
Every job's summary says how its files came out: signed and with what, notarized or not, or unsigned and why.
The release workflow takes the secrets from the GitHub environment `release`, which only the `release` branch uses; create it with the secrets below and, if wanted, required reviewers.

| Secret | Used by |
|---|---|
| `WINDOWS_CERTIFICATE` (base64 .pfx), `WINDOWS_CERTIFICATE_PASSWORD` | `sign.ps1`, with signtool and a certificate file |
| `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT`, `AZURE_CERT_PROFILE` | `sign.ps1`, with Azure Trusted Signing (all six, or it counts as none) |
| `APPLE_CERTIFICATE` (base64 .p12), `APPLE_CERTIFICATE_PASSWORD`, `KEYCHAIN_PASSWORD` | `import-cert.sh`: the Developer ID identity |
| `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID` | `package.sh` and `verify.sh`: notarization |

`sign.ps1` signs the exe and the MSI with the description "ReasonCraft"; `REASONCRAFT_SIGN_URL` adds a "more information" link (`/du`), which is left out until there is a homepage.

## The release flow

1. On `main`: `cargo xtask version set X.Y.Z`, update `CHANGELOG.md`, and commit once the [checklist](#release-checklist)'s first two items hold.
2. Merge `main` into the `release` branch and push it.
   `.github/workflows/release.yml` builds every artifact, writes `SHA256SUMS.txt`, and creates (or refreshes) the draft release `vX.Y.Z`, titled "ReasonCraft vX.Y.Z", marked as a pre-release for `X.Y.Z-pre`.
   It refuses to touch a release of that tag that is already published.
3. Download the draft's files and work through the rest of the checklist.
4. Publish the draft; GitHub then creates the tag `vX.Y.Z` on the commit the workflow built.

For a dry run, start the release workflow by hand (workflow_dispatch) on any branch, optionally with a version such as `0.2.0-rc.1`: it builds everything under that version (`cargo xtask version set` inside the runners only) and keeps the files as workflow artifacts (`release-windows-x64`, …, `release-checksums`); only a run on `release` drafts a release.
On `release` the version input must stay empty: the workflow refuses it, because the draft's tag would then not match the commit's `Cargo.toml`.

## Continuous integration

- `.github/workflows/ci.yml` runs the quality gate (`cargo fmt --check`, clippy with `-D warnings`, `cargo doc --workspace --no-deps --document-private-items` with `RUSTDOCFLAGS=-D warnings`, `cargo test --workspace` and `cargo check --release --all-targets`) on `windows-latest`, `macos-latest` and `ubuntu-latest`, with the toolchain from `rust-toolchain.toml`.
  On Linux it first installs egui's build dependencies, as the Docker image does.
  A Linux job runs `cargo deny check` and checks that `crates.txt` is current (see [Licence notices](#licence-notices)).
  Run by hand, it also runs the timing tests: the `#[ignore]`d tests, in a release build (`cargo test --workspace --release -j 4 -- --ignored`).
  The tests marked `#[ignore = "manual: …"]` write sample files for looking at by eye when their environment variable is set, and do nothing otherwise.
- `.github/workflows/packaging-lint.yml` runs when the workflows, `packaging/` or the icons change: actionlint (1.7.12, checked by SHA-256), shellcheck on `packaging/**/*.sh`, xmllint on the WiX, plist and Linux XML files, `desktop-file-validate`, `appstreamcli validate` on the metainfo, `check-version-order.sh` (nfpm 2.47.0, checked by SHA-256), a PowerShell parse of `packaging/windows/*.ps1`, a check that no WiX GUID is used twice, and, on Windows, `test-install-policy.ps1`.
- `.github/workflows/release.yml` is described [above](#the-release-flow). Its jobs: `version` (the input or Cargo.toml's version, checked), `windows` (x64, x86 and arm64; x64 also runs `test-packaging-script.ps1`), `macos`, `linux` (x86_64 on `ubuntu-22.04`, aarch64 on `ubuntu-22.04-arm`, so the binaries need only glibc 2.35), `flatpak` (both architectures, on Ubuntu 24.04) and `draft`.

The workflows are self-contained: they use no reusable workflow and nothing outside this repository.
Every release job has a time limit (`timeout-minutes`; macOS 90 minutes, for two builds and two notarizations), and the draft is made only when every job succeeded.
Their actions are pinned to commit SHAs, with the release tag in a comment; to upgrade one, look up the new tag's commit (`gh api repos/<owner>/<repo>/commits/<tag> --jq .sha`).

### Compatibility with the Craft suite's reusable workflow

The scripts keep to the interface of the Craft suite's reusable release workflow (`storytold/craft-common-actions`, `.github/workflows/release.yml`), so `release.yml` could later become a short caller of it without changing a script:
- the caller passes `app-name: reasoncraft`, `app-title: ReasonCraft`, `craft-fonts-ref: ''`, and `has-web`, `has-riscv64`, `has-freebsd` and `has-arch` all `false`;
- every script path, argument and artifact name above already matches;
- `cargo xtask version set` and `packaging/env.sh` serve its `version` job;
- it sets no `REASONCRAFT_*` variable, which is not needed: the version comes from Cargo.toml after `version set`;
- its `git-sources` gate passes: there are no git dependencies, and `.cargo/config.toml` holds only the `xtask` alias;
- its Linux job installs no `libgtk-3-dev`, which ReasonCraft should not need (rfd 0.17 uses the XDG portal); confirm that by building in a container without it before switching;
- it always checks out craft-i18n, which is harmless (the scripts ignore `CRAFT_I18N_DIR`, as they do `CRAFT_FONTS_DIR`);
- it **requires** the signing secrets on the `release` branch, where ours only warns;
- the suite's CI uses its `gate.yml` with `cargo xtask ci`, which ReasonCraft would have to add to use it.

## Licence notices

The About page shows the full licence texts of everything ReasonCraft bundles (File > About > Licences), embedded in the binary.
The texts of the fonts, the spell checker and the installer parts (WiX's dialogs and custom action in the MSI, and the AppImage runtime with the six libraries it links statically, in `appimage-runtime/`) live in `app/reasoncraft/assets/licences/`; the dictionaries' come from `crates/reasoncraft_spell/dict/*/LICENSE`.
The installers also carry `LICENSE-MIT`, `LICENSE-APACHE` and `THIRD-PARTY.md`.
The Rust crates' licences are in `app/reasoncraft/assets/licences/crates.txt`, generated with [cargo-about](https://github.com/EmbarkStudios/cargo-about) from the app's run-time dependencies on every release target (configuration: `xtask/about.toml`).
Regenerate and commit it whenever `Cargo.lock` changes (CI regenerates it and fails if that changes the file):

```sh
cargo install cargo-about --version 0.9.3 --locked --features cli
cargo xtask notices
```

The licences accepted are MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, BSL-1.0 and Unicode-3.0; MPL-2.0 only for `spellbook` (ruling Y2), and OFL-1.1 and the Ubuntu Font Licence only for egui's bundled fonts (`epaint_default_fonts`, ruling S6-T13-4).
`xtask/about.toml` and `deny.toml` hold the same list and the same targets (Windows x64, x86 and arm64, Linux x86_64 and aarch64, macOS x86_64 and aarch64); change them together.
[cargo-deny](https://github.com/EmbarkStudios/cargo-deny) checks it in CI, with the RustSec advisories and the crates' sources (`cargo deny check`; an advisory that does not apply is ignored in `deny.toml` with its reason).

## App icon

The master is `app/reasoncraft/assets/icon.svg`, an original design.
After changing it, regenerate the PNG sizes (`assets/icons/icon-<size>.png`), the window icon (`assets/icon.png`), `assets/icon.ico` (16, 32, 48 and 256 px) and `assets/icon.icns`, and commit them:

```sh
cargo xtask icons
```

The `xtask` crate is a developer tool and is never shipped.
On Windows, `app/reasoncraft/build.rs` puts the icon and the version information (product name, publisher, version from `Cargo.toml`) into the exe.

## Renderer

The app draws with wgpu first. Where no GPU driver is usable, wgpu falls back to software rendering (WARP on Windows), so the app also runs in VMs and remote sessions. If wgpu cannot open the window, the app tries OpenGL (glow). If that fails too, it shows an error dialog in the OS language instead of quitting silently (`src/startup.rs`).

## Not published to crates.io

The `reasoncraft` crate has `publish = false`.
Its About page embeds the workspace's `THIRD-PARTY.md` with `include_str!` from outside the crate folder, which `cargo package` would reject.
The packaging scripts build from the workspace, so the installers are unaffected.

## Release checklist

1. The CI and packaging lint workflows are green on all platforms, `macos-latest` included, for the commit to be released; run CI once by hand too, for the timing tests (or run `cargo test --workspace --release -j 4 -- --ignored` locally).
2. `THIRD-PARTY.md` and the crates' licence texts (`cargo xtask notices`, see [Licence notices](#licence-notices)) are current: running it leaves no change to commit.
3. Merge into `release` and wait for the draft release; every job is green and the draft holds the files of the table at the top (three MSIs and zips, the DMG, two each of tar.gz, deb, rpm, AppImage, `.zsync` and Flatpak) and `SHA256SUMS.txt`, and `sha256sum -c SHA256SUMS.txt` passes on the downloads.
   The run's summary says, per job, how the files were signed; it is what was intended (signed and notarized once the secrets exist, otherwise unsigned).
   Once signing secrets exist, check the downloads themselves: on Windows `Get-AuthenticodeSignature reasoncraft-<v>-windows-x64.msi` (and the exe inside the zip) shows `Valid` with the expected signer and a timestamp; on a Mac `spctl --assess --type open --context context:primary-signature -vv reasoncraft-<v>-macos-universal.dmg` says `source=Notarized Developer ID`, and `xcrun stapler validate` passes on the DMG.
4. Windows:
   - check the x64 MSI in the test VM (see above): `0 failure(s)`, the VM left off without its checkpoint;
   - check the x64 and x86 MSIs together in the VM (`verify-cross-arch-in-vm.ps1`, see above): `0 failure(s)`, the VM left off without its checkpoint;
   - run it once interactively there: the licence page shows both licences, "Only for me" installs without asking for an administrator, and the finish page starts ReasonCraft;
   - run it once more and choose "Everyone on this computer": it asks for an administrator and installs into Program Files (after uninstalling the per-user copy);
   - start the portable zip's exe on a machine without the Visual C++ redistributable;
   - on Windows on Arm (a `windows-11-arm` runner or a device), install the arm64 MSI and start the app.
5. macOS, on a real Mac (CI cannot show a window):
   - open the DMG, drag the app to Applications, and start it (unsigned: right-click → Open the first time; signed and notarized: no warning);
   - make, save and reopen a map; export it as PNG and PDF; copy a map as an image and paste it into another app;
   - check that Quit with unsaved changes asks first;
   - check the known gap: double-clicking a `.rcraft` file starts the app without opening the file.
6. Linux, on a desktop (X11 and Wayland if possible):
   - install the `.deb` with `sudo apt install ./reasoncraft-<v>-linux-x86_64.deb` and start ReasonCraft from the app menu;
   - install the `.rpm` on Fedora (`sudo dnf install ./reasoncraft-<v>-linux-x86_64.rpm`) and on openSUSE Tumbleweed or Leap 16 (Leap 15's glibc is too old; `sudo zypper install ./reasoncraft-<v>-linux-x86_64.rpm`), and start it;
   - install the Flatpak with `flatpak install --user reasoncraft-<v>-linux-x86_64.flatpak` and start it; open and save a map in Documents;
   - start the AppImage on a second distribution;
   - double-click a `.rcraft` file: it opens in ReasonCraft;
   - copy a map as an image and paste it into another app, and copy and paste text between ReasonCraft and another app (the real clipboard);
   - open and save through the file dialogs.
7. At 200 % interface size (File > Settings), look over the ribbon tabs, the File pages, the side panels and a dialog in both languages: nothing is cut off or overlaps.
8. Publish the draft.
