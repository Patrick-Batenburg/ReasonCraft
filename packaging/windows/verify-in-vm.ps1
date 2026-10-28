<#
.SYNOPSIS
  Checks the ReasonCraft MSI inside the Hyper-V test VM: per user and per machine, a major upgrade,
  the refusal of a second copy in the other scope, opening a map by its association, and uninstall.

.DESCRIPTION
  Runs on the host (elevated, for Hyper-V) and never installs anything on the host. The VM is
  reached through PowerShell Direct with the credential saved in %USERPROFILE%\.vm-sandbox-cred.xml
  (Import-Clixml); no keyboard or mouse input is sent anywhere. The VM routine is fixed:
    checkpoint -Checkpoint -> start -> checks -> turn off -> restore the checkpoint -> remove it,
  and the VM is left Off, also when a check fails or the script throws.

  Checks, in order:
    1. per user: -OlderMsi, then -Msi (a major upgrade); files, ARP, registry, association, shortcut
    2. between the two: -Msi with ALLUSERS=1 is refused ("already installed for your account") and
       HKLM stays clean; after the upgrade, the same package with ALLUSERS=1 only reconfigures the
       per-user copy (Windows Installer's maintenance mode) and adds no second copy
    3. opening "Een kaart.rcraft" (with accents) by its association starts the app with the path
       as one quoted argument; screen capture; the renderer fallback dialog
    4. per-user uninstall: everything from 1 is gone; a map in Documents and the app's recovery
       folder survive
    5. per machine (ALLUSERS=1): the same as 1 and 2 the other way round, and opening by association
    6. per-machine uninstall: everything from 5 is gone
  Output in -OutDir: msi-vm-transcript.txt, the msiexec logs, msi-vm-*.png.

.EXAMPLE
  cargo xtask sample-map target\sample.rcraft
  .\packaging\windows\verify-in-vm.ps1 -Msi dist\release\reasoncraft-0.1.0-windows-x64.msi `
      -OlderMsi target\vm-old\reasoncraft-0.0.1-windows-x64.msi -Sample target\sample.rcraft -OutDir target\vm-check
#>
param(
    [Parameter(Mandatory)] [string] $Msi,
    [Parameter(Mandatory)] [string] $OlderMsi,
    [Parameter(Mandatory)] [string] $Sample,
    [Parameter(Mandatory)] [string] $OutDir,
    [string] $VMName = 'Windows 11 Sandbox',
    [string] $Checkpoint = 'before-reasoncraft'
)
$ErrorActionPreference = 'Stop'
# Fail fast, before any checkpoint, on a missing input.
$Msi = (Resolve-Path $Msi).Path
$OlderMsi = (Resolve-Path $OlderMsi).Path
$Sample = (Resolve-Path $Sample).Path
. (Join-Path $PSScriptRoot 'vm-checks.ps1')
$UpgradeCode = (Get-Content -Raw (Join-Path $PSScriptRoot 'install-policy.json') | ConvertFrom-Json).upgradeCode

# The new MSI's ProductVersion, read from its database (read-only; nothing is installed).
function Get-MsiVersion([string] $Path) {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($Path, 0))
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @("SELECT Value FROM Property WHERE Property='ProductVersion'"))
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
    $rec = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
    $v = $rec.GetType().InvokeMember('StringData', 'GetProperty', $null, $rec, 1)
    $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null) | Out-Null
    return $v
}
$NewVersion = Get-MsiVersion $Msi
$OldVersion = Get-MsiVersion $OlderMsi
if ([version]$OldVersion -ge [version]$NewVersion) { throw "-OlderMsi ($OldVersion) must be older than -Msi ($NewVersion)." }

New-Item -ItemType Directory -Force $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path
Start-Transcript -Path (Join-Path $OutDir 'msi-vm-transcript.txt') -Force | Out-Null
$failures = [System.Collections.Generic.List[string]]::new()
function Check([string] $what, [bool] $ok) {
    Write-Host ("[{0}] {1}" -f ($(if ($ok) { 'PASS' } else { 'FAIL' })), $what)
    if (-not $ok) { $failures.Add($what) }
}
function Show($o) { $o | Format-List | Out-String -Width 200 | Write-Host }

# Everything an install in $scope must have put in place.
function Assert-Installed($s, [string] $scope) {
    Show $s
    $exe = Join-Path $s.DirPath 'reasoncraft.exe'
    Check "${scope}: reasoncraft.exe in $($s.DirPath)" $s.Exe
    Check "${scope}: LICENSE-MIT, LICENSE-APACHE and THIRD-PARTY.md beside it" $s.Notices
    Check "${scope}: Windows Installer has $NewVersion in this scope" ($s.Products -eq 1 -and $s.Version -eq $NewVersion)
    Check "${scope}: one product with the UpgradeCode in either scope" ($s.Related -eq 1)
    Check "${scope}: one ARP entry, $NewVersion" ($s.ArpTotal -eq 1 -and $s.ArpVersion -eq $NewVersion)
    Check "${scope}: Software\ReasonCraft\InstallDir is the install folder" ("$($s.InstallDirKey)".TrimEnd('\') -eq $s.DirPath)
    Check "${scope}: App Paths\reasoncraft.exe points to the exe" ($s.AppPath -eq $exe)
    Check "${scope}: RegisteredApplications names the Capabilities" ($s.Registered -eq 'Software\ReasonCraft\Capabilities')
    Check "${scope}: .rcraft defaults to ReasonCraft.rcraft" ($s.ExtDefault -eq 'ReasonCraft.rcraft')
    Check "${scope}: .rcraft Content Type is application/x-reasoncraft" ($s.ContentType -eq 'application/x-reasoncraft')
    Check "${scope}: the ProgID is described as ""ReasonCraft map""" ($s.ProgId -eq 'ReasonCraft map')
    Check "${scope}: the ProgID opens ""$exe"" ""%1""" ($s.OpenCommand -eq ('"{0}" "%1"' -f $exe))
    Check "${scope}: HKCR\.rcraft resolves to the ProgID" ($s.HkcrResolves -eq 'ReasonCraft.rcraft')
    Check "${scope}: ReasonCraft.lnk in the Start menu" $s.StartMenu
    Check "${scope}: no desktop shortcut" (-not $s.Desktop)
}
# Nothing of an install left in $scope.
function Assert-Absent($s, [string] $what) {
    Show $s
    $left = @(foreach ($p in 'Dir', 'Exe', 'ProductKey', 'InstallDirKey', 'AppPath', 'Registered', 'AppsKey',
            'ProgIdKey', 'ExtKey', 'StartMenu', 'Desktop') { if ($s.$p) { $p } })
    if ($s.Products) { $left += 'Products' }
    Check "${what} (left: $($left -join ', '))" (-not $left)
}
# A newer build in the other scope must be refused by install-policy.wxi's launch condition.
function Assert-Refused([string] $label, [string[]] $msiArgs, [string] $message, [string] $keep, [string] $other) {
    Check "${label}: refused with 1603" ((Invoke-Msi $session $newGuest $msiArgs $label) -eq 1603)
    Check "${label}: the log says ""$message""" ((Get-MsiLog $session $label) -like "*$message*")
    Assert-Absent (Get-InstallState $session $other $UpgradeCode) "${label}: nothing added in the $other scope"
    $s = Get-InstallState $session $keep $UpgradeCode
    Check "${label}: the $keep copy ($OldVersion) is still the only one" ($s.Products -eq 1 -and $s.Version -eq $OldVersion -and $s.Related -eq 1)
}
# The very same package in the other scope: Windows Installer finds the product already installed
# and only reconfigures that copy (maintenance mode). It must never add a second copy.
function Assert-NoSecondCopy([string] $label, [string[]] $msiArgs, [string] $keep, [string] $other) {
    Check "${label}: exits 0 (maintenance of the existing copy)" ((Invoke-Msi $session $newGuest $msiArgs $label) -eq 0)
    Assert-Absent (Get-InstallState $session $other $UpgradeCode) "${label}: nothing added in the $other scope"
    $s = Get-InstallState $session $keep $UpgradeCode
    Check "${label}: the $keep copy is still the only one" ($s.Products -eq 1 -and $s.Exe -and $s.Related -eq 1 -and $s.ArpTotal -eq 1)
}
function Assert-Opened($o, [string] $exe, [string] $file) {
    Show $o
    Check "opening '$file' starts $exe" ($o.Started -and $o.ExePath -eq $exe)
    Check 'with the path as one quoted argument' ($o.CommandLine -eq ('"{0}" "{1}"' -f $exe, $file))
    Check 'the app stays up with its window' $o.StillUp
}

$vm = Get-VM -Name $VMName
Write-Host "VM '$VMName' is $($vm.State); testing $NewVersion over $OldVersion"
if (Get-VMSnapshot -VMName $VMName -Name $Checkpoint -ErrorAction SilentlyContinue) {
    throw "A checkpoint '$Checkpoint' already exists; restore or remove it first."
}
# Hyper-V can list a new checkpoint only some time after Checkpoint-VM returns (seen after a failed
# Start-VM: the restore found nothing, the checkpoint showed up later). Wait for it.
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
    Write-Host ('Connected to ' + (Invoke-Command -Session $session { "$env:COMPUTERNAME as $(whoami)" }))
    $session = Enter-VMDesktop $session $VMName $cred
    Check 'the test user is signed in to the VM desktop' (Invoke-Command -Session $session $script:SignedIn)

    # The accented name is built from code points so that this file's encoding cannot matter.
    $accented = "$([char]0xC9)$([char]0xE9)n kaart.rcraft"
    $newGuest = 'C:\rc-test\' + (Split-Path -Leaf $Msi)
    $oldGuest = 'C:\rc-test\' + (Split-Path -Leaf $OlderMsi)
    $plainMap = 'C:\rc-test\Sample map.rcraft'
    $accentMap = "C:\rc-test\$accented"
    Invoke-Command -Session $session { New-Item -ItemType Directory -Force C:\rc-test | Out-Null }
    Copy-Item -ToSession $session -Path $Msi -Destination $newGuest
    Copy-Item -ToSession $session -Path $OlderMsi -Destination $oldGuest
    Copy-Item -ToSession $session -Path $Sample -Destination $plainMap
    Copy-Item -ToSession $session -Path $Sample -Destination $accentMap
    $docs = Invoke-Command -Session $session -ArgumentList $plainMap {
        param($map)
        # A document of the user's own, which no uninstall may touch.
        $docs = [Environment]::GetFolderPath('MyDocuments')
        Copy-Item $map (Join-Path $docs 'Keep me.rcraft')
        $docs
    }
    Write-Host 'MSIs and maps copied in'

    # --- 1. Per user, with a major upgrade -------------------------------------------------------
    Check "per user: $OldVersion installs (exit 0)" ((Invoke-Msi $session $oldGuest @() 'user-old') -eq 0)
    $s = Get-InstallState $session user $UpgradeCode
    Check "per user: $OldVersion is registered per user" ($s.Products -eq 1 -and $s.Version -eq $OldVersion)

    # --- 2. A newer build for everyone over it is refused ---------------------------------------
    Assert-Refused 'cross-machine' @('ALLUSERS=1') 'This app is already installed for your account' user machine

    Check "per user: $NewVersion upgrades it (exit 0)" ((Invoke-Msi $session $newGuest @() 'user-new') -eq 0)
    $user = Get-InstallState $session user $UpgradeCode
    Assert-Installed $user 'per user'
    Check 'per user: in %LOCALAPPDATA%\Programs\ReasonCraft' ($user.DirPath -like '*\AppData\Local\Programs\ReasonCraft')
    Assert-Absent (Get-InstallState $session machine $UpgradeCode) 'per user: nothing under HKLM or Program Files'
    $userExe = Join-Path $user.DirPath 'reasoncraft.exe'
    $version = Invoke-Command -Session $session -ArgumentList $userExe { param($exe) (& $exe --version | Out-String).Trim() }
    Check "per user: the installed exe says 'ReasonCraft $NewVersion'" ($version -eq "ReasonCraft $NewVersion")

    Assert-NoSecondCopy 'same-machine' @('ALLUSERS=1') user machine

    # --- 3. Open by association, then the renderer fallback -------------------------------------
    Assert-Opened (Test-OpenByAssociation $session $accentMap) $userExe $accentMap
    Save-VMScreen (Join-Path $OutDir 'msi-vm-user-open.png') $VMName
    $fallback = Test-RendererFallback $session $userExe $plainMap
    Show $fallback
    Check 'wgpu without a backend gives way' ($fallback.Output -like '*the wgpu renderer did not start*')
    Check 'OpenGL is tried next, then the error dialog' ($fallback.Output -like '*no renderer opened the window*opengl 2.0*')
    Check 'the app waits on its error dialog' $fallback.Waiting
    Save-VMScreen (Join-Path $OutDir 'msi-vm-fallback.png') $VMName
    Stop-App $session

    # --- 4. Uninstall per user ------------------------------------------------------------------
    # The app's own data must survive; make sure there is some to survive.
    $hadRecovery = Invoke-Command -Session $session {
        $rec = Join-Path $env:LOCALAPPDATA 'ReasonCraft\recovery'
        $had = Test-Path $rec
        if (-not $had) { New-Item -ItemType Directory -Force $rec | Out-Null; Set-Content (Join-Path $rec 'marker.txt') 'user data' }
        $had
    }
    Write-Host "The app had created its recovery folder: $hadRecovery"
    Check 'per user: uninstall exits 0' ((Invoke-Msi $session $newGuest @() 'user-uninstall' '/x') -eq 0)
    $gone = Get-InstallState $session user $UpgradeCode
    Assert-Absent $gone 'per user: uninstall left no files, shortcuts or keys'
    Check 'per user: no ARP entry and no product left' ($gone.ArpTotal -eq 0 -and $gone.Related -eq 0)
    Check 'per user: HKCR\.rcraft no longer resolves' (-not $gone.HkcrResolves)
    $kept = Invoke-Command -Session $session -ArgumentList $docs {
        param($docs)
        [pscustomobject]@{
            Doc      = Test-Path (Join-Path $docs 'Keep me.rcraft')
            Recovery = Test-Path (Join-Path $env:LOCALAPPDATA 'ReasonCraft\recovery')
        }
    }
    Check 'Documents\Keep me.rcraft survives' $kept.Doc
    Check '%LOCALAPPDATA%\ReasonCraft\recovery survives' $kept.Recovery

    # --- 5. Per machine -------------------------------------------------------------------------
    $elevated = Invoke-Command -Session $session {
        ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
            [Security.Principal.WindowsBuiltInRole]::Administrator)
    }
    Check 'the VM session is elevated' $elevated
    if (-not $elevated) { throw 'A per-machine install needs an elevated session.' }
    Check "per machine: $OldVersion installs (exit 0)" ((Invoke-Msi $session $oldGuest @('ALLUSERS=1') 'machine-old') -eq 0)
    Assert-Refused 'cross-user' @() 'This app is already installed for everyone' machine user
    Check "per machine: $NewVersion upgrades it (exit 0)" ((Invoke-Msi $session $newGuest @('ALLUSERS=1') 'machine-new') -eq 0)
    $machine = Get-InstallState $session machine $UpgradeCode
    Assert-Installed $machine 'per machine'
    $programFiles = Invoke-Command -Session $session { $env:ProgramW6432 }
    Check 'per machine: in %ProgramW6432%\ReasonCraft' ($machine.DirPath -eq "$programFiles\ReasonCraft")
    Assert-Absent (Get-InstallState $session user $UpgradeCode) 'per machine: nothing under HKCU or the profile'
    Assert-NoSecondCopy 'same-user' @() machine user
    $machineExe = Join-Path $machine.DirPath 'reasoncraft.exe'
    Assert-Opened (Test-OpenByAssociation $session $accentMap) $machineExe $accentMap
    Save-VMScreen (Join-Path $OutDir 'msi-vm-machine-open.png') $VMName
    Stop-App $session

    # --- 6. Uninstall per machine ---------------------------------------------------------------
    Check 'per machine: uninstall exits 0' ((Invoke-Msi $session $newGuest @() 'machine-uninstall' '/x') -eq 0)
    $gone = Get-InstallState $session machine $UpgradeCode
    Assert-Absent $gone 'per machine: uninstall left no files, shortcuts or keys'
    Check 'per machine: no ARP entry and no product left' ($gone.ArpTotal -eq 0 -and $gone.Related -eq 0)
    Check 'per machine: HKCR\.rcraft no longer resolves' (-not $gone.HkcrResolves)
    Check 'Documents\Keep me.rcraft still survives' (Invoke-Command -Session $session -ArgumentList $docs { param($d) Test-Path (Join-Path $d 'Keep me.rcraft') })
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
