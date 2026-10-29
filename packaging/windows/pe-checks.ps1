<#
.SYNOPSIS
  PE header checks for the release exe, dot-sourced by package.ps1 and test-packaging-script.ps1.
.DESCRIPTION
  Assert-ReleaseExe stops a build from being packaged when the exe is for another machine than
  its label, would open a console window, or needs the Visual C++ runtime (the release links
  the C runtime statically, +crt-static, so the MSI and the portable zip need no redistributable).
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

# The DLL names in the import directory, read by walking the headers: the optional header's data
# directory 1 (imports), the section table to turn RVAs into file offsets, then each import
# descriptor's Name until the all-zero descriptor.
function Get-PeImports([string] $Path) {
  $bytes = [IO.File]::ReadAllBytes($Path)
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  $sections = [BitConverter]::ToUInt16($bytes, $pe + 6)
  $optionalSize = [BitConverter]::ToUInt16($bytes, $pe + 20)
  $optional = $pe + 24
  $magic = [BitConverter]::ToUInt16($bytes, $optional)
  $directories = switch ($magic) {
    0x10B { $optional + 96 }   # PE32
    0x20B { $optional + 112 }  # PE32+
    default { throw "$Path has an unknown optional header (magic 0x$('{0:X}' -f $magic))" }
  }
  $importRva = [BitConverter]::ToUInt32($bytes, $directories + 8)
  if ($importRva -eq 0) { return @() }
  $table = $optional + $optionalSize
  $toOffset = {
    param([uint32] $Rva)
    for ($i = 0; $i -lt $sections; $i++) {
      $s = $table + 40 * $i
      $virtualSize = [BitConverter]::ToUInt32($bytes, $s + 8)
      $virtualAddress = [BitConverter]::ToUInt32($bytes, $s + 12)
      $rawSize = [BitConverter]::ToUInt32($bytes, $s + 16)
      $rawPointer = [BitConverter]::ToUInt32($bytes, $s + 20)
      $span = [Math]::Max($virtualSize, $rawSize)
      if ($Rva -ge $virtualAddress -and $Rva -lt $virtualAddress + $span) { return [int64]$rawPointer + ($Rva - $virtualAddress) }
    }
    throw "$Path`: RVA 0x$('{0:X}' -f $Rva) lies in no section"
  }
  $names = [Collections.Generic.List[string]]::new()
  $descriptor = & $toOffset $importRva
  while ($true) {
    $nameRva = [BitConverter]::ToUInt32($bytes, $descriptor + 12)
    $thunks = [BitConverter]::ToUInt32($bytes, $descriptor + 16)
    if ($nameRva -eq 0 -and $thunks -eq 0) { break }
    $start = & $toOffset $nameRva
    $end = $start
    while ($bytes[$end] -ne 0) { $end++ }
    $names.Add([Text.Encoding]::ASCII.GetString($bytes, $start, $end - $start))
    $descriptor += 20
  }
  return $names.ToArray()
}

# Throws unless the exe is for $Arch, links the C runtime statically and is a GUI app.
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
  Write-Output "ok $name`: $Arch, GUI subsystem, static C runtime"
}
