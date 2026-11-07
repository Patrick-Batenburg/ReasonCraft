# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/test-packaging-script.ps1
# at aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: the PE checks, sign.ps1's handling
# of missing and partly configured secrets, and the MSI version of a pre-release are checked too.

<#
.SYNOPSIS
  Check the real packaging script without installing anything.
.DESCRIPTION
  1. package.ps1 runs one policy check on the MSI, before it signs that MSI once (read from its syntax tree).
  2. Assert-ReleaseExe accepts the x64 release exe, refuses it labelled arm64 ("machine"), refuses
     the debug exe, which imports the C runtime, and a copy without DependentLoadFlags 0x800.
  3. sign.ps1 without signing secrets warns, leaves the file unsigned and exits 0, also on the
     release branch; a partly configured set warns, and fails with
     REASONCRAFT_REFUSE_PARTIAL_SIGNING=1; a certificate signs (with a stub signtool); each
     outcome lands in the job summary, and no secret's value is printed.
  4. package.ps1 refuses versions an MSI cannot hold (major or minor above 255, patch above 65535)
     before building anything, and -SkipBuild with REASONCRAFT_VERSION=0.2.0-rc.1 writes an MSI
     whose ProductVersion is 0.2.0; the real job summary (GITHUB_STEP_SUMMARY) stays untouched.
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
# A copy of the release exe with DependentLoadFlags cleared stands for a build without build.rs's flag.
$unflagged = Join-Path $TargetDir 'packaging-script-test-unflagged.exe'
$layout = Get-PeLayout $release
$config = ConvertTo-PeOffset $layout ([BitConverter]::ToUInt32($layout.Bytes, $layout.Directories + 80))
$field = if ($layout.Magic -eq 0x10B) { 0x36 } else { 0x4E }
$layout.Bytes[$config + $field] = 0; $layout.Bytes[$config + $field + 1] = 0
[IO.File]::WriteAllBytes($unflagged, $layout.Bytes)
Assert-Throws 'an exe that may load its DLLs from its own folder is refused' 'DependentLoadFlags' { Assert-ReleaseExe $unflagged 'x64' }
Remove-Item -LiteralPath $unflagged
$imports = @(Get-PeImports $release)
if ('KERNEL32.dll' -notin $imports) { throw "Import walk found no KERNEL32.dll: $($imports -join ', ')" }

# ---- 3. signing without secrets -------------------------------------------------------------------
$work = Join-Path $TargetDir 'packaging-script-test'
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $work | Out-Null
$copy = Join-Path $work 'reasoncraft.exe'
Copy-Item -LiteralPath $release $copy
$azureVars = 'AZURE_TENANT_ID', 'AZURE_CLIENT_ID', 'AZURE_CLIENT_SECRET', 'AZURE_SIGNING_ENDPOINT',
  'AZURE_SIGNING_ACCOUNT', 'AZURE_CERT_PROFILE'
$signVars = @('WINDOWS_CERTIFICATE', 'WINDOWS_CERTIFICATE_PASSWORD') + $azureVars +
  'REASONCRAFT_REFUSE_PARTIAL_SIGNING', 'SIGNTOOL', 'GITHUB_STEP_SUMMARY', 'GITHUB_ACTIONS'
$saved = @{}
foreach ($name in $signVars + 'REASONCRAFT_VERSION', 'DIST') { $saved[$name] = [Environment]::GetEnvironmentVariable($name) }
# The real job summary (in CI) must come out of this self-test unchanged.
function Get-RealSummary {
  $path = $saved['GITHUB_STEP_SUMMARY']
  if ($path -and (Test-Path -LiteralPath $path)) { (Get-FileHash -LiteralPath $path).Hash } else { '' }
}
$realSummary = Get-RealSummary
# Runs sign.ps1 on the copy with only the given settings; returns its exit code, output and the
# job summary it wrote. SIGNTOOL is a stub that logs its arguments, so no real signing happens.
$summaryFile = Join-Path $work 'summary.md'
$stub = Join-Path $work 'signtool-stub.cmd'
Set-Content -LiteralPath $stub -Encoding Ascii -Value "@echo signtool-stub %* >> `"$(Join-Path $work 'signtool.log')`""
function Invoke-Sign([hashtable] $Settings) {
  foreach ($name in $signVars) { [Environment]::SetEnvironmentVariable($name, $null) }
  Remove-Item -LiteralPath $summaryFile -ErrorAction SilentlyContinue
  $env:GITHUB_STEP_SUMMARY = $summaryFile
  $env:SIGNTOOL = $stub
  foreach ($key in $Settings.Keys) { [Environment]::SetEnvironmentVariable($key, $Settings[$key]) }
  $out = & $Shell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'sign.ps1') $copy 2>&1 | Out-String
  $summary = if (Test-Path -LiteralPath $summaryFile) { Get-Content -Raw -LiteralPath $summaryFile } else { '' }
  [pscustomobject]@{ Exit = $LASTEXITCODE; Out = $out; Summary = $summary }
}
function Assert-Sign([string] $What, [hashtable] $Settings, [int] $Exit, [string] $Output, [string] $Summary) {
  $r = Invoke-Sign $Settings
  # PowerShell wraps an error message over lines, each continued after a '|'.
  $flat = $r.Out -replace '\s*\|\s*', ' ' -replace '\s+', ' '
  if ($r.Exit -ne $Exit) { throw "$What`: exit $($r.Exit), expected $Exit. $($r.Out)" }
  if ($flat -notmatch $Output) { throw "$What`: output lacks '$Output'. $($r.Out)" }
  if ($r.Summary -notmatch $Summary) { throw "$What`: summary '$($r.Summary)' lacks '$Summary'." }
  if ($r.Out -match 'secret-value') { throw "$What`: a secret's value was printed." }
  Write-Output "PASS $What"
}
$oneAzure = @{ AZURE_TENANT_ID = 'secret-value' }
$fiveAzure = @{}; foreach ($name in $azureVars[0..4]) { $fiveAzure[$name] = 'secret-value' }
$fakePfx = [Convert]::ToBase64String([byte[]](1..16))
try {
  Assert-Sign 'sign.ps1 without secrets warns, leaves the file unsigned and exits 0' @{} 0 'leaving unsigned' 'unsigned\*\* \(no signing secrets\)'
  Assert-Sign 'on the release branch too, no secrets at all is allowed (ruling PK-7)' @{ REASONCRAFT_REFUSE_PARTIAL_SIGNING = '1' } 0 'leaving unsigned' 'no signing secrets'
  Assert-Sign 'one Azure setting of six warns and leaves the file unsigned' $oneAzure 0 'partly configured.*lacks AZURE_CLIENT_ID' 'signing partly configured'
  Assert-Sign 'five Azure settings of six fail on the release branch' ($fiveAzure + @{ REASONCRAFT_REFUSE_PARTIAL_SIGNING = '1' }) 1 'partly configured.*lacks AZURE_CERT_PROFILE' 'the build failed'
  Assert-Sign 'a certificate password without the certificate fails on the release branch' @{ WINDOWS_CERTIFICATE_PASSWORD = 'secret-value'; REASONCRAFT_REFUSE_PARTIAL_SIGNING = '1' } 1 'WINDOWS_CERTIFICATE_PASSWORD is set but WINDOWS_CERTIFICATE is not' 'the build failed'
  Assert-Sign 'a certificate with leftover Azure settings fails on the release branch' ($oneAzure + @{ WINDOWS_CERTIFICATE = $fakePfx; WINDOWS_CERTIFICATE_PASSWORD = 'secret-value'; REASONCRAFT_REFUSE_PARTIAL_SIGNING = '1' }) 1 'partly configured' 'the build failed'
  Assert-Sign 'a certificate signs (stub signtool) and the summary says so' @{ WINDOWS_CERTIFICATE = $fakePfx; WINDOWS_CERTIFICATE_PASSWORD = 'secret-value'; REASONCRAFT_REFUSE_PARTIAL_SIGNING = '1' } 0 'Signing with WINDOWS_CERTIFICATE' 'signed with a certificate file'
  $log = Get-Content -Raw -LiteralPath (Join-Path $work 'signtool.log')
  if ($log -notmatch '(?m)^signtool-stub sign .*/f .*reasoncraft\.exe' -or $log -notmatch '(?m)^signtool-stub verify /pa') { throw "Unexpected signtool calls: $log" }
  if ((Get-AuthenticodeSignature -LiteralPath $copy).Status -eq 'Valid') { throw 'The stub signed the file.' }
  foreach ($name in $signVars) { [Environment]::SetEnvironmentVariable($name, $saved[$name]) }
  # The test MSI below must not add a line to the real job summary (the finally block restores it).
  $env:GITHUB_STEP_SUMMARY = $null

  # ---- 4. versions an MSI cannot hold, and a pre-release version ---------------------------------
  foreach ($bad in '256.0.0', '1.256.0', '1.2.65536', '1.2') {
    $env:REASONCRAFT_VERSION = $bad
    $env:DIST = Join-Path $work 'dist-refused'
    $out = & $Shell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'package.ps1') -Arch x64 -SkipBuild 2>&1 | Out-String
    if ($LASTEXITCODE -eq 0 -or ($out -replace '\s*\|\s*', ' ' -replace '\s+', ' ') -notmatch 'MSI ProductVersion|is not X\.Y\.Z') {
      throw "package.ps1 accepted version $bad (exit $LASTEXITCODE): $out"
    }
    if (@(Get-ChildItem -LiteralPath $env:DIST -ErrorAction SilentlyContinue).Count) { throw "package.ps1 wrote files for version $bad." }
  }
  Write-Output 'PASS versions beyond the MSI limits (255.255.65535) are refused before anything is built.'
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
  if ((Get-RealSummary) -ne $realSummary) { throw 'The self-test wrote to the real job summary (GITHUB_STEP_SUMMARY).' }
  Write-Output 'PASS the real job summary is untouched by the test packages.'
} finally {
  foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name]) }
}
Write-Output 'PASS test-packaging-script.ps1'
