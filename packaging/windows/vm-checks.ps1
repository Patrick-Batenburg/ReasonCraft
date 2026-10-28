<#
.SYNOPSIS
  Helpers for verify-in-vm.ps1, dot-sourced on the host. Each one runs inside the Hyper-V test
  VM through a PowerShell Direct session; the guest blocks are Windows PowerShell 5.1. Nothing
  here sends keyboard or mouse input, to the guest or to the host.
#>

# Wait for the guest's heartbeat, then for PowerShell Direct (a guest booted after $BootedAfter).
function Connect-VMSession([string] $VMName, [pscredential] $Credential, [datetime] $BootedAfter = [datetime]::MinValue) {
    $deadline = (Get-Date).AddMinutes(6)
    while ((Get-VM -Name $VMName).Heartbeat -notlike 'Ok*' -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 5 }
    while ((Get-Date) -lt $deadline) {
        $s = New-PSSession -VMName $VMName -Credential $Credential -ErrorAction SilentlyContinue
        if ($s) {
            $boot = Invoke-Command -Session $s { (Get-CimInstance Win32_OperatingSystem).LastBootUpTime }
            if ($boot -gt $BootedAfter) { return $s }
            Remove-PSSession $s
        }
        Start-Sleep -Seconds 5
    }
    throw 'The VM did not answer over PowerShell Direct.'
}

$script:SignedIn = {
    @(Get-CimInstance Win32_Process -Filter "Name='explorer.exe'" | Where-Object {
        (Invoke-CimMethod -InputObject $_ -MethodName GetOwner).User -eq $env:USERNAME }).Count -gt 0
}

# A window needs a signed-in desktop. If the VM sits at the lock screen, turn on automatic sign-in
# for the test user (guest registry only, undone by the checkpoint) and restart the guest.
# Returns the (possibly new) session.
function Enter-VMDesktop($Session, [string] $VMName, [pscredential] $Credential) {
    if (Invoke-Command -Session $Session $script:SignedIn) { return $Session }
    Write-Host 'Nobody is signed in: enabling automatic sign-in in the guest and restarting it'
    $plain = $Credential.GetNetworkCredential()
    $boot = Invoke-Command -Session $Session -ArgumentList $plain.UserName, $plain.Password {
        param($user, $password)
        $winlogon = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
        Set-ItemProperty $winlogon AutoAdminLogon '1'
        Set-ItemProperty $winlogon DefaultUserName $user.Split('\')[-1]
        Set-ItemProperty $winlogon DefaultDomainName $env:COMPUTERNAME
        Set-ItemProperty $winlogon DefaultPassword $password
        (Get-CimInstance Win32_OperatingSystem).LastBootUpTime
    }
    Invoke-Command -Session $Session { Restart-Computer -Force }
    Remove-PSSession $Session
    $down = (Get-Date).AddMinutes(2)
    while ((Get-VM -Name $VMName).Heartbeat -like 'Ok*' -and (Get-Date) -lt $down) { Start-Sleep 2 }
    $Session = Connect-VMSession $VMName $Credential $boot
    $deadline = (Get-Date).AddMinutes(3)
    while (-not (Invoke-Command -Session $Session $script:SignedIn) -and (Get-Date) -lt $deadline) { Start-Sleep 5 }
    Start-Sleep -Seconds 20  # let the desktop settle
    return $Session
}

# msiexec /i|/x "<Path>" <Arguments> /qn /l*v C:\rc-test\<Log>.log, waited for. Returns the exit code.
function Invoke-Msi($Session, [string] $Path, [string[]] $Arguments, [string] $Log, [ValidateSet('/i', '/x')] [string] $Action = '/i') {
    $code = Invoke-Command -Session $Session -ArgumentList $Action, $Path, ($Arguments -join ' '), $Log {
        param($action, $path, $extra, $log)
        $line = '{0} "{1}" {2} /qn /l*v "C:\rc-test\{3}.log"' -f $action, $path, $extra, $log
        (Start-Process msiexec.exe -ArgumentList $line -Wait -PassThru).ExitCode
    }
    Write-Host ("msiexec {0} {1} {2} -> {3}" -f $Action, (Split-Path -Leaf $Path), ($Arguments -join ' '), $code)
    return $code
}

function Get-MsiLog($Session, [string] $Log) {
    Invoke-Command -Session $Session -ArgumentList $Log {
        param($log) Get-Content "C:\rc-test\$log.log" -Raw -ErrorAction SilentlyContinue
    }
}

# What an install left in one scope: files, shortcuts, ARP entries and registry keys.
function Get-InstallState($Session, [ValidateSet('user', 'machine')] [string] $Scope, [string] $UpgradeCode) {
    Invoke-Command -Session $Session -ArgumentList $Scope, $UpgradeCode {
        param($scope, $upgradeCode)
        $hive = if ($scope -eq 'user') { 'HKCU:' } else { 'HKLM:' }
        $dir = if ($scope -eq 'user') { Join-Path $env:LOCALAPPDATA 'Programs\ReasonCraft' } else { Join-Path $env:ProgramW6432 'ReasonCraft' }
        $menu = if ($scope -eq 'user') { "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" } else { "$env:ProgramData\Microsoft\Windows\Start Menu\Programs" }
        # An explicit $null: a function that outputs nothing yields AutomationNull, which arrives on
        # the host as an empty, truthy object.
        function Val([string] $key, [string] $name = '') {
            $k = Get-Item -LiteralPath $key -ErrorAction SilentlyContinue
            if ($k) { return $k.GetValue($name) }
            return $null
        }
        # Every ARP entry named ReasonCraft, in both hives and both registry views. Windows Installer
        # writes a per-user product's entry under HKLM too, so the scope comes from AssignmentType.
        $arp = @(foreach ($root in 'HKCU:', 'HKLM:') {
            foreach ($sub in 'Software\Microsoft\Windows\CurrentVersion\Uninstall', 'Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall') {
                Get-ChildItem "$root\$sub" -ErrorAction SilentlyContinue | ForEach-Object {
                    if ($_.GetValue('DisplayName') -eq 'ReasonCraft') {
                        [pscustomobject]@{ Key = $_.Name; Version = $_.GetValue('DisplayVersion') }
                    }
                }
            }
        })
        # Products sharing the UpgradeCode, in this user's and the machine's context, and the scope
        # (AssignmentType 0 = per user, 1 = per machine) of each.
        $inst = New-Object -ComObject WindowsInstaller.Installer
        $t = $inst.GetType()
        $list = $t.InvokeMember('RelatedProducts', 'GetProperty', $null, $inst, @("{$upgradeCode}"))
        $related = $list.GetType().InvokeMember('Count', 'GetProperty', $null, $list, $null)
        $want = if ($scope -eq 'user') { '0' } else { '1' }
        $mine = @(for ($i = 0; $i -lt $related; $i++) {
            $code = $list.GetType().InvokeMember('Item', 'GetProperty', $null, $list, @($i))
            if ($t.InvokeMember('ProductInfo', 'GetProperty', $null, $inst, @($code, 'AssignmentType')) -eq $want) {
                $t.InvokeMember('ProductInfo', 'GetProperty', $null, $inst, @($code, 'VersionString'))
            }
        })
        $classes = "$hive\Software\Classes"
        $desktops = [Environment]::GetFolderPath('Desktop'), [Environment]::GetFolderPath('CommonDesktopDirectory')
        [pscustomobject]@{
            DirPath       = $dir
            Dir           = Test-Path -LiteralPath $dir
            Exe           = Test-Path -LiteralPath (Join-Path $dir 'reasoncraft.exe')
            Notices       = @('LICENSE-MIT', 'LICENSE-APACHE', 'THIRD-PARTY.md' | Where-Object { Test-Path (Join-Path $dir $_) }).Count -eq 3
            Products      = $mine.Count           # in this scope
            Version       = if ($mine.Count) { $mine[0] } else { $null }
            Related       = $related              # in either scope
            ArpTotal      = $arp.Count
            ArpVersion    = if ($arp.Count) { $arp[0].Version } else { $null }
            ArpKeys       = ($arp.Key -join '; ')
            ProductKey    = Test-Path "$hive\Software\ReasonCraft"
            InstallDirKey = Val "$hive\Software\ReasonCraft" 'InstallDir'
            AppPath       = Val "$hive\Software\Microsoft\Windows\CurrentVersion\App Paths\reasoncraft.exe"
            Registered    = Val "$hive\Software\RegisteredApplications" 'ReasonCraft'
            AppsKey       = Test-Path "$classes\Applications\reasoncraft.exe"
            ProgIdKey     = Test-Path "$classes\ReasonCraft.rcraft"
            ProgId        = Val "$classes\ReasonCraft.rcraft"
            OpenCommand   = Val "$classes\ReasonCraft.rcraft\shell\open\command"
            ExtKey        = Test-Path "$classes\.rcraft"
            ExtDefault    = Val "$classes\.rcraft"
            ContentType   = Val "$classes\.rcraft" 'Content Type'
            HkcrResolves  = Val 'Registry::HKEY_CLASSES_ROOT\.rcraft'
            StartMenu     = Test-Path "$menu\ReasonCraft.lnk"
            Desktop       = @($desktops | Where-Object { $_ -and (Test-Path (Join-Path $_ 'ReasonCraft.lnk')) }).Count -gt 0
        }
    }
}

# The "double-click": the shell opens the file by its association, in the signed-in user's
# desktop (an interactive scheduled task; no input is injected). Catches reasoncraft.exe however
# briefly it lives, with its command line, and whether it is still up 15 s later.
function Test-OpenByAssociation($Session, [string] $File) {
    Invoke-Command -Session $Session -ArgumentList $File {
        param($file)
        Get-Process reasoncraft -ErrorAction SilentlyContinue | Stop-Process -Force
        Start-Sleep -Seconds 1
        $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive
        $action = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument ('/c start "" "' + $file + '"')
        Register-ScheduledTask -TaskName 'rc-open' -Action $action -Principal $principal -Force | Out-Null
        Start-ScheduledTask -TaskName 'rc-open'
        $seen = $null
        $deadline = (Get-Date).AddSeconds(20)
        while (-not $seen -and (Get-Date) -lt $deadline) {
            $seen = Get-CimInstance Win32_Process -Filter "Name='reasoncraft.exe'" | Select-Object -First 1
            if (-not $seen) { Start-Sleep -Milliseconds 50 }
        }
        Start-Sleep -Seconds 15
        Unregister-ScheduledTask -TaskName 'rc-open' -Confirm:$false -ErrorAction SilentlyContinue
        [pscustomobject]@{
            Started     = [bool]$seen
            ExePath     = $seen.ExecutablePath
            CommandLine = $seen.CommandLine
            StillUp     = [bool](Get-Process reasoncraft -ErrorAction SilentlyContinue)
        }
    }
}

# With WGPU_BACKEND naming a backend Windows lacks, wgpu cannot start; the app tries OpenGL next,
# which this VM lacks too (no OpenGL 2), so the start-up error dialog must show. The exe is a GUI
# exe: it prints only because its output is redirected to a file.
function Test-RendererFallback($Session, [string] $Exe, [string] $File) {
    Invoke-Command -Session $Session -ArgumentList $Exe, $File {
        param($exe, $file)
        Get-Process reasoncraft -ErrorAction SilentlyContinue | Stop-Process -Force
        Start-Sleep -Seconds 2
        $arguments = '/c "set WGPU_BACKEND=metal&& "' + $exe + '" "' + $file + '" > C:\rc-test\fallback.txt 2>&1"'
        # A console host without a window, so that only the dialog shows.
        $action = New-ScheduledTaskAction -Execute 'conhost.exe' -Argument ('--headless cmd.exe ' + $arguments)
        $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive
        Register-ScheduledTask -TaskName 'rc-fallback' -Action $action -Principal $principal -Force | Out-Null
        Start-ScheduledTask -TaskName 'rc-fallback'
        Start-Sleep -Seconds 15
        Unregister-ScheduledTask -TaskName 'rc-fallback' -Confirm:$false
        [pscustomobject]@{
            Waiting = [bool](Get-Process reasoncraft -ErrorAction SilentlyContinue)  # on its dialog
            Output  = Get-Content C:\rc-test\fallback.txt -Raw -ErrorAction SilentlyContinue
        }
    }
}

function Stop-App($Session) {
    Invoke-Command -Session $Session {
        Get-Process reasoncraft -ErrorAction SilentlyContinue | Stop-Process -Force
        Start-Sleep -Seconds 2
    }
}

# The VM screen through WMI (no input is sent to the VM or the host).
function Save-VMScreen([string] $Path, [string] $VMName = $script:VMName) {
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
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    Write-Host "VM screen saved to $Path ($w x $h)"
}
