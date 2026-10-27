<#
.SYNOPSIS
  Checks the ReasonCraft Windows installer inside the Hyper-V test VM:
  silent install, the .rcraft association, opening a map, silent uninstall.

.DESCRIPTION
  Runs on the host, never changes the host. The VM is reached through
  PowerShell Direct with the credential saved in
  %USERPROFILE%\.vm-sandbox-cred.xml (Import-Clixml). Order:
    checkpoint "before-reasoncraft" -> start -> copy in -> install /S ->
    install /S again (an upgrade) -> assertions + screen capture ->
    renderer fallback (error dialog) -> uninstall /S -> assertions ->
    turn off -> restore the checkpoint -> remove it (the VM is left off).
  The steps and results go to -OutDir as task-11-vm-transcript.txt, the VM
  screen as task-11-vm-screen.png and task-11-vm-fallback.png.

.EXAMPLE
  cargo run -p xtask -- sample-map target\sample.rcraft
  .\verify-in-vm.ps1 -Installer target\release\packager\reasoncraft_0.1.0_x64-setup.exe `
                     -Sample target\sample.rcraft -OutDir target\vm-check
#>
param(
    [Parameter(Mandatory)] [string] $Installer,
    [Parameter(Mandatory)] [string] $Sample,
    [Parameter(Mandatory)] [string] $OutDir,
    [string] $VMName = 'Windows 11 Sandbox',
    [string] $Checkpoint = 'before-reasoncraft'
)
$ErrorActionPreference = 'Stop'
$Installer = (Resolve-Path $Installer).Path
$Sample = (Resolve-Path $Sample).Path
New-Item -ItemType Directory -Force $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path
Start-Transcript -Path (Join-Path $OutDir 'task-11-vm-transcript.txt') -Force | Out-Null
$failures = [System.Collections.Generic.List[string]]::new()
function Check([string] $what, [bool] $ok) {
    Write-Host ("[{0}] {1}" -f ($(if ($ok) { 'PASS' } else { 'FAIL' })), $what)
    if (-not $ok) { $failures.Add($what) }
}

# The VM screen through WMI (no input is sent to the VM or the host).
function Save-VMScreen([string] $path) {
    Add-Type -AssemblyName System.Drawing
    $ns = 'root\virtualization\v2'
    $vm = Get-CimInstance -Namespace $ns -ClassName Msvm_ComputerSystem -Filter "ElementName='$VMName'"
    $mgmt = Get-CimInstance -Namespace $ns -ClassName Msvm_VirtualSystemManagementService
    $vs = Get-CimAssociatedInstance -InputObject $vm -ResultClassName Msvm_VirtualSystemSettingData |
        Where-Object VirtualSystemType -eq 'Microsoft:Hyper-V:System:Realized'
    $head = Get-CimAssociatedInstance -InputObject $vm -ResultClassName Msvm_VideoHead | Select-Object -First 1
    $w = $head.CurrentHorizontalResolution; $h = $head.CurrentVerticalResolution
    $r = Invoke-CimMethod -InputObject $mgmt -MethodName GetVirtualSystemThumbnailImage `
        -Arguments @{ TargetSystem = $vs; WidthPixels = [uint16]$w; HeightPixels = [uint16]$h }
    $bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format16bppRgb565)
    $data = $bmp.LockBits((New-Object System.Drawing.Rectangle(0, 0, $w, $h)),
        [System.Drawing.Imaging.ImageLockMode]::WriteOnly, $bmp.PixelFormat)
    for ($y = 0; $y -lt $h; $y++) {
        [System.Runtime.InteropServices.Marshal]::Copy($r.ImageData, $y * $w * 2,
            [IntPtr]($data.Scan0.ToInt64() + $y * $data.Stride), $w * 2)
    }
    $bmp.UnlockBits($data)
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    Write-Host "VM screen saved to $path ($w x $h)"
}

$vm = Get-VM -Name $VMName
Write-Host "VM '$VMName' is $($vm.State)"
if (Get-VMSnapshot -VMName $VMName -Name $Checkpoint -ErrorAction SilentlyContinue) {
    throw "A checkpoint '$Checkpoint' already exists; remove or restore it first."
}
Checkpoint-VM -Name $VMName -SnapshotName $Checkpoint
Write-Host "Checkpoint '$Checkpoint' taken"
try {
    $cred = Import-Clixml "$env:USERPROFILE\.vm-sandbox-cred.xml"
    if ($vm.State -ne 'Running') { Start-VM -Name $VMName }
    # Wait for the guest's heartbeat, then for PowerShell Direct (a guest
    # booted after `$bootedAfter`). Failed attempts stay out of the transcript.
    function Connect([datetime] $bootedAfter = [datetime]::MinValue) {
        $deadline = (Get-Date).AddMinutes(6)
        while ((Get-VM -Name $VMName).Heartbeat -notlike 'Ok*' -and (Get-Date) -lt $deadline) {
            Start-Sleep -Seconds 5
        }
        while ((Get-Date) -lt $deadline) {
            $s = New-PSSession -VMName $VMName -Credential $cred -ErrorAction SilentlyContinue
            if ($s) {
                $boot = Invoke-Command -Session $s { (Get-CimInstance Win32_OperatingSystem).LastBootUpTime }
                if ($boot -gt $bootedAfter) { return $s }
                Remove-PSSession $s
            }
            Start-Sleep -Seconds 5
        }
        throw 'The VM did not answer over PowerShell Direct.'
    }
    $signedIn = {
        @(Get-CimInstance Win32_Process -Filter "Name='explorer.exe'" | Where-Object {
            (Invoke-CimMethod -InputObject $_ -MethodName GetOwner).User -eq $env:USERNAME }).Count -gt 0
    }
    $session = Connect
    Write-Host ('Connected to ' + (Invoke-Command -Session $session { "$env:COMPUTERNAME as $(whoami)" }))

    # A window needs a signed-in desktop. If the VM sits at the lock screen,
    # turn on automatic sign-in for the test user (guest registry only, undone
    # by the checkpoint) and restart the guest. No input is sent.
    if (-not (Invoke-Command -Session $session $signedIn)) {
        Write-Host 'Nobody is signed in: enabling automatic sign-in in the guest and restarting it'
        $plain = $cred.GetNetworkCredential()
        $boot = Invoke-Command -Session $session -ArgumentList $plain.UserName, $plain.Password {
            param($user, $password)
            $winlogon = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
            $name = $user.Split('\')[-1]
            Set-ItemProperty $winlogon AutoAdminLogon '1'
            Set-ItemProperty $winlogon DefaultUserName $name
            Set-ItemProperty $winlogon DefaultDomainName $env:COMPUTERNAME
            Set-ItemProperty $winlogon DefaultPassword $password
            (Get-CimInstance Win32_OperatingSystem).LastBootUpTime
        }
        Invoke-Command -Session $session { Restart-Computer -Force }
        Remove-PSSession $session
        $down = (Get-Date).AddMinutes(2)
        while ((Get-VM -Name $VMName).Heartbeat -like 'Ok*' -and (Get-Date) -lt $down) { Start-Sleep 2 }
        $session = Connect $boot
        $deadline = (Get-Date).AddMinutes(3)
        while (-not (Invoke-Command -Session $session $signedIn) -and (Get-Date) -lt $deadline) { Start-Sleep 5 }
        Start-Sleep -Seconds 20  # let the desktop settle
    }
    $interactive = Invoke-Command -Session $session $signedIn
    Check 'the test user is signed in to the VM desktop' $interactive

    Invoke-Command -Session $session { New-Item -ItemType Directory -Force C:\rc-test | Out-Null }
    Copy-Item -ToSession $session -Path $Installer -Destination C:\rc-test\setup.exe
    Copy-Item -ToSession $session -Path $Sample -Destination 'C:\rc-test\Sample map.rcraft'
    Write-Host 'Installer and sample copied in'

    # --- Install ---------------------------------------------------------
    $installed = Invoke-Command -Session $session {
        # A document of the user's own, which the uninstaller must leave.
        $docs = [Environment]::GetFolderPath('MyDocuments')
        Copy-Item 'C:\rc-test\Sample map.rcraft' (Join-Path $docs 'Keep me.rcraft')
        $first = Start-Process C:\rc-test\setup.exe -ArgumentList '/S' -Wait -PassThru
        # Install again over it, as an upgrade does.
        $p = Start-Process C:\rc-test\setup.exe -ArgumentList '/S' -Wait -PassThru
        $uninst = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ReasonCraft'
        $dir = (Get-ItemProperty $uninst -ErrorAction SilentlyContinue).InstallLocation
        $dir = if ($dir) { $dir.Trim('"') } else { "$env:LOCALAPPDATA\Programs\ReasonCraft" }
        $ext = Get-Item 'HKCU:\Software\Classes\.rcraft' -ErrorAction SilentlyContinue
        $progId = if ($ext) { $ext.GetValue('') } else { $null }
        $cmdKey = "HKCU:\Software\Classes\$progId\shell\open\command"
        [pscustomobject]@{
            ExitCode = '{0}, then {1}' -f $first.ExitCode, $p.ExitCode
            Dir      = $dir
            Exe      = Test-Path (Join-Path $dir 'reasoncraft.exe')
            Uninst   = Test-Path $uninst
            ProgId   = $progId
            TypeName = if ($progId) { (Get-Item "HKCU:\Software\Classes\$progId" -ErrorAction SilentlyContinue).GetValue('') } else { $null }
            Backup   = if ($ext) { $ext.GetValue("$progId`_backup") } else { $null }
            Product  = Test-Path 'HKCU:\Software\ReasonCraft contributors\ReasonCraft'
            Command  = if ($progId) { (Get-Item $cmdKey -ErrorAction SilentlyContinue).GetValue('') } else { $null }
            Assoc    = (Get-ItemProperty 'Registry::HKEY_CLASSES_ROOT\.rcraft' -ErrorAction SilentlyContinue).'(default)'
            Docs     = $docs
            Shortcut = [bool](Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" -Recurse -Filter ReasonCraft.lnk)
            Desktop  = Test-Path ([Environment]::GetFolderPath('Desktop') + '\ReasonCraft.lnk')
        }
    }
    $installed | Format-List | Out-String | Write-Host
    Check 'installer exits 0, twice' ($installed.ExitCode -eq '0, then 0')
    Check 'reasoncraft.exe installed per user' ($installed.Exe -and $installed.Dir -like '*\AppData\Local\Programs\ReasonCraft')
    Check 'uninstall entry under HKCU' $installed.Uninst
    Check 'Start menu shortcut' $installed.Shortcut
    Check 'desktop shortcut' $installed.Desktop
    Check '.rcraft ProgID is "ReasonCraft.rcraft"' ($installed.ProgId -eq 'ReasonCraft.rcraft')
    Check 'the ProgID is described as "ReasonCraft map"' ($installed.TypeName -eq 'ReasonCraft map')
    Check 'the reinstall left no backup naming our own ProgID' (-not $installed.Backup)
    Check 'product key under HKCU\Software\ReasonCraft contributors' $installed.Product
    Check 'ProgID opens reasoncraft.exe with the file' ($installed.Command -like '"*\reasoncraft.exe" "%1"')
    Check 'HKCR .rcraft resolves to the ProgID' ($installed.Assoc -eq 'ReasonCraft.rcraft')

    # --- Open the sample through the association --------------------------
    # The app must start in the signed-in user's desktop, so the shell
    # open runs as an interactive scheduled task (no input is injected).
    $opened = Invoke-Command -Session $session {
        $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive
        function Run-AsUser([string] $name, [string] $arguments) {
            $action = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument $arguments
            Register-ScheduledTask -TaskName $name -Action $action -Principal $principal -Force | Out-Null
            Start-ScheduledTask -TaskName $name
        }
        # Double-click equivalent: the shell opens the file by its association.
        Run-AsUser 'rc-open' '/c start "" "C:\rc-test\Sample map.rcraft"'
        # Catch the process however briefly it lives, with its command line.
        $seen = $null
        $deadline = (Get-Date).AddSeconds(15)
        while (-not $seen -and (Get-Date) -lt $deadline) {
            $seen = Get-CimInstance Win32_Process -Filter "Name='reasoncraft.exe'" | Select-Object -First 1
            if (-not $seen) { Start-Sleep -Milliseconds 50 }
        }
        Start-Sleep -Seconds 15
        $running = [bool](Get-Process reasoncraft -ErrorAction SilentlyContinue)
        $output = $null
        if (-not $running) {
            # Run the exe itself with the file, keeping what it prints.
            $exe = Join-Path $env:LOCALAPPDATA 'Programs\ReasonCraft\reasoncraft.exe'
            Run-AsUser 'rc-direct' ('/c ""' + $exe + '" "C:\rc-test\Sample map.rcraft" > C:\rc-test\output.txt 2>&1"')
            Start-Sleep -Seconds 15
            $output = 'exit code {0}: {1}' -f (Get-ScheduledTaskInfo -TaskName 'rc-direct').LastTaskResult,
                (Get-Content C:\rc-test\output.txt -Raw -ErrorAction SilentlyContinue)
        }
        'rc-open', 'rc-direct' | ForEach-Object {
            Unregister-ScheduledTask -TaskName $_ -Confirm:$false -ErrorAction SilentlyContinue
        }
        [pscustomobject]@{
            Started     = [bool]$seen
            CommandLine = $seen.CommandLine
            StillUp     = $running
            DirectRun   = $output
        }
    }
    $opened | Format-List | Out-String | Write-Host
    Check 'opening the sample starts reasoncraft.exe' $opened.Started
    Check 'with the file as its argument' ($opened.CommandLine -like '*"C:\rc-test\Sample map.rcraft"')
    Check 'the app stays up with its window' $opened.StillUp
    Save-VMScreen (Join-Path $OutDir 'task-11-vm-screen.png')

    # --- Renderer fallback -------------------------------------------------
    # With WGPU_BACKEND naming a backend Windows lacks, wgpu cannot start;
    # the app tries OpenGL next, which this VM lacks too (no OpenGL 2), so
    # the start-up error dialog must show.
    $fallback = Invoke-Command -Session $session {
        Get-Process reasoncraft -ErrorAction SilentlyContinue | Stop-Process -Force
        Start-Sleep -Seconds 2
        $exe = Join-Path $env:LOCALAPPDATA 'Programs\ReasonCraft\reasoncraft.exe'
        $arguments = '/c "set WGPU_BACKEND=metal&& "' + $exe + '" "C:\rc-test\Sample map.rcraft" > C:\rc-test\fallback.txt 2>&1"'
        # A console host without a window, so that only the dialog shows.
        $action = New-ScheduledTaskAction -Execute 'conhost.exe' -Argument ('--headless cmd.exe ' + $arguments)
        $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive
        Register-ScheduledTask -TaskName 'rc-fallback' -Action $action -Principal $principal -Force | Out-Null
        Start-ScheduledTask -TaskName 'rc-fallback'
        Start-Sleep -Seconds 15
        Unregister-ScheduledTask -TaskName 'rc-fallback' -Confirm:$false
        [pscustomobject]@{
            # Still running: waiting on its dialog.
            Waiting = [bool](Get-Process reasoncraft -ErrorAction SilentlyContinue)
            Output  = Get-Content C:\rc-test\fallback.txt -Raw -ErrorAction SilentlyContinue
        }
    }
    $fallback | Format-List | Out-String | Write-Host
    Check 'wgpu without a backend gives way' ($fallback.Output -like '*the wgpu renderer did not start*')
    Check 'OpenGL is tried next, then the error dialog' ($fallback.Output -like '*no renderer opened the window*opengl 2.0*')
    Check 'the app waits on its error dialog' $fallback.Waiting
    Save-VMScreen (Join-Path $OutDir 'task-11-vm-fallback.png')

    # --- Uninstall -------------------------------------------------------
    $removed = Invoke-Command -Session $session -ArgumentList $installed.Dir, $installed.Docs {
        param($dir, $docs)
        Get-Process reasoncraft -ErrorAction SilentlyContinue | Stop-Process -Force
        Start-Sleep -Seconds 2
        # The NSIS uninstaller copies itself away and returns at once: wait.
        Start-Process (Join-Path $dir 'uninstall.exe') -ArgumentList '/S' -Wait
        $deadline = (Get-Date).AddSeconds(90)
        while ((Test-Path (Join-Path $dir 'reasoncraft.exe')) -and (Get-Date) -lt $deadline) { Start-Sleep 2 }
        Start-Sleep -Seconds 3
        [pscustomobject]@{
            Exe      = Test-Path (Join-Path $dir 'reasoncraft.exe')
            Dir      = Test-Path $dir
            Uninst   = Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ReasonCraft'
            Ext      = Test-Path 'HKCU:\Software\Classes\.rcraft'
            ProgId   = Test-Path 'HKCU:\Software\Classes\ReasonCraft.rcraft'
            Product  = Test-Path 'HKCU:\Software\ReasonCraft contributors'
            Assoc    = (Get-ItemProperty 'Registry::HKEY_CLASSES_ROOT\.rcraft' -ErrorAction SilentlyContinue).'(default)'
            Shortcut = [bool](Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" -Recurse -Filter ReasonCraft.lnk)
            Desktop  = Test-Path ([Environment]::GetFolderPath('Desktop') + '\ReasonCraft.lnk')
            KeptDoc  = Test-Path (Join-Path $docs 'Keep me.rcraft')
            AppData  = Test-Path "$env:LOCALAPPDATA\ReasonCraft\recovery"
        }
    }
    $removed | Format-List | Out-String | Write-Host
    Check 'exe removed' (-not $removed.Exe)
    Check 'install folder removed' (-not $removed.Dir)
    Check 'uninstall entry removed' (-not $removed.Uninst)
    Check 'Start menu shortcut removed' (-not $removed.Shortcut)
    Check 'desktop shortcut removed' (-not $removed.Desktop)
    Check '.rcraft key removed' (-not $removed.Ext)
    Check 'ProgID removed' (-not $removed.ProgId)
    Check 'HKCR .rcraft no longer resolves' (-not $removed.Assoc)
    Check 'publisher and product keys removed' (-not $removed.Product)
    Check 'the document in Documents survives' $removed.KeptDoc
    Check "the app's recovery folder (user data) is left alone" $removed.AppData
    Remove-PSSession $session
}
finally {
    Write-Host 'Turning the VM off and restoring the checkpoint'
    Stop-VM -Name $VMName -TurnOff -Force -ErrorAction Continue
    Restore-VMSnapshot -VMName $VMName -Name $Checkpoint -Confirm:$false
    Remove-VMSnapshot -VMName $VMName -Name $Checkpoint -Confirm:$false
    Write-Host "VM '$VMName' is $((Get-VM -Name $VMName).State); checkpoints left: $(@(Get-VMSnapshot -VMName $VMName | Where-Object Name -eq $Checkpoint).Count)"
    Write-Host ("{0} failure(s)" -f $failures.Count)
    $failures | ForEach-Object { Write-Host "  - $_" }
    Stop-Transcript | Out-Null
}
if ($failures.Count) { exit 1 }
