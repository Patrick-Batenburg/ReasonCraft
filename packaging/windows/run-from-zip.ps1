<#
.SYNOPSIS
  Run the exe of a portable zip as users get it: `reasoncraft.exe --version` must print the version.

.DESCRIPTION
  Unpacks the zip into a temporary folder and runs the exe in it with its output redirected to a
  file, so the line arrives whatever the console (a window exe has none of its own), on a
  throw-away profile (REASONCRAFT_PROFILE_DIR) and with a time limit, should it ever open its
  window instead. The release workflow runs it on each Windows zip, arm64 on a windows-11-arm
  runner.

.EXAMPLE
  pwsh packaging/windows/run-from-zip.ps1 -Zip dist/release/reasoncraft-0.1.0-windows-x64-portable.zip -Version 0.1.0
#>
param(
  [Parameter(Mandatory = $true)] [string] $Zip,
  [Parameter(Mandatory = $true)] [string] $Version
)
$ErrorActionPreference = 'Stop'

$work = Join-Path ([IO.Path]::GetTempPath()) "reasoncraft-run-$([Guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $work | Out-Null
$savedProfile = $env:REASONCRAFT_PROFILE_DIR
try {
  Expand-Archive -LiteralPath $Zip -DestinationPath $work
  $exe = @(Get-ChildItem -LiteralPath $work -Recurse -Filter reasoncraft.exe)
  if ($exe.Count -ne 1) { throw "Expected one reasoncraft.exe in $Zip, found $($exe.Count)." }
  $env:REASONCRAFT_PROFILE_DIR = Join-Path $work 'profile'
  $out = Join-Path $work 'stdout.txt'
  $err = Join-Path $work 'stderr.txt'
  $process = Start-Process -FilePath $exe[0].FullName -ArgumentList '--version' -PassThru -NoNewWindow `
    -RedirectStandardOutput $out -RedirectStandardError $err
  # Holding the handle keeps the exit code readable after WaitForExit with a time limit.
  $null = $process.Handle
  if (-not $process.WaitForExit(60000)) {
    $process.Kill()
    throw 'reasoncraft.exe --version did not exit within a minute (it opened the window?).'
  }
  $line = "$(Get-Content -Raw -LiteralPath $out)".Trim()
  if ($process.ExitCode -ne 0) { throw "reasoncraft.exe --version exited $($process.ExitCode): $line $(Get-Content -Raw -LiteralPath $err)" }
  if ($line -ne "ReasonCraft $Version") { throw "reasoncraft.exe --version printed '$line', expected 'ReasonCraft $Version'." }
  Write-Output "PASS $(Split-Path -Leaf $Zip): $line"
} finally {
  $env:REASONCRAFT_PROFILE_DIR = $savedProfile
  Remove-Item -Recurse -Force -LiteralPath $work -ErrorAction SilentlyContinue
}
