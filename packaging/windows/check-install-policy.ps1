# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/check-install-policy.ps1
# at aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: no launcher (per-user only) branch,
# no licence-page route of other apps, the install folder directly under Program Files / Programs,
# no checks of saved options or shield buttons (ReasonCraft has neither), and the HKLM searches
# checked in both registry views.

<#
.SYNOPSIS
  Check the compiled installation policy without installing any product.
.DESCRIPTION
  Uses Windows Installer's safe session (OpenPackage option 1). Only property-setting custom
  actions are exercised. No AppSearch, install, repair, removal or file-writing action runs.
#>
param([Parameter(Mandatory = $true)] [string] $MsiPath)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$policy = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'install-policy.json') | ConvertFrom-Json
function Read-Rows($Database, [string] $Sql, [int] $Columns) {
  $view = $Database.OpenView($Sql)
  try {
    [void]$view.Execute()
    while ($null -ne ($record = $view.Fetch())) {
      try {
        $values = @(for ($i = 1; $i -le $Columns; $i++) {
          $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @($i))
        })
        ,$values
      } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($record) }
    }
  } finally {
    [void]$view.Close()
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
  }
}
function Set-SessionProperty($Session, [string] $Name, [string] $Value) {
  [void]$Session.GetType().InvokeMember('Property', 'SetProperty', $null, $Session, @($Name, $Value))
}
function Get-SessionProperty($Session, [string] $Name) {
  $Session.GetType().InvokeMember('Property', 'GetProperty', $null, $Session, @($Name))
}
function Get-PackageTemplate($Database) {
  # Summary information property 7 is the platform: "Intel;1033", "x64;1033" or "Arm64;1033".
  $summary = $Database.GetType().InvokeMember('SummaryInformation', 'GetProperty', $null, $Database, @(0))
  try { [string]$summary.GetType().InvokeMember('Property', 'GetProperty', $null, $summary, @(7)) }
  finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($summary) }
}
$installer = $null; $database = $null; $session = $null
try {
  $installer = New-Object -ComObject WindowsInstaller.Installer
  $file = (Resolve-Path -LiteralPath $MsiPath).Path
  $database = $installer.OpenDatabase($file, 0)
  $session = $installer.OpenPackage($file, 1)
  $properties = @{}
  foreach ($row in (Read-Rows $database 'SELECT `Property`, `Value` FROM `Property`' 2)) { $properties[$row[0]] = $row[1] }
  $secureProperties = @($properties['SecureCustomProperties'] -split ';')
  if ('INSTALLFOLDER' -notin $secureProperties) {
    throw 'The selected installation folder must survive elevation (secure INSTALLFOLDER).'
  }
  if ($properties['ALLUSERS'] -ne '2' -or $properties['MSIINSTALLPERUSER'] -ne '1') {
    throw 'Policy regression: new packages must default to the per-user dual-scope context.'
  }
  if ($properties['UpgradeCode'].Trim('{}') -ne $policy.upgradeCode) { throw 'The existing upgrade identity must not change.' }
  $directories = @{}
  foreach ($row in (Read-Rows $database 'SELECT `Directory`, `Directory_Parent`, `DefaultDir` FROM `Directory`' 3)) {
    $directories[$row[0]] = @($row[1], $row[2])
  }
  # DefaultDir may hold a short name first ("short|long").
  if (($directories['INSTALLFOLDER'][1] -split '\|')[-1] -ne $policy.name) {
    throw "Expected the app's own folder, $($policy.name), directly under Program Files."
  }
  $programs = $directories['INSTALLFOLDER'][0]
  if ($programs -eq 'ProgramFiles6432Folder') { $programs = $directories[$programs][0] }
  if ($programs -notin 'ProgramFilesFolder', 'ProgramFiles64Folder') {
    throw 'Use Windows Installer native folder redirection, not a hard-coded profile path.'
  }
  $guards = @(Read-Rows $database 'SELECT `Condition`, `Description` FROM `LaunchCondition`' 2 |
    Where-Object { $_[0] -match 'CRAFT_(USER|MACHINE)_DIR' })
  if ($guards.Count -ne 2) { throw 'Expected both duplicate-installation guards.' }
  $cases = 0
  foreach ($scope in @('', '2', '1')) {
    foreach ($user in @('', 'D:\Existing user app')) {
      foreach ($machine in @('', 'E:\Existing machine app')) {
        foreach ($installed in @('', '1')) {
          Set-SessionProperty $session 'ALLUSERS' $scope
          Set-SessionProperty $session 'Installed' $installed
          Set-SessionProperty $session 'CRAFT_USER_DIR' $user
          Set-SessionProperty $session 'CRAFT_MACHINE_DIR' $machine
          $allowed = $true
          foreach ($guard in $guards) {
            $value = $session.EvaluateCondition($guard[0])
            if ($value -notin 0, 1) { throw 'Invalid duplicate guard.' }
            if ($value -eq 0) { $allowed = $false }
          }
          $opposite = if ($scope -eq '1') { $user } else { $machine }
          $expected = [bool]$installed -or -not [bool]$opposite
          if ($allowed -ne $expected) { throw "Scope regression: ALLUSERS='$scope', user=$([bool]$user), machine=$([bool]$machine), maintenance=$([bool]$installed)." }
          $cases++
        }
      }
    }
  }
  # Execute the actual authored type-51 actions in a safe session, not copied resolver logic.
  foreach ($scope in @('', '2', '1')) {
    foreach ($explicit in @('', 'F:\Chosen exact app')) {
      Set-SessionProperty $session 'ALLUSERS' $scope
      Set-SessionProperty $session 'INSTALLFOLDER' $explicit
      Set-SessionProperty $session 'CRAFT_USER_DIR' 'D:\Existing user app'
      Set-SessionProperty $session 'CRAFT_MACHINE_DIR' 'E:\Existing machine app'
      $action = if ($scope -eq '1') { 'CraftRestoreMachineFolder' } else { 'CraftRestoreUserFolder' }
      # Evaluate the compiled condition, so the test exercises it instead of calling the action directly.
      $restore = @(Read-Rows $database ('SELECT `Condition` FROM `InstallExecuteSequence` WHERE `Action` = ''{0}''' -f $action) 1)[0]
      $selected = $session.EvaluateCondition($restore[0]) -eq 1
      if ($selected -eq [bool]$explicit) { throw "$action must run only when no folder was supplied (condition: $($restore[0]))." }
      if ($selected -and $session.DoAction($action) -ne 1) { throw 'Remembered-folder property action failed.' }
      $expected = if ($explicit) { $explicit } elseif ($scope -eq '1') { 'E:\Existing machine app' } else { 'D:\Existing user app' }
      if ((Get-SessionProperty $session 'INSTALLFOLDER') -ne $expected) { throw 'Custom or remembered folder was not preserved.' }
    }
  }
  $ui = @{}
  foreach ($row in (Read-Rows $database 'SELECT `Action`, `Sequence` FROM `InstallUISequence`' 2)) { $ui[$row[0]] = [int]$row[1] }
  if (-not ($ui['AppSearch'] -lt $ui['CraftPreferMachineScope'] -and
      $ui['CraftPreferMachineScope'] -lt $ui['CraftScopeDlg'] -and
      $ui['CraftScopeDlg'] -lt $ui['FindRelatedProducts'] -and
      $ui['FindRelatedProducts'] -lt $ui['LaunchConditions'] -and
      $ui['CraftScopeDlg'] -lt $ui['CraftRestoreUserFolder'] -and $ui['CraftRestoreUserFolder'] -lt $ui['CostInitialize'])) {
    throw 'Scope must be chosen after AppSearch and before FindRelatedProducts, launch checks and costing.'
  }
  # Windows Installer resolves Program Files for the context it started in, before the wizard's scope choice.
  # The wizard therefore sets the new-install folder itself, and only in the wizard.
  $defaults = @(Read-Rows $database 'SELECT `Action`, `Sequence`, `Condition` FROM `InstallUISequence`' 3 | Where-Object { $_[0] -like 'CraftDefault*' })
  if ($defaults.Count -lt 2) { throw 'The wizard must set the new-install folder after the scope page; Program Files is resolved before the choice.' }
  foreach ($row in $defaults) {
    if (-not ($ui['CraftScopeDlg'] -lt [int]$row[1] -and [int]$row[1] -lt $ui['CostInitialize'])) { throw "$($row[0]) must run after the scope page and before costing." }
  }
  if (@(Read-Rows $database 'SELECT `Action` FROM `InstallExecuteSequence`' 1 | Where-Object { $_[0] -like 'CraftDefault*' }).Count) {
    throw 'Silent installs must not use the wizard-only default folder actions.'
  }
  # The machine default comes from the environment of the package architecture, not from a folder name.
  # x86 packages use Program Files (x86) on 64-bit Windows; x64 and Arm64 packages use the 64-bit root.
  $machineRoot = if ((Get-PackageTemplate $database) -match '^Intel') {
    if (Get-SessionProperty $session 'VersionNT64') { ${env:ProgramFiles(x86)} } else { $env:ProgramFiles }
  } elseif ($env:ProgramW6432) { $env:ProgramW6432 } else { $env:ProgramFiles }
  $machineFolder = (Join-Path $machineRoot $policy.name) + '\'
  foreach ($scope in @('1', '')) {
    Set-SessionProperty $session 'ALLUSERS' $scope
    Set-SessionProperty $session 'INSTALLFOLDER' ''
    Set-SessionProperty $session 'CRAFT_MACHINE_DIR' ''
    Set-SessionProperty $session 'CRAFT_USER_DIR' ''
    foreach ($row in @($defaults | Sort-Object { [int]$_[1] })) {
      if ($session.EvaluateCondition($row[2]) -eq 1 -and $session.DoAction($row[0]) -ne 1) { throw "$($row[0]) failed." }
    }
    $folder = Get-SessionProperty $session 'INSTALLFOLDER'
    $wanted = if ($scope -eq '1') { $machineFolder } else { (Join-Path $env:LOCALAPPDATA "Programs\$($policy.name)") + '\' }
    if ($folder -ine $wanted) { throw "Wizard default folder for ALLUSERS='$scope' is '$folder', expected '$wanted'." }
  }
  $execute = @{}
  foreach ($row in (Read-Rows $database 'SELECT `Action`, `Sequence` FROM `InstallExecuteSequence`' 2)) { $execute[$row[0]] = [int]$row[1] }
  foreach ($action in @('CraftRestoreUserFolder','CraftRestoreMachineFolder')) {
    if (-not ($execute['AppSearch'] -lt $execute[$action] -and $execute[$action] -lt $execute['CostInitialize'])) {
      throw 'Silent updates must restore the folder before directory costing.'
    }
  }
  foreach ($resolver in @('CraftUFromLegacyExe','CraftMFromLegacyExe','CraftMOFromLegacyExe')) {
    if (-not ($execute['AppSearch'] -lt $execute[$resolver] -and $execute[$resolver] -lt $execute['LaunchConditions'])) {
      throw 'Every legacy folder resolver must run after AppSearch and before the launch checks.'
    }
  }
  # A per-machine copy registers in its own registry view (x86 in WOW6432Node on 64-bit Windows),
  # and a search reads one view. Every HKLM search must therefore exist in both views, so a copy of
  # another architecture is found too, and a package never installs beside it in the other scope.
  $locators = @(Read-Rows $database 'SELECT `Signature_`, `Root`, `Key`, `Name`, `Type` FROM `RegLocator`' 5)
  $machineLocators = @($locators | Where-Object { $_[1] -eq '2' })
  if ($machineLocators.Count -lt 2) { throw 'Expected the HKLM searches for an existing per-machine copy.' }
  foreach ($row in $machineLocators) {
    $type = if ($row[4]) { [int]$row[4] } else { 0 }
    $twins = @($machineLocators | Where-Object {
      $other = if ($_[4]) { [int]$_[4] } else { 0 }
      $_[2] -eq $row[2] -and $_[3] -eq $row[3] -and ($other -band 15) -eq ($type -band 15) -and ($other -band 16) -ne ($type -band 16)
    })
    if ($twins.Count -eq 0) { throw "HKLM search $($row[0]) ($($row[2]) / $($row[3])) must also search the other registry view." }
  }
  # Run the compiled resolvers with the InstallDir of a per-machine copy seen in one view only: it
  # must become CRAFT_MACHINE_DIR, and a per-user install must then be refused.
  $appSearch = @{}
  foreach ($row in (Read-Rows $database 'SELECT `Property`, `Signature_` FROM `AppSearch`' 2)) { $appSearch[$row[1]] = $row[0] }
  $setters = @(Read-Rows $database 'SELECT `Action`, `Target` FROM `CustomAction` WHERE `Source` = ''CRAFT_MACHINE_DIR''' 2 |
    Sort-Object { $execute[$_[0]] })
  $viewProperties = @($machineLocators | Where-Object { $_[2] -eq "Software\$($policy.name)" -and $_[3] -eq 'InstallDir' } |
    ForEach-Object { $appSearch[$_[0]] })
  if ($viewProperties.Count -ne 2) { throw 'Expected the InstallDir search in both registry views.' }
  foreach ($property in $viewProperties) {
    foreach ($setter in $setters) { Set-SessionProperty $session ($setter[1].Trim('[]')) '' }
    Set-SessionProperty $session 'CRAFT_MACHINE_DIR' ''
    Set-SessionProperty $session 'CRAFT_USER_DIR' ''
    Set-SessionProperty $session 'Installed' ''
    Set-SessionProperty $session 'ALLUSERS' ''
    Set-SessionProperty $session $property 'E:\Machine copy in one view\'
    foreach ($setter in $setters) {
      $condition = @(Read-Rows $database ('SELECT `Condition` FROM `InstallExecuteSequence` WHERE `Action` = ''{0}''' -f $setter[0]) 1)[0][0]
      if ($session.EvaluateCondition($condition) -eq 1 -and $session.DoAction($setter[0]) -ne 1) { throw "$($setter[0]) failed." }
    }
    if ((Get-SessionProperty $session 'CRAFT_MACHINE_DIR') -ne 'E:\Machine copy in one view\') {
      throw "A per-machine copy found through $property is not taken as the existing machine copy."
    }
    $refused = @($guards | Where-Object { $session.EvaluateCondition($_[0]) -eq 0 }).Count -gt 0
    if (-not $refused) { throw "A per-user install beside a per-machine copy found through $property is not refused." }
  }
  # Switching the first-page choice must reset BOTH properties, including a back-and-forth change.
  $events = @(Read-Rows $database 'SELECT `Event`, `Argument`, `Condition`, `Ordering` FROM `ControlEvent` WHERE `Dialog_` = ''CraftScopeDlg'' AND `Control_` = ''Next'' ORDER BY `Ordering`' 4)
  # Model the full-UI upgrade entry with an existing machine-wide copy. Run
  # the actual preference action, then the compiled Next events before discovery.
  Set-SessionProperty $session 'ALLUSERS' '2'
  Set-SessionProperty $session 'CRAFT_SCOPE' 'user'
  Set-SessionProperty $session 'CRAFT_USER_DIR' ''
  Set-SessionProperty $session 'CRAFT_MACHINE_DIR' 'E:\Existing machine app'
  $preference = @(Read-Rows $database 'SELECT `Condition` FROM `InstallUISequence` WHERE `Action` = ''CraftPreferMachineScope''' 1)[0]
  if ($session.EvaluateCondition($preference[0]) -ne 1 -or $session.DoAction('CraftPreferMachineScope') -ne 1 -or
      (Get-SessionProperty $session 'CRAFT_SCOPE') -ne 'machine') {
    throw 'Full-UI machine upgrades must initially select the existing machine context.'
  }
  foreach ($choice in @('machine','user','machine','user')) {
    Set-SessionProperty $session 'CRAFT_SCOPE' $choice
    foreach ($event in $events) {
      if ($event[0] -notmatch '^\[([^\]]+)\]$') { continue }
      $property = $Matches[1]
      if ($session.EvaluateCondition($event[2]) -eq 1) {
        Set-SessionProperty $session $property $(if ($event[1] -eq '{}') { '' } else { $event[1] })
      }
    }
    $allUsers = Get-SessionProperty $session 'ALLUSERS'
    $perUser = Get-SessionProperty $session 'MSIINSTALLPERUSER'
    if (($choice -eq 'machine' -and ($allUsers -ne '1' -or $perUser)) -or
        ($choice -eq 'user' -and ($allUsers -or $perUser -ne '1'))) {
      throw 'Scope selector does not reset the requested installation context.'
    }
  }
  # WordCraft's checks of saved installer options (CraftLoadOption*), of elevation-shield buttons on
  # its custom welcome and shortcut pages and of their "for all users" text are left out:
  # ReasonCraft has none of them (WixUI_Minimal and the scope page), so they would check nothing.
  $registry = @(Read-Rows $database 'SELECT `Root`, `Key`, `Name`, `Value` FROM `Registry`' 4)
  if (@($registry | Where-Object { $_[0] -in '1', '2' }).Count) { throw 'Registration must follow the selected context, not force HKCU or HKLM.' }
  if (@($registry | Where-Object { $_[0] -eq '0' -and $_[1] -match '^Software\\Classes(\\|$)' }).Count) {
    throw 'HKCR keys must be relative to Classes.'
  }
  foreach ($entry in $policy.registry) {
    $found = @($registry | Where-Object { $_[0] -eq $entry[0] -and $_[1] -eq $entry[1] -and $_[2] -eq $entry[2] -and $_[3] -eq $entry[3] })
    if ($found.Count -ne 1) { throw "Lost or changed registration: $($entry[1]) / $($entry[2])" }
  }
  $installDir = @($registry | Where-Object { $_[0] -eq '-1' -and $_[1] -eq "Software\$($policy.name)" -and $_[2] -eq 'InstallDir' -and $_[3] -eq '[INSTALLFOLDER]' })
  if ($installDir.Count -ne 1) { throw 'A context-sensitive installation-folder record is required.' }
  Write-Output "PASS $($policy.name): $cases scope cases, folder preservation, action sequencing, both registry views and registry contract. No installation performed."
} finally {
  foreach ($com in @($session, $database, $installer)) {
    if ($null -ne $com) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($com) }
  }
}
