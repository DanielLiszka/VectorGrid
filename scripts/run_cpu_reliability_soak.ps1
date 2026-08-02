<#
.SYNOPSIS
Runs the offline, single-process CPU reliability soak.

.DESCRIPTION
The short mode is a bounded maintainer/CI smoke. Operator mode is the
six-hours-or-longer authority lane and is intentionally opt-in. Both modes use
synthetic data only, clear provider/broker credentials, trap external HTTP
traffic, and redirect LOCALAPPDATA/APPDATA/TEMP into a marker-owned isolation
tree.

The wrapper serializes build and execution with the repository benchmark mutex,
uses a validated Cargo target directory, and records the exact binary and
harness-source identities in fresh evidence. It also safely recovers abandoned
marker-owned isolation trees left by an interrupted prior run.
#>
param(
  [ValidateSet("short", "operator")]
  [string]$Mode = "short",
  [double]$Hours = 6.0,
  [int]$Cycles = 0,
  [int]$Rows = 0,
  [int]$CancelEvery = 2,
  [int]$WarmupCycles = 1,
  [string]$OutDir = "output/qa/cpu-reliability-soak",
  [ValidateSet("release", "dev")]
  [string]$BuildProfile = "release",
  [string]$CargoTargetDir,
  [string]$PrebuiltBinaryPath,
  [switch]$SkipBuild,
  [switch]$PlanOnly,
  [switch]$RecoverStaleOnly,
  [switch]$SourceManifestOnly,
  [switch]$BuildInputProbeOnly,
  [switch]$ToolchainProbeOnly
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$isolationPrefix = "vectorgrid-soak-"
$isolationMarkerName = ".vectorgrid-cpu-reliability-soak-owner.json"
$cargoTargetPrefix = "cpu-reliability-soak-"
$cargoTargetMarkerName = ".vectorgrid-cpu-reliability-soak-target-owner.json"
$minimumOperatorSeconds = 6 * 60 * 60
$minimumOperatorMeasuredCycles = 30
$maximumOperatorSampleGapMs = 15 * 60 * 1000
$mibBytes = 1024.0 * 1024.0

function Resolve-RepoPath {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$RepoRoot
  )

  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $Path))
}

function Assert-PositiveFinite {
  param(
    [Parameter(Mandatory = $true)][double]$Value,
    [Parameter(Mandatory = $true)][string]$Name
  )

  if ([double]::IsNaN($Value) -or [double]::IsInfinity($Value) -or $Value -le 0) {
    throw "$Name must be finite and > 0"
  }
}

function Test-PathWithin {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Parent
  )

  $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
  $fullParent = [System.IO.Path]::GetFullPath($Parent).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
  if ([string]::Equals($fullPath, $fullParent, [StringComparison]::OrdinalIgnoreCase)) {
    return $true
  }
  $prefix = $fullParent + [System.IO.Path]::DirectorySeparatorChar
  return $fullPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)
}

function Assert-NoReparseAncestors {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Context
  )

  $cursor = [System.IO.Path]::GetFullPath($Path)
  while (-not [string]::IsNullOrWhiteSpace($cursor)) {
    if (Test-Path -LiteralPath $cursor) {
      $item = Get-Item -Force -LiteralPath $cursor
      if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "$Context must not use a reparse-point path or ancestor: $cursor"
      }
    }
    $parent = [System.IO.Directory]::GetParent($cursor)
    if ($null -eq $parent) {
      break
    }
    $next = $parent.FullName
    if ([string]::Equals($next, $cursor, [StringComparison]::OrdinalIgnoreCase)) {
      break
    }
    $cursor = $next
  }
}

function Assert-SafeDirectoryPath {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Context
  )

  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $root = [System.IO.Path]::GetPathRoot($fullPath)
  if (
    [string]::IsNullOrWhiteSpace($root) -or
    [string]::Equals(
      $fullPath.TrimEnd('\', '/'),
      $root.TrimEnd('\', '/'),
      [StringComparison]::OrdinalIgnoreCase
    )
  ) {
    throw "$Context must not be a filesystem root: $fullPath"
  }
  if (Test-Path -LiteralPath $fullPath -PathType Leaf) {
    throw "$Context must be a directory, not a file: $fullPath"
  }
  Assert-NoReparseAncestors -Path $fullPath -Context $Context
  return $fullPath
}

function Assert-SafeCargoTargetDirectory {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$ResolvedOutDir
  )

  $fullPath = Assert-SafeDirectoryPath -Path $Path -Context "Cargo target directory"
  if ([string]::Equals($fullPath, $RepoRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Cargo target directory must not be the repository root"
  }
  if (Test-PathWithin -Path $RepoRoot -Parent $fullPath) {
    throw "Cargo target directory must not contain the repository: $fullPath"
  }
  if (
    (Test-PathWithin -Path $fullPath -Parent $ResolvedOutDir) -or
    (Test-PathWithin -Path $ResolvedOutDir -Parent $fullPath)
  ) {
    throw "Cargo target directory and soak evidence directory must not overlap"
  }
  $gitDir = Join-Path $RepoRoot ".git"
  if (Test-PathWithin -Path $fullPath -Parent $gitDir) {
    throw "Cargo target directory must not be inside .git"
  }
  return $fullPath
}

function Get-Sha256Hex {
  param([Parameter(Mandatory = $true)][string]$Path)

  if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "Cannot hash missing file: $Path"
  }
  Assert-NoReparseAncestors -Path $Path -Context "SHA-256 input"
  $stream = [System.IO.File]::Open(
    $Path,
    [System.IO.FileMode]::Open,
    [System.IO.FileAccess]::Read,
    [System.IO.FileShare]::Read
  )
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha.Dispose()
    $stream.Dispose()
  }
}

function Resolve-ExecutableFile {
  param([Parameter(Mandatory = $true)][string]$Name)

  $command = Get-Command -Name $Name -CommandType Application -ErrorAction Stop |
    Select-Object -First 1
  if ($null -eq $command -or [string]::IsNullOrWhiteSpace([string]$command.Source)) {
    throw "Required executable was not found: $Name"
  }
  $path = [System.IO.Path]::GetFullPath([string]$command.Source)
  if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Resolved executable is not a file: $path"
  }
  Assert-NoReparseAncestors -Path $path -Context "$Name executable"
  return $path
}

function Get-StringSha256Hex {
  param([Parameter(Mandatory = $true)][string]$Text)

  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha.Dispose()
  }
}

function ConvertTo-SoakAsciiLowerInvariant {
  param([Parameter(Mandatory = $true)][string]$Text)

  $builder = [System.Text.StringBuilder]::new($Text.Length)
  foreach ($character in $Text.ToCharArray()) {
    $code = [int][char]$character
    if ($code -ge [int][char]'A' -and $code -le [int][char]'Z') {
      [void]$builder.Append([char]($code + 32))
    } else {
      [void]$builder.Append($character)
    }
  }
  return $builder.ToString()
}

function Test-SoakFullyQualifiedPath {
  param([Parameter(Mandatory = $true)][string]$Path)

  try {
    if (![System.IO.Path]::IsPathRooted($Path)) {
      return $false
    }
    $root = [System.IO.Path]::GetPathRoot($Path)
    [void][System.IO.Path]::GetFullPath($Path)
    if (
      [string]::IsNullOrWhiteSpace($root) -or
      [string]::Equals($root, "\", [StringComparison]::Ordinal) -or
      [string]::Equals($root, "/", [StringComparison]::Ordinal) -or
      $root.EndsWith(":", [StringComparison]::Ordinal)
    ) {
      return $false
    }
    return $true
  } catch {
    return $false
  }
}

function Get-SoakNormalizedFullyQualifiedPath {
  param([Parameter(Mandatory = $true)][string]$Path)

  if (!(Test-SoakFullyQualifiedPath -Path $Path)) {
    throw "Path is not fully qualified: $Path"
  }
  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $root = [System.IO.Path]::GetPathRoot($fullPath)
  if (
    [string]::Equals(
      $fullPath,
      $root,
      [StringComparison]::OrdinalIgnoreCase
    )
  ) {
    return $fullPath
  }
  return $fullPath.TrimEnd('\', '/')
}

function Test-SoakNativePathWithin {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Parent
  )

  $fullPath = Get-SoakNormalizedFullyQualifiedPath -Path $Path
  $fullParent = Get-SoakNormalizedFullyQualifiedPath -Path $Parent
  $pathKey = ConvertTo-SoakAsciiLowerInvariant -Text $fullPath
  $parentKey = ConvertTo-SoakAsciiLowerInvariant -Text $fullParent
  if ([string]::Equals($pathKey, $parentKey, [StringComparison]::Ordinal)) {
    return $true
  }
  $prefix = $parentKey.TrimEnd('\', '/') +
    [System.IO.Path]::DirectorySeparatorChar
  return $pathKey.StartsWith($prefix, [StringComparison]::Ordinal)
}

function Get-SoakNativeBuildOverrideEnvironmentNames {
  $fixedNames = @(
    "CC",
    "CXX",
    "CPP",
    "AR",
    "RANLIB",
    "LD",
    "LINKER",
    "ASM",
    "NASM",
    "WINDRES",
    "RC",
    "COMSPEC",
    "EMBED_RESOURCE_RC_PATH",
    "EMBED_RESOURCE_RC_ATTESTATION_NONCE",
    "VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION",
    "CFLAGS",
    "CXXFLAGS",
    "CPPFLAGS",
    "LDFLAGS",
    "ARFLAGS",
    "ASMFLAGS",
    "CL",
    "_CL_",
    "LINK",
    "_LINK_",
    "CRATE_CC_NO_DEFAULTS",
    "CC_ENABLE_DEBUG_OUTPUT",
    "LIBSQLITE3_SYS_USE_PKG_CONFIG",
    "LIBSQLITE3_FLAGS",
    "SQLITE_MAX_VARIABLE_NUMBER",
    "SQLITE_MAX_EXPR_DEPTH",
    "SQLITE_MAX_COLUMN",
    "ZSTD_SYS_USE_PKG_CONFIG",
    "RING_PREGENERATE_ASM",
    "PERL_EXECUTABLE",
    "ICU4X_DATA_DIR",
    "REMOVE_UNUSED_COMMANDS",
    "__TAURI_WORKSPACE__",
    "RUSTC_STAGE",
    "LZMA_API_STATIC",
    "STATIC_VCRUNTIME",
    "CXXSTDLIB",
    "RANLIBFLAGS",
    "PKG_CONFIG",
    "NO_VCPKG",
    "VSTEL_MSBuildProjectFullPath",
    "VSCMD_ARG_HOST_ARCH",
    "VSCMD_ARG_TGT_ARCH",
    "VSCMD_ARG_APP_PLAT",
    "VSCMD_ARG_VCVARS_VER",
    "VSCMD_ARG_WINSDK",
    "VSCMD_ARG_VCVARS_SPECTRE",
    "VSCMD_VER",
    "VSCMD_DEBUG",
    "VSCMD_SKIP_SENDTELEMETRY",
    "VSCMD_START_DIR",
    "CommandPromptType",
    "Platform",
    "PreferredToolArchitecture",
    "__VSCMD_PREINIT_PATH",
    "WindowsLibPath",
    "NETFXSDKDir",
    "INCLUDE",
    "EXTERNAL_INCLUDE",
    "IFCPATH",
    "LIB",
    "LIBPATH",
    "VSINSTALLDIR",
    "VCINSTALLDIR",
    "VCToolsInstallDir",
    "VCToolsVersion",
    "VCToolsRedistDir",
    "VCIDEInstallDir",
    "VS170COMNTOOLS",
    "VisualStudioVersion",
    "WindowsSdkDir",
    "WindowsSDKVersion",
    "WindowsSDK_ExecutablePath_x64",
    "WindowsSDK_ExecutablePath_x86",
    "WindowsSdkBinPath",
    "WindowsSDKLibVersion",
    "WindowsSdkVerBinPath",
    "UniversalCRTSdkDir",
    "UCRTVersion",
    "DevEnvDir",
    "ExtensionSdkDir",
    "FrameworkDir",
    "FrameworkDir64",
    "FrameworkVersion",
    "FrameworkVersion64",
    "Framework40Version",
    "__DOTNET_ADD_64BIT",
    "__DOTNET_PREFERRED_BITNESS"
  )
  $toolTokens =
    "(CC|CXX|CPP|AR|RANLIB|LD|LINKER|ASM|NASM|WINDRES|RC|CFLAGS|CXXFLAGS|CPPFLAGS|LDFLAGS|ARFLAGS|ASMFLAGS|CXXSTDLIB|RANLIBFLAGS)"
  $dynamicNames = @(
    Get-ChildItem Env: | Where-Object {
      $_.Name -match "^${toolTokens}_.+$" -or
      $_.Name -match "^.+_${toolTokens}$" -or
      $_.Name -match "^.+_NO_VCPKG$" -or
      $_.Name -match "^VSCMD_ARG_.+$" -or
      $_.Name -match "^(?i:_?VC_|VCLIB_|__DOTNET_|__VCVARS|VCVARS_).*$" -or
      $_.Name -match "^(?i:_{0,2}VSCMD|VCINSTALL|VCIDE|VCTOOLS|VISUALSTUDIO|VS.*COMNTOOLS|WINDOWSSDK|WINDOWS_SDK|UNIVERSALCRT|UCRT|FRAMEWORK(DIR|VERSION)|NETFXSDK|EXTENSIONSDK|DEVENV).*$" -or
      $_.Name -match "^(PKG_CONFIG|VCPKG|VCPKGRS|OPENSSL|SQLITE3|LIBSQLITE3|ZSTD|RING|PERL)_.+$"
    } | ForEach-Object { $_.Name }
  )
  $normalizedNames = [System.Collections.Generic.Dictionary[string, bool]]::new(
    [StringComparer]::OrdinalIgnoreCase
  )
  foreach ($rawName in @($fixedNames + $dynamicNames)) {
    $canonicalName = ([string]$rawName).ToUpperInvariant()
    if (![string]::IsNullOrWhiteSpace($canonicalName)) {
      $normalizedNames[$canonicalName] = $true
    }
  }
  $result = [string[]]@($normalizedNames.Keys)
  [Array]::Sort($result, [StringComparer]::Ordinal)
  return $result
}

function Get-SoakExistingPathListDirectories {
  param(
    [Parameter(Mandatory = $true)][string]$Value,
    [Parameter(Mandatory = $true)][string]$EnvironmentName
  )

  $directories = [System.Collections.Generic.Dictionary[string, string]]::new(
    [StringComparer]::Ordinal
  )
  foreach ($rawSegment in $Value.Split([char]';')) {
    $segment = ([string]$rawSegment).Trim()
    if (
      $segment.Length -ge 2 -and
      $segment.StartsWith('"', [StringComparison]::Ordinal) -and
      $segment.EndsWith('"', [StringComparison]::Ordinal)
    ) {
      $segment = $segment.Substring(1, $segment.Length - 2).Trim()
    }
    if ([string]::IsNullOrWhiteSpace($segment)) {
      continue
    }
    if ($segment -match "[`t`r`n]") {
      throw "$EnvironmentName contains a non-canonical path-list segment"
    }
    if (!(Test-SoakFullyQualifiedPath -Path $segment)) {
      throw "$EnvironmentName contains a non-fully-qualified path-list segment: $segment"
    }
    $path = Get-SoakNormalizedFullyQualifiedPath -Path $segment
    if (Test-Path -LiteralPath $path) {
      if (!(Test-Path -LiteralPath $path -PathType Container)) {
        throw "$EnvironmentName path-list segment is not a directory: $path"
      }
      Assert-NoReparseAncestors `
        -Path $path `
        -Context "$EnvironmentName path-list directory"
      $key = ConvertTo-SoakAsciiLowerInvariant -Text $path
      if (!$directories.ContainsKey($key)) {
        $directories[$key] = $path
      }
    }
  }
  $keys = [string[]]@($directories.Keys)
  [Array]::Sort($keys, [StringComparer]::Ordinal)
  return @($keys | ForEach-Object { $directories[$_] })
}

function Resolve-SoakPathExecutable {
  param(
    [Parameter(Mandatory = $true)][string]$PathValue,
    [Parameter(Mandatory = $true)][string]$ExecutableName
  )

  foreach ($rawSegment in $PathValue.Split([char]';')) {
    $segment = ([string]$rawSegment).Trim()
    if (
      $segment.Length -ge 2 -and
      $segment.StartsWith('"', [StringComparison]::Ordinal) -and
      $segment.EndsWith('"', [StringComparison]::Ordinal)
    ) {
      $segment = $segment.Substring(1, $segment.Length - 2).Trim()
    }
    if ([string]::IsNullOrWhiteSpace($segment)) {
      continue
    }
    if (
      $segment -match "[`t`r`n]" -or
      !(Test-SoakFullyQualifiedPath -Path $segment)
    ) {
      throw "Pinned PATH contains a non-canonical segment: $segment"
    }
    $directory = Get-SoakNormalizedFullyQualifiedPath -Path $segment
    if (!(Test-Path -LiteralPath $directory -PathType Container)) {
      continue
    }
    $candidate = [System.IO.Path]::GetFullPath(
      (Join-Path $directory $ExecutableName)
    )
    if (Test-Path -LiteralPath $candidate) {
      if (!(Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "PATH-resolved native tool is not an ordinary file: $candidate"
      }
      Assert-NoReparseAncestors `
        -Path $candidate `
        -Context "PATH-resolved native tool $ExecutableName"
      return $candidate
    }
  }
  throw "Pinned PATH does not resolve required native tool: $ExecutableName"
}

function Get-SoakNativeInputManifest {
  param(
    [Parameter(Mandatory = $true)][string]$VisualStudioInstallation,
    [Parameter(Mandatory = $true)][string]$VcToolsVersion,
    [Parameter(Mandatory = $true)][string]$WindowsSdkRoot
  )

  $vcToolsRoot = Join-Path $VisualStudioInstallation "VC\Tools\MSVC\$VcToolsVersion"
  $vsCommon7ToolsRoot = Join-Path $VisualStudioInstallation "Common7\Tools"
  $vsDevCmdPath = Join-Path $vsCommon7ToolsRoot "VsDevCmd.bat"
  $vsDevCmdTree = Join-Path $vsCommon7ToolsRoot "vsdevcmd"
  if (
    !(Test-Path -LiteralPath $vsDevCmdPath -PathType Leaf) -or
    !(Test-Path -LiteralPath $vsDevCmdTree -PathType Container)
  ) {
    throw "Selected Visual Studio Common7 tools omit VsDevCmd.bat or its vsdevcmd tree"
  }
  Assert-NoReparseAncestors `
    -Path $vsDevCmdPath `
    -Context "Visual Studio VsDevCmd.bat"
  Assert-NoReparseAncestors `
    -Path $vsDevCmdTree `
    -Context "Visual Studio vsdevcmd tree"
  $rootSpecs = @(
    [ordered]@{ label = "vc_tools"; path = $vcToolsRoot },
    [ordered]@{
      label = "vc_auxiliary_build"
      path = (Join-Path $VisualStudioInstallation "VC\Auxiliary\Build")
    },
    [ordered]@{
      label = "vs_common7_tools"
      path = $vsCommon7ToolsRoot
    },
    [ordered]@{ label = "windows_sdk_bin"; path = (Join-Path $WindowsSdkRoot "bin") },
    [ordered]@{ label = "windows_sdk_include"; path = (Join-Path $WindowsSdkRoot "Include") },
    [ordered]@{ label = "windows_sdk_lib"; path = (Join-Path $WindowsSdkRoot "Lib") }
  )
  foreach ($optionalSpec in @(
      [ordered]@{
        label = "windows_sdk_references"
        path = (Join-Path $WindowsSdkRoot "References")
      },
      [ordered]@{
        label = "windows_sdk_union_metadata"
        path = (Join-Path $WindowsSdkRoot "UnionMetadata")
      },
      [ordered]@{
        label = "netfx_sdk"
        path = [Environment]::GetEnvironmentVariable("NETFXSDKDir", "Process")
      }
    )) {
    if (
      [string]$optionalSpec.label -eq "netfx_sdk" -and
      ![string]::IsNullOrWhiteSpace([string]$optionalSpec.path) -and
      !(Test-Path -LiteralPath ([string]$optionalSpec.path) -PathType Container)
    ) {
      throw "Pinned NETFXSDKDir does not identify a native input directory: $($optionalSpec.path)"
    }
    if (
      ![string]::IsNullOrWhiteSpace([string]$optionalSpec.path) -and
      (Test-Path -LiteralPath ([string]$optionalSpec.path) -PathType Container)
    ) {
      $rootSpecs += $optionalSpec
    }
  }

  $pathEnvironmentNames = [string[]]@(
    "EXTERNAL_INCLUDE",
    "IFCPATH",
    "INCLUDE",
    "LIB",
    "LIBPATH"
  )
  $pathEnvironmentRows = @(
    foreach ($name in $pathEnvironmentNames) {
      $value = [Environment]::GetEnvironmentVariable($name, "Process")
      if ([string]::IsNullOrWhiteSpace([string]$value)) {
        if ($name -eq "IFCPATH") {
          continue
        }
        throw "Pinned native path environment is missing $name"
      }
      if ([string]$value -match "[`t`r`n]") {
        throw "Pinned native path environment contains a control character: $name"
      }
      [ordered]@{
        name = $name
        value = [string]$value
        value_sha256 = Get-StringSha256Hex -Text ([string]$value)
      }
    }
  )
  $uncoveredEnvironmentPaths =
    [System.Collections.Generic.Dictionary[string, string]]::new(
      [StringComparer]::Ordinal
    )
  foreach ($row in $pathEnvironmentRows) {
    foreach (
      $directory in @(
        Get-SoakExistingPathListDirectories `
          -Value ([string]$row.value) `
          -EnvironmentName ([string]$row.name)
      )
    ) {
      $covered = $false
      foreach ($spec in $rootSpecs) {
        if (
          Test-SoakNativePathWithin `
            -Path $directory `
            -Parent ([System.IO.Path]::GetFullPath([string]$spec.path))
        ) {
          $covered = $true
          break
        }
      }
      if (!$covered) {
        $key = ConvertTo-SoakAsciiLowerInvariant -Text $directory
        if (!$uncoveredEnvironmentPaths.ContainsKey($key)) {
          $uncoveredEnvironmentPaths[$key] = $directory
        }
      }
    }
  }
  $uncoveredKeys = [string[]]@($uncoveredEnvironmentPaths.Keys)
  [Array]::Sort($uncoveredKeys, [StringComparer]::Ordinal)
  for ($index = 0; $index -lt $uncoveredKeys.Count; $index++) {
    $rootSpecs += [ordered]@{
      label = "environment_path_{0:D4}" -f $index
      path = [string]$uncoveredEnvironmentPaths[$uncoveredKeys[$index]]
    }
  }

  $rootMap = [System.Collections.Generic.Dictionary[string, object]]::new(
    [StringComparer]::Ordinal
  )
  $fileMap = [System.Collections.Generic.Dictionary[string, object]]::new(
    [StringComparer]::Ordinal
  )
  foreach ($spec in $rootSpecs) {
    $label = [string]$spec.label
    $root = [System.IO.Path]::GetFullPath([string]$spec.path)
    if (!(Test-Path -LiteralPath $root -PathType Container)) {
      throw "Required native input root is missing: $root"
    }
    Assert-NoReparseAncestors -Path $root -Context "Native input root $label"
    if ($label -match "[`t`r`n]" -or $root -match "[`t`r`n]") {
      throw "Native input root provenance contains a non-canonical control character"
    }
    $rootMap[$label] = [ordered]@{ label = $label; path = $root }
    $rootPrefix = $root.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    foreach ($item in Get-ChildItem -Force -LiteralPath $root -Recurse) {
      if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Native input manifest refuses reparse-point input: $($item.FullName)"
      }
      if (!$item.PSIsContainer) {
        $absolute = [System.IO.Path]::GetFullPath($item.FullName)
        if (!$absolute.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
          throw "Native input escaped its attested root: $absolute"
        }
        $relative = $absolute.Substring($rootPrefix.Length).Replace('\', '/')
        if ($relative -match "[`t`r`n]") {
          throw "Native input relative path contains a non-canonical control character"
        }
        $key = "$label`t$relative"
        if ($fileMap.ContainsKey($key)) {
          throw "Duplicate native input manifest row: $key"
        }
        $fileMap[$key] = [ordered]@{
          root = $label
          path = $relative
          bytes = [long]$item.Length
          sha256 = Get-Sha256Hex -Path $absolute
        }
      }
    }
  }
  $rootKeys = [string[]]@($rootMap.Keys)
  [Array]::Sort($rootKeys, [StringComparer]::Ordinal)
  $rootRows = @($rootKeys | ForEach-Object { $rootMap[$_] })
  $fileKeys = [string[]]@($fileMap.Keys)
  [Array]::Sort($fileKeys, [StringComparer]::Ordinal)
  $fileRows = @($fileKeys | ForEach-Object { $fileMap[$_] })
  $canonicalRows = @(
    $pathEnvironmentRows | ForEach-Object {
      "path_environment`t{0}`t{1}" -f $_.name, $_.value_sha256
    }
    $rootRows | ForEach-Object {
      "root`t{0}`t{1}" -f $_.label, $_.path
    }
    $fileRows | ForEach-Object {
      "file`t{0}`t{1}`t{2}`t{3}" -f $_.root, $_.path, $_.bytes, $_.sha256
    }
  )
  $canonical = $canonicalRows -join "`n"
  $totalBytes = [long]0
  foreach ($row in $fileRows) {
    $totalBytes += [long]$row.bytes
  }
  return [pscustomobject][ordered]@{
    schema_version = 3
    kind = "vectorgrid_cpu_reliability_soak_native_inputs"
    aggregate_algorithm =
      "sha256(path_environment_tab_name_tab_value_sha256_then_root_tab_label_tab_absolute_path_then_file_tab_root_label_tab_relative_path_tab_bytes_tab_sha256_lf)"
    aggregate_sha256 = Get-StringSha256Hex -Text $canonical
    path_environment_count = $pathEnvironmentRows.Count
    path_environment = [object[]]$pathEnvironmentRows
    root_count = $rootRows.Count
    file_count = $fileRows.Count
    total_bytes = $totalBytes
    roots = [object[]]$rootRows
    files = [object[]]$fileRows
  }
}

function Get-SoakRegistryPackageTreeManifest {
  param(
    [Parameter(Mandatory = $true)][string]$Label,
    [Parameter(Mandatory = $true)][string]$Root
  )

  $rootPath = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
  if (!(Test-Path -LiteralPath $rootPath -PathType Container)) {
    throw "Cargo registry package tree is missing: $rootPath"
  }
  Assert-NoReparseAncestors -Path $rootPath -Context "Cargo registry package tree $Label"
  if ($Label -match "[`t`r`n]" -or $rootPath -match "[`t`r`n]") {
    throw "Cargo registry package tree provenance contains a control character"
  }
  $rootPrefix = $rootPath + [System.IO.Path]::DirectorySeparatorChar
  $fileMap = [System.Collections.Generic.Dictionary[string, object]]::new(
    [StringComparer]::Ordinal
  )
  foreach ($item in Get-ChildItem -Force -LiteralPath $rootPath -Recurse) {
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Cargo registry package tree refuses reparse input: $($item.FullName)"
    }
    if (!$item.PSIsContainer) {
      $absolute = [System.IO.Path]::GetFullPath($item.FullName)
      if (!$absolute.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Cargo registry package input escaped its attested root: $absolute"
      }
      $relative = $absolute.Substring($rootPrefix.Length).Replace('\', '/')
      if ([string]::IsNullOrWhiteSpace($relative) -or $relative -match "[`t`r`n]") {
        throw "Cargo registry package relative path is not canonical: $relative"
      }
      if ($fileMap.ContainsKey($relative)) {
        throw "Duplicate Cargo registry package row: $relative"
      }
      $fileMap[$relative] = [ordered]@{
        path = $relative
        bytes = [long]$item.Length
        sha256 = Get-Sha256Hex -Path $absolute
      }
    }
  }
  $fileKeys = [string[]]@($fileMap.Keys)
  [Array]::Sort($fileKeys, [StringComparer]::Ordinal)
  $files = @($fileKeys | ForEach-Object { $fileMap[$_] })
  if ($files.Count -eq 0 -or !$fileMap.ContainsKey("Cargo.toml")) {
    throw "Cargo registry package tree is empty or omits Cargo.toml: $rootPath"
  }
  $totalBytes = [long]0
  foreach ($row in $files) {
    $totalBytes += [long]$row.bytes
  }
  $canonical = @(
    "root`t$Label`t$rootPath"
    $files | ForEach-Object {
      "file`t{0}`t{1}`t{2}" -f $_.path, $_.bytes, $_.sha256
    }
  ) -join "`n"
  return [pscustomobject][ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_registry_package_tree"
    aggregate_algorithm =
      "sha256(root_tab_label_tab_absolute_path_then_file_tab_relative_path_tab_bytes_tab_sha256_lf)"
    label = $Label
    root_path = $rootPath
    file_count = $files.Count
    total_bytes = $totalBytes
    files = [object[]]$files
    aggregate_sha256 = Get-StringSha256Hex -Text $canonical
  }
}

function Get-SoakEmbedResourceCargoResolution {
  param(
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$CargoHome,
    [Parameter(Mandatory = $true)][string]$CargoExecutablePath,
    [Parameter(Mandatory = $true)][string]$HostTargetTriple
  )

  $repoRootPath = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd('\', '/')
  $cargoHomePath = [System.IO.Path]::GetFullPath($CargoHome).TrimEnd('\', '/')
  $cargoPath = [System.IO.Path]::GetFullPath($CargoExecutablePath)
  $rootManifestRelativePath = "src-tauri/Cargo.toml"
  $rootManifestPath = [System.IO.Path]::GetFullPath(
    (Join-Path $repoRootPath $rootManifestRelativePath)
  )
  $embedManifestRelativePath = "vendor/embed-resource-3.0.6/Cargo.toml"
  $embedManifestPath = [System.IO.Path]::GetFullPath(
    (Join-Path $repoRootPath $embedManifestRelativePath)
  )
  foreach ($pathSpec in @(
      [ordered]@{ path = $repoRootPath; kind = "container"; label = "Cargo metadata repository" },
      [ordered]@{ path = $cargoHomePath; kind = "container"; label = "Cargo home" },
      [ordered]@{ path = $cargoPath; kind = "leaf"; label = "Pinned Cargo executable" },
      [ordered]@{ path = $rootManifestPath; kind = "leaf"; label = "Root package manifest" },
      [ordered]@{ path = $embedManifestPath; kind = "leaf"; label = "Vendored embed-resource manifest" }
    )) {
    if (
      [string]$pathSpec.kind -eq "container" -and
      !(Test-Path -LiteralPath ([string]$pathSpec.path) -PathType Container)
    ) {
      throw "$($pathSpec.label) is missing: $($pathSpec.path)"
    }
    if (
      [string]$pathSpec.kind -eq "leaf" -and
      !(Test-Path -LiteralPath ([string]$pathSpec.path) -PathType Leaf)
    ) {
      throw "$($pathSpec.label) is missing: $($pathSpec.path)"
    }
    Assert-NoReparseAncestors `
      -Path ([string]$pathSpec.path) `
      -Context ([string]$pathSpec.label)
  }
  if ($HostTargetTriple -ne "x86_64-pc-windows-msvc") {
    throw "Cargo build-chain attestation requires x86_64-pc-windows-msvc"
  }

  $metadataArguments = [string[]]@(
    "metadata",
    "--locked",
    "--offline",
    "--format-version",
    "1",
    "--filter-platform",
    $HostTargetTriple,
    "--no-default-features",
    "--features",
    "ta_desktop_demo_app/maintainer-tools"
  )
  Push-Location $repoRootPath
  try {
    $metadataLines = @(& $cargoPath @metadataArguments)
    $metadataExitCode = $LASTEXITCODE
  } finally {
    Pop-Location
  }
  if ($metadataExitCode -ne 0 -or $metadataLines.Count -eq 0) {
    throw "Pinned locked/offline Cargo metadata failed (exit code $metadataExitCode)"
  }
  $metadataText = $metadataLines -join "`n"
  try {
    $metadata = $metadataText | ConvertFrom-Json
  } catch {
    throw "Pinned Cargo metadata returned invalid JSON: $($_.Exception.Message)"
  }
  if ([int]$metadata.version -ne 1 -or $null -eq $metadata.resolve) {
    throw "Pinned Cargo metadata format or resolved graph is invalid"
  }
  $metadataWorkspaceRoot = [System.IO.Path]::GetFullPath(
    [string]$metadata.workspace_root
  ).TrimEnd('\', '/')
  Assert-NoReparseAncestors `
    -Path $metadataWorkspaceRoot `
    -Context "Cargo metadata workspace root"
  if (
    ![string]::Equals(
      $metadataWorkspaceRoot,
      $repoRootPath,
      [StringComparison]::OrdinalIgnoreCase
    )
  ) {
    throw "Cargo metadata workspace root differs from the pinned repository"
  }

  $rootPackages = @($metadata.packages | Where-Object {
      [string]$_.name -eq "ta_desktop_demo_app"
    })
  if ($rootPackages.Count -ne 1 -or [string]$rootPackages[0].version -ne "0.1.0") {
    throw "Cargo metadata must resolve exactly one ta_desktop_demo_app 0.1.0 package"
  }
  $rootPackage = $rootPackages[0]
  $rootPackageId = [string]$rootPackage.id
  $resolvedRootManifestPath = [System.IO.Path]::GetFullPath(
    [string]$rootPackage.manifest_path
  )
  if (
    $null -ne $rootPackage.source -or
    !$rootPackageId.StartsWith("path+file:///", [StringComparison]::Ordinal) -or
    !$rootPackageId.EndsWith(
      "#ta_desktop_demo_app@0.1.0",
      [StringComparison]::Ordinal
    ) -or
    ![string]::Equals(
      $resolvedRootManifestPath,
      $rootManifestPath,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    @($metadata.workspace_members | Where-Object {
        [string]::Equals([string]$_, $rootPackageId, [StringComparison]::Ordinal)
      }).Count -ne 1
  ) {
    throw "Cargo metadata did not resolve the canonical root workspace package"
  }
  Assert-NoReparseAncestors `
    -Path $resolvedRootManifestPath `
    -Context "Cargo-resolved root package manifest"

  $tauriBuildPackages = @($metadata.packages | Where-Object {
      [string]$_.name -eq "tauri-build"
    })
  if ($tauriBuildPackages.Count -ne 1 -or [string]$tauriBuildPackages[0].version -ne "2.5.6") {
    throw "Cargo metadata must resolve exactly one tauri-build 2.5.6 package"
  }
  $tauriBuildPackage = $tauriBuildPackages[0]
  $tauriBuildPackageId = [string]$tauriBuildPackage.id
  $tauriBuildSource = [string]$tauriBuildPackage.source
  if (
    $tauriBuildPackageId -ne
      "registry+https://github.com/rust-lang/crates.io-index#tauri-build@2.5.6" -or
    $tauriBuildSource -ne
      "registry+https://github.com/rust-lang/crates.io-index"
  ) {
    throw "Cargo metadata resolved an unexpected tauri-build package"
  }
  $tauriBuildManifestPath = [System.IO.Path]::GetFullPath(
    [string]$tauriBuildPackage.manifest_path
  )
  Assert-NoReparseAncestors `
    -Path $tauriBuildManifestPath `
    -Context "Cargo-resolved tauri-build manifest"
  $tauriBuildTree = Get-SoakRegistryPackageTreeManifest `
    -Label "tauri-build@2.5.6" `
    -Root (Split-Path -Parent $tauriBuildManifestPath)

  $tauriWinresPackages = @($metadata.packages | Where-Object {
      [string]$_.name -eq "tauri-winres"
    })
  if (
    $tauriWinresPackages.Count -ne 1 -or
    [string]$tauriWinresPackages[0].version -ne "0.3.5"
  ) {
    throw "Cargo metadata must resolve exactly one tauri-winres 0.3.5 package"
  }
  $tauriWinresPackage = $tauriWinresPackages[0]
  $tauriWinresPackageId = [string]$tauriWinresPackage.id
  $tauriWinresSource = [string]$tauriWinresPackage.source
  if (
    $tauriWinresPackageId -ne
      "registry+https://github.com/rust-lang/crates.io-index#tauri-winres@0.3.5" -or
    $tauriWinresSource -ne
      "registry+https://github.com/rust-lang/crates.io-index"
  ) {
    throw "Cargo metadata resolved an unexpected tauri-winres package"
  }
  $tauriWinresManifestPath = [System.IO.Path]::GetFullPath(
    [string]$tauriWinresPackage.manifest_path
  )
  Assert-NoReparseAncestors `
    -Path $tauriWinresManifestPath `
    -Context "Cargo-resolved tauri-winres manifest"
  $tauriWinresTree = Get-SoakRegistryPackageTreeManifest `
    -Label "tauri-winres@0.3.5" `
    -Root (Split-Path -Parent $tauriWinresManifestPath)
  $registrySourceRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $cargoHomePath "registry\src")
  )
  foreach ($treeSpec in @(
      [ordered]@{
        tree = $tauriBuildTree
        directory_name = "tauri-build-2.5.6"
      },
      [ordered]@{
        tree = $tauriWinresTree
        directory_name = "tauri-winres-0.3.5"
      }
    )) {
    if (
      !(Test-PathWithin `
          -Path ([string]$treeSpec.tree.root_path) `
          -Parent $registrySourceRoot) -or
      (Split-Path -Leaf ([string]$treeSpec.tree.root_path)) -ne
        [string]$treeSpec.directory_name
    ) {
      throw "Cargo registry package tree escaped the pinned CARGO_HOME registry source"
    }
  }

  $embedPackages = @($metadata.packages | Where-Object {
      [string]$_.name -eq "embed-resource"
    })
  if ($embedPackages.Count -ne 1 -or [string]$embedPackages[0].version -ne "3.0.6") {
    throw "Cargo metadata must resolve exactly one embed-resource 3.0.6 package"
  }
  $embedPackage = $embedPackages[0]
  $embedPackageId = [string]$embedPackage.id
  $resolvedEmbedManifestPath = [System.IO.Path]::GetFullPath(
    [string]$embedPackage.manifest_path
  )
  if (
    $null -ne $embedPackage.source -or
    !$embedPackageId.StartsWith("path+file:///", [StringComparison]::Ordinal) -or
    !$embedPackageId.EndsWith(
      "#embed-resource@3.0.6",
      [StringComparison]::Ordinal
    ) -or
    ![string]::Equals(
      $resolvedEmbedManifestPath,
      $embedManifestPath,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    @($metadata.workspace_members | Where-Object {
        [string]::Equals([string]$_, $embedPackageId, [StringComparison]::Ordinal)
      }).Count -ne 1
  ) {
    throw "Cargo metadata did not resolve the canonical vendored embed-resource package"
  }
  Assert-NoReparseAncestors `
    -Path $resolvedEmbedManifestPath `
    -Context "Cargo-resolved embed-resource manifest"

  $rootNodes = @($metadata.resolve.nodes | Where-Object {
      [string]$_.id -eq $rootPackageId
    })
  $tauriBuildNodes = @($metadata.resolve.nodes | Where-Object {
      [string]$_.id -eq $tauriBuildPackageId
    })
  $tauriWinresNodes = @($metadata.resolve.nodes | Where-Object {
      [string]$_.id -eq $tauriWinresPackageId
    })
  if (
    $rootNodes.Count -ne 1 -or
    $tauriBuildNodes.Count -ne 1 -or
    $tauriWinresNodes.Count -ne 1 -or
    @($rootNodes[0].features).Count -ne 1 -or
    [string]$rootNodes[0].features[0] -ne "maintainer-tools"
  ) {
    throw "Cargo metadata does not contain the exact maintainer-tools root resolve node"
  }

  $rootDeclaredEdges = @($rootPackage.dependencies | Where-Object {
      [string]$_.name -eq "tauri-build"
    })
  $tauriBuildDeclaredEdges = @($tauriBuildPackage.dependencies | Where-Object {
      [string]$_.name -eq "tauri-winres"
    })
  $tauriWinresDeclaredEdges = @($tauriWinresPackage.dependencies | Where-Object {
      [string]$_.name -eq "embed-resource"
    })
  if (
    $rootDeclaredEdges.Count -ne 1 -or
    [string]$rootDeclaredEdges[0].req -ne "^2" -or
    [string]$rootDeclaredEdges[0].kind -ne "build" -or
    $null -ne $rootDeclaredEdges[0].target -or
    $null -ne $rootDeclaredEdges[0].rename -or
    [bool]$rootDeclaredEdges[0].optional -or
    ![bool]$rootDeclaredEdges[0].uses_default_features -or
    @($rootDeclaredEdges[0].features).Count -ne 0 -or
    $null -ne $rootDeclaredEdges[0].registry
  ) {
    throw "Root package does not declare the exact tauri-build build dependency"
  }
  if (
    $tauriBuildDeclaredEdges.Count -ne 1 -or
    [string]$tauriBuildDeclaredEdges[0].req -ne "^0.3" -or
    $null -ne $tauriBuildDeclaredEdges[0].kind -or
    $null -ne $tauriBuildDeclaredEdges[0].target -or
    $null -ne $tauriBuildDeclaredEdges[0].rename -or
    [bool]$tauriBuildDeclaredEdges[0].optional -or
    ![bool]$tauriBuildDeclaredEdges[0].uses_default_features -or
    @($tauriBuildDeclaredEdges[0].features).Count -ne 0 -or
    $null -ne $tauriBuildDeclaredEdges[0].registry
  ) {
    throw "tauri-build does not declare the exact normal tauri-winres dependency"
  }
  if (
    $tauriWinresDeclaredEdges.Count -ne 1 -or
    [string]$tauriWinresDeclaredEdges[0].req -ne "^3" -or
    $null -ne $tauriWinresDeclaredEdges[0].kind -or
    $null -ne $tauriWinresDeclaredEdges[0].target -or
    $null -ne $tauriWinresDeclaredEdges[0].rename -or
    [bool]$tauriWinresDeclaredEdges[0].optional -or
    ![bool]$tauriWinresDeclaredEdges[0].uses_default_features -or
    @($tauriWinresDeclaredEdges[0].features).Count -ne 0 -or
    $null -ne $tauriWinresDeclaredEdges[0].registry
  ) {
    throw "tauri-winres does not declare the exact normal embed-resource dependency"
  }

  $rootResolvedEdges = @($rootNodes[0].deps | Where-Object {
      [string]$_.name -eq "tauri_build"
    })
  $tauriBuildResolvedEdges = @($tauriBuildNodes[0].deps | Where-Object {
      [string]$_.name -eq "tauri_winres"
    })
  $tauriWinresResolvedEdges = @($tauriWinresNodes[0].deps | Where-Object {
      [string]$_.name -eq "embed_resource"
    })
  if (
    $rootResolvedEdges.Count -ne 1 -or
    [string]$rootResolvedEdges[0].pkg -ne $tauriBuildPackageId -or
    @($rootResolvedEdges[0].dep_kinds).Count -ne 1 -or
    [string]$rootResolvedEdges[0].dep_kinds[0].kind -ne "build" -or
    $null -ne $rootResolvedEdges[0].dep_kinds[0].target
  ) {
    throw "Root package did not resolve its build edge to exact tauri-build"
  }
  if (
    $tauriBuildResolvedEdges.Count -ne 1 -or
    [string]$tauriBuildResolvedEdges[0].pkg -ne $tauriWinresPackageId -or
    @($tauriBuildResolvedEdges[0].dep_kinds).Count -ne 1 -or
    $null -ne $tauriBuildResolvedEdges[0].dep_kinds[0].kind -or
    $null -ne $tauriBuildResolvedEdges[0].dep_kinds[0].target
  ) {
    throw "tauri-build did not resolve its normal edge to exact tauri-winres"
  }
  if (
    $tauriWinresResolvedEdges.Count -ne 1 -or
    [string]$tauriWinresResolvedEdges[0].pkg -ne $embedPackageId -or
    @($tauriWinresResolvedEdges[0].dep_kinds).Count -ne 1 -or
    $null -ne $tauriWinresResolvedEdges[0].dep_kinds[0].kind -or
    $null -ne $tauriWinresResolvedEdges[0].dep_kinds[0].target
  ) {
    throw "tauri-winres did not resolve its normal edge to vendored embed-resource"
  }

  foreach ($canonicalText in @(
      $repoRootPath,
      $cargoHomePath,
      $cargoPath,
      $rootPackageId,
      $resolvedRootManifestPath,
      $tauriBuildPackageId,
      $tauriBuildSource,
      $tauriBuildManifestPath,
      $tauriWinresPackageId,
      $tauriWinresSource,
      $tauriWinresManifestPath,
      $embedPackageId,
      $resolvedEmbedManifestPath
    )) {
    if ([string]$canonicalText -match "[`t`r`n]") {
      throw "Cargo build-chain provenance contains a non-canonical control character"
    }
  }

  $cargoItem = Get-Item -Force -LiteralPath $cargoPath
  $rootManifestItem = Get-Item -Force -LiteralPath $rootManifestPath
  $embedManifestItem = Get-Item -Force -LiteralPath $embedManifestPath
  $cargoSha256 = Get-Sha256Hex -Path $cargoPath
  $rootManifestSha256 = Get-Sha256Hex -Path $rootManifestPath
  $embedManifestSha256 = Get-Sha256Hex -Path $embedManifestPath
  $metadataSha256 = Get-StringSha256Hex -Text $metadataText
  $canonical = @(
    "metadata_format_version`t1",
    "metadata_output_sha256`t$metadataSha256",
    "metadata_arguments`t$($metadataArguments -join "`t")",
    "workspace_root`t$repoRootPath",
    "cargo_home`t$cargoHomePath",
    "workspace_member_count`t$(@($metadata.workspace_members).Count)",
    "root_resolved_features`tmaintainer-tools",
    "cargo`t$cargoPath`t$([long]$cargoItem.Length)`t$cargoSha256",
    "root_package_match_count`t1",
    "root_package`t$rootPackageId`t0.1.0`tnull`t$resolvedRootManifestPath`t$rootManifestRelativePath`t$([long]$rootManifestItem.Length)`t$rootManifestSha256",
    "tauri_build_match_count`t1",
    "tauri_build`t$tauriBuildPackageId`t2.5.6`t$tauriBuildSource`t$tauriBuildManifestPath`t$($tauriBuildTree.aggregate_sha256)",
    "tauri_winres_match_count`t1",
    "tauri_winres`t$tauriWinresPackageId`t0.3.5`t$tauriWinresSource`t$tauriWinresManifestPath`t$($tauriWinresTree.aggregate_sha256)",
    "embed_resource_match_count`t1",
    "embed_resource`t$embedPackageId`t3.0.6`tnull`t$resolvedEmbedManifestPath`t$embedManifestRelativePath`t$([long]$embedManifestItem.Length)`t$embedManifestSha256",
    "root_to_tauri_build_declared_match_count`t1",
    "root_to_tauri_build_resolved_match_count`t1",
    "root_to_tauri_build_edge`t$rootPackageId`ttauri-build`ttauri_build`t$tauriBuildPackageId`tbuild`tnull`t^2",
    "tauri_build_to_winres_declared_match_count`t1",
    "tauri_build_to_winres_resolved_match_count`t1",
    "tauri_build_to_winres_edge`t$tauriBuildPackageId`ttauri-winres`ttauri_winres`t$tauriWinresPackageId`tnormal`tnull`t^0.3",
    "winres_to_embed_declared_match_count`t1",
    "winres_to_embed_resolved_match_count`t1",
    "winres_to_embed_edge`t$tauriWinresPackageId`tembed-resource`tembed_resource`t$embedPackageId`tnormal`tnull`t^3"
  ) -join "`n"

  return [pscustomobject][ordered]@{
    schema_version = 2
    kind = "vectorgrid_cpu_reliability_soak_windows_build_dependency_chain"
    aggregate_algorithm = "sha256(canonical_tab_rows_lf)"
    metadata_format_version = 1
    metadata_output_sha256 = $metadataSha256
    metadata_arguments = [object[]]$metadataArguments
    workspace_root = $repoRootPath
    cargo_home = $cargoHomePath
    workspace_member_count = @($metadata.workspace_members).Count
    root_resolved_features = [object[]]@("maintainer-tools")
    cargo = [ordered]@{
      path = $cargoPath
      bytes = [long]$cargoItem.Length
      sha256 = $cargoSha256
    }
    root_package_name_match_count = 1
    root_package = [ordered]@{
      package_id = $rootPackageId
      name = "ta_desktop_demo_app"
      version = "0.1.0"
      source = $null
      manifest_path = $resolvedRootManifestPath
      manifest_repo_relative_path = $rootManifestRelativePath
      manifest_bytes = [long]$rootManifestItem.Length
      manifest_sha256 = $rootManifestSha256
      workspace_member = $true
    }
    tauri_build_package_name_match_count = 1
    tauri_build = [ordered]@{
      package_id = $tauriBuildPackageId
      name = "tauri-build"
      version = "2.5.6"
      source = $tauriBuildSource
      manifest_path = $tauriBuildManifestPath
      source_tree = $tauriBuildTree
    }
    tauri_winres_package_name_match_count = 1
    tauri_winres = [ordered]@{
      package_id = $tauriWinresPackageId
      name = "tauri-winres"
      version = "0.3.5"
      source = $tauriWinresSource
      manifest_path = $tauriWinresManifestPath
      source_tree = $tauriWinresTree
    }
    embed_resource_package_name_match_count = 1
    embed_resource = [ordered]@{
      package_id = $embedPackageId
      name = "embed-resource"
      version = "3.0.6"
      source = $null
      manifest_path = $resolvedEmbedManifestPath
      manifest_repo_relative_path = $embedManifestRelativePath
      manifest_bytes = [long]$embedManifestItem.Length
      manifest_sha256 = $embedManifestSha256
      workspace_member = $true
    }
    root_to_tauri_build_declared_edge_match_count = 1
    root_to_tauri_build_resolved_edge_match_count = 1
    root_to_tauri_build_edge = [ordered]@{
      from_package_id = $rootPackageId
      declared_dependency_name = "tauri-build"
      resolved_dependency_name = "tauri_build"
      to_package_id = $tauriBuildPackageId
      kind = "build"
      target = $null
      declared_requirement = "^2"
    }
    tauri_build_to_winres_declared_edge_match_count = 1
    tauri_build_to_winres_resolved_edge_match_count = 1
    tauri_build_to_winres_edge = [ordered]@{
      from_package_id = $tauriBuildPackageId
      declared_dependency_name = "tauri-winres"
      resolved_dependency_name = "tauri_winres"
      to_package_id = $tauriWinresPackageId
      kind = "normal"
      target = $null
      declared_requirement = "^0.3"
    }
    winres_to_embed_declared_edge_match_count = 1
    winres_to_embed_resolved_edge_match_count = 1
    winres_to_embed_edge = [ordered]@{
      from_package_id = $tauriWinresPackageId
      declared_dependency_name = "embed-resource"
      resolved_dependency_name = "embed_resource"
      to_package_id = $embedPackageId
      kind = "normal"
      target = $null
      declared_requirement = "^3"
    }
    aggregate_sha256 = Get-StringSha256Hex -Text $canonical
  }
}

function Get-SoakNativeBuildProvenance {
  param(
    [Parameter(Mandatory = $true)][string]$HostTargetTriple,
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$CargoHome,
    [Parameter(Mandatory = $true)][string]$CargoExecutablePath,
    [Parameter(Mandatory = $true)][string]$ResourceCompilerAttestationNonce,
    [string[]]$PinnedEnvironmentNames = @(),
    [object]$ResourceCompilerInvocation = $null,
    [switch]$ConfigurePinnedEnvironment
  )

  if ($env:OS -ne "Windows_NT" -or $HostTargetTriple -ne "x86_64-pc-windows-msvc") {
    throw "Native build provenance currently supports only x86_64-pc-windows-msvc"
  }
  if ($ResourceCompilerAttestationNonce -notmatch '^[a-f0-9]{32}$') {
    throw "Resource compiler attestation nonce must be 32 lowercase hexadecimal characters"
  }

  $systemRoot = [Environment]::GetEnvironmentVariable("SystemRoot", "Machine")
  if ([string]::IsNullOrWhiteSpace($systemRoot)) {
    $systemRoot = [Environment]::GetFolderPath(
      [Environment+SpecialFolder]::Windows
    )
  }
  $systemRoot = [System.IO.Path]::GetFullPath($systemRoot).TrimEnd('\', '/')
  $cmdPath = [System.IO.Path]::GetFullPath(
    (Join-Path $systemRoot "System32\cmd.exe")
  )
  if (!(Test-Path -LiteralPath $cmdPath -PathType Leaf)) {
    throw "Canonical Windows command processor is missing: $cmdPath"
  }
  Assert-NoReparseAncestors -Path $cmdPath -Context "Canonical Windows command processor"

  $vswherePath = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
  if (!(Test-Path -LiteralPath $vswherePath -PathType Leaf)) {
    throw "Visual Studio discovery tool is missing: $vswherePath"
  }
  $vswherePath = [System.IO.Path]::GetFullPath($vswherePath)
  Assert-NoReparseAncestors -Path $vswherePath -Context "Visual Studio discovery tool"
  $installationLines = @(
    & $vswherePath -latest -products * `
      -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
      -property installationPath
  )
  if ($LASTEXITCODE -ne 0 -or $installationLines.Count -ne 1) {
    throw "Could not resolve exactly one default Visual Studio C++ installation"
  }
  $installationPath = [System.IO.Path]::GetFullPath(([string]$installationLines[0]).Trim())
  Assert-NoReparseAncestors -Path $installationPath -Context "Visual Studio installation"

  $versionFile = Join-Path $installationPath "VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt"
  if (!(Test-Path -LiteralPath $versionFile -PathType Leaf)) {
    throw "Visual C++ default tool-version file is missing: $versionFile"
  }
  Assert-NoReparseAncestors -Path $versionFile -Context "Visual C++ default tool version"
  $vcToolsVersion = ([string](Get-Content -Raw -LiteralPath $versionFile)).Trim()
  if ($vcToolsVersion -notmatch '^\d+\.\d+\.\d+$') {
    throw "Visual C++ default tool version is malformed: $vcToolsVersion"
  }
  $vcToolsRoot = Join-Path $installationPath "VC\Tools\MSVC\$vcToolsVersion"
  $vcBinDir = Join-Path $vcToolsRoot "bin\Hostx64\x64"
  $targetLinkerEnvironmentName =
    "CARGO_TARGET_{0}_LINKER" -f
      $HostTargetTriple.Replace("-", "_").ToUpperInvariant()
  if ($targetLinkerEnvironmentName -notmatch '^CARGO_TARGET_[A-Z0-9_]+_LINKER$') {
    throw "Host target triple cannot form a canonical Cargo linker environment name"
  }
  $targetLinkerPath = [System.IO.Path]::GetFullPath(
    (Join-Path $vcBinDir "link.exe")
  )

  $kitsRoot = [string](Get-ItemPropertyValue `
      -LiteralPath "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows Kits\Installed Roots" `
      -Name "KitsRoot10" `
      -ErrorAction Stop)
  $kitsRoot = [System.IO.Path]::GetFullPath($kitsRoot)
  Assert-NoReparseAncestors -Path $kitsRoot -Context "Windows SDK root"
  $sdkVersionDirectories = @(
    Get-ChildItem -LiteralPath (Join-Path $kitsRoot "bin") -Directory |
      Where-Object {
        $_.Name -match '^\d+\.\d+\.\d+\.\d+$' -and
        (Test-Path -LiteralPath (Join-Path $_.FullName "x64\rc.exe") -PathType Leaf)
      } |
      Sort-Object { [version]$_.Name }
  )
  if ($sdkVersionDirectories.Count -eq 0) {
    throw "No usable x64 Windows SDK tool directory was found under $kitsRoot"
  }
  $sdkVersionDirectory = $sdkVersionDirectories[-1]
  $sdkVersion = [string]$sdkVersionDirectory.Name
  $sdkBinDir = Join-Path $sdkVersionDirectory.FullName "x64"
  $resourceCompilerPath = [System.IO.Path]::GetFullPath(
    (Join-Path $sdkBinDir "rc.exe")
  )
  $vcvarsAllPath = Join-Path $installationPath "VC\Auxiliary\Build\vcvarsall.bat"
  if (!(Test-Path -LiteralPath $vcvarsAllPath -PathType Leaf)) {
    throw "Pinned Visual C++ environment initializer is missing: $vcvarsAllPath"
  }
  $vcvarsAllPath = [System.IO.Path]::GetFullPath($vcvarsAllPath)

  $explicitSelectionEnvironmentNames = [string[]]@(
    $targetLinkerEnvironmentName,
    "COMSPEC",
    "EMBED_RESOURCE_RC_ATTESTATION_NONCE",
    "EMBED_RESOURCE_RC_PATH"
  )
  $vcvarsEnvironmentNames = [string[]]@()
  if ($ConfigurePinnedEnvironment) {
    $baselineEnvironment = [System.Collections.Generic.Dictionary[string, string]]::new(
      [StringComparer]::OrdinalIgnoreCase
    )
    foreach ($entry in Get-ChildItem Env:) {
      $canonicalName = ([string]$entry.Name).ToUpperInvariant()
      if ($baselineEnvironment.ContainsKey($canonicalName)) {
        throw "Process environment contains a case-insensitive duplicate name: $canonicalName"
      }
      $baselineEnvironment[$canonicalName] = [string]$entry.Value
    }
    $vcvarsCommand =
      'call "' + $vcvarsAllPath + '" x64 ' + $sdkVersion +
      ' -vcvars_ver=' + $vcToolsVersion + ' >nul && set'
    $vcvarsLines = @(& $cmdPath /d /s /c $vcvarsCommand)
    if ($LASTEXITCODE -ne 0 -or $vcvarsLines.Count -eq 0) {
      throw "Pinned vcvarsall environment initialization failed"
    }
    $vcvarsEnvironment = [System.Collections.Generic.Dictionary[string, string]]::new(
      [StringComparer]::OrdinalIgnoreCase
    )
    foreach ($line in $vcvarsLines) {
      $separator = ([string]$line).IndexOf("=")
      if ($separator -gt 0) {
        $canonicalName =
          ([string]$line).Substring(0, $separator).ToUpperInvariant()
        if ($vcvarsEnvironment.ContainsKey($canonicalName)) {
          throw "vcvarsall emitted a case-insensitive duplicate name: $canonicalName"
        }
        $vcvarsEnvironment[$canonicalName] =
          ([string]$line).Substring($separator + 1)
      }
    }
    $vcvarsEnvironmentNames = [string[]]@(
      $vcvarsEnvironment.Keys | Where-Object {
        $explicitSelectionEnvironmentNames -notcontains [string]$_ -and
        (
          !$baselineEnvironment.ContainsKey([string]$_) -or
          ![string]::Equals(
            [string]$baselineEnvironment[[string]$_],
            [string]$vcvarsEnvironment[[string]$_],
            [StringComparison]::Ordinal
          )
        )
      }
    )
    [Array]::Sort($vcvarsEnvironmentNames, [StringComparer]::Ordinal)
    if ($vcvarsEnvironmentNames.Count -eq 0) {
      throw "Pinned vcvarsall did not produce any changed environment names"
    }
    foreach ($name in $vcvarsEnvironmentNames) {
      if (
        $name -match '^(?i:CARGO_|RUST|TAURI_|HTTP_PROXY$|HTTPS_PROXY$|ALL_PROXY$|NO_PROXY$|POLYGON_|EMBED_RESOURCE_)'
      ) {
        throw "vcvarsall attempted to change a protected soak environment name: $name"
      }
      [Environment]::SetEnvironmentVariable(
        $name,
        [string]$vcvarsEnvironment[$name],
        "Process"
      )
    }
    [Environment]::SetEnvironmentVariable(
      "COMSPEC",
      $cmdPath,
      "Process"
    )
    [Environment]::SetEnvironmentVariable(
      "EMBED_RESOURCE_RC_PATH",
      $resourceCompilerPath,
      "Process"
    )
    [Environment]::SetEnvironmentVariable(
      "EMBED_RESOURCE_RC_ATTESTATION_NONCE",
      $ResourceCompilerAttestationNonce,
      "Process"
    )
    [Environment]::SetEnvironmentVariable(
      $targetLinkerEnvironmentName,
      $targetLinkerPath,
      "Process"
    )
    $selectionEnvironmentNames = [string[]]@(
      $vcvarsEnvironmentNames + $explicitSelectionEnvironmentNames |
        Select-Object -Unique
    )
    [Array]::Sort($selectionEnvironmentNames, [StringComparer]::Ordinal)
  } else {
    if ($PinnedEnvironmentNames.Count -eq 0) {
      throw "Post-build native provenance requires the pinned vcvars environment-name set"
    }
    $pinnedNameSet = [System.Collections.Generic.Dictionary[string, bool]]::new(
      [StringComparer]::OrdinalIgnoreCase
    )
    foreach ($rawName in $PinnedEnvironmentNames) {
      $canonicalName = ([string]$rawName).ToUpperInvariant()
      if (
        [string]$rawName -cne $canonicalName -or
        $pinnedNameSet.ContainsKey($canonicalName)
      ) {
        throw "Pinned native environment names must be uppercase and case-insensitively unique"
      }
      $pinnedNameSet[$canonicalName] = $true
    }
    $selectionEnvironmentNames = [string[]]@($pinnedNameSet.Keys)
    [Array]::Sort($selectionEnvironmentNames, [StringComparer]::Ordinal)
    $vcvarsEnvironmentNames = [string[]]@(
      $selectionEnvironmentNames | Where-Object {
        $explicitSelectionEnvironmentNames -notcontains $_
      }
    )
  }

  $requiredEnvironment = @(
    "PATH",
    "COMSPEC",
    $targetLinkerEnvironmentName,
    "EMBED_RESOURCE_RC_ATTESTATION_NONCE",
    "EMBED_RESOURCE_RC_PATH",
    "EXTERNAL_INCLUDE",
    "FRAMEWORK40VERSION",
    "__DOTNET_ADD_64BIT",
    "__DOTNET_PREFERRED_BITNESS",
    "INCLUDE",
    "LIB",
    "LIBPATH",
    "VSINSTALLDIR",
    "VCINSTALLDIR",
    "VCTOOLSINSTALLDIR",
    "VCTOOLSVERSION",
    "WINDOWSSDKDIR",
    "WINDOWSSDKVERSION",
    "UNIVERSALCRTSDKDIR",
    "UCRTVERSION",
    "VSCMD_ARG_HOST_ARCH",
    "VSCMD_ARG_TGT_ARCH",
    "VSCMD_ARG_VCVARS_VER",
    "VSCMD_ARG_WINSDK"
  )
  foreach ($name in $requiredEnvironment) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
      throw "Pinned native build environment is missing $name"
    }
  }
  $normalizedVsInstall = [System.IO.Path]::GetFullPath($env:VSINSTALLDIR).TrimEnd('\', '/')
  $normalizedDiscoveredVs = $installationPath.TrimEnd('\', '/')
  $normalizedSdkRoot = [System.IO.Path]::GetFullPath($env:WindowsSdkDir).TrimEnd('\', '/')
  $normalizedDiscoveredSdkRoot = $kitsRoot.TrimEnd('\', '/')
  $normalizedResourceCompilerOverride =
    [System.IO.Path]::GetFullPath($env:EMBED_RESOURCE_RC_PATH)
  if (
    ![string]::Equals(
      $normalizedVsInstall,
      $normalizedDiscoveredVs,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    ![string]::Equals(
      ([string]$env:VCToolsVersion).TrimEnd('\'),
      $vcToolsVersion,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    ![string]::Equals(
      $normalizedSdkRoot,
      $normalizedDiscoveredSdkRoot,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    ![string]::Equals(
      ([string]$env:WindowsSDKVersion).TrimEnd('\'),
      $sdkVersion,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    ![string]::Equals(
      $normalizedResourceCompilerOverride,
      $resourceCompilerPath,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    $env:VSCMD_ARG_HOST_ARCH -ne "x64" -or
    $env:VSCMD_ARG_TGT_ARCH -ne "x64" -or
    ![string]::Equals(
      [System.IO.Path]::GetFullPath($env:COMSPEC),
      $cmdPath,
      [StringComparison]::OrdinalIgnoreCase
    ) -or
    $env:EMBED_RESOURCE_RC_ATTESTATION_NONCE -ne
      $ResourceCompilerAttestationNonce
  ) {
    throw "Pinned native build environment does not match the discovered VC/SDK selection"
  }

  $toolSpecs = @(
    [ordered]@{ name = "vswhere"; path = $vswherePath },
    [ordered]@{ name = "vcvarsall"; path = $vcvarsAllPath },
    [ordered]@{ name = "cmd"; path = $cmdPath },
    [ordered]@{ name = "cl"; path = (Join-Path $vcBinDir "cl.exe") },
    [ordered]@{ name = "link"; path = $targetLinkerPath },
    [ordered]@{ name = "lib"; path = (Join-Path $vcBinDir "lib.exe") },
    [ordered]@{ name = "ml64"; path = (Join-Path $vcBinDir "ml64.exe") },
    [ordered]@{ name = "rc"; path = $resourceCompilerPath },
    [ordered]@{ name = "mt"; path = (Join-Path $sdkBinDir "mt.exe") }
  )
  $toolMap = [System.Collections.Generic.Dictionary[string, object]]::new(
    [StringComparer]::Ordinal
  )
  foreach ($spec in $toolSpecs) {
    $name = [string]$spec.name
    $path = [System.IO.Path]::GetFullPath([string]$spec.path)
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
      throw "Required native build tool is missing: $path"
    }
    Assert-NoReparseAncestors -Path $path -Context "Native build tool $name"
    $item = Get-Item -Force -LiteralPath $path
    $toolMap[$name] = [ordered]@{
      name = $name
      path = $path
      bytes = [long]$item.Length
      sha256 = Get-Sha256Hex -Path $path
    }
  }
  $toolKeys = [string[]]@($toolMap.Keys)
  [Array]::Sort($toolKeys, [StringComparer]::Ordinal)
  $tools = @($toolKeys | ForEach-Object { $toolMap[$_] })
  $pathValue = [Environment]::GetEnvironmentVariable("PATH", "Process")
  if ([string]::IsNullOrWhiteSpace([string]$pathValue) -or $pathValue -match "[`t`r`n]") {
    throw "Pinned PATH is missing or non-canonical"
  }
  $pathResolvedTools = @(
    foreach ($name in [string[]]@("cl", "lib", "link", "ml64", "mt", "rc")) {
      $executableName = "$name.exe"
      $resolvedPath = Resolve-SoakPathExecutable `
        -PathValue ([string]$pathValue) `
        -ExecutableName $executableName
      $resolvedPathKey =
        ConvertTo-SoakAsciiLowerInvariant -Text $resolvedPath
      $attestedPathKey =
        ConvertTo-SoakAsciiLowerInvariant -Text ([string]$toolMap[$name].path)
      if (
        ![string]::Equals(
          $resolvedPathKey,
          $attestedPathKey,
          [StringComparison]::Ordinal
        )
      ) {
        throw "Pinned PATH resolves $executableName outside its attested tool path: $resolvedPath"
      }
      [ordered]@{
        name = $name
        executable_name = $executableName
        path = [string]$toolMap[$name].path
        bytes = [long]$toolMap[$name].bytes
        sha256 = [string]$toolMap[$name].sha256
      }
    }
  )
  $pathToolCanonical = ($pathResolvedTools | ForEach-Object {
      "{0}`t{1}`t{2}`t{3}`t{4}" -f
        $_.name, $_.executable_name, $_.path, $_.bytes, $_.sha256
    }) -join "`n"
  $pathValueSha256 = Get-StringSha256Hex -Text ([string]$pathValue)
  $pathToolAggregateCanonical =
    "path_value_sha256`t$pathValueSha256`n$pathToolCanonical"
  $pathToolResolution = [ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_path_tool_resolution"
    environment_name = "PATH"
    value = [string]$pathValue
    value_sha256 = $pathValueSha256
    resolution_policy =
      "first_existing_ordinary_executable_in_recorded_path_order"
    tool_count = $pathResolvedTools.Count
    tools = [object[]]$pathResolvedTools
    aggregate_algorithm =
      "sha256(path_value_sha256_then_name_tab_executable_tab_path_tab_bytes_tab_sha256_lf)"
    aggregate_sha256 = Get-StringSha256Hex -Text $pathToolAggregateCanonical
  }
  $resourceCompilerExecution = [ordered]@{
    environment_name = "EMBED_RESOURCE_RC_PATH"
    path = [string]$toolMap["rc"].path
    bytes = [long]$toolMap["rc"].bytes
    sha256 = [string]$toolMap["rc"].sha256
  }
  $cargoResolution = Get-SoakEmbedResourceCargoResolution `
    -RepoRoot $RepoRoot `
    -CargoHome $CargoHome `
    -CargoExecutablePath $CargoExecutablePath `
    -HostTargetTriple $HostTargetTriple
  $resourceCompilerCandidates = @(
    foreach ($directory in $sdkVersionDirectories) {
      $path = [System.IO.Path]::GetFullPath((Join-Path $directory.FullName "x64\rc.exe"))
      $item = Get-Item -Force -LiteralPath $path
      [ordered]@{
        version = [string]$directory.Name
        path = $path
        bytes = [long]$item.Length
        sha256 = Get-Sha256Hex -Path $path
      }
    }
  )
  $nativeInputManifest = Get-SoakNativeInputManifest `
    -VisualStudioInstallation $installationPath `
    -VcToolsVersion $vcToolsVersion `
    -WindowsSdkRoot $kitsRoot
  $environment = @(
    foreach ($name in $selectionEnvironmentNames) {
      $value = [Environment]::GetEnvironmentVariable($name, "Process")
      [ordered]@{
        name = $name
        present = $null -ne $value
        value_sha256 = if ($null -eq $value) {
          $null
        } else {
          Get-StringSha256Hex -Text ([string]$value)
        }
      }
    }
  )
  $environmentCanonical = ($environment | ForEach-Object {
      "{0}`t{1}`t{2}" -f $_.name, ([bool]$_.present).ToString().ToLowerInvariant(), $_.value_sha256
    }) -join "`n"
  $toolCanonical = ($tools | ForEach-Object {
      "{0}`t{1}`t{2}`t{3}" -f $_.name, $_.path, $_.bytes, $_.sha256
    }) -join "`n"
  $resourceCompilerCanonical = ($resourceCompilerCandidates | ForEach-Object {
      "{0}`t{1}`t{2}`t{3}" -f $_.version, $_.path, $_.bytes, $_.sha256
    }) -join "`n"
  $resourceCompilerExecutionCanonical =
    "{0}`t{1}`t{2}`t{3}" -f
      $resourceCompilerExecution.environment_name,
      $resourceCompilerExecution.path,
      $resourceCompilerExecution.bytes,
      $resourceCompilerExecution.sha256
  $vcvarsEnvironmentCanonical = $vcvarsEnvironmentNames -join "`t"
  $preExecutionCanonical = @(
    "host_target_triple`t$HostTargetTriple",
    "system_root`t$systemRoot",
    "visual_studio_installation`t$installationPath",
    "vc_tools_version`t$vcToolsVersion",
    "windows_sdk_root`t$kitsRoot",
    "windows_sdk_version`t$sdkVersion",
    "tools`t$toolCanonical",
    "path_tool_resolution`t$($pathToolResolution.aggregate_sha256)",
    "resource_compiler_execution`t$resourceCompilerExecutionCanonical",
    "resource_compiler_candidates`t$resourceCompilerCanonical",
    "cargo_resolution`t$($cargoResolution.aggregate_sha256)",
    "native_inputs`t$($nativeInputManifest.aggregate_sha256)",
    "vcvars_environment_names`t$vcvarsEnvironmentCanonical",
    "environment`t$environmentCanonical"
  ) -join "`n"
  $preExecutionAggregate = Get-StringSha256Hex -Text $preExecutionCanonical
  $aggregateCanonical = $preExecutionCanonical
  if ($null -ne $ResourceCompilerInvocation) {
    $expectedMarkerValue =
      "$ResourceCompilerAttestationNonce|$resourceCompilerPath"
    $expectedMarkerHash = Get-StringSha256Hex -Text $expectedMarkerValue
    if (
      [int]$ResourceCompilerInvocation.schema_version -ne 1 -or
      [string]$ResourceCompilerInvocation.kind -ne
        "vectorgrid_cpu_reliability_soak_resource_compiler_invocation" -or
      [int]$ResourceCompilerInvocation.build_script_event_match_count -ne 1 -or
      [int]$ResourceCompilerInvocation.marker_match_count -ne 1 -or
      [string]$ResourceCompilerInvocation.package_id -ne
        [string]$cargoResolution.root_package.package_id -or
      [string]$ResourceCompilerInvocation.package_name -ne
        "ta_desktop_demo_app" -or
      [string]$ResourceCompilerInvocation.target_triple -ne
        $HostTargetTriple -or
      [string]::IsNullOrWhiteSpace(
        [string]$ResourceCompilerInvocation.build_script_out_dir
      ) -or
      [string]$ResourceCompilerInvocation.environment_name -ne
        "VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION" -or
      [string]$ResourceCompilerInvocation.nonce -ne
        $ResourceCompilerAttestationNonce -or
      ![string]::Equals(
        [string]$ResourceCompilerInvocation.resource_compiler_path,
        $resourceCompilerPath,
        [StringComparison]::Ordinal
      ) -or
      [long]$ResourceCompilerInvocation.resource_compiler_bytes -ne
        [long]$resourceCompilerExecution.bytes -or
      [string]$ResourceCompilerInvocation.resource_compiler_sha256 -ne
        [string]$resourceCompilerExecution.sha256 -or
      [string]$ResourceCompilerInvocation.marker_value_sha256 -ne
        $expectedMarkerHash -or
      ![bool]$ResourceCompilerInvocation.file_lock_held_during_build -or
      [string]$ResourceCompilerInvocation.file_lock_share_mode -ne
        "read_access_share_read_only_denies_write_delete"
    ) {
      throw "Resource compiler invocation event does not match the pinned build selection"
    }
    $invocationCanonical =
      "{0}`t{1}`t{2}`t{3}`t{4}`t{5}`t{6}`t{7}`t{8}`t{9}`t{10}" -f
        $ResourceCompilerInvocation.package_id,
        $ResourceCompilerInvocation.package_name,
        $ResourceCompilerInvocation.target_triple,
        $ResourceCompilerInvocation.build_script_out_dir,
        $ResourceCompilerInvocation.environment_name,
        $ResourceCompilerInvocation.nonce,
        $ResourceCompilerInvocation.resource_compiler_path,
        $ResourceCompilerInvocation.resource_compiler_bytes,
        $ResourceCompilerInvocation.resource_compiler_sha256,
        $ResourceCompilerInvocation.marker_value_sha256,
        $ResourceCompilerInvocation.file_lock_share_mode
    $aggregateCanonical =
      "$preExecutionCanonical`nresource_compiler_invocation`t$invocationCanonical"
  }

  return [pscustomobject][ordered]@{
    schema_version = 6
    kind = "vectorgrid_cpu_reliability_soak_native_build_provenance"
    host_target_triple = $HostTargetTriple
    selection_policy =
      "pinned_cmd_vcvarsall_plus_absolute_target_linker_plus_success_attested_rc_plus_locked_offline_build_chain"
    ambient_override_environment_cleared = $true
    pinned_environment_applied = $true
    resource_compiler_execution_pinned = $true
    resource_compiler_candidate_set_attested = $true
    cargo_resolution_attested = $true
    resource_compiler_success_attested = $null -ne $ResourceCompilerInvocation
    resource_compiler_write_delete_lock_required = $true
    system_root = $systemRoot
    visual_studio_installation = $installationPath
    vc_tools_version = $vcToolsVersion
    windows_sdk_root = $kitsRoot
    windows_sdk_version = $sdkVersion
    path_tool_resolution = $pathToolResolution
    target_linker_path_pinned = $true
    target_linker_environment_name = $targetLinkerEnvironmentName
    vcvars_environment_selection_policy =
      "all_changed_or_added_vcvars_names_plus_explicit_target_linker_and_rc_attestation"
    vcvars_environment_name_count = $vcvarsEnvironmentNames.Count
    vcvars_environment_names = [object[]]$vcvarsEnvironmentNames
    environment = [object[]]$environment
    environment_aggregate_sha256 = Get-StringSha256Hex -Text $environmentCanonical
    tools = [object[]]$tools
    resource_compiler_execution = $resourceCompilerExecution
    resource_compiler_candidates = [object[]]$resourceCompilerCandidates
    resource_compiler_attestation_nonce = $ResourceCompilerAttestationNonce
    resource_compiler_invocation = $ResourceCompilerInvocation
    cargo_resolution = $cargoResolution
    native_inputs = $nativeInputManifest
    pre_execution_aggregate_sha256 = $preExecutionAggregate
    aggregate_sha256 = Get-StringSha256Hex -Text $aggregateCanonical
  }
}

function Get-SoakToolchainInputManifest {
  param(
    [Parameter(Mandatory = $true)][string]$Sysroot,
    [Parameter(Mandatory = $true)][string]$HostTargetTriple
  )

  $roots = @(
    (Join-Path $Sysroot "bin"),
    (Join-Path (Join-Path (Join-Path (Join-Path $Sysroot "lib") "rustlib") $HostTargetTriple) "lib")
  )
  $sysrootPrefix = $Sysroot.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
  $files = [System.Collections.Generic.Dictionary[string, string]]::new(
    [StringComparer]::OrdinalIgnoreCase
  )
  foreach ($root in $roots) {
    if (!(Test-Path -LiteralPath $root -PathType Container)) {
      throw "Required Rust toolchain input root is missing: $root"
    }
    Assert-NoReparseAncestors -Path $root -Context "Rust toolchain input root"
    foreach ($item in Get-ChildItem -Force -LiteralPath $root -Recurse) {
      if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Rust toolchain input manifest refuses reparse-point input: $($item.FullName)"
      }
      if (!$item.PSIsContainer) {
        $absolute = [System.IO.Path]::GetFullPath($item.FullName)
        if (!$absolute.StartsWith($sysrootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
          throw "Rust toolchain input escaped the selected sysroot: $absolute"
        }
        $relative = $absolute.Substring($sysrootPrefix.Length).Replace('\', '/')
        $files[$relative] = $absolute
      }
    }
  }
  $sortedPaths = [string[]]@($files.Keys)
  [Array]::Sort($sortedPaths, [StringComparer]::Ordinal)
  $rows = @(
    foreach ($relativePath in $sortedPaths) {
      $absolute = $files[$relativePath]
      $item = Get-Item -Force -LiteralPath $absolute
      [ordered]@{
        path = $relativePath
        bytes = [long]$item.Length
        sha256 = Get-Sha256Hex -Path $absolute
      }
    }
  )
  $canonical = ($rows | ForEach-Object {
      "{0}`t{1}`t{2}" -f $_.path, $_.bytes, $_.sha256
    }) -join "`n"
  $totalBytes = [long]0
  foreach ($row in $rows) {
    $totalBytes += [long]$row.bytes
  }
  return [pscustomobject][ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_toolchain_inputs"
    root = $Sysroot
    aggregate_algorithm = "sha256(relative_path_tab_bytes_tab_sha256_lf)"
    aggregate_sha256 = Get-StringSha256Hex -Text $canonical
    file_count = $rows.Count
    total_bytes = $totalBytes
    files = $rows
  }
}

function Get-SoakToolchainProvenance {
  param(
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [switch]$IncludeInputManifest,
    [switch]$ProbeSelectedExecutables
  )

  $rustupInvocationPath = Resolve-ExecutableFile -Name "rustup"
  Push-Location $RepoRoot
  try {
    # Resolve directory-scoped tools through the ordinary rustup executable,
    # not its cargo/rustc proxy symlinks, then execute the selected binaries
    # directly for the build and provenance probes.
    $cargoWhichLines = @(& $rustupInvocationPath which cargo 2>&1)
    if ($LASTEXITCODE -ne 0 -or $cargoWhichLines.Count -ne 1) {
      throw "Failed to resolve repository-selected Cargo through rustup"
    }
    $rustcWhichLines = @(& $rustupInvocationPath which rustc 2>&1)
    if ($LASTEXITCODE -ne 0 -or $rustcWhichLines.Count -ne 1) {
      throw "Failed to resolve repository-selected rustc through rustup"
    }
    $cargoToolchainPath =
      [System.IO.Path]::GetFullPath(([string]$cargoWhichLines[0]).Trim())
    $rustcToolchainPath =
      [System.IO.Path]::GetFullPath(([string]$rustcWhichLines[0]).Trim())
    foreach ($tool in @(
        @{ Name = "Cargo toolchain executable"; Path = $cargoToolchainPath },
        @{ Name = "rustc toolchain executable"; Path = $rustcToolchainPath }
      )) {
      if (!(Test-Path -LiteralPath $tool.Path -PathType Leaf)) {
        throw "$($tool.Name) was not found: $($tool.Path)"
      }
      Assert-NoReparseAncestors -Path $tool.Path -Context $tool.Name
    }

    $cargoVersionLines = @(& $cargoToolchainPath --version --verbose 2>&1)
    if ($LASTEXITCODE -ne 0) {
      throw "Failed to capture Cargo version provenance"
    }
    $rustcVersionLines = @(& $rustcToolchainPath --version --verbose 2>&1)
    if ($LASTEXITCODE -ne 0) {
      throw "Failed to capture rustc version provenance"
    }
    $hostTargetLines = @(
      $rustcVersionLines | ForEach-Object { [string]$_ } |
        Where-Object { $_ -match '^host:\s*([A-Za-z0-9_.-]+)\s*$' }
    )
    if ($hostTargetLines.Count -ne 1) {
      throw "Failed to resolve exactly one host target triple from rustc --version --verbose"
    }
    $hostTargetTriple = (
      [regex]::Match([string]$hostTargetLines[0], '^host:\s*([A-Za-z0-9_.-]+)\s*$')
    ).Groups[1].Value
    $rustcSysrootLines = @(& $rustcToolchainPath --print sysroot 2>&1)
    if ($LASTEXITCODE -ne 0 -or $rustcSysrootLines.Count -ne 1) {
      throw "Failed to resolve the repository-selected rustc sysroot"
    }
    $rustcSysroot = [System.IO.Path]::GetFullPath(([string]$rustcSysrootLines[0]).Trim())
    $sysrootBin = [System.IO.Path]::GetFullPath((Join-Path $rustcSysroot "bin"))
    foreach ($toolPath in @($cargoToolchainPath, $rustcToolchainPath)) {
      if (!(Test-PathWithin -Path $toolPath -Parent $sysrootBin)) {
        throw "Rustup selected a tool outside the reported sysroot: $toolPath"
      }
    }
    $selectedCargoVersion = $null
    $selectedRustcVersion = $null
    if ($ProbeSelectedExecutables) {
      $selectedCargoLines = @(& $cargoToolchainPath --version --verbose 2>&1)
      if ($LASTEXITCODE -ne 0) {
        throw "Failed to probe the selected Cargo executable"
      }
      $selectedRustcLines = @(& $rustcToolchainPath --version --verbose 2>&1)
      if ($LASTEXITCODE -ne 0) {
        throw "Failed to probe the selected rustc executable"
      }
      $selectedSysrootLines = @(& $rustcToolchainPath --print sysroot 2>&1)
      if (
        $LASTEXITCODE -ne 0 -or
        $selectedSysrootLines.Count -ne 1 -or
        [System.IO.Path]::GetFullPath(([string]$selectedSysrootLines[0]).Trim()) -ne
          $rustcSysroot
      ) {
        throw "Selected rustc executable does not report the attested sysroot"
      }
      $selectedCargoVersion =
        (@($selectedCargoLines | ForEach-Object { [string]$_ }) -join "`n").Trim()
      $selectedRustcVersion =
        (@($selectedRustcLines | ForEach-Object { [string]$_ }) -join "`n").Trim()
      if (
        $selectedRustcVersion -notmatch "(?m)^host:\s*$([regex]::Escape($hostTargetTriple))\s*$"
      ) {
        throw "Selected rustc executable reports a different host target"
      }
    }
    $toolchainInputManifest = $null
    if ($IncludeInputManifest) {
      $toolchainInputManifest = Get-SoakToolchainInputManifest `
        -Sysroot $rustcSysroot `
        -HostTargetTriple $hostTargetTriple
    }
    return [pscustomobject][ordered]@{
      rustc_sysroot = $rustcSysroot
      cargo_path = $cargoToolchainPath
      cargo_sha256 = Get-Sha256Hex -Path $cargoToolchainPath
      cargo_invocation_path = $rustupInvocationPath
      cargo_invocation_sha256 = Get-Sha256Hex -Path $rustupInvocationPath
      cargo_version_verbose =
        (@($cargoVersionLines | ForEach-Object { [string]$_ }) -join "`n").Trim()
      rustc_path = $rustcToolchainPath
      rustc_sha256 = Get-Sha256Hex -Path $rustcToolchainPath
      rustc_invocation_path = $rustupInvocationPath
      rustc_invocation_sha256 = Get-Sha256Hex -Path $rustupInvocationPath
      rustc_version_verbose =
        (@($rustcVersionLines | ForEach-Object { [string]$_ }) -join "`n").Trim()
      host_target_triple = $hostTargetTriple
      selected_cargo_version_verbose = $selectedCargoVersion
      selected_rustc_version_verbose = $selectedRustcVersion
      input_manifest = $toolchainInputManifest
    }
  } finally {
    Pop-Location
  }
}

function Resolve-SoakCargoHome {
  param([Parameter(Mandatory = $true)][string]$RepoRoot)

  $source = "default-user-profile"
  $raw = [Environment]::GetEnvironmentVariable("CARGO_HOME", "Process")
  if ([string]::IsNullOrWhiteSpace($raw)) {
    $userHome = [Environment]::GetFolderPath(
      [Environment+SpecialFolder]::UserProfile
    )
    if ([string]::IsNullOrWhiteSpace($userHome)) {
      $userHome = [Environment]::GetEnvironmentVariable("HOME", "Process")
    }
    if ([string]::IsNullOrWhiteSpace($userHome)) {
      throw "Unable to resolve the default Cargo home"
    }
    $raw = Join-Path $userHome ".cargo"
  } else {
    $source = "inherited-environment"
  }
  $resolved = Resolve-RepoPath -Path $raw -RepoRoot $RepoRoot
  $resolved = Assert-SafeDirectoryPath -Path $resolved -Context "Cargo home"
  return [pscustomobject][ordered]@{
    path = $resolved
    source = $source
  }
}

function Get-SoakCargoConfigurationProvenance {
  param(
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$CargoHome
  )

  $candidatePaths = [System.Collections.Generic.Dictionary[string, string]]::new(
    [StringComparer]::OrdinalIgnoreCase
  )
  $cursor = [System.IO.DirectoryInfo]::new($RepoRoot)
  while ($null -ne $cursor) {
    foreach ($fileName in @("config.toml", "config")) {
      $candidate = [System.IO.Path]::GetFullPath(
        (Join-Path (Join-Path $cursor.FullName ".cargo") $fileName)
      )
      $candidatePaths[$candidate] = $candidate
    }
    $cursor = $cursor.Parent
  }
  foreach ($fileName in @("config.toml", "config")) {
    $candidate = [System.IO.Path]::GetFullPath((Join-Path $CargoHome $fileName))
    $candidatePaths[$candidate] = $candidate
  }

  $sortedPaths = [string[]]@($candidatePaths.Keys)
  [Array]::Sort($sortedPaths, [StringComparer]::Ordinal)
  $rows = @(
    foreach ($candidate in $sortedPaths) {
      Assert-NoReparseAncestors -Path $candidate -Context "Cargo configuration input"
      if (Test-Path -LiteralPath $candidate -PathType Container) {
        throw "Cargo configuration candidate must not be a directory: $candidate"
      }
      $exists = Test-Path -LiteralPath $candidate -PathType Leaf
      $bytes = $null
      $sha256 = $null
      if ($exists) {
        $item = Get-Item -Force -LiteralPath $candidate
        $bytes = [long]$item.Length
        $sha256 = Get-Sha256Hex -Path $candidate
      }
      [ordered]@{
        path = $candidate
        exists = [bool]$exists
        bytes = $bytes
        sha256 = $sha256
      }
    }
  )
  $canonical = ($rows | ForEach-Object {
      $existsText = if ($_.exists) { "1" } else { "0" }
      $bytesText = if ($_.exists) { [string]$_.bytes } else { "-" }
      $hashText = if ($_.exists) { [string]$_.sha256 } else { "-" }
      "{0}`t{1}`t{2}`t{3}" -f $_.path, $existsText, $bytesText, $hashText
    }) -join "`n"
  $workspaceConfigPath = [System.IO.Path]::GetFullPath(
    (Join-Path (Join-Path $RepoRoot ".cargo") "config.toml")
  )
  $workspaceConfigRow = @(
    $rows | Where-Object {
      [string]::Equals(
        [string]$_.path,
        $workspaceConfigPath,
        [StringComparison]::OrdinalIgnoreCase
      )
    }
  )
  $externalExistingCount = @(
    $rows | Where-Object {
      $_.exists -and
      ![string]::Equals(
        [string]$_.path,
        $workspaceConfigPath,
        [StringComparison]::OrdinalIgnoreCase
      )
    }
  ).Count
  return [pscustomobject][ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_cargo_configuration"
    aggregate_algorithm = "sha256(absolute_path_tab_exists_tab_bytes_tab_sha256_lf)"
    cargo_home = $CargoHome
    repo_root = $RepoRoot
    workspace_config_path = $workspaceConfigPath
    workspace_config_sha256 = if (
      $workspaceConfigRow.Count -eq 1 -and $workspaceConfigRow[0].exists
    ) {
      [string]$workspaceConfigRow[0].sha256
    } else {
      $null
    }
    external_existing_file_count = $externalExistingCount
    release_authority_eligible = (
      $workspaceConfigRow.Count -eq 1 -and
      $workspaceConfigRow[0].exists -and
      $externalExistingCount -eq 0
    )
    candidate_count = $rows.Count
    existing_file_count = @($rows | Where-Object { $_.exists }).Count
    aggregate_sha256 = Get-StringSha256Hex -Text $canonical
    candidates = $rows
  }
}

function Get-SoakSourceManifest {
  param([Parameter(Mandatory = $true)][string]$RepoRoot)

  Assert-NoReparseAncestors -Path $RepoRoot -Context "Soak source repository"
  $files = [System.Collections.Generic.Dictionary[string, string]]::new(
    [StringComparer]::OrdinalIgnoreCase
  )
  $specificPaths = @(
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "tauri.conf.json",
    "tauri.windows.conf.json",
    "src-tauri/Cargo.toml",
    "src-tauri/build.rs",
    "src-tauri/command-risk-policy.json",
    "src-tauri/tauri.conf.json",
    "src-tauri/tauri.windows.conf.json",
    "scripts/run_cpu_reliability_soak.ps1",
    "scripts/benchmark_lock.ps1"
  )
  foreach ($relativePath in $specificPaths) {
    $absolutePath = Join-Path $RepoRoot $relativePath
    if (!(Test-Path -LiteralPath $absolutePath -PathType Leaf)) {
      throw "Required soak source input is missing: $absolutePath"
    }
    Assert-NoReparseAncestors -Path $absolutePath -Context "Soak source input"
    $files[$relativePath.Replace('\', '/')] = $absolutePath
  }

  $sourceRoots = @(
    (Join-Path $RepoRoot ".cargo"),
    (Join-Path $RepoRoot "crates"),
    (Join-Path $RepoRoot "src-tauri/assets"),
    (Join-Path $RepoRoot "src-tauri/src"),
    (Join-Path $RepoRoot "src-tauri/capabilities"),
    (Join-Path $RepoRoot "src-tauri/gen"),
    (Join-Path $RepoRoot "src-tauri/icons"),
    (Join-Path $RepoRoot "src-tauri/resources"),
    (Join-Path $RepoRoot "vendor/embed-resource-3.0.6"),
    (Join-Path $RepoRoot "vendor/vector-ta-0.2.4")
  )
  $repoPrefix = $RepoRoot.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
  foreach ($sourceRoot in $sourceRoots) {
    if (!(Test-Path -LiteralPath $sourceRoot -PathType Container)) {
      continue
    }
    Assert-NoReparseAncestors -Path $sourceRoot -Context "Soak source root"
    foreach ($item in Get-ChildItem -LiteralPath $sourceRoot -Recurse -Force) {
      if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Soak source fingerprint refuses reparse-point input: $($item.FullName)"
      }
      if (!$item.PSIsContainer) {
        $absoluteItemPath = [System.IO.Path]::GetFullPath($item.FullName)
        if (!$absoluteItemPath.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)) {
          throw "Soak source fingerprint input escaped the repository: $absoluteItemPath"
        }
        $relativePath = $absoluteItemPath.Substring($repoPrefix.Length).Replace('\', '/')
        $files[$relativePath] = $item.FullName
      }
    }
  }

  $sortedPaths = [string[]]@($files.Keys)
  [Array]::Sort($sortedPaths, [StringComparer]::Ordinal)
  $rows = @(
    foreach ($relativePath in $sortedPaths) {
      $absolutePath = $files[$relativePath]
      $item = Get-Item -LiteralPath $absolutePath
      [ordered]@{
        path = $relativePath
        bytes = [long]$item.Length
        sha256 = Get-Sha256Hex -Path $absolutePath
      }
    }
  )
  $canonical = ($rows | ForEach-Object {
      "{0}`t{1}`t{2}" -f $_.path, $_.bytes, $_.sha256
    }) -join "`n"
  return [pscustomobject][ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_source_manifest"
    aggregate_algorithm = "sha256(relative_path_tab_bytes_tab_sha256_lf)"
    aggregate_sha256 = Get-StringSha256Hex -Text $canonical
    file_count = $rows.Count
    files = $rows
  }
}

function Write-NewUtf8File {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Text
  )

  $stream = [System.IO.File]::Open(
    $Path,
    [System.IO.FileMode]::CreateNew,
    [System.IO.FileAccess]::Write,
    [System.IO.FileShare]::None
  )
  try {
    $writer = [System.IO.StreamWriter]::new(
      $stream,
      [System.Text.UTF8Encoding]::new($false)
    )
    try {
      $writer.Write($Text)
      $writer.Flush()
      $stream.Flush($true)
    } finally {
      $writer.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
}

function Assert-NoReparseTree {
  param([Parameter(Mandatory = $true)][string]$Root)

  $rootItem = Get-Item -Force -LiteralPath $Root
  if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Refusing recursive cleanup of reparse-point root: $Root"
  }
  foreach ($item in Get-ChildItem -Force -LiteralPath $Root -Recurse) {
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Refusing recursive cleanup of a tree containing a reparse point: $($item.FullName)"
    }
  }
}

function Remove-OwnedSoakIsolation {
  param(
    [Parameter(Mandatory = $true)][string]$IsolationRoot,
    [Parameter(Mandatory = $true)][string]$ExpectedParent,
    [switch]$RequireStoppedOwner
  )

  if (!(Test-Path -LiteralPath $IsolationRoot)) {
    return $false
  }
  $resolvedRoot = [System.IO.Path]::GetFullPath($IsolationRoot)
  $resolvedParent = [System.IO.Path]::GetFullPath($ExpectedParent)
  if (!(Test-PathWithin -Path $resolvedRoot -Parent $resolvedParent)) {
    throw "Refusing cleanup outside the expected run directory: $resolvedRoot"
  }
  $leaf = [System.IO.Path]::GetFileName($resolvedRoot)
  if (!$leaf.StartsWith($isolationPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing cleanup of an unexpected isolation leaf: $resolvedRoot"
  }
  Assert-NoReparseTree -Root $resolvedRoot
  $markerPath = Join-Path $resolvedRoot $isolationMarkerName
  if (!(Test-Path -LiteralPath $markerPath -PathType Leaf)) {
    throw "Refusing cleanup of unmarked soak isolation: $resolvedRoot"
  }
  $marker = Get-Content -Raw -LiteralPath $markerPath | ConvertFrom-Json
  if (
    [int]$marker.schema_version -ne 1 -or
    [string]$marker.kind -ne "vectorgrid_cpu_reliability_soak_isolation" -or
    [string]::IsNullOrWhiteSpace([string]$marker.nonce)
  ) {
    throw "Refusing cleanup of invalid soak ownership marker: $markerPath"
  }
  $ownerPid = 0
  if (![int]::TryParse([string]$marker.pid, [ref]$ownerPid) -or $ownerPid -le 0) {
    throw "Refusing cleanup with invalid soak owner PID: $markerPath"
  }
  if ($RequireStoppedOwner -and (Get-Process -Id $ownerPid -ErrorAction SilentlyContinue)) {
    return $false
  }
  Remove-Item -Force -Recurse -LiteralPath $resolvedRoot
  return $true
}

function Remove-OwnedSoakCargoTarget {
  param(
    [Parameter(Mandatory = $true)][string]$TargetDir,
    [Parameter(Mandatory = $true)][string]$ExpectedParent,
    [string]$ExpectedRunId,
    [switch]$RequireStoppedOwner
  )

  if (!(Test-Path -LiteralPath $TargetDir)) {
    return $false
  }
  $resolvedTarget = [System.IO.Path]::GetFullPath($TargetDir)
  $resolvedParent = [System.IO.Path]::GetFullPath($ExpectedParent)
  if (!(Test-PathWithin -Path $resolvedTarget -Parent $resolvedParent)) {
    throw "Refusing Cargo-target cleanup outside the expected root: $resolvedTarget"
  }
  $leaf = [System.IO.Path]::GetFileName($resolvedTarget)
  if (!$leaf.StartsWith($cargoTargetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing cleanup of an unexpected Cargo-target leaf: $resolvedTarget"
  }
  Assert-NoReparseTree -Root $resolvedTarget
  $markerPath = Join-Path $resolvedTarget $cargoTargetMarkerName
  if (!(Test-Path -LiteralPath $markerPath -PathType Leaf)) {
    throw "Refusing cleanup of unmarked soak Cargo target: $resolvedTarget"
  }
  $marker = Get-Content -Raw -LiteralPath $markerPath | ConvertFrom-Json
  if (
    [int]$marker.schema_version -ne 1 -or
    [string]$marker.kind -ne "vectorgrid_cpu_reliability_soak_cargo_target" -or
    [string]::IsNullOrWhiteSpace([string]$marker.run_id) -or
    [string]::IsNullOrWhiteSpace([string]$marker.nonce)
  ) {
    throw "Refusing cleanup of invalid soak Cargo-target marker: $markerPath"
  }
  if (
    ![string]::IsNullOrWhiteSpace($ExpectedRunId) -and
    [string]$marker.run_id -ne $ExpectedRunId
  ) {
    throw "Refusing cleanup of a Cargo target owned by another run: $resolvedTarget"
  }
  $ownerPid = 0
  if (![int]::TryParse([string]$marker.pid, [ref]$ownerPid) -or $ownerPid -le 0) {
    throw "Refusing Cargo-target cleanup with invalid owner PID: $markerPath"
  }
  if ($RequireStoppedOwner -and (Get-Process -Id $ownerPid -ErrorAction SilentlyContinue)) {
    return $false
  }
  Remove-Item -Force -Recurse -LiteralPath $resolvedTarget
  return $true
}

function Recover-StaleSoakCargoTargets {
  param([Parameter(Mandatory = $true)][string]$CargoTargetRoot)

  if (!(Test-Path -LiteralPath $CargoTargetRoot -PathType Container)) {
    return 0
  }
  $recovered = 0
  foreach ($candidate in Get-ChildItem -Force -LiteralPath $CargoTargetRoot -Directory -Filter "$cargoTargetPrefix*") {
    if (($candidate.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Refusing stale Cargo-target recovery through a reparse point: $($candidate.FullName)"
    }
    $removed = Remove-OwnedSoakCargoTarget `
      -TargetDir $candidate.FullName `
      -ExpectedParent $CargoTargetRoot `
      -RequireStoppedOwner
    if (!$removed) {
      throw "A CPU reliability soak Cargo-target owner is still running: $($candidate.FullName)"
    }
    $recovered += 1
  }
  return $recovered
}

function Recover-StaleSoakIsolations {
  param([Parameter(Mandatory = $true)][string]$ResolvedOutDir)

  if (!(Test-Path -LiteralPath $ResolvedOutDir -PathType Container)) {
    return 0
  }
  $recovered = 0
  foreach ($runDirectory in Get-ChildItem -Force -LiteralPath $ResolvedOutDir -Directory) {
    if (($runDirectory.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Refusing stale-isolation recovery through reparse-point run directory: $($runDirectory.FullName)"
    }
    foreach ($candidate in Get-ChildItem -Force -LiteralPath $runDirectory.FullName -Directory -Filter "$isolationPrefix*") {
      $removed = Remove-OwnedSoakIsolation `
        -IsolationRoot $candidate.FullName `
        -ExpectedParent $runDirectory.FullName `
        -RequireStoppedOwner
      if (!$removed) {
        throw "A CPU reliability soak isolation owner is still running: $($candidate.FullName)"
      }
      $recovered += 1
    }
  }
  return $recovered
}

function Restore-ProcessEnvironment {
  param([Parameter(Mandatory = $true)]$Snapshot)

  foreach ($name in $Snapshot.Keys) {
    $value = $Snapshot[$name]
    if ($null -eq $value) {
      [Environment]::SetEnvironmentVariable($name, $null, "Process")
    } else {
      [Environment]::SetEnvironmentVariable($name, [string]$value, "Process")
    }
  }
}

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$resolvedOutDir = Assert-SafeDirectoryPath `
  -Path (Resolve-RepoPath -Path $OutDir -RepoRoot $repoRoot) `
  -Context "Soak evidence directory"

$targetSource = "repository-default"
$requestedTarget = Join-Path $repoRoot "target"
if ($PSBoundParameters.ContainsKey("CargoTargetDir")) {
  if ([string]::IsNullOrWhiteSpace($CargoTargetDir)) {
    throw "CargoTargetDir must not be empty when explicitly provided"
  }
  $requestedTarget = $CargoTargetDir
  $targetSource = "explicit-parameter"
} elseif (![string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  $requestedTarget = $env:CARGO_TARGET_DIR
  $targetSource = "inherited-environment"
}
$resolvedCargoTargetRoot = Assert-SafeCargoTargetDirectory `
  -Path (Resolve-RepoPath -Path $requestedTarget -RepoRoot $repoRoot) `
  -RepoRoot $repoRoot `
  -ResolvedOutDir $resolvedOutDir
$cargoHomeResolution = Resolve-SoakCargoHome -RepoRoot $repoRoot
$resolvedCargoHome = [string]$cargoHomeResolution.path
$cargoBuildTargetEnv = "host-default"

$runId = "{0}-{1}" -f (Get-Date -Format "yyyyMMdd-HHmmss"), ([guid]::NewGuid().ToString("N").Substring(0, 8))
$resolvedCargoTargetDir = $resolvedCargoTargetRoot
if (!$SkipBuild) {
  $resolvedCargoTargetDir = Assert-SafeCargoTargetDirectory `
    -Path (Join-Path $resolvedCargoTargetRoot "cpu-reliability-soak-$runId") `
    -RepoRoot $repoRoot `
    -ResolvedOutDir $resolvedOutDir
}
$runDir = Join-Path $resolvedOutDir $runId
$isolationRoot = Join-Path $runDir ("$isolationPrefix$runId")
$evidencePath = Join-Path $runDir "cpu-reliability-soak-evidence.json"
$sourceManifestPath = Join-Path $runDir "soak-source-manifest.json"
$launcherManifestPath = Join-Path $runDir "soak-launcher-manifest.json"

$exclusiveModes = @(
  @(
    $PlanOnly,
    $RecoverStaleOnly,
    $SourceManifestOnly,
    $BuildInputProbeOnly,
    $ToolchainProbeOnly
  ) |
    Where-Object { $_ }
)
if ($exclusiveModes.Count -gt 1) {
  throw "PlanOnly, RecoverStaleOnly, SourceManifestOnly, BuildInputProbeOnly, and ToolchainProbeOnly are mutually exclusive"
}

if ($CancelEvery -le 0) {
  throw "CancelEvery must be > 0 because the reliability soak must exercise cancellation"
}
if ($WarmupCycles -lt 0) {
  throw "WarmupCycles must be >= 0"
}

if ($Mode -eq "short") {
  $effectiveCycles = if ($Cycles -gt 0) { $Cycles } else { 4 }
  $effectiveRows = if ($Rows -gt 0) { $Rows } else { 4000 }
  $durationSeconds = 0
  $retainedSamples = 64
  $retainedCycles = 16
  $maxPrivateSlope = 8.0
  $maxPrivateGrowth = 128.0
  $maxWorkingSetSlope = 8.0
  $maxWorkingSetGrowth = 128.0
  $maxHandleSlope = 1.0
  $maxHandleGrowth = 8
} else {
  Assert-PositiveFinite -Value $Hours -Name "Hours"
  if ($Hours -lt 6.0) {
    throw "Operator authority requires Hours >= 6"
  }
  if ($Cycles -ne 0) {
    throw "Operator authority does not allow an explicit Cycles limit; duration must terminate the run"
  }
  if ($SkipBuild) {
    throw "Operator authority requires a fresh build and does not allow SkipBuild"
  }
  if ($BuildProfile -ne "release") {
    throw "Operator authority requires BuildProfile=release"
  }
  if ($CancelEvery -ne 2) {
    throw "Operator authority requires CancelEvery=2"
  }
  if ($WarmupCycles -ne 1) {
    throw "Operator authority requires WarmupCycles=1"
  }
  if ($Rows -gt 0 -and $Rows -lt 20000) {
    throw "Operator authority requires Rows >= 20000"
  }
  $durationValue = [Math]::Ceiling($Hours * 3600.0)
  if ($durationValue -gt [int]::MaxValue) {
    throw "Hours is too large"
  }
  $effectiveCycles = 1000000
  $effectiveRows = if ($Rows -gt 0) { $Rows } else { 20000 }
  $durationSeconds = [int]$durationValue
  $retainedSamples = 256
  $retainedCycles = 64
  $maxPrivateSlope = 0.5
  $maxPrivateGrowth = 128.0
  $maxWorkingSetSlope = 0.5
  $maxWorkingSetGrowth = 128.0
  $maxHandleSlope = 0.10
  $maxHandleGrowth = 8
}

if ($effectiveCycles -lt 4) {
  throw "Cycles must be >= 4 so the post-warmup slope check has meaningful samples"
}
if ($effectiveRows -lt 2048) {
  throw "Rows must be >= 2048 so the cancellation workload spans multiple CPU tiles"
}
if ($WarmupCycles -ge ($effectiveCycles - 2)) {
  throw "WarmupCycles must leave at least three measured cycles"
}
if ($CancelEvery -gt $effectiveCycles) {
  throw "CancelEvery must schedule at least one cancellation cycle"
}

$exeName = if ($env:OS -eq "Windows_NT") {
  "ta_cpu_reliability_soak.exe"
} else {
  "ta_cpu_reliability_soak"
}
$profileDir = if ($BuildProfile -eq "release") { "release" } else { "debug" }
$binaryProfileRoot = $resolvedCargoTargetDir
if (
  $cargoBuildTargetEnv -ne "host-default" -and
  $cargoBuildTargetEnv -match '^[A-Za-z0-9_.-]+$'
) {
  $binaryProfileRoot = Join-Path $resolvedCargoTargetDir $cargoBuildTargetEnv
}
$binaryPath = Join-Path (Join-Path $binaryProfileRoot $profileDir) $exeName
if ($SkipBuild) {
  if ([string]::IsNullOrWhiteSpace($PrebuiltBinaryPath)) {
    throw "SkipBuild requires an explicit PrebuiltBinaryPath"
  }
  $binaryPath = Resolve-RepoPath -Path $PrebuiltBinaryPath -RepoRoot $repoRoot
  if (!(Test-PathWithin -Path $binaryPath -Parent $resolvedCargoTargetDir)) {
    throw "PrebuiltBinaryPath must be inside the validated Cargo target directory"
  }
  Assert-NoReparseAncestors -Path $binaryPath -Context "Prebuilt CPU reliability soak binary"
} elseif ($PSBoundParameters.ContainsKey("PrebuiltBinaryPath")) {
  throw "PrebuiltBinaryPath is only valid with SkipBuild"
}

if ($SourceManifestOnly) {
  $sourceManifest = Get-SoakSourceManifest -RepoRoot $repoRoot
  [pscustomobject][ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_source_manifest_check"
    status = "passed"
    file_count = $sourceManifest.file_count
    aggregate_sha256 = $sourceManifest.aggregate_sha256
    includes_rust_toolchain = @($sourceManifest.files.path) -contains "rust-toolchain.toml"
    includes_cargo_config = @($sourceManifest.files.path | Where-Object {
        $_.StartsWith(".cargo/", [StringComparison]::Ordinal)
      }).Count -gt 0
    includes_windows_tauri_config =
      @($sourceManifest.files.path) -contains "src-tauri/tauri.windows.conf.json"
    includes_tauri_resources = @($sourceManifest.files.path | Where-Object {
        $_.StartsWith("src-tauri/resources/", [StringComparison]::Ordinal)
      }).Count -gt 0
    includes_vendored_embed_resource = @($sourceManifest.files.path | Where-Object {
        $_.StartsWith("vendor/embed-resource-3.0.6/", [StringComparison]::Ordinal)
      }).Count -gt 0
    frontend_dist_effective = $false
    context_asset_mode = "dev_url_no_embedded_frontend_assets"
    includes_vendored_vector_ta = @($sourceManifest.files.path | Where-Object {
        $_.StartsWith("vendor/vector-ta-0.2.4/", [StringComparison]::Ordinal)
      }).Count -gt 0
  } | ConvertTo-Json -Depth 4
  return
}

if ($BuildInputProbeOnly) {
  $cargoConfiguration = Get-SoakCargoConfigurationProvenance `
    -RepoRoot $repoRoot `
    -CargoHome $resolvedCargoHome
  [pscustomobject][ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_build_input_probe"
    status = "passed"
    repo_root = $repoRoot
    cargo_home_source = [string]$cargoHomeResolution.source
    cargo_configuration = $cargoConfiguration
    release_authority_eligible =
      [bool]$cargoConfiguration.release_authority_eligible
    tauri_config_override_present_before_isolation =
      ![string]::IsNullOrWhiteSpace(
        [Environment]::GetEnvironmentVariable("TAURI_CONFIG", "Process")
      )
    tauri_target_override_present_before_isolation =
      ![string]::IsNullOrWhiteSpace(
        [Environment]::GetEnvironmentVariable("TAURI_ENV_TARGET_TRIPLE", "Process")
      )
    tauri_overrides_will_be_cleared_for_build_and_runtime = $true
    cargo_configuration_environment_will_be_cleared_except_allowlist = $true
  } | ConvertTo-Json -Depth 8
  return
}

if ($ToolchainProbeOnly) {
  $toolchain = Get-SoakToolchainProvenance -RepoRoot $repoRoot -IncludeInputManifest
  [pscustomobject][ordered]@{
    schema_version = 1
    kind = "vectorgrid_cpu_reliability_soak_toolchain_probe"
    status = "passed"
    repo_root = $repoRoot
    toolchain = $toolchain
  } | ConvertTo-Json -Depth 5
  return
}

if ($PlanOnly) {
  $planTargetTriple = $null
  $planBinaryPath = $binaryPath
  if (!$SkipBuild) {
    $planToolchain = Get-SoakToolchainProvenance -RepoRoot $repoRoot
    $planTargetTriple = [string]$planToolchain.host_target_triple
    $planBinaryPath = Join-Path `
      (Join-Path (Join-Path $resolvedCargoTargetDir $planTargetTriple) $profileDir) `
      $exeName
  }
  [pscustomobject][ordered]@{
    schema_version = 2
    mode = $Mode
    build_profile = $BuildProfile
    fresh_build_required = $Mode -eq "operator"
    skip_build = [bool]$SkipBuild
    operator_release_authority_requested = $Mode -eq "operator"
    minimum_operator_duration_seconds = $minimumOperatorSeconds
    minimum_operator_measured_cycles = $minimumOperatorMeasuredCycles
    maximum_operator_sample_gap_ms = $maximumOperatorSampleGapMs
    cycles = $effectiveCycles
    duration_seconds = $durationSeconds
    rows = $effectiveRows
    cancel_every = $CancelEvery
    warmup_cycles = $WarmupCycles
    cargo_target_dir = $resolvedCargoTargetDir
    cargo_target_root = $resolvedCargoTargetRoot
    cargo_target_source = $targetSource
    cargo_build_target_env = $cargoBuildTargetEnv
    cargo_target_triple = $planTargetTriple
    cargo_home = $resolvedCargoHome
    binary_path_hint = $planBinaryPath
    evidence_path = $evidencePath
    isolation_root = $isolationRoot
    synthetic_data_only = $true
    provider_credentials_used = $false
    provider_network_calls = $false
    computer_use = $false
  } | ConvertTo-Json -Depth 5
  return
}

$benchmarkLockScript = Join-Path $PSScriptRoot "benchmark_lock.ps1"
if (!(Test-Path -LiteralPath $benchmarkLockScript -PathType Leaf)) {
  throw "Benchmark lock helper was not found: $benchmarkLockScript"
}
. $benchmarkLockScript

if ($RecoverStaleOnly) {
  $recoveryLock = $null
  try {
    $recoveryLock = Enter-BenchmarkLock -Context "CPU reliability soak stale-isolation recovery"
    $recovered = Recover-StaleSoakIsolations -ResolvedOutDir $resolvedOutDir
    $recoveredTargets = Recover-StaleSoakCargoTargets `
      -CargoTargetRoot $resolvedCargoTargetRoot
    [pscustomobject][ordered]@{
      schema_version = 1
      kind = "vectorgrid_cpu_reliability_soak_recovery"
      status = "passed"
      recovered_stale_isolations = $recovered
      recovered_stale_cargo_targets = $recoveredTargets
      evidence_root = $resolvedOutDir
    } | ConvertTo-Json -Depth 4
  } finally {
    Exit-BenchmarkLock -Handle $recoveryLock
  }
  return
}

$credentialEnvNames = @(
  "ALPACA_API_KEY",
  "ALPACA_API_KEY_ID",
  "ALPACA_API_SECRET",
  "ALPACA_API_SECRET_KEY",
  "ALPACA_KEY",
  "ALPACA_KEY_ID",
  "ALPACA_SECRET",
  "ALPACA_SECRET_KEY",
  "ALPACA_API_BASE_URL",
  "APCA_API_KEY",
  "APCA_API_KEY_ID",
  "APCA_API_SECRET",
  "APCA_API_SECRET_KEY",
  "APCA_API_BASE_URL",
  "POLYGON_KEY",
  "POLYGON_API_KEY",
  "POLYGON_TOKEN",
  "POLYGON_API_BASE_URL"
)
$networkEnvNames = @(
  "HTTP_PROXY",
  "HTTPS_PROXY",
  "ALL_PROXY",
  "NO_PROXY",
  "http_proxy",
  "https_proxy",
  "all_proxy",
  "no_proxy",
  "CARGO_NET_OFFLINE",
  "CARGO_TERM_COLOR"
)
$ambientCargoEnvNames = @(
  Get-ChildItem Env: | Where-Object {
    $_.Name -match '^CARGO_'
  } | ForEach-Object { $_.Name }
)
$ambientTauriEnvNames = @(
  Get-ChildItem Env: | Where-Object {
    $_.Name -match '^TAURI_'
  } | ForEach-Object { $_.Name }
)
$nativeBuildOverrideEnvNames = @(Get-SoakNativeBuildOverrideEnvironmentNames)
$managedEnvNames = @(
  $credentialEnvNames +
  $networkEnvNames +
    $ambientCargoEnvNames +
    $ambientTauriEnvNames +
    $nativeBuildOverrideEnvNames +
    @(
    "CARGO_HOME",
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_BUILD_DIR",
    "CARGO_BUILD_TARGET",
    "CARGO_INCREMENTAL",
    "CARGO_BUILD_INCREMENTAL",
    "RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTFLAGS",
    "RUSTC",
    "RUSTC_WORKSPACE_WRAPPER",
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "RUSTUP_TOOLCHAIN",
    "RUSTC_BOOTSTRAP",
    "RUSTC_STAGE",
    "TAURI_CONFIG",
    "TAURI_ENV_TARGET_TRIPLE",
      "SCCACHE_NO_DAEMON",
      "PATH",
      "TEMP",
    "TMP",
    "VECTORBT_CPU_RAM_BUDGET_MB",
    "VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE"
  )
)
$targetSpecificBuildEnvNames = @(
  Get-ChildItem Env: | Where-Object {
    $_.Name -match '^CARGO_TARGET_.+_(RUSTFLAGS|LINKER)$'
  } | ForEach-Object { $_.Name }
)
$profileBuildEnvNames = @(
  Get-ChildItem Env: | Where-Object {
    $_.Name -match '^CARGO_PROFILE_.+$'
  } | ForEach-Object { $_.Name }
)
$managedEnvNames = @(
  ($managedEnvNames + $targetSpecificBuildEnvNames + $profileBuildEnvNames) |
    Sort-Object -Unique
)
$environmentSnapshot = [System.Collections.Generic.Dictionary[string, object]]::new(
  [StringComparer]::Ordinal
)
foreach ($name in $managedEnvNames) {
  $environmentSnapshot[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

$benchmarkLock = $null
$recoveredStaleIsolations = 0
$recoveredStaleCargoTargets = 0
$binarySha256 = $null
$sourceManifestSha256 = $null
$launcherManifestSha256 = $null
$launchStartedUtc = $null
$completedSuccessfully = $false
$cargoArtifactEmitted = $false
$cargoArtifactFresh = $false
$cargoTargetInitiallyEmpty = $false
$buildArgs = @()
$cargoTargetTriple = $null
$cargoConfiguration = $null
$nativeBuildProvenance = $null
$nativeBuildProvenanceBefore = $null
$resourceCompilerAttestationNonce = [guid]::NewGuid().ToString("N")
$pinnedNativeEnvironmentNames = [string[]]@()
$runtimeClearedPinnedNativeEnvironmentNames = [string[]]@()
$resourceCompilerLease = $null
$resourceCompilerInvocation = $null
$retainedBinaryPath = Join-Path $runDir $exeName
$retainedBinarySha256 = $null

try {
  $benchmarkLock = Enter-BenchmarkLock -Context "CPU reliability soak build and run"
  $recoveredStaleIsolations = Recover-StaleSoakIsolations -ResolvedOutDir $resolvedOutDir
  $recoveredStaleCargoTargets = Recover-StaleSoakCargoTargets `
    -CargoTargetRoot $resolvedCargoTargetRoot

  if (Test-Path -LiteralPath $runDir) {
    throw "Fresh soak run directory already exists: $runDir"
  }
  [System.IO.Directory]::CreateDirectory($runDir) | Out-Null
  if (!$SkipBuild -and (Test-Path -LiteralPath $resolvedCargoTargetDir)) {
    throw "Fresh soak Cargo target unexpectedly already exists: $resolvedCargoTargetDir"
  }
  $cargoTargetInitiallyEmpty = !(Test-Path -LiteralPath $resolvedCargoTargetDir)
  [System.IO.Directory]::CreateDirectory($resolvedCargoTargetDir) | Out-Null
  if (!$SkipBuild) {
    $cargoTargetMarker = [ordered]@{
      schema_version = 1
      kind = "vectorgrid_cpu_reliability_soak_cargo_target"
      run_id = $runId
      pid = $PID
      nonce = [guid]::NewGuid().ToString("N")
    }
    Write-NewUtf8File `
      -Path (Join-Path $resolvedCargoTargetDir $cargoTargetMarkerName) `
      -Text ($cargoTargetMarker | ConvertTo-Json -Compress)
  }
  $resolvedCargoTargetDir = Assert-SafeCargoTargetDirectory `
    -Path $resolvedCargoTargetDir `
    -RepoRoot $repoRoot `
    -ResolvedOutDir $resolvedOutDir
  $buildTempDir = Join-Path $resolvedCargoTargetDir "_cpu-reliability-soak-temp"
  [System.IO.Directory]::CreateDirectory($buildTempDir) | Out-Null
  Assert-NoReparseAncestors -Path $buildTempDir -Context "CPU reliability soak build temp"

  foreach ($name in $credentialEnvNames) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }
  foreach ($name in @($ambientCargoEnvNames + $ambientTauriEnvNames)) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }
  foreach ($name in $nativeBuildOverrideEnvNames) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }
  $remainingCredentials = @(
    $credentialEnvNames | Where-Object {
      $null -ne [Environment]::GetEnvironmentVariable($_, "Process")
    }
  )
  if ($remainingCredentials.Count -gt 0) {
    throw "Credential-free soak preflight failed to clear: $($remainingCredentials -join ', ')"
  }
  $env:CARGO_TARGET_DIR = $resolvedCargoTargetDir
  $env:CARGO_HOME = $resolvedCargoHome
  Remove-Item Env:CARGO_BUILD_TARGET_DIR, Env:CARGO_BUILD_BUILD_DIR -ErrorAction SilentlyContinue
  $env:CARGO_INCREMENTAL = "0"
  $env:CARGO_BUILD_INCREMENTAL = "false"
  $env:TEMP = $buildTempDir
  $env:TMP = $buildTempDir
  $env:CARGO_NET_OFFLINE = "true"
  $env:CARGO_TERM_COLOR = "never"
  $env:HTTP_PROXY = "http://127.0.0.1:9"
  $env:HTTPS_PROXY = "http://127.0.0.1:9"
  $env:ALL_PROXY = "http://127.0.0.1:9"
  $env:NO_PROXY = "localhost,127.0.0.1,::1"
  $env:http_proxy = $env:HTTP_PROXY
  $env:https_proxy = $env:HTTPS_PROXY
  $env:all_proxy = $env:ALL_PROXY
  $env:no_proxy = $env:NO_PROXY
  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:RUSTC_WORKSPACE_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER = ""
  $env:RUSTFLAGS = ""
  $env:CARGO_BUILD_RUSTFLAGS = ""
  $env:CARGO_ENCODED_RUSTFLAGS = ""
  foreach ($name in @(
      "RUSTC",
      "CARGO_BUILD_RUSTC",
      "RUSTUP_TOOLCHAIN",
      "RUSTC_BOOTSTRAP",
      "RUSTC_STAGE"
    )) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }
  Remove-Item Env:TAURI_CONFIG, Env:TAURI_ENV_TARGET_TRIPLE, Env:CARGO_BUILD_TARGET `
    -ErrorAction SilentlyContinue
  foreach ($name in $targetSpecificBuildEnvNames) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }
  foreach ($name in $profileBuildEnvNames) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }
  $env:SCCACHE_NO_DAEMON = "1"
  $env:VECTORBT_CPU_RAM_BUDGET_MB = "64"
  $env:VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE = "1"

  $toolchainProvenance = Get-SoakToolchainProvenance `
    -RepoRoot $repoRoot `
    -IncludeInputManifest `
    -ProbeSelectedExecutables
  $cargoTargetTriple = [string]$toolchainProvenance.host_target_triple
  $cargoExecutablePath = [string]$toolchainProvenance.cargo_path
  $rustcToolchainPath = [string]$toolchainProvenance.rustc_path
  if (!$SkipBuild) {
    $nativeBuildProvenanceBefore = Get-SoakNativeBuildProvenance `
      -HostTargetTriple $cargoTargetTriple `
      -RepoRoot $repoRoot `
      -CargoHome $resolvedCargoHome `
      -CargoExecutablePath $cargoExecutablePath `
      -ResourceCompilerAttestationNonce $resourceCompilerAttestationNonce `
      -ConfigurePinnedEnvironment
    $pinnedNativeEnvironmentNames = [string[]]@(
      $nativeBuildProvenanceBefore.environment | ForEach-Object {
        [string]$_.name
      }
    )
    $pinnedPathEnvironmentNames = @(
      $pinnedNativeEnvironmentNames | Where-Object {
        [string]::Equals([string]$_, "PATH", [StringComparison]::OrdinalIgnoreCase)
      }
    )
    if ($pinnedPathEnvironmentNames.Count -ne 1) {
      throw "Pinned native environment must contain exactly one PATH entry"
    }
    $runtimeClearedPinnedNativeEnvironmentNames = [string[]]@(
      $pinnedNativeEnvironmentNames | Where-Object {
        ![string]::Equals([string]$_, "PATH", [StringComparison]::OrdinalIgnoreCase)
      }
    )
    $leasePath = [string]$nativeBuildProvenanceBefore.resource_compiler_execution.path
    $resourceCompilerLease = [System.IO.File]::Open(
      $leasePath,
      [System.IO.FileMode]::Open,
      [System.IO.FileAccess]::Read,
      [System.IO.FileShare]::Read
    )
    if (
      [long]$resourceCompilerLease.Length -ne
        [long]$nativeBuildProvenanceBefore.resource_compiler_execution.bytes -or
      (Get-Sha256Hex -Path $leasePath) -ne
        [string]$nativeBuildProvenanceBefore.resource_compiler_execution.sha256
    ) {
      throw "Pinned rc.exe changed between attestation and the deny-write/delete lease"
    }
  }
  Push-Location $repoRoot
  try {
    $cargoConfigurationBefore = Get-SoakCargoConfigurationProvenance `
      -RepoRoot $repoRoot `
      -CargoHome $resolvedCargoHome
    if (![bool]$cargoConfigurationBefore.release_authority_eligible) {
      throw "Release-authority soak refuses Cargo configuration outside the tracked workspace .cargo/config.toml"
    }
    $sourceManifestBefore = Get-SoakSourceManifest -RepoRoot $repoRoot
    if (!$SkipBuild) {
      $buildArgs = @(
        "build",
        "--locked",
        "--offline",
        "--target-dir", $resolvedCargoTargetDir,
        "--target", $cargoTargetTriple,
        "-p", "ta_desktop_demo_app",
        "--no-default-features",
        "--features", "maintainer-tools",
        "--bin", "ta_cpu_reliability_soak"
      )
      if ($BuildProfile -eq "release") {
        $buildArgs += "--release"
      }
      $buildArgs += "--message-format=json-render-diagnostics"
      # Pin Cargo's compiler path while empty wrapper/rustflag variables
      # override any ambient Cargo configuration.
      $env:RUSTC = $rustcToolchainPath
      $env:CARGO_BUILD_RUSTC = $rustcToolchainPath
      $cargoOutput = @(& $cargoExecutablePath @buildArgs)
      $cargoExitCode = $LASTEXITCODE
      Remove-Item Env:RUSTC, Env:CARGO_BUILD_RUSTC -ErrorAction SilentlyContinue
      $artifactCandidates = @(
        foreach ($line in $cargoOutput) {
          $message = $null
          try {
            $message = [string]$line | ConvertFrom-Json
          } catch {
            Write-Host ([string]$line)
            continue
          }
          if (
            [string]$message.reason -eq "compiler-artifact" -and
            [string]$message.package_id -eq
              [string]$nativeBuildProvenanceBefore.cargo_resolution.root_package.package_id -and
            [string]$message.target.name -eq "ta_cpu_reliability_soak" -and
            @($message.target.kind) -contains "bin" -and
            ![string]::IsNullOrWhiteSpace([string]$message.executable)
          ) {
            $message
          }
        }
      )
      if ($cargoExitCode -ne 0) {
        foreach ($line in $cargoOutput) {
          Write-Host ([string]$line)
        }
        throw "Failed to build CPU reliability soak harness (exit code $cargoExitCode)"
      }
      if ($artifactCandidates.Count -ne 1) {
        throw "Cargo did not emit exactly one ta_cpu_reliability_soak executable artifact"
      }
      $buildScriptEvents = @(
        foreach ($line in $cargoOutput) {
          try {
            $message = [string]$line | ConvertFrom-Json
          } catch {
            continue
          }
          if ([string]$message.reason -eq "build-script-executed") {
            $message
          }
        }
      )
      $rootPackageId =
        [string]$nativeBuildProvenanceBefore.cargo_resolution.root_package.package_id
      $rootBuildScriptEvents = @($buildScriptEvents | Where-Object {
          [string]$_.package_id -eq $rootPackageId
        })
      $attestationEnvironmentName =
        "VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION"
      $markerEvents = @(
        foreach ($event in $buildScriptEvents) {
          foreach ($pair in @($event.env)) {
            if (
              @($pair).Count -eq 2 -and
              [string]$pair[0] -eq $attestationEnvironmentName
            ) {
              [pscustomobject]@{
                package_id = [string]$event.package_id
                value = [string]$pair[1]
              }
            }
          }
        }
      )
      if (
        $rootBuildScriptEvents.Count -ne 1 -or
        $markerEvents.Count -ne 1 -or
        [string]$markerEvents[0].package_id -ne $rootPackageId
      ) {
        throw "Cargo did not emit one success-attested resource-compiler marker on the exact app build script"
      }
      $expectedResourceCompilerPath =
        [string]$nativeBuildProvenanceBefore.resource_compiler_execution.path
      $expectedMarkerValue =
        "$resourceCompilerAttestationNonce|$expectedResourceCompilerPath"
      if ([string]$markerEvents[0].value -ne $expectedMarkerValue) {
        throw "Cargo app build-script resource-compiler marker differs from the pinned nonce/path"
      }
      $buildScriptOutDir = [System.IO.Path]::GetFullPath(
        [string]$rootBuildScriptEvents[0].out_dir
      )
      if (
        !(Test-Path -LiteralPath $buildScriptOutDir -PathType Container) -or
        !(Test-PathWithin -Path $buildScriptOutDir -Parent $resolvedCargoTargetDir)
      ) {
        throw "Attested app build-script output escaped the fresh Cargo target"
      }
      Assert-NoReparseAncestors `
        -Path $buildScriptOutDir `
        -Context "Attested app build-script output"
      $resourceCompilerInvocation = [pscustomobject][ordered]@{
        schema_version = 1
        kind = "vectorgrid_cpu_reliability_soak_resource_compiler_invocation"
        build_script_event_match_count = 1
        marker_match_count = 1
        package_id = $rootPackageId
        package_name = "ta_desktop_demo_app"
        target_triple = $cargoTargetTriple
        build_script_out_dir = $buildScriptOutDir
        environment_name = $attestationEnvironmentName
        nonce = $resourceCompilerAttestationNonce
        resource_compiler_path = $expectedResourceCompilerPath
        resource_compiler_bytes =
          [long]$nativeBuildProvenanceBefore.resource_compiler_execution.bytes
        resource_compiler_sha256 =
          [string]$nativeBuildProvenanceBefore.resource_compiler_execution.sha256
        marker_value_sha256 = Get-StringSha256Hex -Text $expectedMarkerValue
        file_lock_held_during_build = $null -ne $resourceCompilerLease
        file_lock_share_mode =
          "read_access_share_read_only_denies_write_delete"
      }
      $binaryPath = [System.IO.Path]::GetFullPath([string]$artifactCandidates[0].executable)
      $cargoArtifactFresh = [bool]$artifactCandidates[0].fresh
      $cargoArtifactEmitted = $true
      if ($cargoArtifactFresh) {
        throw "Fresh isolated Cargo target unexpectedly reused a cached soak artifact"
      }
    }
    $cargoConfiguration = Get-SoakCargoConfigurationProvenance `
      -RepoRoot $repoRoot `
      -CargoHome $resolvedCargoHome
    if (
      [string]$cargoConfiguration.aggregate_sha256 -ne
        [string]$cargoConfigurationBefore.aggregate_sha256
    ) {
      throw "Effective Cargo configuration changed during the build"
    }
    if (!$SkipBuild) {
      $nativeBuildProvenance = Get-SoakNativeBuildProvenance `
        -HostTargetTriple $cargoTargetTriple `
        -RepoRoot $repoRoot `
        -CargoHome $resolvedCargoHome `
        -CargoExecutablePath $cargoExecutablePath `
        -ResourceCompilerAttestationNonce $resourceCompilerAttestationNonce `
        -PinnedEnvironmentNames $pinnedNativeEnvironmentNames `
        -ResourceCompilerInvocation $resourceCompilerInvocation
      if (
        [string]$nativeBuildProvenance.cargo_resolution.aggregate_sha256 -ne
          [string]$nativeBuildProvenanceBefore.cargo_resolution.aggregate_sha256
      ) {
        throw "Locked/offline Cargo metadata resolution changed during the build"
      }
      if (
        [string]$nativeBuildProvenance.pre_execution_aggregate_sha256 -ne
          [string]$nativeBuildProvenanceBefore.pre_execution_aggregate_sha256
      ) {
        throw "Native compiler/linker/Windows SDK inputs changed during the build"
      }
      if ($null -eq $resourceCompilerLease) {
        throw "Pinned rc.exe deny-write/delete lease was lost before build completion"
      }
      $resourceCompilerLease.Dispose()
      $resourceCompilerLease = $null
      foreach ($name in @(
          $nativeBuildOverrideEnvNames + $pinnedNativeEnvironmentNames |
            Select-Object -Unique
        )) {
        Remove-Item "Env:$name" -ErrorAction SilentlyContinue
      }
      [Environment]::SetEnvironmentVariable(
        "PATH",
        $environmentSnapshot["PATH"],
        "Process"
      )
      if (
        ![string]::Equals(
          [string][Environment]::GetEnvironmentVariable("PATH", "Process"),
          [string]$environmentSnapshot["PATH"],
          [StringComparison]::Ordinal
        )
      ) {
        throw "Ambient PATH was not restored exactly after the native build"
      }
    }
    $sourceManifest = Get-SoakSourceManifest -RepoRoot $repoRoot
    if (
      !$SkipBuild -and
      [string]$sourceManifest.aggregate_sha256 -ne [string]$sourceManifestBefore.aggregate_sha256
    ) {
      throw "Soak source inputs changed during the build; refusing to run an unattested binary"
    }

    $sourceManifestJson = $sourceManifest | ConvertTo-Json -Depth 8
    Write-NewUtf8File -Path $sourceManifestPath -Text $sourceManifestJson
    $sourceManifestSha256 = Get-Sha256Hex -Path $sourceManifestPath

    if (!(Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
      throw "CPU reliability soak binary was not found: $binaryPath"
    }
    if (!(Test-PathWithin -Path $binaryPath -Parent $resolvedCargoTargetDir)) {
      throw "Cargo-emitted soak binary escaped the validated target directory: $binaryPath"
    }
    Assert-NoReparseAncestors -Path $binaryPath -Context "CPU reliability soak binary"
    $binarySha256 = Get-Sha256Hex -Path $binaryPath
    if (Test-Path -LiteralPath $retainedBinaryPath) {
      throw "Fresh retained soak binary path unexpectedly exists: $retainedBinaryPath"
    }
    Copy-Item -LiteralPath $binaryPath -Destination $retainedBinaryPath
    Assert-NoReparseAncestors -Path $retainedBinaryPath -Context "Retained CPU reliability soak binary"
    $retainedBinarySha256 = Get-Sha256Hex -Path $retainedBinaryPath
    if ($retainedBinarySha256 -ne $binarySha256) {
      throw "Retained soak binary hash differs from the Cargo-emitted executable"
    }
    $compilerOverrideEnvNames = @(
      "RUSTC_WRAPPER",
      "CARGO_BUILD_RUSTC_WRAPPER",
      "RUSTC_WORKSPACE_WRAPPER",
      "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
      "RUSTC",
      "CARGO_BUILD_RUSTC",
      "RUSTFLAGS",
      "CARGO_BUILD_RUSTFLAGS",
      "CARGO_ENCODED_RUSTFLAGS",
      "RUSTUP_TOOLCHAIN",
      "RUSTC_BOOTSTRAP",
      "RUSTC_STAGE"
    ) + $targetSpecificBuildEnvNames + $nativeBuildOverrideEnvNames +
      $runtimeClearedPinnedNativeEnvironmentNames
    $remainingCompilerOverrides = @(
      $compilerOverrideEnvNames | Where-Object {
        $value = [Environment]::GetEnvironmentVariable($_, "Process")
        $null -ne $value -and ![string]::IsNullOrWhiteSpace($value)
      }
    )
    if ($remainingCompilerOverrides.Count -gt 0) {
      throw "Compiler override environment was not cleared: $($remainingCompilerOverrides -join ', ')"
    }
    $allowedCargoEnvironment = @(
      "CARGO_HOME",
      "CARGO_TARGET_DIR",
      "CARGO_INCREMENTAL",
      "CARGO_BUILD_INCREMENTAL",
      "CARGO_NET_OFFLINE",
      "CARGO_TERM_COLOR"
    )
    $unexpectedCargoEnvironment = @(
      Get-ChildItem Env: | Where-Object {
        $_.Name -match '^CARGO_' -and
        $allowedCargoEnvironment -notcontains $_.Name -and
        ![string]::IsNullOrWhiteSpace([string]$_.Value)
      } | ForEach-Object { $_.Name }
    )
    $unexpectedTauriEnvironment = @(
      Get-ChildItem Env: | Where-Object {
        $_.Name -match '^TAURI_' -and
        ![string]::IsNullOrWhiteSpace([string]$_.Value)
      } | ForEach-Object { $_.Name }
    )
    if ($unexpectedCargoEnvironment.Count -gt 0) {
      throw "Unexpected Cargo configuration environment remained: $($unexpectedCargoEnvironment -join ', ')"
    }
    if ($unexpectedTauriEnvironment.Count -gt 0) {
      throw "Unexpected Tauri build environment remained: $($unexpectedTauriEnvironment -join ', ')"
    }
    $launcherFeatures = [object[]]@()
    if (!$SkipBuild) {
      $launcherFeatures = [object[]]@("maintainer-tools")
    }
    $launcherManifest = [ordered]@{
      schema_version = 1
      kind = "vectorgrid_cpu_reliability_soak_launcher_manifest"
      run_id = $runId
      build = [ordered]@{
        profile = $BuildProfile
        fresh_build_invocation = -not [bool]$SkipBuild
        cargo_artifact_emitted = $cargoArtifactEmitted
        cargo_artifact_fresh = $cargoArtifactFresh
        cargo_build_target_env = $cargoBuildTargetEnv
        cargo_target_triple = $cargoTargetTriple
        cargo_target_dir = $resolvedCargoTargetDir
        cargo_target_root = $resolvedCargoTargetRoot
        cargo_target_dir_cli_pinned = -not [bool]$SkipBuild
        cargo_target_triple_cli_pinned = -not [bool]$SkipBuild
        target_dir_initially_empty = $cargoTargetInitiallyEmpty
        recovered_stale_cargo_targets = $recoveredStaleCargoTargets
        target_cleanup_marker = $cargoTargetMarkerName
        cargo_artifact_path = $binaryPath
        build_temp_dir = $buildTempDir
        cargo_arguments = @($buildArgs)
        no_default_features = -not [bool]$SkipBuild
        features = $launcherFeatures
        locked = -not [bool]$SkipBuild
        offline = -not [bool]$SkipBuild
        compiler_override_environment_cleared = $true
        native_build_override_environment_cleared = $true
        native_build_environment_attested = -not [bool]$SkipBuild
        compiler_path_pinned_during_build = -not [bool]$SkipBuild
        target_linker_path_pinned_during_build = -not [bool]$SkipBuild
        cargo_execution_uses_toolchain_binary = -not [bool]$SkipBuild
        profile_override_environment_cleared = $true
        incremental_compilation_disabled = $true
        tauri_config_environment_cleared = $true
        tauri_target_environment_cleared = $true
        custom_protocol_disabled = $true
        context_asset_mode = "dev_url_no_embedded_frontend_assets"
        frontend_dist_tree_effective = $false
        cargo_configuration_environment_cleared_except_allowlist = $true
      }
      cargo_configuration = $cargoConfiguration
      binary = [ordered]@{
        path = $binaryPath
        retained_path = $retainedBinaryPath
        sha256 = $binarySha256
      }
      source_manifest = [ordered]@{
        path = $sourceManifestPath
        sha256 = $sourceManifestSha256
        aggregate_sha256 = [string]$sourceManifest.aggregate_sha256
        file_count = [int]$sourceManifest.file_count
      }
      toolchain = $toolchainProvenance
      native_build = $nativeBuildProvenance
      host = [ordered]@{
        os_version = [Environment]::OSVersion.VersionString
        os_environment = [string]$env:OS
        processor_architecture = [string]$env:PROCESSOR_ARCHITECTURE
        processor_identifier = [string]$env:PROCESSOR_IDENTIFIER
        logical_processor_count = [Environment]::ProcessorCount
        process_is_64_bit = [Environment]::Is64BitProcess
      }
    }
    Write-NewUtf8File `
      -Path $launcherManifestPath `
      -Text ($launcherManifest | ConvertTo-Json -Depth 8)
    $launcherManifestSha256 = Get-Sha256Hex -Path $launcherManifestPath
    if (Test-Path -LiteralPath $evidencePath) {
      throw "Fresh evidence path unexpectedly already exists: $evidencePath"
    }

    $arguments = @(
      "--out", $evidencePath,
      "--isolation-root", $isolationRoot,
      "--run-id", $runId,
      "--cargo-target-dir", $resolvedCargoTargetDir,
      "--build-profile", $BuildProfile,
      "--cargo-build-target-env", $cargoBuildTargetEnv,
      "--cargo-target-triple", $cargoTargetTriple,
      "--cargo-artifact-path", $binaryPath,
      "--cargo-artifact-emitted", $cargoArtifactEmitted.ToString().ToLowerInvariant(),
      "--cargo-artifact-fresh", $cargoArtifactFresh.ToString().ToLowerInvariant(),
      "--binary-sha256", $binarySha256,
      "--source-manifest", $sourceManifestPath,
      "--source-manifest-sha256", $sourceManifestSha256,
      "--launcher-manifest", $launcherManifestPath,
      "--launcher-manifest-sha256", $launcherManifestSha256,
      "--recovered-stale-isolations", "$recoveredStaleIsolations",
      "--mode-label", $Mode,
      "--cycles", "$effectiveCycles",
      "--duration-seconds", "$durationSeconds",
      "--cancel-every", "$CancelEvery",
      "--rows", "$effectiveRows",
      "--warmup-cycles", "$WarmupCycles",
      "--retained-samples", "$retainedSamples",
      "--retained-cycles", "$retainedCycles",
      "--max-private-slope-mib-per-cycle", "$maxPrivateSlope",
      "--max-private-growth-mib", "$maxPrivateGrowth",
      "--max-working-set-slope-mib-per-cycle", "$maxWorkingSetSlope",
      "--max-working-set-growth-mib", "$maxWorkingSetGrowth",
      "--max-handle-slope-per-cycle", "$maxHandleSlope",
      "--max-handle-growth", "$maxHandleGrowth",
      "--max-catalog-growth", "0"
    )
    if (!$SkipBuild) {
      $arguments += "--fresh-build"
    }

    $launchStartedUtc = [DateTime]::UtcNow
    Write-Host "Running offline CPU reliability soak (mode=$Mode, cycles=$effectiveCycles, duration_seconds=$durationSeconds)..."
    & $binaryPath @arguments
    $soakExitCode = $LASTEXITCODE
    if ($soakExitCode -ne 0) {
      throw "CPU reliability soak failed with exit code $soakExitCode. Inspect $evidencePath"
    }
  } finally {
    Pop-Location
  }

  if (!(Test-Path -LiteralPath $evidencePath -PathType Leaf)) {
    throw "CPU reliability soak did not produce evidence: $evidencePath"
  }
  $evidenceItem = Get-Item -LiteralPath $evidencePath
  if (($evidenceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "CPU reliability soak evidence must be an ordinary non-reparse file"
  }
  if ($evidenceItem.LastWriteTimeUtc -lt $launchStartedUtc.AddSeconds(-2)) {
    throw "CPU reliability soak evidence predates this invocation: $evidencePath"
  }
  $evidence = Get-Content -Raw -LiteralPath $evidencePath | ConvertFrom-Json
  if (
    [int]$evidence.schema_version -ne 2 -or
    [string]$evidence.kind -ne "vectorgrid_cpu_reliability_soak_evidence" -or
    [string]$evidence.run_id -ne $runId -or
    [string]$evidence.mode_label -ne $Mode -or
    [string]$evidence.status -ne "passed"
  ) {
    throw "CPU reliability soak evidence identity/status validation failed: $evidencePath"
  }
  $evidenceStartedAt = [DateTimeOffset]::Parse([string]$evidence.started_at).UtcDateTime
  if ($evidenceStartedAt -lt $launchStartedUtc.AddSeconds(-2)) {
    throw "CPU reliability soak evidence started_at predates this invocation"
  }
  $evidenceCompletedAt = [DateTimeOffset]::Parse([string]$evidence.completed_at).UtcDateTime
  if ($evidenceCompletedAt -lt $evidenceStartedAt) {
    throw "CPU reliability soak completed_at predates started_at"
  }
  if (
    [string]$evidence.provenance.binary.sha256 -ne $binarySha256 -or
    [string]$evidence.provenance.binary.path -ne $retainedBinaryPath -or
    [string]$evidence.provenance.binary.execution_path -ne $binaryPath -or
    [string]$evidence.provenance.binary.retained_copy_sha256 -ne $retainedBinarySha256 -or
    ![bool]$evidence.provenance.binary.retained_copy_matched_expected -or
    ![bool]$evidence.provenance.binary.matched_expected -or
    [string]$evidence.provenance.build_profile -ne $BuildProfile -or
    [string]$evidence.provenance.cargo_build_target_env -ne $cargoBuildTargetEnv -or
    [string]$evidence.provenance.cargo_target_triple -ne $cargoTargetTriple -or
    [bool]$evidence.provenance.cargo_artifact_emitted -ne $cargoArtifactEmitted -or
    [bool]$evidence.provenance.cargo_artifact_fresh -ne $cargoArtifactFresh -or
    [string]$evidence.provenance.source_manifest.sha256 -ne $sourceManifestSha256 -or
    ![bool]$evidence.provenance.source_manifest.matched_expected -or
    [int]$evidence.provenance.source_manifest.contents.file_count -ne
      [int]$sourceManifest.file_count -or
    [string]$evidence.provenance.source_manifest.contents.aggregate_sha256 -ne
      [string]$sourceManifest.aggregate_sha256 -or
    [string]$evidence.provenance.launcher_manifest.sha256 -ne $launcherManifestSha256 -or
    ![bool]$evidence.provenance.launcher_manifest.matched_expected -or
    [string]$evidence.provenance.launcher_manifest.contents.run_id -ne $runId -or
    ![bool]$evidence.provenance.launcher_manifest.contents.build.compiler_override_environment_cleared -or
    ![bool]$evidence.provenance.launcher_manifest.contents.build.native_build_override_environment_cleared -or
    ![bool]$evidence.provenance.launcher_manifest.contents.build.tauri_config_environment_cleared -or
    ![bool]$evidence.provenance.launcher_manifest.contents.build.tauri_target_environment_cleared -or
    ![bool]$evidence.provenance.launcher_manifest.contents.build.custom_protocol_disabled -or
    [string]$evidence.provenance.launcher_manifest.contents.build.context_asset_mode -ne
      "dev_url_no_embedded_frontend_assets" -or
    [bool]$evidence.provenance.launcher_manifest.contents.build.frontend_dist_tree_effective -or
    [bool]$evidence.provenance.launcher_manifest.contents.build.target_dir_initially_empty -ne
      (-not [bool]$SkipBuild) -or
    [string]$evidence.provenance.launcher_manifest.contents.cargo_configuration.aggregate_sha256 -ne
      [string]$cargoConfiguration.aggregate_sha256 -or
    [string]$evidence.provenance.launcher_manifest.contents.toolchain.input_manifest.aggregate_sha256 -ne
      [string]$toolchainProvenance.input_manifest.aggregate_sha256 -or
    [bool]$evidence.provenance.launcher_manifest.contents.build.compiler_path_pinned_during_build -ne
      (-not [bool]$SkipBuild) -or
    [bool]$evidence.provenance.launcher_manifest.contents.build.target_linker_path_pinned_during_build -ne
      (-not [bool]$SkipBuild) -or
    [string]$evidence.provenance.cargo_target_dir -ne $resolvedCargoTargetDir -or
    ![bool]$evidence.provenance.benchmark_lock.inherited -or
    ![bool]$evidence.provenance.benchmark_lock.mutex_contention_verified -or
    [int]$evidence.provenance.recovered_stale_isolations -ne $recoveredStaleIsolations -or
    [bool]$evidence.provenance.fresh_build -ne (-not [bool]$SkipBuild)
  ) {
    throw "CPU reliability soak provenance validation failed"
  }
  $compiledResourceCompilerAttestation =
    $evidence.provenance.compiled_resource_compiler_attestation
  $compiledResourceCompilerAttestationRequired = -not [bool]$SkipBuild
  if (
    [string]$compiledResourceCompilerAttestation.environment_name -ne
      "VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION" -or
    [bool]$compiledResourceCompilerAttestation.required -ne
      $compiledResourceCompilerAttestationRequired -or
    [bool]$compiledResourceCompilerAttestation.matched_native_invocation -ne
      $compiledResourceCompilerAttestationRequired
  ) {
    throw "Compiled resource-compiler attestation evidence does not match the build mode"
  }
  if ($compiledResourceCompilerAttestationRequired) {
    if (
      ![bool]$compiledResourceCompilerAttestation.present -or
      [string]$compiledResourceCompilerAttestation.value_sha256 -ne
        [string]$resourceCompilerInvocation.marker_value_sha256
    ) {
      throw "Compiled resource-compiler attestation does not match the exact native invocation"
    }
  } elseif ($null -ne $compiledResourceCompilerAttestation.value_sha256) {
    throw "Prebuilt evidence must not claim a matched compiled resource-compiler invocation"
  }
  if ((Get-Sha256Hex -Path $sourceManifestPath) -ne $sourceManifestSha256) {
    throw "CPU reliability soak source manifest changed during execution"
  }
  if ((Get-Sha256Hex -Path $launcherManifestPath) -ne $launcherManifestSha256) {
    throw "CPU reliability soak launcher manifest changed during execution"
  }
  if ((Get-Sha256Hex -Path $binaryPath) -ne $binarySha256) {
    throw "CPU reliability soak binary changed during execution"
  }
  $cargoConfigurationAfter = Get-SoakCargoConfigurationProvenance `
    -RepoRoot $repoRoot `
    -CargoHome $resolvedCargoHome
  if (
    [string]$cargoConfigurationAfter.aggregate_sha256 -ne
      [string]$cargoConfiguration.aggregate_sha256
  ) {
    throw "Effective Cargo configuration changed during execution"
  }
  $toolchainAfter = Get-SoakToolchainProvenance `
    -RepoRoot $repoRoot `
    -IncludeInputManifest `
    -ProbeSelectedExecutables
  if (
    [string]$toolchainAfter.input_manifest.aggregate_sha256 -ne
      [string]$toolchainProvenance.input_manifest.aggregate_sha256 -or
    [string]$toolchainAfter.cargo_sha256 -ne [string]$toolchainProvenance.cargo_sha256 -or
    [string]$toolchainAfter.rustc_sha256 -ne [string]$toolchainProvenance.rustc_sha256
  ) {
    throw "Selected Rust toolchain changed during execution"
  }
  if (
    [bool]$evidence.offline_contract.provider_credentials_used -or
    [bool]$evidence.offline_contract.provider_network_calls -or
    [bool]$evidence.offline_contract.real_broker_orders -or
    ![bool]$evidence.offline_contract.synthetic_data_only -or
    ![bool]$evidence.offline_contract.credential_environment_cleared -or
    ![bool]$evidence.offline_contract.repo_dotenv_credential_fallback_disabled -or
    ![bool]$evidence.offline_contract.external_http_proxy_trapped -or
    ![bool]$evidence.offline_contract.cargo_dependency_resolution_offline -or
    ![bool]$evidence.offline_contract.compiler_override_environment_cleared -or
    ![bool]$evidence.offline_contract.native_build_override_environment_cleared -or
    [bool]$evidence.offline_contract.target_linker_path_pinned_during_build -ne
      (-not [bool]$SkipBuild) -or
    [bool]$evidence.offline_contract.compiled_resource_compiler_attestation_matched -ne
      (-not [bool]$SkipBuild) -or
    ![bool]$evidence.offline_contract.build_environment_attested -or
    ![bool]$evidence.offline_contract.tauri_config_environment_cleared -or
    ![bool]$evidence.offline_contract.tauri_target_environment_cleared -or
    ![bool]$evidence.offline_contract.compiled_tauri_config_override_absent -or
    [string]$evidence.offline_contract.compiled_tauri_target_triple -ne $cargoTargetTriple
  ) {
    throw "CPU reliability soak evidence violated the offline provider contract"
  }
  if (
    [int]$evidence.config.cycle_limit -ne $effectiveCycles -or
    [int]$evidence.config.duration_seconds -ne $durationSeconds -or
    [int]$evidence.config.cancel_every -ne $CancelEvery -or
    [int]$evidence.config.rows -ne $effectiveRows -or
    [string]$evidence.config.cargo_target_triple -ne $cargoTargetTriple -or
    [int]$evidence.config.validation_candidates -ne 4 -or
    [int]$evidence.config.cpu_tile_ram_budget_mb -ne 64 -or
    [int]$evidence.config.warmup_cycles -ne $WarmupCycles -or
    [int]$evidence.config.allowed_failures -ne 0 -or
    [int]$evidence.config.retained_samples_limit -ne $retainedSamples -or
    [int]$evidence.config.retained_cycles_limit -ne $retainedCycles -or
    [double]$evidence.config.thresholds.private_slope_bytes_per_cycle -ne
      ([double]$maxPrivateSlope * $mibBytes) -or
    [double]$evidence.config.thresholds.private_growth_bytes -ne
      ([double]$maxPrivateGrowth * $mibBytes) -or
    [double]$evidence.config.thresholds.working_set_slope_bytes_per_cycle -ne
      ([double]$maxWorkingSetSlope * $mibBytes) -or
    [double]$evidence.config.thresholds.working_set_growth_bytes -ne
      ([double]$maxWorkingSetGrowth * $mibBytes) -or
    [double]$evidence.config.thresholds.handle_slope_per_cycle -ne
      [double]$maxHandleSlope -or
    [int]$evidence.config.thresholds.handle_growth -ne $maxHandleGrowth -or
    [int]$evidence.config.thresholds.catalog_growth -ne 0
  ) {
    throw "CPU reliability soak evidence configuration differs from the launched contract"
  }
  if (
    [int]$evidence.counts.failures_total -ne 0 -or
    [int]$evidence.counts.cycles_completed -le ($WarmupCycles + 2) -or
    ![bool]$evidence.catalog.passed -or
    [int]$evidence.catalog.growth -ne 0 -or
    ![bool]$evidence.artifact_integrity.passed -or
    [int]$evidence.artifact_integrity.artifacts_checked -ne
      [int]$evidence.counts.cycles_completed -or
    ![bool]$evidence.cancellation.passed -or
    [int]$evidence.cancellation.probes_expected -le 0 -or
    [int]$evidence.cancellation.probes_observed -lt
      [int]$evidence.cancellation.probes_expected -or
    [int]$evidence.cancellation.cancellation_cycles_expected -le 0 -or
    [int]$evidence.cancellation.positive_partial_progress_probes_observed -lt
      [int]$evidence.cancellation.cancellation_cycles_expected -or
    [int]$evidence.cancellation.pre_start_validation_probes_observed -lt
      [int]$evidence.cancellation.cancellation_cycles_expected -or
    [string]$evidence.cancellation.mode -ne
      "positive_partial_progress_optimization_cancel_and_pre_start_validation_cancel" -or
    ![bool]$evidence.managed_state.passed -or
    [string]$evidence.managed_state.lifecycle -ne
      "single_tauri_managed_app_state_full_run" -or
    [string]$evidence.managed_state.initial_load_source -ne "csv_parsed_and_cached" -or
    [int]$evidence.managed_state.initial_loads -ne 1 -or
    [int]$evidence.managed_state.in_memory_reuses -ne
      [int]$evidence.counts.cycles_completed -or
    ![bool]$evidence.managed_state.run_slot_released_after_final_cycle -or
    ![bool]$evidence.managed_state.retained_through_last_process_sample -or
    ![bool]$evidence.managed_state.product_run_guard_and_cancel_token_used -or
    ![bool]$evidence.managed_state.invalidated_once_after_final_sample -or
    ![bool]$evidence.managed_state.lookup_rejected_after_invalidation
  ) {
    throw "CPU reliability soak workflow-integrity evidence was incomplete"
  }
  if (
    ![bool]$evidence.isolation.root_cleaned -or
    ![bool]$evidence.isolation.app_data_root_cleaned -or
    [bool]$evidence.isolation.source_app_data_touched -or
    ![bool]$evidence.isolation.resolved_app_data_strictly_contained -or
    [string]$evidence.isolation.requested_app_data_root -ne
      [string]$evidence.isolation.resolved_app_data_root -or
    ![bool]$evidence.isolation.cleanup_requires_owned_marker -or
    (Test-Path -LiteralPath $isolationRoot)
  ) {
    throw "CPU reliability soak did not clean its owned isolation tree"
  }
  foreach ($metricName in @("working_set", "private_bytes", "handles")) {
    $metric = $evidence.monitoring.$metricName
    if (
      ![bool]$metric.supported -or
      ![bool]$metric.complete -or
      ![bool]$metric.passed -or
      [int]$metric.measured_samples -lt 3 -or
      [int]$metric.measured_samples -ne [int]$metric.expected_samples
    ) {
      throw "CPU reliability soak $metricName trend was not authority-eligible"
    }
  }
  $storage = $evidence.monitoring.owned_storage
  if (
    ![bool]$storage.passed -or
    [int64]$storage.max_residual_artifact_or_temp_files -ne 0
  ) {
    throw "CPU reliability soak left residual artifacts or showed owned-storage growth"
  }
  foreach ($storageMetricName in @("file_count", "directory_count", "total_bytes")) {
    $storageMetric = $storage.$storageMetricName
    if (
      ![bool]$storageMetric.supported -or
      ![bool]$storageMetric.complete -or
      ![bool]$storageMetric.passed -or
      [int]$storageMetric.measured_samples -lt 3 -or
      [int]$storageMetric.measured_samples -ne [int]$storageMetric.expected_samples
    ) {
      throw "CPU reliability soak $storageMetricName storage trend was not eligible"
    }
  }
  if ($Mode -eq "operator") {
    $minimumMeasuredCycles = [int]$evidence.authority.minimum_operator_measured_cycles
    $maximumSampleGapMs = [int64]$evidence.authority.maximum_operator_sample_gap_ms
    if (
      $minimumMeasuredCycles -ne $minimumOperatorMeasuredCycles -or
      $maximumSampleGapMs -ne $maximumOperatorSampleGapMs -or
      [double]$evidence.authority.owned_storage_thresholds.max_growth_bytes -ne
        (64.0 * $mibBytes) -or
      [double]$evidence.authority.owned_storage_thresholds.max_slope_bytes_per_cycle -ne
        (64.0 * 1024.0) -or
      [double]$evidence.authority.owned_storage_thresholds.max_file_growth -ne 4.0 -or
      [double]$evidence.authority.owned_storage_thresholds.max_file_slope_per_cycle -ne
        0.01 -or
      [double]$evidence.authority.owned_storage_thresholds.max_directory_growth -ne 4.0 -or
      [double]$evidence.authority.owned_storage_thresholds.max_directory_slope_per_cycle -ne
        0.01 -or
      [int]$evidence.authority.owned_storage_thresholds.max_residual_artifact_or_temp_files -ne
        0 -or
      ![bool]$evidence.termination.duration_reached -or
      ![bool]$evidence.termination.operator_sampling_contract_passed -or
      [int64]$evidence.elapsed_ms -lt ([int64]$minimumOperatorSeconds * 1000) -or
      [int]$evidence.monitoring.working_set.measured_samples -lt $minimumMeasuredCycles -or
      [int]$storage.file_count.measured_samples -lt $minimumMeasuredCycles -or
      [int]$storage.directory_count.measured_samples -lt $minimumMeasuredCycles -or
      [int]$storage.total_bytes.measured_samples -lt $minimumMeasuredCycles -or
      [int64]$evidence.monitoring.max_cycle_sample_gap_ms -gt $maximumSampleGapMs -or
      ![bool]$evidence.authority.operator_release_eligible
    ) {
      throw "Operator soak did not satisfy the six-hour release-authority contract"
    }
  } elseif ([bool]$evidence.authority.operator_release_eligible) {
    throw "Short soak must not claim operator release authority"
  }
  $completedSuccessfully = $true
} finally {
  try {
    if ($null -ne $resourceCompilerLease) {
      $resourceCompilerLease.Dispose()
      $resourceCompilerLease = $null
    }
    foreach ($name in $pinnedNativeEnvironmentNames) {
      Remove-Item "Env:$name" -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $isolationRoot) {
      Remove-OwnedSoakIsolation `
        -IsolationRoot $isolationRoot `
        -ExpectedParent $runDir `
        -RequireStoppedOwner | Out-Null
    }
    if (!$SkipBuild -and (Test-Path -LiteralPath $resolvedCargoTargetDir)) {
      Remove-OwnedSoakCargoTarget `
        -TargetDir $resolvedCargoTargetDir `
        -ExpectedParent $resolvedCargoTargetRoot `
        -ExpectedRunId $runId | Out-Null
    }
  } finally {
    try {
      Restore-ProcessEnvironment -Snapshot $environmentSnapshot
    } finally {
      Exit-BenchmarkLock -Handle $benchmarkLock
    }
  }
}

if (!$completedSuccessfully) {
  throw "CPU reliability soak launcher did not complete successfully"
}
if (!$SkipBuild -and (Test-Path -LiteralPath $resolvedCargoTargetDir)) {
  throw "Owned fresh Cargo target remained after launcher cleanup"
}
if (
  !(Test-Path -LiteralPath $retainedBinaryPath -PathType Leaf) -or
  (Get-Sha256Hex -Path $retainedBinaryPath) -ne $retainedBinarySha256
) {
  throw "Retained attested soak binary is missing or changed after target cleanup"
}

$evidenceSha256 = Get-Sha256Hex -Path $evidencePath
Write-Host "CPU reliability soak passed."
Write-Host "Evidence: $evidencePath"
Write-Host "Evidence SHA-256: $evidenceSha256"
