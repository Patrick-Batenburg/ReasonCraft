# Building a release

ReasonCraft's installers are made with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager).
Its configuration is `[package.metadata.packager]` in `app/reasoncraft/Cargo.toml`; the files it uses live in `app/reasoncraft/packaging/`.
Each platform's installers are built on that platform.

```sh
cargo install cargo-packager --version 0.11.8 --locked
```

The version is pinned in this file, in `.github/workflows/release.yml` and in `packaging/linux/Dockerfile`.
The Windows installer's NSIS script is vendored from this version (see below), so bumping it means re-vendoring `packaging/windows/installer.nsi` from the new version's template, with the marked changes, and changing all three places together.

## Licence notices

The About page shows the full licence texts of everything ReasonCraft bundles (File > About > Licences), embedded in the binary.
The texts of the fonts, the spell checker and the installer live in `app/reasoncraft/assets/licences/`; the dictionaries' come from `crates/reasoncraft_spell/dict/*/LICENSE`.
The Rust crates' licences are in `app/reasoncraft/assets/licences/crates.txt`, generated with [cargo-about](https://github.com/EmbarkStudios/cargo-about) from the app's run-time dependencies on all release platforms (configuration: `xtask/about.toml`).
Regenerate and commit it whenever `Cargo.lock` changes:

```sh
cargo install cargo-about --version 0.9.3 --locked --features cli
cargo run -p xtask -- notices
```

## App icon

The master is `app/reasoncraft/assets/icon.svg`, an original design.
After changing it, regenerate the PNG sizes (`assets/icons/icon-<size>.png`), the window icon (`assets/icon.png`), `assets/icon.ico` (16, 32, 48 and 256 px) and `assets/icon.icns`, and commit them:

```sh
cargo run -p xtask -- icons
```

The `xtask` crate is a developer tool and is never shipped.
On Windows, `app/reasoncraft/build.rs` puts the icon and the version information (product name, publisher, version from `Cargo.toml`) into the exe.

## Windows

```sh
cargo packager --release -p reasoncraft --formats nsis
```

This builds the release exe and writes `target/release/packager/reasoncraft_<version>_x64-setup.exe`.
cargo-packager names the installer after the binary, hence the lower case.
It downloads NSIS the first time.

The installer:
- installs for the current user only, into `%LOCALAPPDATA%\Programs\ReasonCraft`, without asking for administrator rights (the app's own data, such as the recovery folder, lives apart in `%LOCALAPPDATA%\ReasonCraft`);
- adds Start menu and desktop shortcuts and an entry in Settings → Apps;
- associates `.rcraft` with the ProgID `ReasonCraft.rcraft` (shown as "ReasonCraft map") under `HKCU\Software\Classes`, which opens `"reasoncraft.exe" "%1"`; a reinstall or upgrade keeps the record of what `.rcraft` opened with before;
- shows both licence texts on its licence page (`packaging/LICENSE.txt`, written by `cargo run -p xtask -- licence`);
- runs silently with `/S`. The uninstaller (`uninstall.exe /S`) removes the files, the shortcuts, the association (restoring whatever `.rcraft` opened with before) and its registry keys, and leaves the user's documents and the app's data alone.

The NSIS script is cargo-packager's own template, kept as `packaging/windows/installer.nsi` with a few marked changes:
the ProgID is `<product>.<ext>`; the uninstaller removes the file association (the upstream template never does), also after a reinstall, and the publisher and product registry keys; the exe path in the association is quoted; the per-user install goes under `Programs`; and the ApplicationID plugin is not used.
It is vendored from cargo-packager 0.11.8: when you upgrade cargo-packager, re-vendor it from the new version's template and carry the marked changes over.
The installer uses zlib compression, whose decompressor is under the zlib licence.

### Checking the installer in the test VM

`packaging/windows/verify-in-vm.ps1` checks the installer inside the Hyper-V VM "Windows 11 Sandbox".
It reaches the VM with PowerShell Direct, using the credential saved in `%USERPROFILE%\.vm-sandbox-cred.xml`, and it sends no keyboard or mouse input.

```powershell
cargo run -p xtask -- sample-map target\sample.rcraft
.\app\reasoncraft\packaging\windows\verify-in-vm.ps1 `
    -Installer target\release\packager\reasoncraft_0.1.0_x64-setup.exe `
    -Sample target\sample.rcraft -OutDir target\vm-check
```

The script:
1. takes the checkpoint `before-reasoncraft` and starts the VM;
2. turns on automatic sign-in and restarts the guest, if nobody is signed in;
3. installs silently twice (as an upgrade does), then checks the files, the shortcuts, the association and the registry keys;
4. opens the sample through the association, checks the app's command line, and saves the VM screen;
5. starts the app with `WGPU_BACKEND=metal`, so that wgpu cannot start and OpenGL, which the VM lacks, is tried next, and checks that the error dialog shows;
6. uninstalls silently, then checks that everything is gone except a map in Documents and the app's data;
7. turns the VM off, restores the checkpoint, and deletes it.

The script needs no changes to the VM's settings.
The checkpoint undoes everything it changes inside the guest.

## Renderer

The app draws with wgpu first. Where no GPU driver is usable, wgpu falls back to software rendering (WARP on Windows), so the app also runs in VMs and remote sessions. If wgpu cannot open the window, the app tries OpenGL (glow). If that fails too, it shows an error dialog in the OS language instead of quitting silently (`src/startup.rs`).

## macOS

```sh
cargo packager --release -p reasoncraft --formats app,dmg
```

There is no Mac to build or test on yet: the CI and release workflows build and test on `macos-latest`, and a check on a real Mac is part of the release checklist.

The bundle's `Info.plist` declares the `.rcraft` document type (role Editor).
Known gap: macOS hands a double-clicked document to an app through an Apple Event, not on the command line, and the app does not handle that event yet.
Double-clicking a `.rcraft` file starts ReasonCraft without opening the file.

## Linux

```sh
cargo packager --release -p reasoncraft --formats deb,appimage
```

The desktop entry (`packaging/linux/reasoncraft.desktop`) passes the files as `%F` and declares `MimeType=application/x-reasoncraft;`.
`packaging/linux/reasoncraft-mime.xml` installs to `/usr/share/mime/packages/` and tells shared-mime-info that `*.rcraft` files are `application/x-reasoncraft`.

The binary links only libc: winit, wgpu and glow open the X11, Wayland, xkbcommon, EGL and OpenGL libraries at run time, so dpkg cannot work out the dependencies.
The `.deb` names them in `depends` under `[package.metadata.packager.deb]`; a test checks the list.
Its package name is `reasoncraft` (cargo-packager would otherwise make `reason-craft` of the product name).

To build without FUSE (in a container, or on CI), set `APPIMAGE_EXTRACT_AND_RUN=1` for the AppImage.

### Building and testing in Docker

`packaging/linux/Dockerfile` is a Debian bookworm image with the pinned toolchain, egui's build dependencies and cargo-packager.
Build it from the repository root, so that it picks up `rust-toolchain.toml`:

```sh
docker build -f app/reasoncraft/packaging/linux/Dockerfile -t reasoncraft-linux .
```

Then run the tests and build the `.deb` in a copy of the checked-in files, with the `target` folder in a volume (not in the host's checkout):

```sh
docker run --rm -m 16g -v "$PWD:/repo:ro" -v rc-target:/src/target reasoncraft-linux bash -c '
  git config --global --add safe.directory /repo &&
  git -C /repo ls-files -z | tar -C /repo --null -T - -cf - | tar -xf - -C /src &&
  cargo test --workspace -j 4 &&
  cargo packager --release -p reasoncraft --formats deb'
```

The `.deb` lands in the volume, under `/src/target/release/packager/`.

`-m 16g` caps the container's memory and can be changed to suit the machine; with less memory, also lower `-j 4`, as each parallel link needs a few gigabytes.

The tests are headless and need no display.
The clipboard is checked only through the tests' fake clipboard; copying to and pasting from the real X11 or Wayland clipboard is a manual check (see the release checklist).

## Not published to crates.io

The `reasoncraft` crate has `publish = false`.
Its About page embeds the workspace's `THIRD-PARTY.md` with `include_str!` from outside the crate folder, which `cargo package` would reject.
cargo-packager builds from the workspace, so the installers are unaffected.
`THIRD-PARTY.md` lists every bundled third-party component, including the installer's.

## Continuous integration

`.github/workflows/ci.yml` runs the quality gate (`cargo fmt --check`, clippy with `-D warnings`, `cargo test --workspace`) on `windows-latest`, `macos-latest` and `ubuntu-latest`, with the toolchain from `rust-toolchain.toml`.
On Linux it first installs egui's build dependencies, the same as the Docker image.

`.github/workflows/release.yml` runs on a tag `v*` (or by hand).
It builds the NSIS installer on Windows, the app bundle and `.dmg` on macOS, and the `.deb` and AppImage on Linux, and keeps them as workflow artifacts.
The artifacts are named after the tag, or after the commit for a run by hand.
It publishes nothing.

## Release checklist

1. The CI workflow is green on all three platforms, `macos-latest` included, for the commit to be tagged.
2. `THIRD-PARTY.md`, `packaging/LICENSE.txt` (`cargo run -p xtask -- licence`) and the crates' licence texts (`cargo run -p xtask -- notices`, see [Licence notices](#licence-notices)) are current: running both leaves no change to commit.
3. Tag the version (`v<version>`) and download the installers from the release workflow's run.
4. Windows: check the installer in the test VM (see above), then run it once interactively there (not with `/S`) and check that its licence page shows both licences.
5. macOS, on a real Mac (CI cannot show a window):
   - open the `.dmg`, drag the app to Applications, and start it (it is not signed, so open it with right-click → Open the first time);
   - make, save and reopen a map; export it as PNG and PDF; copy a map as an image and paste it into another app;
   - check that Quit with unsaved changes asks first;
   - check the known gap: double-clicking a `.rcraft` file starts the app without opening the file.
6. Linux, on a desktop (X11 and Wayland if possible):
   - install the `.deb` with `sudo apt install ./reasoncraft_<version>_amd64.deb` and start ReasonCraft from the app menu;
   - double-click a `.rcraft` file: it opens in ReasonCraft;
   - copy a map as an image and paste it into another app, and copy and paste text between ReasonCraft and another app (the real clipboard);
   - open and save through the file dialogs;
   - start the AppImage on a second distribution.
7. At 200 % interface size (File > Settings), look over the ribbon tabs, the File pages, the side panels and a dialog in both languages: nothing is cut off or overlaps.
