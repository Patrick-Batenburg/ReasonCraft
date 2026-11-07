<#
.SYNOPSIS
  PE header checks for the release exe, dot-sourced by package.ps1 and test-packaging-script.ps1.
.DESCRIPTION
  Assert-ReleaseExe stops a build from being packaged when the exe is for another machine than
  its label, would open a console window, needs the Visual C++ runtime (the release links
  the C runtime statically, +crt-static, so the MSI and the portable zip need no redistributable),
  or would look for its imported DLLs outside System32 (DLL planting; build.rs sets the flag).
#>

# The machine field of the COFF header, per package architecture.
$script:PeMachines = @{ x64 = 0x8664; x86 = 0x14C; arm64 = 0xAA64 }

# Machine (COFF header) and Subsystem (optional header: 2 = Windows GUI, 3 = console).
function Get-PeHeader([string] $Path) {
  $bytes = [IO.File]::ReadAllBytes($Path)
  if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) { throw "$Path is not a PE file (no MZ header)" }
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  if ($pe -lt 0 -or $pe + 0x60 -gt $bytes.Length -or [BitConverter]::ToUInt32($bytes, $pe) -ne 0x4550) {
    throw "$Path is not a PE file (no PE signature)"
  }
  return @{ Machine = [int][BitConverter]::ToUInt16($bytes, $pe + 4); Subsystem = [int][BitConverter]::ToUInt16($bytes, $pe + 0x5C) }
}

# The headers needed to find a data directory: the bytes, the data directories' offset, the section
# table and the optional header's magic (0x10B = PE32, 0x20B = PE32+).
function Get-PeLayout([string] $Path) {
  $bytes = [IO.File]::ReadAllBytes($Path)
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  $optional = $pe + 24
  $magic = [BitConverter]::ToUInt16($bytes, $optional)
  $directories = switch ($magic) {
    0x10B { $optional + 96 }   # PE32
    0x20B { $optional + 112 }  # PE32+
    default { throw "$Path has an unknown optional header (magic 0x$('{0:X}' -f $magic))" }
  }
  return @{
    Path = $Path; Bytes = $bytes; Magic = $magic; Directories = $directories
    Sections = [BitConverter]::ToUInt16($bytes, $pe + 6)
    Table = $optional + [BitConverter]::ToUInt16($bytes, $pe + 20)
  }
}

# The file offset of an RVA, through the section table.
function ConvertTo-PeOffset($Layout, [uint32] $Rva) {
  $bytes = $Layout.Bytes
  for ($i = 0; $i -lt $Layout.Sections; $i++) {
    $s = $Layout.Table + 40 * $i
    $virtualSize = [BitConverter]::ToUInt32($bytes, $s + 8)
    $virtualAddress = [BitConverter]::ToUInt32($bytes, $s + 12)
    $rawSize = [BitConverter]::ToUInt32($bytes, $s + 16)
    $rawPointer = [BitConverter]::ToUInt32($bytes, $s + 20)
    $span = [Math]::Max($virtualSize, $rawSize)
    if ($Rva -ge $virtualAddress -and $Rva -lt $virtualAddress + $span) { return [int64]$rawPointer + ($Rva - $virtualAddress) }
  }
  throw "$($Layout.Path)`: RVA 0x$('{0:X}' -f $Rva) lies in no section"
}

# The DLL names in the import directory (data directory 1): each import descriptor's Name until
# the all-zero descriptor.
function Get-PeImports([string] $Path) {
  $layout = Get-PeLayout $Path
  $bytes = $layout.Bytes
  $importRva = [BitConverter]::ToUInt32($bytes, $layout.Directories + 8)
  if ($importRva -eq 0) { return @() }
  $names = [Collections.Generic.List[string]]::new()
  $descriptor = ConvertTo-PeOffset $layout $importRva
  while ($true) {
    $nameRva = [BitConverter]::ToUInt32($bytes, $descriptor + 12)
    $thunks = [BitConverter]::ToUInt32($bytes, $descriptor + 16)
    if ($nameRva -eq 0 -and $thunks -eq 0) { break }
    $start = ConvertTo-PeOffset $layout $nameRva
    $end = $start
    while ($bytes[$end] -ne 0) { $end++ }
    $names.Add([Text.Encoding]::ASCII.GetString($bytes, $start, $end - $start))
    $descriptor += 20
  }
  return $names.ToArray()
}

# DependentLoadFlags from the load configuration (data directory 10): where the loader looks for
# the DLLs the exe imports. 0 when the exe has no load configuration or one too old to hold it.
function Get-PeDependentLoadFlags([string] $Path) {
  $layout = Get-PeLayout $Path
  $rva = [BitConverter]::ToUInt32($layout.Bytes, $layout.Directories + 10 * 8)
  if ($rva -eq 0) { return 0 }
  $config = ConvertTo-PeOffset $layout $rva
  $field = if ($layout.Magic -eq 0x10B) { 0x36 } else { 0x4E }
  if ([BitConverter]::ToUInt32($layout.Bytes, $config) -lt $field + 2) { return 0 }
  return [int][BitConverter]::ToUInt16($layout.Bytes, $config + $field)
}

# Throws unless the exe is for $Arch, links the C runtime statically, is a GUI app and imports its
# DLLs from System32 only.
function Assert-ReleaseExe([string] $Path, [string] $Arch) {
  $name = Split-Path -Leaf $Path
  $wanted = $script:PeMachines[$Arch]
  if ($null -eq $wanted) { throw "unknown architecture '$Arch'" }
  $header = Get-PeHeader $Path
  if ($header.Machine -ne $wanted) {
    throw "$name is for machine 0x$('{0:X}' -f $header.Machine), expected 0x$('{0:X}' -f $wanted) ($Arch)"
  }
  $crt = @(Get-PeImports $Path | Where-Object { $_ -match '^(vcruntime|msvcp)\d+.*\.dll$|^api-ms-win-crt-' })
  if ($crt.Count) { throw "$name imports the C runtime ($($crt -join ', ')); build with +crt-static" }
  if ($header.Subsystem -ne 2) { throw "$name has PE subsystem $($header.Subsystem), expected 2 (Windows GUI)" }
  # 0x800 = LOAD_LIBRARY_SEARCH_SYSTEM32, from build.rs: no imported DLL from a planted copy.
  $flags = Get-PeDependentLoadFlags $Path
  if (($flags -band 0x800) -eq 0) { throw "$name has DependentLoadFlags 0x$('{0:X}' -f $flags), expected 0x800 (System32 only; see build.rs)" }
  Write-Output "ok $name`: $Arch, GUI subsystem, static C runtime, imported DLLs from System32 only"
}
