<#
.SYNOPSIS
  Checks in the Hyper-V test VM that the x64 and x86 MSIs never install side by side in the two
  scopes: a per-machine copy of one architecture refuses a per-user install of the other.

.DESCRIPTION
  An x86 package on 64-bit Windows writes its per-machine registry keys in the 32-bit view
  (WOW6432Node) and, without Bitness, searches only that view; an x64 package only the 64-bit view.
  install-policy.wxi therefore searches HKLM in both views (ruling PK-18). This script installs:
    1. the x64 MSI for everyone, then the x86 MSI only for me: refused (1603, "already installed
       for everyone"), nothing installed per user; the x64 copy is removed again;
    2. the x86 MSI for everyone (checks that its keys land in WOW6432Node and its files in
       Program Files (x86)), then the x64 MSI only for me: refused the same way; then removed;
    3. with -BaseX86Msi (the x86 MSI built from the sources before the fix), step 1 again with that
       package: its InstallDir search misses the x64 copy and only the App Paths search (a key both
       views share, which needs the exe in place) refuses it; with App Paths\reasoncraft.exe
       removed, the fixed x86 MSI is still refused while the base one installs a second copy.
  The VM routine is the one of verify-in-vm.ps1: checkpoint -> start -> checks -> turn off ->
  restore the checkpoint -> remove it, and the VM is left Off, also after a failure. Runs on the host
  (elevated, for Hyper-V) and never installs anything there; no keyboard or mouse input is sent,
  and the app is never started.

.EXAMPLE
  .\packaging\windows\verify-cross-arch-in-vm.ps1 -X64Msi dist\release\reasoncraft-0.1.0-windows-x64.msi `
      -X86Msi dist\release\reasoncraft-0.1.0-windows-x86.msi -OutDir target\vm-cross-arch
#>
param(
    [Parameter(Mandatory)] [string] $X64Msi,
    [Parameter(Mandatory)] [string] $X86Msi,
    [Parameter(Mandatory)] [string] $OutDir,
    [string] $BaseX86Msi,
    [string] $VMName = 'Windows 11 Sandbox',
    [string] $Checkpoint = 'before-reasoncraft-cross-arch'
)
$ErrorActionPreference = 'Stop'
$X64Msi = (Resolve-Path $X64Msi).Path
$X86Msi = (Resolve-Path $X86Msi).Path
if ($BaseX86Msi) { $BaseX86Msi = (Resolve-Path $BaseX86Msi).Path }
. (Join-Path $PSScriptRoot 'vm-checks.ps1')
$UpgradeCode = (Get-Content -Raw (Join-Path $PSScriptRoot 'install-policy.json') | ConvertFrom-Json).upgradeCode

New-Item -ItemType Directory -Force $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path
Start-Transcript -Path (Join-Path $OutDir 'cross-arch-transcript.txt') -Force | Out-Null
$failures = [System.Collections.Generic.List[string]]::new()
function Check([string] $what, [bool] $ok) {
    Write-Host ("[{0}] {1}" -f ($(if ($ok) { 'PASS' } else { 'FAIL' })), $what)
    if (-not $ok) { $failures.Add($what) }
}

# Where ReasonCraft is registered and installed: both HKLM views, HKCU, the three folders, and
# the products with the UpgradeCode per scope (AssignmentType 0 = per user, 1 = per machine).
$GetState = {
    param($upgradeCode)
    function InstallDir([string] $hive, [string] $view) {
        $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($hive, $view)
        $key = $base.OpenSubKey('SOFTWARE\ReasonCraft')
        if ($key) { return $key.GetValue('InstallDir') }
        return $null
    }
    $inst = New-Object -ComObject WindowsInstaller.Installer
    $t = $inst.GetType()
    $list = $t.InvokeMember('RelatedProducts', 'GetProperty', $null, $inst, @("{$upgradeCode}"))
    $count = $list.GetType().InvokeMember('Count', 'GetProperty', $null, $list, $null)
    $types = @(for ($i = 0; $i -lt $count; $i++) {
        $code = $list.GetType().InvokeMember('Item', 'GetProperty', $null, $list, @($i))
        $t.InvokeMember('ProductInfo', 'GetProperty', $null, $inst, @($code, 'AssignmentType'))
    })
    [pscustomobject]@{
        Products        = $count
        PerUser         = @($types | Where-Object { $_ -eq '0' }).Count
        PerMachine      = @($types | Where-Object { $_ -eq '1' }).Count
        Hklm64          = InstallDir 'LocalMachine' 'Registry64'
        Hklm32          = InstallDir 'LocalMachine' 'Registry32'
        Hkcu            = InstallDir 'CurrentUser' 'Registry64'
        ProgramFiles    = Test-Path (Join-Path $env:ProgramW6432 'ReasonCraft\reasoncraft.exe')
        ProgramFilesX86 = Test-Path (Join-Path ${env:ProgramFiles(x86)} 'ReasonCraft\reasoncraft.exe')
        UserPrograms    = Test-Path (Join-Path $env:LOCALAPPDATA 'Programs\ReasonCraft\reasoncraft.exe')
    }
}
function Get-State { $s = Invoke-Command -Session $session -ArgumentList $UpgradeCode $GetState; $s | Format-List | Out-String -Width 200 | Write-Host; $s }
function Assert-Clean([string] $what) {
    $s = Get-State
    Check "${what}: nothing of ReasonCraft left" (-not $s.Products -and -not $s.Hklm64 -and -not $s.Hklm32 -and -not $s.Hkcu -and
        -not $s.ProgramFiles -and -not $s.ProgramFilesX86 -and -not $s.UserPrograms)
}
# A per-user install of the other architecture beside a per-machine copy must be refused.
function Assert-UserRefused([string] $label, [string] $guestMsi) {
    $code = Invoke-Msi $session $guestMsi @() $label
    Check "${label}: refused with 1603" ($code -eq 1603)
    Check "${label}: the log says ""already installed for everyone""" ((Get-MsiLog $session $label) -like '*This app is already installed for everyone*')
    $s = Get-State
    Check "${label}: no per-user copy (product, HKCU key or Programs folder)" ($s.PerUser -eq 0 -and -not $s.Hkcu -and -not $s.UserPrograms)
    Check "${label}: the per-machine copy is still the only one" ($s.Products -eq 1 -and $s.PerMachine -eq 1)
}

$vm = Get-VM -Name $VMName
Write-Host "VM '$VMName' is $($vm.State)"
if (Get-VMSnapshot -VMName $VMName -Name $Checkpoint -ErrorAction SilentlyContinue) {
    throw "A checkpoint '$Checkpoint' already exists; restore or remove it first."
}
function Find-Checkpoint([int] $seconds) {
    $deadline = (Get-Date).AddSeconds($seconds)
    do {
        $c = Get-VMSnapshot -VMName $VMName -Name $Checkpoint -ErrorAction SilentlyContinue
        if ($c) { return $c }
        Start-Sleep -Seconds 2
    } while ((Get-Date) -lt $deadline)
    return $null
}
$session = $null
$checkpointTaken = $false
try {
    $checkpointTaken = $true  # from here on, `finally` restores and removes whatever got taken
    Checkpoint-VM -Name $VMName -SnapshotName $Checkpoint
    if (-not (Find-Checkpoint 120)) { throw "The checkpoint '$Checkpoint' did not appear." }
    Write-Host "Checkpoint '$Checkpoint' taken"
    $cred = Import-Clixml "$env:USERPROFILE\.vm-sandbox-cred.xml"
    if ((Get-VM -Name $VMName).State -ne 'Running') { Start-VM -Name $VMName }
    $session = Connect-VMSession $VMName $cred
    Write-Host ('Connected to ' + (Invoke-Command -Session $session { "$env:COMPUTERNAME as $(whoami), 64-bit OS: $([Environment]::Is64BitOperatingSystem)" }))
    Invoke-Command -Session $session { New-Item -ItemType Directory -Force C:\rc-test | Out-Null }
    $x64 = 'C:\rc-test\x64.msi'; $x86 = 'C:\rc-test\x86.msi'; $base = 'C:\rc-test\x86-base.msi'
    Copy-Item -ToSession $session -Path $X64Msi -Destination $x64
    Copy-Item -ToSession $session -Path $X86Msi -Destination $x86
    if ($BaseX86Msi) { Copy-Item -ToSession $session -Path $BaseX86Msi -Destination $base }
    Assert-Clean 'before'

    Write-Host '--- 1. x64 for everyone, then x86 only for me'
    Check 'x64 for everyone installs' ((Invoke-Msi $session $x64 @('ALLUSERS=1') 'x64-machine') -eq 0)
    $s = Get-State
    Check 'x64 for everyone: InstallDir in the 64-bit view only' ($s.Hklm64 -and -not $s.Hklm32 -and $s.ProgramFiles)
    Assert-UserRefused 'x86-user-over-x64-machine' $x86
    Check 'x64 for everyone uninstalls' ((Invoke-Msi $session $x64 @() 'x64-machine-remove' '/x') -eq 0)
    Assert-Clean 'after step 1'

    Write-Host '--- 2. x86 for everyone, then x64 only for me'
    Check 'x86 for everyone installs' ((Invoke-Msi $session $x86 @('ALLUSERS=1') 'x86-machine') -eq 0)
    $s = Get-State
    Check 'x86 for everyone: InstallDir in the 32-bit view (WOW6432Node) only, files in Program Files (x86)' (
        $s.Hklm32 -and -not $s.Hklm64 -and $s.ProgramFilesX86 -and -not $s.ProgramFiles)
    Assert-UserRefused 'x64-user-over-x86-machine' $x64
    Check 'x86 for everyone uninstalls' ((Invoke-Msi $session $x86 @() 'x86-machine-remove' '/x') -eq 0)
    Assert-Clean 'after step 2'

    if ($BaseX86Msi) {
        # Before the fix the x86 package read Software\ReasonCraft\InstallDir in the 32-bit view only
        # and missed the x64 copy; it was refused only through the App Paths search, a key that
        # WOW64 shares between the views and that needs the exe in place. Without that key (a broken
        # or partly removed registration) it installed a second copy.
        Write-Host '--- 3. before the fix: x64 for everyone, then the base x86 MSI only for me'
        Check 'x64 for everyone installs' ((Invoke-Msi $session $x64 @('ALLUSERS=1') 'base-x64-machine') -eq 0)
        $code = Invoke-Msi $session $base @() 'base-x86-user'
        $log = Get-MsiLog $session 'base-x86-user'
        Check 'base x86: refused (1603), but its InstallDir search finds nothing; only App Paths caught the copy' (
            $code -eq 1603 -and $log -notlike '*Adding CRAFT_M_DIR property*' -and $log -like "*Adding CRAFT_M_EXEFOLDER property. Its value is 'C:\Program Files\ReasonCraft\'*")
        Check 'fixed x86: InstallDir found in the 64-bit view' ((Get-MsiLog $session 'x86-user-over-x64-machine') -like "*Adding CRAFT_MO_DIR property. Its value is 'C:\Program Files\ReasonCraft\'*")
        Invoke-Command -Session $session { Remove-Item 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\reasoncraft.exe' -Recurse }
        Write-Host 'Removed App Paths\reasoncraft.exe (a damaged registration of the x64 copy)'
        Assert-UserRefused 'fixed-x86-user-without-app-paths' $x86
        $code = Invoke-Msi $session $base @() 'base-x86-user-without-app-paths'
        $s = Get-State
        Write-Host ("BEFORE THE FIX: the base x86 MSI per user exits {0}; per-user products {1}, per-machine {2}, per-user exe {3}" -f
            $code, $s.PerUser, $s.PerMachine, $s.UserPrograms)
        Check 'base x86 without App Paths: installs a second, per-user copy (the bug the fix removes)' (
            $code -eq 0 -and $s.PerUser -eq 1 -and $s.PerMachine -eq 1 -and $s.UserPrograms)
    }
}
catch {
    $failures.Add("script error: $_")
    Write-Host "ERROR: $_`n$($_.ScriptStackTrace)"
}
finally {
    if ($session) {
        try { Copy-Item -FromSession $session -Path 'C:\rc-test\*.log' -Destination $OutDir -ErrorAction Stop }
        catch { Write-Host "Could not copy the msiexec logs: $_" }
        Remove-PSSession $session -ErrorAction SilentlyContinue
    }
    Write-Host 'Turning the VM off and restoring the checkpoint'
    if ($checkpointTaken) {
        try { Stop-VM -Name $VMName -TurnOff -Force } catch { Write-Host "Stop-VM: $_" }
        if (Find-Checkpoint 180) {
            try { Restore-VMSnapshot -VMName $VMName -Name $Checkpoint -Confirm:$false } catch { Write-Host "Restore-VMSnapshot: $_" }
            try { Remove-VMSnapshot -VMName $VMName -Name $Checkpoint -Confirm:$false } catch { Write-Host "Remove-VMSnapshot: $_" }
        } else { Write-Host "No checkpoint '$Checkpoint' to restore" }
    }
    $state = (Get-VM -Name $VMName).State
    if ($state -eq 'Saved') { Remove-VMSavedState -VMName $VMName }
    elseif ($state -ne 'Off') { Stop-VM -Name $VMName -TurnOff -Force }
    Start-Sleep -Seconds 10  # let a late checkpoint show up before counting
    $left = @(Get-VMSnapshot -VMName $VMName | Where-Object Name -eq $Checkpoint).Count
    Write-Host "VM '$VMName' is $((Get-VM -Name $VMName).State); '$Checkpoint' checkpoints left: $left"
    if ($left) { $failures.Add("the checkpoint '$Checkpoint' is still there") }
    Write-Host ("{0} failure(s)" -f $failures.Count)
    $failures | ForEach-Object { Write-Host "  - $_" }
    Stop-Transcript | Out-Null
}
if ($failures.Count) { exit 1 }
