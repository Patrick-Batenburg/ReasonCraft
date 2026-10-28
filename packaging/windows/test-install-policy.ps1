# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/test-install-policy.ps1
# at aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: only ReasonCraft's WiX variables
# and payloads (no models, fonts, CLI or custom dialog bitmaps); it does not run
# test-packaging-script.ps1, which needs a built app; it runs Windows Installer's ICE validation.

<#
.SYNOPSIS
  Build non-executable installer metadata fixtures and check their install policy; never install them.
.DESCRIPTION
  For each architecture, compiles the real reasoncraft.wxs with text files in place of the exe,
  validates it (wix msi validate, the ICE checks; only the known messages listed below pass), then
  runs check-install-policy.ps1 on the result. Needs WiX 5.0.2 with the UI and Util
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

# Windows Installer's validation (the ICE checks of `wix msi validate`). Every message fails the run
# except the known ones below, each matched as narrowly as its message allows.
$knownIce = @(
  # [%ProgramFiles(x86)]: ICE03 reads the parenthesis in the environment variable's name as a bad
  # format string, but Windows Installer expands it (the x86 package's per-machine default folder,
  # checked in check-install-policy.ps1 and in the VM).
  @{ Ice = 'ICE03'; Arch = 'x86'; Pattern = '^Invalid format string; Table: CustomAction, Column: Target, Key\(s\): CraftDefaultMachineFolder$' },
  # The x86 package searches HKLM's 64-bit view on purpose, to find a per-machine x64 or Arm64 copy
  # (install-policy.wxi, the CRAFT_MO_* searches); Windows Installer supports that, ICE80 only warns.
  @{ Ice = 'ICE80'; Arch = 'x86'; Pattern = '^This 32Bit Package is using 64 bit Locator Type in RegLocator table entry CRAFT_MO(_DIRSearch|_FOLDERSearch|_PATHSearch|LegacyFolder|LegacyFile)$' },
  # A dual-scope package writes its registry under HKMU, which ICE57 cannot place per user or per
  # machine; the scope is chosen at install time (install-policy.wxi).
  @{ Ice = 'ICE57'; Pattern = "^Component 'ReasonCraftApp' has a registry entry that can be either per-user or per-machine and a per-machine KeyPath\.$" },
  # AllowSameVersionUpgrades: successive pre-releases of one X.Y.Z share an MSI version and replace
  # each other (reasoncraft.wxs, MajorUpgrade).
  @{ Ice = 'ICE61'; Pattern = '^This product should remove only older versions of itself\. The Maximum version is not less than the current product\.' },
  # The file-type registrations name the exe of the other component; both are in the one feature,
  # so they are always installed together.
  @{ Ice = 'ICE69'; Pattern = "^Mismatched component reference\. Entry '[^']+' of the Registry table belongs to component 'ReasonCraftFileTypes'\. However, the formatted string in column 'Value' references file 'ReasonCraftExe' which belongs to component 'ReasonCraftApp'\. Components are in the same feature\.$" }
)
function Test-MsiValidation([string] $Msi, [string] $Arch) {
  $output = @(& wix msi validate $Msi 2>&1 | ForEach-Object { "$_" })
  $exit = $LASTEXITCODE
  $unexpected = @()
  $known = 0
  foreach ($line in $output) {
    if ($line -notmatch ': (error|warning) WIX\d+: (.*)$') { continue }
    $text = $Matches[2]
    $ok = $false
    if ($text -match '^(ICE\d+): (.*)$') {
      $ice = $Matches[1]; $message = $Matches[2]
      $ok = @($knownIce | Where-Object {
        $_.Ice -eq $ice -and (-not $_.ContainsKey('Arch') -or $_.Arch -eq $Arch) -and $message -match $_.Pattern
      }).Count -gt 0
    }
    if ($ok) { $known++ } else { $unexpected += $line }
  }
  if ($unexpected.Count) { throw "ICE validation of $Arch found:`n$($unexpected -join "`n")" }
  # wix exits non-zero for an ICE error, also a known one (ICE03 above); anything else must be clean.
  if ($exit -ne 0 -and $known -eq 0) { throw "wix msi validate failed for $Arch (exit $exit):`n$($output -join "`n")" }
  Write-Output "PASS $Arch ICE validation ($known known message(s), each explained in test-install-policy.ps1)."
}

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
  Test-MsiValidation $msi $arch
  $shell = (Get-Process -Id $PID).Path
  & $shell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'check-install-policy.ps1') -MsiPath $msi
  if ($LASTEXITCODE -ne 0) { throw "Compiled installer policy failed for $arch (exit $LASTEXITCODE)." }
  Write-Output "PASS $arch metadata-only package. This is not an executable installer."
}
