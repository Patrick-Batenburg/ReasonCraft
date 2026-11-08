# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/package.ps1 at aadb077),
# MIT OR Apache-2.0, used under Apache-2.0; changed: one app exe (no CLI and no CLI smoke run), no
# craft-fonts, PE checks in pe-checks.ps1 (also the C runtime imports), the licence RTF from
# `cargo xtask licence-rtf`, REASONCRAFT_VERSION, rustup target add, CARGO_BUILD_JOBS.

<#
.SYNOPSIS
  Build, sign and package ReasonCraft for Windows.

.DESCRIPTION
  Produces, in $env:DIST (default: dist\release):
    reasoncraft-<version>-windows-<arch>.msi            installer (WiX 5), one dual-scope package:
                                                        for the current user by default (no admin
                                                        rights), "Everyone" per machine on request
    reasoncraft-<version>-windows-<arch>-portable.zip   reasoncraft.exe, README, licences, notices

  The exe links the C runtime statically (+crt-static), so neither needs the Visual C++
  redistributable; pe-checks.ps1 refuses an exe for another machine, a console exe or one that
  imports the C runtime. Signing is delegated to sign.ps1 (skipped with a warning when no signing
  secrets are set).

  -BuildOnly builds the exe (and the xtask the packaging runs) and stops; -SkipBuild then packages
  that build without compiling anything. The release workflow runs them as two steps, so that no
  build script or proc macro ever runs with the signing secrets in its environment.

  Environment: REASONCRAFT_VERSION (default: [workspace.package] version in Cargo.toml), DIST,
  CARGO_TARGET_DIR, CARGO_BUILD_JOBS (default 4).

  Needs: Rust (the MSVC toolchain; the script adds the target), the Windows SDK only for signing,
  and WiX 5.0.2 with its UI and Util extensions:
    dotnet tool install --global wix --version 5.0.2
    wix extension add -g WixToolset.UI.wixext/5.0.2 WixToolset.Util.wixext/5.0.2

.EXAMPLE
  pwsh packaging/windows/package.ps1 -Arch x64
  pwsh packaging/windows/package.ps1 -Arch x86 -SkipBuild
  pwsh packaging/windows/package.ps1 -Arch x64 -BuildOnly    # then -SkipBuild with the secrets
  pwsh packaging/windows/package.ps1 -Arch arm64     # cross-compiled; needs the MSVC ARM64 build tools
#>
param(
  [ValidateSet('x64', 'x86', 'arm64')] [string] $Arch = 'x64',
  [switch] $SkipBuild,
  [switch] $BuildOnly
)
$ErrorActionPreference = 'Stop'
if ($SkipBuild -and $BuildOnly) { throw '-SkipBuild and -BuildOnly exclude each other.' }
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
. (Join-Path $PSScriptRoot 'pe-checks.ps1')

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

# The version lives in one place: [workspace.package] version in the root Cargo.toml.
$Version = $env:REASONCRAFT_VERSION
if (-not $Version) {
  $inPkg = $false
  foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
    if ($line -match '^\s*\[') { $inPkg = ($line.Trim() -eq '[workspace.package]'); continue }
    if ($inPkg -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $Version = $Matches[1]; break }
  }
}
if (-not $Version) { throw 'could not read [workspace.package] version from Cargo.toml' }
# MSI ProductVersion is numeric (major.minor.build); pre-release tags are dropped there, so all
# pre-releases of one X.Y.Z and X.Y.Z itself share one MSI version. The MSI allows same-version
# upgrades, so each replaces whichever is installed, in either direction: rc.1 run over rc.2
# installs the older build (ruling PK-18). Windows Installer compares only major (<= 255), minor
# (<= 255) and build (<= 65535); a larger number would not upgrade reliably, so it is refused here.
$MsiVersion = ($Version -split '-')[0]
if ($MsiVersion -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
  throw "Version '$Version' is not X.Y.Z or X.Y.Z-pre."
}
if ([long]$Matches[1] -gt 255 -or [long]$Matches[2] -gt 255 -or [long]$Matches[3] -gt 65535) {
  throw "Version '$Version' does not fit an MSI ProductVersion (major and minor at most 255, patch at most 65535)."
}

$Target = switch ($Arch) { 'x64' { 'x86_64-pc-windows-msvc' } 'x86' { 'i686-pc-windows-msvc' } 'arm64' { 'aarch64-pc-windows-msvc' } }
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist\release' }
New-Item -ItemType Directory -Force -Path $Dist | Out-Null
$Dist = (Resolve-Path $Dist).Path
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
if (-not [IO.Path]::IsPathRooted($TargetDir)) { $TargetDir = Join-Path $Root $TargetDir }
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '4' }

Write-Output "ReasonCraft $Version for Windows $Arch ($Target)"
Push-Location $Root
try {
  if (-not $SkipBuild) {
    # The pinned toolchain (rust-toolchain.toml) may lack the target even when stable has it.
    Invoke-Native "rustup target add $Target" { rustup target add $Target }
    # Static CRT: no VC++ redistributable needed. Scoped to the target so host build scripts and
    # proc-macros are unaffected.
    # Unset again afterwards: on the host's own target it would also rebuild the xtask below.
    $flagVar = 'CARGO_TARGET_' + ($Target.ToUpper() -replace '-', '_') + '_RUSTFLAGS'
    $oldFlags = [Environment]::GetEnvironmentVariable($flagVar)
    [Environment]::SetEnvironmentVariable($flagVar, '-C target-feature=+crt-static')
    try { Invoke-Native "cargo build ($Target)" { cargo build --release --locked -p reasoncraft --target $Target } }
    finally { [Environment]::SetEnvironmentVariable($flagVar, $oldFlags) }
    # The xtask that writes the licence page below, so that `cargo xtask` then compiles nothing.
    Invoke-Native 'cargo build (xtask)' { cargo build --locked -p xtask }
  }

  $Bin = Join-Path $TargetDir "$Target\release"
  Assert-ReleaseExe (Join-Path $Bin 'reasoncraft.exe') $Arch
  if ($BuildOnly) {
    Write-Output "Built $(Join-Path $Bin 'reasoncraft.exe'); package it with -SkipBuild."
    return
  }

  $Stage = Join-Path $TargetDir "windows-package\$Arch"
  Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force -Path $Stage | Out-Null
  Copy-Item (Join-Path $Bin 'reasoncraft.exe') $Stage
  & (Join-Path $PSScriptRoot 'sign.ps1') (Join-Path $Stage 'reasoncraft.exe')

  # The licences and notices ship next to the exe.
  $AppNotices = Join-Path $Stage 'app-notices'
  New-Item -ItemType Directory -Force -Path $AppNotices | Out-Null
  Copy-Item -LiteralPath (Join-Path $Root 'LICENSE-MIT'), (Join-Path $Root 'LICENSE-APACHE'), (Join-Path $Root 'THIRD-PARTY.md') -Destination $AppNotices

  # The installer's licence page, built from LICENSE-MIT and LICENSE-APACHE so it never drifts.
  $LicenseRtf = Join-Path $Stage 'licence.rtf'
  Invoke-Native 'cargo xtask licence-rtf' { cargo xtask licence-rtf $LicenseRtf }

  # One MSI from one dual-scope source (see reasoncraft.wxs and install-policy.wxi): per user by
  # default into %LOCALAPPDATA%\Programs\ReasonCraft, per machine into Program Files\ReasonCraft.
  $Msi = Join-Path $Dist "reasoncraft-$Version-windows-$Arch.msi"
  Invoke-Native 'wix build' {
    wix build (Join-Path $PSScriptRoot 'reasoncraft.wxs') -arch $Arch `
      -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
      -d "Version=$MsiVersion" -d "BinDir=$Stage" -d "IconPath=$(Join-Path $Root 'app\reasoncraft\assets\icon.ico')" `
      -d "LicenseRtf=$LicenseRtf" -d "AppNoticesDir=$AppNotices" `
      -o $Msi
  }
  # Check the compiled scope, path and registration contract before signing.
  $shell = (Get-Process -Id $PID).Path
  & $shell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'check-install-policy.ps1') -MsiPath $Msi
  if ($LASTEXITCODE -ne 0) { throw "Compiled installer policy failed for $Msi." }
  # wix writes its debug symbols (.wixpdb) next to the MSI; keep them out of the release assets.
  Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))
  & (Join-Path $PSScriptRoot 'sign.ps1') $Msi

  # ---- portable zip ----------------------------------------------------------------------------
  $Portable = Join-Path $TargetDir "windows-package\reasoncraft-$Version-windows-$Arch-portable"
  Remove-Item -Recurse -Force $Portable -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force -Path $Portable | Out-Null
  Copy-Item (Join-Path $Stage 'reasoncraft.exe') $Portable
  foreach ($f in 'README.md', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD-PARTY.md') {
    Copy-Item (Join-Path $Root $f) $Portable
  }
  $Zip = Join-Path $Dist "reasoncraft-$Version-windows-$Arch-portable.zip"
  Remove-Item -Force $Zip -ErrorAction SilentlyContinue
  Compress-Archive -Path $Portable -DestinationPath $Zip
  Get-Item $Msi, $Zip | Format-Table Name, Length
} finally {
  Pop-Location
}
