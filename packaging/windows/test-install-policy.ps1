# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/test-install-policy.ps1
# at aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: only ReasonCraft's WiX variables
# and payloads (no models, fonts, CLI or custom dialog bitmaps); it does not run
# test-packaging-script.ps1, which needs a built app.

<#
.SYNOPSIS
  Build non-executable installer metadata fixtures and check their install policy; never install them.
.DESCRIPTION
  For each architecture, compiles the real reasoncraft.wxs with text files in place of the exe,
  then runs check-install-policy.ps1 on the result. Needs WiX 5.0.2 with the UI and Util
  extensions; does not compile the Rust app.
.EXAMPLE
  pwsh packaging/windows/test-install-policy.ps1 -OutputDirectory target/install-policy-tests
#>
param(
  [Parameter(Mandatory = $true)] [string] $OutputDirectory,
  [ValidateSet('x64','x86','arm64')] [string[]] $Architectures = @('x64','x86','arm64')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$policy = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'install-policy.json') | ConvertFrom-Json
[void](New-Item -ItemType Directory -Force -Path $OutputDirectory)
$output = (Resolve-Path -LiteralPath $OutputDirectory).Path
foreach ($arch in $Architectures) {
  $stage = Join-Path $output $arch
  [void](New-Item -ItemType Directory -Force -Path $stage)
  foreach ($name in $policy.payloads) {
    $destination = Join-Path $stage $name
    [void](New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($destination)))
    Set-Content -LiteralPath $destination -Encoding Ascii -Value 'NON-EXECUTABLE MSI METADATA TEST FIXTURE'
  }
  $notices = Join-Path $stage 'notices'
  [void](New-Item -ItemType Directory -Force -Path $notices)
  Set-Content -LiteralPath (Join-Path $notices 'notices-fixture.txt') -Value 'Metadata test fixture, not a user file.'
  Set-Content -LiteralPath (Join-Path $stage 'licence.rtf') -Encoding Ascii -Value '{\rtf1 Metadata test fixture.}'
  $sources = @($policy.sources | ForEach-Object { Join-Path $PSScriptRoot $_ })
  $msi = Join-Path $stage "$($policy.id)-NOT-INSTALLABLE.msi"
  $extensions = @($policy.extensions | ForEach-Object { '-ext'; "WixToolset.$_.wixext" })
  & wix build @sources -arch $arch @extensions `
    -culture en-US -d 'Version=0.0.1' `
    -d "BinDir=$stage" -d "IconPath=$(Join-Path $root $policy.icon)" `
    -d "AppNoticesDir=$notices" -d "LicenseRtf=$(Join-Path $stage 'licence.rtf')" `
    -o $msi @($policy.localization | ForEach-Object { '-loc'; Join-Path $PSScriptRoot $_ })
  if ($LASTEXITCODE -ne 0) { throw "WiX validation failed for $arch (exit $LASTEXITCODE)." }
  $shell = (Get-Process -Id $PID).Path
  & $shell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'check-install-policy.ps1') -MsiPath $msi
  if ($LASTEXITCODE -ne 0) { throw "Compiled installer policy failed for $arch (exit $LASTEXITCODE)." }
  Write-Output "PASS $arch metadata-only package. This is not an executable installer."
}
