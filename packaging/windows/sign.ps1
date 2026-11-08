# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/sign.ps1 at aadb077),
# MIT OR Apache-2.0, used under Apache-2.0; changed: signs as ReasonCraft, with the repository as the /du
# link unless REASONCRAFT_SIGN_URL names another.

<#
.SYNOPSIS
  Authenticode-sign files with signtool, using whichever signing material is configured.

.DESCRIPTION
  Tries, in order:
    1. A code-signing certificate file:   WINDOWS_CERTIFICATE (base64 .pfx), WINDOWS_CERTIFICATE_PASSWORD
    2. Azure Trusted Signing:             AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET,
                                          AZURE_SIGNING_ENDPOINT (e.g. https://eus.codesigning.azure.net),
                                          AZURE_SIGNING_ACCOUNT, AZURE_CERT_PROFILE
  With neither, it prints a warning and leaves the files unsigned (exit code 0), so test builds
  still produce installers.

  A set that is only partly configured (some AZURE_* but not all six, or the certificate password
  without the certificate) is named in a warning and does not count; with
  REASONCRAFT_REFUSE_PARTIAL_SIGNING=1 (the release workflow sets it on the release branch) it is
  an error instead, so a forgotten secret cannot ship an unsigned release quietly (ruling PK-16).
  In GitHub Actions the outcome is also written to the job summary. No secret's value is printed.

  This is the one place to adapt when the signing setup changes (an HSM/cloud key, a different
  timestamp server, ...). Optional overrides: WINDOWS_TIMESTAMP_URL, SIGNTOOL (path to
  signtool.exe), REASONCRAFT_SIGN_URL (the signature's "more information" link, signtool /du;
  by default the repository, https://github.com/Patrick-Batenburg/ReasonCraft).

.EXAMPLE
  pwsh packaging/windows/sign.ps1 target/windows-package/x64/reasoncraft.exe dist/release/reasoncraft-0.1.0-windows-x64.msi
#>
param(
  [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)]
  [string[]] $Files
)
$ErrorActionPreference = 'Stop'

function Write-Warn([string] $Message) {
  if ($env:GITHUB_ACTIONS) { Write-Output "::warning::$Message" } else { Write-Warning $Message }
}

function Find-SignTool {
  if ($env:SIGNTOOL) { return $env:SIGNTOOL }
  $onPath = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($onPath) { return $onPath.Source }
  $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
  $found = Get-ChildItem -Path $kits -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending | Select-Object -First 1
  if (-not $found) { throw 'signtool.exe not found (install the Windows SDK)' }
  return $found.FullName
}

function Invoke-SignTool([string[]] $Arguments) {
  & $script:SignTool @Arguments
  if ($LASTEXITCODE -ne 0) { throw "signtool $($Arguments[0]) failed with exit code $LASTEXITCODE" }
}

function Write-Summary([string] $Line) {
  if ($env:GITHUB_STEP_SUMMARY) { Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Value $Line -Encoding utf8 }
}

# The names in $Names that are set and those that are not (never their values).
function Get-SettingState([string[]] $Names) {
  $set = @($Names | Where-Object { [Environment]::GetEnvironmentVariable($_) })
  $missing = @($Names | Where-Object { -not [Environment]::GetEnvironmentVariable($_) })
  [pscustomobject]@{ Set = $set; Missing = $missing }
}

$Files = $Files | ForEach-Object { (Resolve-Path $_).Path }
$names = ($Files | ForEach-Object { '`' + (Split-Path -Leaf $_) + '`' }) -join ', '
$haveCert = [bool]$env:WINDOWS_CERTIFICATE
$azureVars = 'AZURE_TENANT_ID', 'AZURE_CLIENT_ID', 'AZURE_CLIENT_SECRET', 'AZURE_SIGNING_ENDPOINT', 'AZURE_SIGNING_ACCOUNT', 'AZURE_CERT_PROFILE'
$azure = Get-SettingState $azureVars
$haveAzure = $azure.Missing.Count -eq 0

# Partly configured sets: some Azure settings, or a certificate password without its certificate.
$partial = @()
if ($azure.Set.Count -and $azure.Missing.Count) {
  $partial += "Azure Trusted Signing has $($azure.Set -join ', ') but lacks $($azure.Missing -join ', ')"
}
if ($env:WINDOWS_CERTIFICATE_PASSWORD -and -not $haveCert) {
  $partial += 'WINDOWS_CERTIFICATE_PASSWORD is set but WINDOWS_CERTIFICATE is not'
}
if ($partial.Count) {
  $message = "Windows signing is partly configured: $($partial -join '; ')."
  if ($env:REASONCRAFT_REFUSE_PARTIAL_SIGNING -eq '1') {
    Write-Summary "- Windows ${names}: not signed, signing is partly configured (the build failed)"
    throw "$message Set all of a set's secrets, or none of them, before releasing."
  }
  Write-Warn $message
}

if (-not $haveCert -and -not $haveAzure) {
  $why = if ($partial.Count) { 'signing partly configured' } else { 'no signing secrets' }
  Write-Warn "Windows signing secrets not set (WINDOWS_CERTIFICATE or AZURE_*): leaving unsigned: $($Files -join ', ')"
  Write-Summary "- Windows ${names}: **unsigned** ($why)"
  exit 0
}

$script:SignTool = Find-SignTool
$signUrl = if ($env:REASONCRAFT_SIGN_URL) { $env:REASONCRAFT_SIGN_URL } else { 'https://github.com/Patrick-Batenburg/ReasonCraft' }
$common = @('sign', '/v', '/fd', 'SHA256', '/td', 'SHA256', '/d', 'ReasonCraft', '/du', $signUrl)
$tmp = Join-Path ([IO.Path]::GetTempPath()) "reasoncraft-sign-$([IO.Path]::GetRandomFileName())"
New-Item -ItemType Directory -Path $tmp | Out-Null

try {
  if ($haveCert) {
    Write-Output "Signing with WINDOWS_CERTIFICATE: $($Files -join ', ')"
    $pfx = Join-Path $tmp 'cert.pfx'
    [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
    $ts = if ($env:WINDOWS_TIMESTAMP_URL) { $env:WINDOWS_TIMESTAMP_URL } else { 'http://timestamp.digicert.com' }
    $signArgs = $common + @('/tr', $ts, '/f', $pfx)
    if ($env:WINDOWS_CERTIFICATE_PASSWORD) { $signArgs += @('/p', $env:WINDOWS_CERTIFICATE_PASSWORD) }
    Invoke-SignTool ($signArgs + $Files)
  }
  else {
    Write-Output "Signing with Azure Trusted Signing ($env:AZURE_SIGNING_ACCOUNT / $env:AZURE_CERT_PROFILE): $($Files -join ', ')"
    # The dlib authenticates with DefaultAzureCredential, which reads AZURE_TENANT_ID,
    # AZURE_CLIENT_ID and AZURE_CLIENT_SECRET from the environment.
    $trustedSigningVersion = '1.0.95'
    $trustedSigningSha256 = '3bfcf1e0a3cb42af1692f0a8ed45c15de070c2de86f28a59b2795d904d8a920f'
    $pkg = Join-Path $tmp 'trusted-signing.zip'
    $uri = "https://api.nuget.org/v3-flatcontainer/microsoft.trusted.signing.client/$trustedSigningVersion/microsoft.trusted.signing.client.$trustedSigningVersion.nupkg"
    Invoke-WebRequest -Uri $uri -OutFile $pkg
    $actualHash = (Get-FileHash -Path $pkg -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $trustedSigningSha256) {
      throw "Microsoft.Trusted.Signing.Client $trustedSigningVersion checksum mismatch"
    }
    Expand-Archive -Path $pkg -DestinationPath (Join-Path $tmp 'client') -Force
    $dlib = Get-ChildItem -Path (Join-Path $tmp 'client') -Recurse -Filter Azure.CodeSigning.Dlib.dll |
      Where-Object { $_.FullName -match '\\x64\\' } | Select-Object -First 1
    if (-not $dlib) { throw 'Azure.CodeSigning.Dlib.dll not found in Microsoft.Trusted.Signing.Client' }
    $metadata = Join-Path $tmp 'metadata.json'
    @{
      Endpoint               = $env:AZURE_SIGNING_ENDPOINT
      CodeSigningAccountName = $env:AZURE_SIGNING_ACCOUNT
      CertificateProfileName = $env:AZURE_CERT_PROFILE
    } | ConvertTo-Json | Set-Content -Path $metadata -Encoding utf8
    $ts = if ($env:WINDOWS_TIMESTAMP_URL) { $env:WINDOWS_TIMESTAMP_URL } else { 'http://timestamp.acs.microsoft.com' }
    Invoke-SignTool ($common + @('/tr', $ts, '/dlib', $dlib.FullName, '/dmdf', $metadata) + $Files)
  }
  Invoke-SignTool (@('verify', '/pa', '/v') + $Files)
  $how = if ($haveCert) { 'a certificate file (WINDOWS_CERTIFICATE)' } else { 'Azure Trusted Signing' }
  Write-Summary "- Windows ${names}: signed with $how and verified (signtool verify /pa)"
}
finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
