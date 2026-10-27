# Building a release

ReasonCraft's installers are made with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager).
Its configuration is `[package.metadata.packager]` in `app/reasoncraft/Cargo.toml`; the files it uses live in `app/reasoncraft/packaging/`.
Each platform's installers are built on that platform.

```sh
cargo install cargo-packager --locked
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
- associates `.rcraft` with the ProgID `ReasonCraft map` under `HKCU\Software\Classes`, which opens `"reasoncraft.exe" "%1"`;
- runs silently with `/S`. The uninstaller (`uninstall.exe /S`) removes the files, the shortcuts and the association, and leaves the user's documents alone.

The NSIS script is cargo-packager's own template, kept as `packaging/windows/installer.nsi` with a few marked changes:
the uninstaller removes the file association (the upstream template never does), the exe path in the association is quoted, the per-user install goes under `Programs`, and the ApplicationID plugin is not used.
When you upgrade cargo-packager, compare that file with the upstream template.
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
3. installs silently, then checks the files, the shortcuts and the association;
4. opens the sample through the association, checks the app's command line, and saves the VM screen;
5. uninstalls silently, then checks that everything is gone except a map in Documents;
6. turns the VM off, restores the checkpoint, and deletes it.

The script needs no changes to the VM's settings.
The checkpoint undoes everything it changes inside the guest.

## Renderer

The app draws with wgpu first. Where no GPU driver is usable, wgpu falls back to software rendering (WARP on Windows), so the app also runs in VMs and remote sessions. If wgpu cannot open the window, the app tries OpenGL (glow). If that fails too, it shows an error dialog in the OS language instead of quitting silently (`src/startup.rs`).

## macOS

```sh
cargo packager --release -p reasoncraft --formats app,dmg
```

The bundle's `Info.plist` declares the `.rcraft` document type (role Editor).
Known gap: macOS hands a double-clicked document to an app through an Apple Event, not on the command line, and the app does not handle that event yet.
Double-clicking a `.rcraft` file starts ReasonCraft without opening the file.

## Linux

```sh
cargo packager --release -p reasoncraft --formats deb,appimage
```

The desktop entry (`packaging/linux/reasoncraft.desktop`) passes the files as `%F` and declares `MimeType=application/x-reasoncraft;`.
`packaging/linux/reasoncraft-mime.xml` installs to `/usr/share/mime/packages/` and tells shared-mime-info that `*.rcraft` files are `application/x-reasoncraft`.

## Not published to crates.io

The `reasoncraft` crate has `publish = false`.
Its About page embeds the workspace's `THIRD-PARTY.md` with `include_str!` from outside the crate folder, which `cargo package` would reject.
cargo-packager builds from the workspace, so the installers are unaffected.
`THIRD-PARTY.md` lists every bundled third-party component, including the installer's.
