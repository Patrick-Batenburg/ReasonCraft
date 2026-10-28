# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/test-packaging-script.ps1
# at aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: the PE checks, sign.ps1 without
# secrets and the MSI version of a pre-release are checked too.

<#
.SYNOPSIS
  Check the real packaging script without installing anything.
.DESCRIPTION
  1. package.ps1 runs one policy check on the MSI, before it signs that MSI once (read from its syntax tree).
  2. Assert-ReleaseExe accepts the x64 release exe, refuses it labelled arm64 ("machine") and refuses
     the debug exe, which imports the C runtime.
  3. sign.ps1 without (or with partial) signing secrets warns, leaves the file unsigned and exits 0.
  4. package.ps1 -SkipBuild with REASONCRAFT_VERSION=0.2.0-rc.1 writes an MSI whose ProductVersion is 0.2.0.
  Needs the x64 release exe (package.ps1 -Arch x64, or cargo build --release with +crt-static) and
  the debug exe (cargo build -p reasoncraft) in CARGO_TARGET_DIR (default: target), and WiX.
#>
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
if (-not [IO.Path]::IsPathRooted($TargetDir)) { $TargetDir = Join-Path $Root $TargetDir }
$Shell = (Get-Process -Id $PID).Path

# ---- 1. the order of the policy check and the signing --------------------------------------------
$tokens = $null; $errors = $null
$script = Join-Path $PSScriptRoot 'package.ps1'
$ast = [Management.Automation.Language.Parser]::ParseFile($script, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'The packaging script does not parse.' }
if ($ast.Extent.Text -match 'windows-user|UserMsi') {
  throw 'ReasonCraft ships one dual-scope MSI; there is no separate per-user installer to package.'
}
$commands = @($ast.FindAll({ param($node) $node -is [Management.Automation.Language.CommandAst] }, $true))
$checks = @($commands | Where-Object {
  $_.InvocationOperator.ToString() -eq 'Ampersand' -and
    @($_.CommandElements | Where-Object { $_.Extent.Text -match 'check-install-policy.ps1' }).Count -gt 0
})
if ($checks.Count -ne 1) { throw 'Expected exactly one policy check in the packaging script.' }
$elements = $checks[0].CommandElements
$index = 0
while ($index -lt $elements.Count -and $elements[$index].Extent.Text -ne '-MsiPath') { $index++ }
if ($index + 1 -ge $elements.Count -or $elements[$index + 1].Extent.Text -ne '$Msi') {
  throw 'The policy check must inspect the MSI being packaged ($Msi).'
}
$signs = @($commands | Where-Object {
  $_.InvocationOperator.ToString() -eq 'Ampersand' -and
    @($_.CommandElements | Where-Object { $_.Extent.Text -match 'sign.ps1' }).Count -gt 0 -and
    $_.CommandElements[-1].Extent.Text -eq '$Msi'
})
if ($signs.Count -ne 1) { throw 'Expected the MSI to be signed exactly once.' }
if ($checks[0].Extent.StartOffset -ge $signs[0].Extent.StartOffset) { throw 'The MSI must pass the policy check before it is signed.' }
Write-Output 'PASS the MSI is policy-checked before it is signed.'

# ---- 2. the PE checks -----------------------------------------------------------------------------
. (Join-Path $PSScriptRoot 'pe-checks.ps1')
$release = Join-Path $TargetDir 'x86_64-pc-windows-msvc\release\reasoncraft.exe'
$debug = Join-Path $TargetDir 'debug\reasoncraft.exe'
foreach ($exe in $release, $debug) {
  if (-not (Test-Path -LiteralPath $exe)) { throw "Missing ${exe}: build it first (see this script's help)." }
}
function Assert-Throws([string] $What, [string] $Pattern, [scriptblock] $Block) {
  try { & $Block | Out-Null } catch {
    if ($_.Exception.Message -match $Pattern) { Write-Output "PASS $What"; return }
    throw "$What failed with '$($_.Exception.Message)', expected '$Pattern'."
  }
  throw "$What did not fail."
}
Assert-ReleaseExe $release 'x64' | Out-Null
Write-Output 'PASS the x64 release exe is an x64 GUI exe with a static C runtime.'
Assert-Throws 'an x64 exe labelled arm64 is refused' 'machine' { Assert-ReleaseExe $release 'arm64' }
Assert-Throws 'the debug exe, which needs the C runtime, is refused' 'C runtime' { Assert-ReleaseExe $debug 'x64' }
$imports = @(Get-PeImports $release)
if ('KERNEL32.dll' -notin $imports) { throw "Import walk found no KERNEL32.dll: $($imports -join ', ')" }

# ---- 3. signing without secrets -------------------------------------------------------------------
$work = Join-Path $TargetDir 'packaging-script-test'
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $work | Out-Null
$copy = Join-Path $work 'reasoncraft.exe'
Copy-Item -LiteralPath $release $copy
$signVars = 'WINDOWS_CERTIFICATE', 'WINDOWS_CERTIFICATE_PASSWORD', 'AZURE_TENANT_ID', 'AZURE_CLIENT_ID',
  'AZURE_CLIENT_SECRET', 'AZURE_SIGNING_ENDPOINT', 'AZURE_SIGNING_ACCOUNT', 'AZURE_CERT_PROFILE'
$saved = @{}
foreach ($name in $signVars + 'REASONCRAFT_VERSION', 'DIST') { $saved[$name] = [Environment]::GetEnvironmentVariable($name) }
try {
  foreach ($name in $signVars) { [Environment]::SetEnvironmentVariable($name, $null) }
  # Partial Azure settings count as none.
  $env:AZURE_TENANT_ID = '00000000-0000-0000-0000-000000000000'
  $out = & $Shell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'sign.ps1') $copy 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) { throw "sign.ps1 without secrets exited $LASTEXITCODE`: $out" }
  if ($out -notmatch 'leaving unsigned') { throw "sign.ps1 without secrets did not warn: $out" }
  if ((Get-AuthenticodeSignature -LiteralPath $copy).Status -eq 'Valid') { throw 'sign.ps1 signed without secrets.' }
  Write-Output 'PASS sign.ps1 with partial secrets warns and leaves the file unsigned.'
  $env:AZURE_TENANT_ID = $null

  # ---- 4. a pre-release version -------------------------------------------------------------------
  $env:REASONCRAFT_VERSION = '0.2.0-rc.1'
  $env:DIST = Join-Path $work 'dist'
  & $Shell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'package.ps1') -Arch x64 -SkipBuild | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "package.ps1 -SkipBuild exited $LASTEXITCODE." }
  $msi = Join-Path $env:DIST 'reasoncraft-0.2.0-rc.1-windows-x64.msi'
  foreach ($file in $msi, (Join-Path $env:DIST 'reasoncraft-0.2.0-rc.1-windows-x64-portable.zip')) {
    if (-not (Test-Path -LiteralPath $file)) { throw "package.ps1 did not write $file." }
  }
  $installer = New-Object -ComObject WindowsInstaller.Installer
  $database = $installer.OpenDatabase($msi, 0)
  $view = $database.OpenView('SELECT `Value` FROM `Property` WHERE `Property` = ''ProductVersion''')
  [void]$view.Execute()
  $record = $view.Fetch()
  $productVersion = $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @(1))
  [void]$view.Close()
  foreach ($com in @($record, $view, $database, $installer)) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($com) }
  if ($productVersion -ne '0.2.0') { throw "ProductVersion of a 0.2.0-rc.1 MSI is '$productVersion', expected 0.2.0." }
  Write-Output 'PASS a 0.2.0-rc.1 MSI has ProductVersion 0.2.0.'
} finally {
  foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name]) }
}
Write-Output 'PASS test-packaging-script.ps1'
