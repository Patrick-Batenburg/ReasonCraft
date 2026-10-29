<#
.SYNOPSIS
  Install WiX 5.0.2 and its extensions from packages checked against pinned SHA-256s.
.DESCRIPTION
  Downloads the wix dotnet tool and the WiX extensions (WixToolset.<name>.wixext) of version 5.0.2
  from nuget.org, checks each .nupkg against the SHA-256 pinned below, and installs them from that
  folder alone (a nuget.config that clears every other package source), so nothing unchecked is
  installed. The tool goes into -ToolPath, which the caller puts on PATH; the extensions into the
  user's WiX extension cache (wix extension add -g), or with -ProjectDirectory into that folder's
  .wix\extensions (wix build run there finds them; used to test this script without touching the
  user's cache).

  WiX stays at exactly 5.0.2: it is under the MS-RL, while WiX 6 and later come with the Open
  Source Maintenance Fee EULA. To pin another package, take its SHA-512 from nuget.org's catalog
  ("packageHash") and check that it matches the download whose SHA-256 is pinned here.
.EXAMPLE
  pwsh packaging/windows/install-wix.ps1 -ToolPath $env:RUNNER_TEMP\wix -Extensions UI, Util
#>
param(
  [Parameter(Mandatory = $true)] [string] $ToolPath,
  [ValidateSet('UI', 'Util')] [string[]] $Extensions = @('UI', 'Util'),
  [string] $ProjectDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Version = '5.0.2'
# SHA-256 of each .nupkg; their SHA-512 match nuget.org's catalog (checked 2026-10-29).
$Pinned = [ordered]@{
  'wix'                    = 'f30ef0c74e2a986126539c5780be93ac24e8136eaf723b1937b26272703ae173'
  'wixtoolset.ui.wixext'   = '5ef2c707614b9f70b6bbadd2d4abcb4124efee215e9b16bfbc80113079a604c7'
  'wixtoolset.util.wixext' = 'dda1cc1b4d3b2305b1246f167c6af6fcded5000be84b3bbff3dda4f442bf4740'
}

$work = Join-Path ([IO.Path]::GetTempPath()) "reasoncraft-wix-$([IO.Path]::GetRandomFileName())"
$feed = Join-Path $work 'feed'
New-Item -ItemType Directory -Force -Path $feed | Out-Null
$configDir = if ($ProjectDirectory) { (New-Item -ItemType Directory -Force -Path $ProjectDirectory).FullName } else { $work }
$config = Join-Path $configDir 'nuget.config'
if ($ProjectDirectory -and (Test-Path -LiteralPath $config)) { throw "$config exists; use an empty -ProjectDirectory." }
try {
  $wanted = @('wix') + @($Extensions | ForEach-Object { "wixtoolset.$($_.ToLowerInvariant()).wixext" })
  foreach ($id in $wanted) {
    $file = Join-Path $feed "$id.$Version.nupkg"
    Invoke-WebRequest -Uri "https://api.nuget.org/v3-flatcontainer/$id/$Version/$id.$Version.nupkg" -OutFile $file
    $hash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne $Pinned[$id]) { throw "$id $Version checksum mismatch: $hash, expected $($Pinned[$id])." }
    Write-Output "Checked $id $Version ($hash)"
  }
  # Only the checked folder: <clear /> drops nuget.org and any source from the user's or machine's
  # NuGet configuration. `wix extension add` reads the nuget.config of its working directory.
  $xml = '<?xml version="1.0" encoding="utf-8"?>' +
    '<configuration><packageSources><clear /><add key="checked" value="' +
    [Security.SecurityElement]::Escape($feed) + '" /></packageSources></configuration>'
  Set-Content -LiteralPath $config -Value $xml -Encoding utf8

  & dotnet tool install wix --version $Version --tool-path $ToolPath --configfile $config
  if ($LASTEXITCODE -ne 0) { throw "WiX $Version installation failed." }
  $wix = Join-Path $ToolPath 'wix.exe'
  Push-Location $configDir
  try {
    foreach ($name in $Extensions) {
      $scope = if ($ProjectDirectory) { @() } else { @('-g') }
      & $wix extension add @scope "WixToolset.$name.wixext/$Version"
      if ($LASTEXITCODE -ne 0) { throw "WiX extension $name installation failed." }
    }
  } finally { Pop-Location }
} finally {
  if ($ProjectDirectory) { Remove-Item -Force -LiteralPath $config -ErrorAction SilentlyContinue }
  Remove-Item -Recurse -Force -LiteralPath $work -ErrorAction SilentlyContinue
}
