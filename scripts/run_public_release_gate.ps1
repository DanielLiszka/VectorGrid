param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [switch]$IncludePerf,
  [switch]$IncludeGpuValidationPerf,
  [switch]$IncludeOptionsGpuPromotion,
  [string]$OptionsGpuArtifactPath = "",
  [string]$OptionsGpuDatasetId = "",
  [string]$OptionsGpuImportCsv = "",
  [string]$OptionsGpuImportRootSymbol = "",
  [string]$OptionsGpuAppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OptionsGpuBaselinePath = "scripts/baselines/options_expression_perf_baseline.json",
  [switch]$IncludeExpandedBrowserQa,
  [switch]$IncludeBaselineUi,
  [switch]$SkipWorkspaceTests,
  [switch]$AllowDirtyWorktree,
  [switch]$AllowUnsignedBeta,
  [string]$PostBuildSignScript = "",
  [string]$ExpectedSignerThumbprint = ""
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 18
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Resolve-OutDir {
  param([string]$Value)
  if (-not [string]::IsNullOrWhiteSpace($Value)) {
    return [System.IO.Path]::GetFullPath($Value)
  }
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
  return Join-Path (Join-Path $repoRoot "output\\public_release_candidates") "${stamp}-${Variant}"
}

function Assert-FreshOutputDirectory {
  param([Parameter(Mandatory = $true)][string]$Path)

  if (-not (Test-Path -LiteralPath $Path)) {
    return
  }
  $item = Get-Item -LiteralPath $Path -Force
  if (
    -not $item.PSIsContainer -or
    ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  ) {
    throw "Public release output must be a new or empty ordinary directory: $Path"
  }
  if ($null -ne (Get-ChildItem -LiteralPath $Path -Force | Select-Object -First 1)) {
    throw "Public release output directory is not empty. Use a fresh path so stale evidence or installers cannot survive into this release: $Path"
  }
}

function Read-JsonFile {
  param([string]$Path)
  if (-not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path $Path)) {
    return Get-Content -Raw $Path | ConvertFrom-Json
  }
  return $null
}

function Get-BytesSha256 {
  param(
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [byte[]]$Bytes
  )

  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  try {
    return ([System.BitConverter]::ToString(
        $sha256.ComputeHash($Bytes)
      )).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha256.Dispose()
  }
}

function Assert-PublicReleaseSourceHasSingleLink {
  param([Parameter(Mandatory = $true)][string]$Path)

  $item = Get-Item -LiteralPath $Path -Force
  $linkTypeProperty = $item.PSObject.Properties["LinkType"]
  $targetProperty = $item.PSObject.Properties["Target"]
  $linkType = if ($null -eq $linkTypeProperty) {
    ""
  } else {
    [string]$linkTypeProperty.Value
  }
  $linkTargets = if ($null -eq $targetProperty) {
    @()
  } else {
    @($targetProperty.Value)
  }
  if ($linkType -eq "HardLink" -or $linkTargets.Count -gt 0) {
    throw "Public-release source files must have exactly one filesystem link."
  }
}

function Get-FileSha256Identity {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [switch]$RetainReadLease
  )

  Assert-PublicReleaseSourceHasSingleLink -Path $Path
  $stream = [System.IO.File]::Open(
    $Path,
    [System.IO.FileMode]::Open,
    [System.IO.FileAccess]::Read,
    [System.IO.FileShare]::Read
  )
  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  $leaseRetained = $false
  try {
    Assert-PublicReleaseSourceHasSingleLink -Path $Path
    $identity = [pscustomobject][ordered]@{
      size_bytes = [int64]$stream.Length
      sha256 = ([System.BitConverter]::ToString(
          $sha256.ComputeHash($stream)
        )).Replace("-", "").ToLowerInvariant()
      read_lease = $null
    }
    Assert-PublicReleaseSourceHasSingleLink -Path $Path
    if ($RetainReadLease) {
      $identity.read_lease = $stream
      $leaseRetained = $true
    }
    return $identity
  } finally {
    $sha256.Dispose()
    if (-not $leaseRetained) {
      $stream.Dispose()
    }
  }
}

function Invoke-PublicReleaseGitQuery {
  param(
    [Parameter(Mandatory = $true)][string]$GitExecutablePath,
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [ValidateSet(
      "head",
      "status",
      "tracked-diff",
      "tracked-index",
      "untracked-files"
    )]
    [string]$Query
  )

  $arguments = switch ($Query) {
    "head" { "rev-parse --verify HEAD" }
    "status" { "status --porcelain=v1 -z --untracked-files=all -- ." }
    "tracked-diff" {
      "diff --binary --full-index --no-ext-diff --no-textconv HEAD -- ."
    }
    "tracked-index" { "ls-files -v -z --cached -- ." }
    "untracked-files" { "ls-files -z --others --exclude-standard -- ." }
  }
  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $GitExecutablePath
  $startInfo.Arguments = $arguments
  $startInfo.WorkingDirectory = $RepositoryRoot
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true

  $process = New-Object System.Diagnostics.Process
  $process.StartInfo = $startInfo
  $stdout = New-Object System.IO.MemoryStream
  try {
    if (-not $process.Start()) {
      throw "Could not start Git source-state query."
    }
    $stdoutTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    if ($process.ExitCode -ne 0) {
      throw "Git source-state query '$Query' failed with exit code $($process.ExitCode): $($stderr.Trim())"
    }
    return [pscustomobject][ordered]@{
      bytes_base64 = [System.Convert]::ToBase64String($stdout.ToArray())
      stderr = [string]$stderr
    }
  } finally {
    $stdout.Dispose()
    $process.Dispose()
  }
}

function ConvertFrom-PublicReleaseGitNulList {
  param(
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [byte[]]$Bytes
  )

  $strictUtf8 = New-Object System.Text.UTF8Encoding($false, $true)
  try {
    $text = $strictUtf8.GetString($Bytes)
  } catch {
    throw "Git returned a source path list that is not valid UTF-8."
  }
  return [pscustomobject][ordered]@{
    values = @(
      $text.Split([char]0) |
        Where-Object { -not [string]::IsNullOrEmpty([string]$_) }
    )
  }
}

function Assert-PublicReleaseManifestPathSafe {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)][string]$RelativePath
  )

  $repositoryPrefix = [System.IO.Path]::GetFullPath(
    $RepositoryRoot
  ).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  ) + [System.IO.Path]::DirectorySeparatorChar
  if (
    [System.IO.Path]::IsPathRooted($RelativePath) -or
    @($RelativePath.Replace("\", "/").Split("/") | Where-Object {
        $_ -eq ".." -or $_ -eq ""
      }).Count -gt 0
  ) {
    throw "Git returned an unsafe public-release source path."
  }
  $fullPath = [System.IO.Path]::GetFullPath(
    (Join-Path $RepositoryRoot $RelativePath)
  )
  if (
    -not $fullPath.StartsWith(
      $repositoryPrefix,
      [System.StringComparison]::OrdinalIgnoreCase
    )
  ) {
    throw "A public-release source path escaped the repository."
  }
  return $fullPath
}

function Assert-PublicReleasePathHasNoReparseAncestor {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)][string]$FullPath
  )

  $resolvedRepositoryRoot = [System.IO.Path]::GetFullPath(
    $RepositoryRoot
  ).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
  $cursor = Split-Path -Parent $FullPath
  while (
    -not [string]::IsNullOrWhiteSpace($cursor) -and
    -not [string]::Equals(
      $cursor,
      $resolvedRepositoryRoot,
      [System.StringComparison]::OrdinalIgnoreCase
    )
  ) {
    if (Test-Path -LiteralPath $cursor) {
      $item = Get-Item -LiteralPath $cursor -Force
      if (
        ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
      ) {
        throw "A public-release source path has a reparse-point ancestor."
      }
    }
    $parent = Split-Path -Parent $cursor
    if ([string]::Equals(
        $parent,
        $cursor,
        [System.StringComparison]::OrdinalIgnoreCase
      )) {
      break
    }
    $cursor = $parent
  }
}

function Get-PublicReleaseRawFileManifest {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [string[]]$RelativePaths,
    [Parameter(Mandatory = $true)]
    [ValidateSet("tracked", "untracked")]
    [string]$Kind,
    [switch]$AllowMissing,
    [switch]$RetainReadLeases
  )

  $paths = @($RelativePaths)
  [System.Array]::Sort($paths, [System.StringComparer]::Ordinal)
  $seen = New-Object 'System.Collections.Generic.HashSet[string]' (
    [System.StringComparer]::Ordinal
  )
  $readLeases = @()
  try {
    $rows = @(
      foreach ($relativePathValue in $paths) {
        $relativePath = [string]$relativePathValue
        if (-not $seen.Add($relativePath)) {
          throw "Git returned a duplicate public-release source path."
        }
        $fullPath = Assert-PublicReleaseManifestPathSafe `
          -RepositoryRoot $RepositoryRoot `
          -RelativePath $relativePath
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
          if ($AllowMissing -and -not (Test-Path -LiteralPath $fullPath)) {
            [ordered]@{
              path = $relativePath.Replace("\", "/")
              state = "missing"
              size_bytes = $null
              sha256 = $null
            }
            continue
          }
          throw "A public-release source was not an ordinary file while snapshotting."
        }
        Assert-PublicReleasePathHasNoReparseAncestor `
          -RepositoryRoot $RepositoryRoot `
          -FullPath $fullPath
        $item = Get-Item -LiteralPath $fullPath -Force
        if (
          ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
        ) {
          throw "A public-release source may not be a reparse point."
        }
        $identity = Get-FileSha256Identity `
          -Path $fullPath `
          -RetainReadLease:$RetainReadLeases
        if ($null -ne $identity.read_lease) {
          $readLeases += $identity.read_lease
        }
        [ordered]@{
          path = $relativePath.Replace("\", "/")
          state = "file"
          size_bytes = [int64]$identity.size_bytes
          sha256 = [string]$identity.sha256
        }
      }
    )
    $manifestText = ConvertTo-Json -Compress -InputObject ([object[]]$rows)
    $manifestBytes = [System.Text.Encoding]::UTF8.GetBytes($manifestText)
    return [pscustomobject][ordered]@{
      file_count = [int]$rows.Count
      size_bytes = [int64]$manifestBytes.Length
      sha256 = Get-BytesSha256 -Bytes $manifestBytes
      base64 = [System.Convert]::ToBase64String($manifestBytes)
      read_leases = [object[]]$readLeases
    }
  } catch {
    foreach ($readLease in $readLeases) {
      $readLease.Dispose()
    }
    throw
  }
}

function Get-PublicReleaseTrackedInventory {
  param(
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [byte[]]$IndexBytes
  )

  $records = @(
    (ConvertFrom-PublicReleaseGitNulList -Bytes $IndexBytes).values
  )
  $paths = @(
    foreach ($recordValue in $records) {
      $record = [string]$recordValue
      if ($record.Length -lt 3 -or $record[1] -ne " ") {
        throw "Git returned a malformed tracked-index record."
      }
      $tag = [string]$record[0]
      if ($tag -cne "H") {
        throw "Public release rejects tracked entries with nonstandard index flags (assume-unchanged, skip-worktree, unmerged, or otherwise non-cached)."
      }
      $record.Substring(2)
    }
  )
  return [pscustomobject][ordered]@{
    file_count = [int]$paths.Count
    paths = [string[]]$paths
    size_bytes = [int64]$IndexBytes.Length
    sha256 = Get-BytesSha256 -Bytes $IndexBytes
    base64 = [System.Convert]::ToBase64String($IndexBytes)
  }
}

function Get-PublicReleaseUntrackedManifest {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [byte[]]$PathListBytes,
    [switch]$RetainReadLeases
  )

  $relativePaths = @(
    (ConvertFrom-PublicReleaseGitNulList -Bytes $PathListBytes).values
  )
  return Get-PublicReleaseRawFileManifest `
    -RepositoryRoot $RepositoryRoot `
    -RelativePaths $relativePaths `
    -Kind "untracked" `
    -RetainReadLeases:$RetainReadLeases
}

function Get-PublicReleaseSourceSnapshot {
  param(
    [Parameter(Mandatory = $true)][string]$GitExecutablePath,
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [switch]$RetainSourceReadLeases
  )

  $headResult = Invoke-PublicReleaseGitQuery `
    -GitExecutablePath $GitExecutablePath `
    -RepositoryRoot $RepositoryRoot `
    -Query "head"
  $headBytes = [System.Convert]::FromBase64String(
    [string]$headResult.bytes_base64
  )
  $head = [System.Text.Encoding]::UTF8.GetString($headBytes).Trim()
  if ($head -notmatch '^[A-Fa-f0-9]{40,64}$') {
    throw "Git returned an invalid HEAD identity for public release: $head"
  }

  $statusResult = Invoke-PublicReleaseGitQuery `
    -GitExecutablePath $GitExecutablePath `
    -RepositoryRoot $RepositoryRoot `
    -Query "status"
  $statusBytes = [System.Convert]::FromBase64String(
    [string]$statusResult.bytes_base64
  )
  $trackedDiffResult = Invoke-PublicReleaseGitQuery `
    -GitExecutablePath $GitExecutablePath `
    -RepositoryRoot $RepositoryRoot `
    -Query "tracked-diff"
  $trackedDiffBytes = [System.Convert]::FromBase64String(
    [string]$trackedDiffResult.bytes_base64
  )
  $trackedIndexResult = Invoke-PublicReleaseGitQuery `
    -GitExecutablePath $GitExecutablePath `
    -RepositoryRoot $RepositoryRoot `
    -Query "tracked-index"
  $trackedIndexBytes = [System.Convert]::FromBase64String(
    [string]$trackedIndexResult.bytes_base64
  )
  $trackedInventory = Get-PublicReleaseTrackedInventory `
    -IndexBytes $trackedIndexBytes
  $trackedManifest = Get-PublicReleaseRawFileManifest `
    -RepositoryRoot $RepositoryRoot `
    -RelativePaths @($trackedInventory.paths) `
    -Kind "tracked" `
    -AllowMissing `
    -RetainReadLeases:$RetainSourceReadLeases
  $untrackedFilesResult = Invoke-PublicReleaseGitQuery `
    -GitExecutablePath $GitExecutablePath `
    -RepositoryRoot $RepositoryRoot `
    -Query "untracked-files"
  $untrackedPathListBytes = [System.Convert]::FromBase64String(
    [string]$untrackedFilesResult.bytes_base64
  )
  try {
    $untrackedManifest = Get-PublicReleaseUntrackedManifest `
      -RepositoryRoot $RepositoryRoot `
      -PathListBytes $untrackedPathListBytes `
      -RetainReadLeases:$RetainSourceReadLeases
  } catch {
    foreach ($readLease in @($trackedManifest.read_leases)) {
      $readLease.Dispose()
    }
    throw
  }
  $statusSha256 = Get-BytesSha256 -Bytes $statusBytes
  $trackedDiffSha256 = Get-BytesSha256 -Bytes $trackedDiffBytes
  $aggregateText = @(
    "head`t$($head.ToLowerInvariant())",
    "status`t$($statusBytes.Length)`t$statusSha256",
    "tracked_diff`t$($trackedDiffBytes.Length)`t$trackedDiffSha256",
    "tracked_index`t$($trackedInventory.file_count)`t$($trackedInventory.size_bytes)`t$($trackedInventory.sha256)",
    "tracked_manifest`t$($trackedManifest.file_count)`t$($trackedManifest.size_bytes)`t$($trackedManifest.sha256)",
    "untracked_manifest`t$($untrackedManifest.file_count)`t$($untrackedManifest.size_bytes)`t$($untrackedManifest.sha256)"
  ) -join "`n"
  $aggregateBytes = [System.Text.Encoding]::UTF8.GetBytes($aggregateText)
  return [pscustomobject][ordered]@{
    schema_version = 2
    head = $head.ToLowerInvariant()
    status_command = "git status --porcelain=v1 -z --untracked-files=all -- ."
    status_encoding = "raw-nul-delimited-stdout"
    status_size_bytes = [int64]$statusBytes.Length
    status_sha256 = $statusSha256
    status_base64 = [System.Convert]::ToBase64String($statusBytes)
    tracked_diff_command = "git diff --binary --full-index --no-ext-diff --no-textconv HEAD -- ."
    tracked_diff_size_bytes = [int64]$trackedDiffBytes.Length
    tracked_diff_sha256 = $trackedDiffSha256
    tracked_diff_base64 = [System.Convert]::ToBase64String($trackedDiffBytes)
    tracked_index_command = "git ls-files -v -z --cached -- ."
    tracked_index_file_count = [int]$trackedInventory.file_count
    tracked_index_size_bytes = [int64]$trackedInventory.size_bytes
    tracked_index_sha256 = [string]$trackedInventory.sha256
    tracked_index_base64 = [string]$trackedInventory.base64
    tracked_manifest_file_count = [int]$trackedManifest.file_count
    tracked_manifest_size_bytes = [int64]$trackedManifest.size_bytes
    tracked_manifest_sha256 = [string]$trackedManifest.sha256
    tracked_manifest_base64 = [string]$trackedManifest.base64
    untracked_manifest_file_count = [int]$untrackedManifest.file_count
    untracked_manifest_size_bytes = [int64]$untrackedManifest.size_bytes
    untracked_manifest_sha256 = [string]$untrackedManifest.sha256
    untracked_manifest_base64 = [string]$untrackedManifest.base64
    source_read_lease_count = [int](
      @($trackedManifest.read_leases).Count +
      @($untrackedManifest.read_leases).Count
    )
    source_read_leases = [object[]]@(
      @($trackedManifest.read_leases) +
      @($untrackedManifest.read_leases)
    )
    source_state_sha256 = Get-BytesSha256 -Bytes $aggregateBytes
    dirty = $statusBytes.Length -gt 0
  }
}

function Assert-PublicReleaseSourceSnapshotStable {
  param(
    [Parameter(Mandatory = $true)]$Initial,
    [Parameter(Mandatory = $true)]$Final
  )

  if (
    [int]$Initial.schema_version -ne 2 -or
    [int]$Final.schema_version -ne 2 -or
    [string]$Initial.status_command -ne
      "git status --porcelain=v1 -z --untracked-files=all -- ." -or
    [string]$Final.status_command -ne [string]$Initial.status_command -or
    [string]$Initial.status_encoding -ne "raw-nul-delimited-stdout" -or
    [string]$Final.status_encoding -ne [string]$Initial.status_encoding -or
    [string]$Initial.tracked_diff_command -ne
      "git diff --binary --full-index --no-ext-diff --no-textconv HEAD -- ." -or
    [string]$Final.tracked_diff_command -ne
      [string]$Initial.tracked_diff_command -or
    [string]$Initial.tracked_index_command -ne
      "git ls-files -v -z --cached -- ." -or
    [string]$Final.tracked_index_command -ne
      [string]$Initial.tracked_index_command
  ) {
    throw "Public release source-state snapshot schema is invalid."
  }
  if ([string]$Final.head -ne [string]$Initial.head) {
    throw "Public release HEAD changed during the release run."
  }
  if (
    [int64]$Final.status_size_bytes -ne [int64]$Initial.status_size_bytes -or
    [string]$Final.status_sha256 -ne [string]$Initial.status_sha256 -or
    [string]$Final.status_base64 -ne [string]$Initial.status_base64
  ) {
    throw "Public release worktree status changed during the release run."
  }
  if (
    [int64]$Final.tracked_diff_size_bytes -ne
      [int64]$Initial.tracked_diff_size_bytes -or
    [string]$Final.tracked_diff_sha256 -ne
      [string]$Initial.tracked_diff_sha256 -or
    [string]$Final.tracked_diff_base64 -ne
      [string]$Initial.tracked_diff_base64 -or
    [int]$Final.tracked_index_file_count -ne
      [int]$Initial.tracked_index_file_count -or
    [int64]$Final.tracked_index_size_bytes -ne
      [int64]$Initial.tracked_index_size_bytes -or
    [string]$Final.tracked_index_sha256 -ne
      [string]$Initial.tracked_index_sha256 -or
    [string]$Final.tracked_index_base64 -ne
      [string]$Initial.tracked_index_base64 -or
    [int]$Final.tracked_manifest_file_count -ne
      [int]$Initial.tracked_manifest_file_count -or
    [int64]$Final.tracked_manifest_size_bytes -ne
      [int64]$Initial.tracked_manifest_size_bytes -or
    [string]$Final.tracked_manifest_sha256 -ne
      [string]$Initial.tracked_manifest_sha256 -or
    [string]$Final.tracked_manifest_base64 -ne
      [string]$Initial.tracked_manifest_base64 -or
    [int]$Final.untracked_manifest_file_count -ne
      [int]$Initial.untracked_manifest_file_count -or
    [int64]$Final.untracked_manifest_size_bytes -ne
      [int64]$Initial.untracked_manifest_size_bytes -or
    [string]$Final.untracked_manifest_sha256 -ne
      [string]$Initial.untracked_manifest_sha256 -or
    [string]$Final.untracked_manifest_base64 -ne
      [string]$Initial.untracked_manifest_base64 -or
    [string]$Final.source_state_sha256 -ne
      [string]$Initial.source_state_sha256
  ) {
    throw "Public release source contents changed during the release run."
  }
}

function Assert-PublicReleaseSourceReadLeasesActive {
  param([Parameter(Mandatory = $true)]$SourceSnapshot)

  $leases = @($SourceSnapshot.source_read_leases)
  if (
    [int]$SourceSnapshot.source_read_lease_count -le 0 -or
    $leases.Count -ne [int]$SourceSnapshot.source_read_lease_count
  ) {
    throw "Public-release source read leases are missing or incomplete."
  }
  foreach ($readLease in $leases) {
    if (
      $null -eq $readLease -or
      -not [bool]$readLease.CanRead -or
      $readLease.SafeFileHandle.IsClosed -or
      $readLease.SafeFileHandle.IsInvalid
    ) {
      throw "A public-release source read lease was not active for the full release interval."
    }
  }
}

function Close-PublicReleaseSourceReadLeases {
  param($SourceSnapshot)

  if ($null -eq $SourceSnapshot) {
    return
  }
  foreach ($readLease in @($SourceSnapshot.source_read_leases)) {
    if ($null -ne $readLease) {
      $readLease.Dispose()
    }
  }
  $SourceSnapshot.source_read_leases = [object[]]@()
}

function Test-PublicReleasePathUnderOrEqualRoot {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Root
  )

  $resolvedPath = [System.IO.Path]::GetFullPath($Path).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
  $resolvedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
  return (
    [string]::Equals(
      $resolvedPath,
      $resolvedRoot,
      [System.StringComparison]::OrdinalIgnoreCase
    ) -or
    $resolvedPath.StartsWith(
      $resolvedRoot + [System.IO.Path]::DirectorySeparatorChar,
      [System.StringComparison]::OrdinalIgnoreCase
    )
  )
}

function Get-PublicReleaseSourceMutationExcludedRoots {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)][string]$ReleaseOutputDirectory
  )

  $relativeGeneratedRoots = @(
    ".git",
    "target",
    "target_check",
    "target_codex",
    "target_alt",
    "output",
    "dist",
    "node_modules",
    "web\node_modules",
    "web\dist",
    "web\playwright-report",
    "web\test-results",
    "web\.vite",
    "test-results",
    "logs",
    ".playwright-cli",
    ".codex_tmp",
    "src-tauri\gen\schemas",
    "src-tauri\resources\native"
  )
  $candidates = @(
    $relativeGeneratedRoots | ForEach-Object {
      [System.IO.Path]::GetFullPath((Join-Path $RepositoryRoot $_))
    }
  )
  if (-not [string]::IsNullOrWhiteSpace($ReleaseOutputDirectory)) {
    $candidates += [System.IO.Path]::GetFullPath($ReleaseOutputDirectory)
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:CARGO_TARGET_DIR)) {
    $cargoTarget = [string]$env:CARGO_TARGET_DIR
    if (-not [System.IO.Path]::IsPathRooted($cargoTarget)) {
      $cargoTarget = Join-Path $RepositoryRoot $cargoTarget
    }
    $candidates += [System.IO.Path]::GetFullPath($cargoTarget)
  }

  $seen = New-Object 'System.Collections.Generic.HashSet[string]' (
    [System.StringComparer]::OrdinalIgnoreCase
  )
  $roots = @(
    foreach ($candidate in $candidates) {
      $resolved = [System.IO.Path]::GetFullPath([string]$candidate).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
      )
      if (
        (Test-PublicReleasePathUnderOrEqualRoot `
          -Path $resolved `
          -Root $RepositoryRoot) -and
        $seen.Add($resolved)
      ) {
        $resolved
      }
    }
  )
  [System.Array]::Sort($roots, [System.StringComparer]::OrdinalIgnoreCase)
  return [string[]]$roots
}

function Test-PublicReleaseSourceMutationPathExcluded {
  param(
    [Parameter(Mandatory = $true)][string]$FullPath,
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [string[]]$ExcludedRoots
  )

  foreach ($root in @($ExcludedRoots)) {
    if (Test-PublicReleasePathUnderOrEqualRoot -Path $FullPath -Root $root) {
      return $true
    }
  }
  return $false
}

function Assert-PublicReleaseSourceMutationExclusionsSafe {
  param(
    [Parameter(Mandatory = $true)]$SourceSnapshot,
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [string[]]$ExcludedRoots
  )

  $trackedIndexBytes = [System.Convert]::FromBase64String(
    [string]$SourceSnapshot.tracked_index_base64
  )
  $trackedInventory = Get-PublicReleaseTrackedInventory `
    -IndexBytes $trackedIndexBytes
  foreach ($relativePath in @($trackedInventory.paths)) {
    $fullPath = Assert-PublicReleaseManifestPathSafe `
      -RepositoryRoot $RepositoryRoot `
      -RelativePath ([string]$relativePath)
    if (Test-PublicReleaseSourceMutationPathExcluded `
        -FullPath $fullPath `
        -ExcludedRoots $ExcludedRoots) {
      throw "A tracked public-release source is inside a generated-root mutation-monitor exclusion."
    }
  }
}

function Get-PublicReleaseSourceMutationRelativePath {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)][string]$FullPath
  )

  $resolvedRoot = [System.IO.Path]::GetFullPath($RepositoryRoot).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
  $resolvedPath = [System.IO.Path]::GetFullPath($FullPath)
  if (-not (Test-PublicReleasePathUnderOrEqualRoot `
      -Path $resolvedPath `
      -Root $resolvedRoot)) {
    return $null
  }
  if ([string]::Equals(
      $resolvedPath.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
      ),
      $resolvedRoot,
      [System.StringComparison]::OrdinalIgnoreCase
    )) {
    return "."
  }
  return $resolvedPath.Substring(
    $resolvedRoot.Length + 1
  ).Replace("\", "/")
}

function Start-PublicReleaseSourceMutationMonitor {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [string[]]$ExcludedRoots
  )

  $resolvedRepositoryRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
  if (-not (Test-Path -LiteralPath $resolvedRepositoryRoot -PathType Container)) {
    throw "Public-release source mutation monitor requires a repository directory."
  }
  $watcher = New-Object System.IO.FileSystemWatcher
  $sourceIdentifiers = @()
  try {
    $watcher.Path = $resolvedRepositoryRoot
    $watcher.Filter = "*"
    $watcher.IncludeSubdirectories = $true
    $watcher.InternalBufferSize = 65536
    $watcher.NotifyFilter = (
      [System.IO.NotifyFilters]::FileName -bor
      [System.IO.NotifyFilters]::DirectoryName -bor
      [System.IO.NotifyFilters]::LastWrite -bor
      [System.IO.NotifyFilters]::Size -bor
      [System.IO.NotifyFilters]::CreationTime -bor
      [System.IO.NotifyFilters]::Security
    )
    $monitorId = [guid]::NewGuid().ToString("N")
    foreach ($eventName in @("Changed", "Created", "Deleted", "Renamed", "Error")) {
      $sourceIdentifier = "vectorgrid-public-source-$monitorId-$($eventName.ToLowerInvariant())"
      Register-ObjectEvent `
        -InputObject $watcher `
        -EventName $eventName `
        -SourceIdentifier $sourceIdentifier | Out-Null
      $sourceIdentifiers += $sourceIdentifier
    }
    $watcher.EnableRaisingEvents = $true
    return [pscustomobject][ordered]@{
      schema_version = 1
      repository_root = $resolvedRepositoryRoot
      excluded_roots = [string[]]@($ExcludedRoots)
      source_identifiers = [string[]]$sourceIdentifiers
      watcher = $watcher
      started_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      completed = $false
      evidence = $null
    }
  } catch {
    foreach ($sourceIdentifier in $sourceIdentifiers) {
      Unregister-Event -SourceIdentifier $sourceIdentifier -ErrorAction SilentlyContinue
      Remove-Event -SourceIdentifier $sourceIdentifier -ErrorAction SilentlyContinue
    }
    $watcher.Dispose()
    throw
  }
}

function Complete-PublicReleaseSourceMutationMonitor {
  param([Parameter(Mandatory = $true)]$Monitor)

  if ([bool]$Monitor.completed) {
    return $Monitor.evidence
  }
  $events = @()
  $infrastructureErrors = @()
  try {
    if ($null -eq $Monitor.watcher -or -not $Monitor.watcher.EnableRaisingEvents) {
      $infrastructureErrors += "error`tmonitor-not-active"
    }
    if (@($Monitor.source_identifiers).Count -ne 5) {
      $infrastructureErrors += "error`tmonitor-subscription-count"
    }
    foreach ($sourceIdentifier in @($Monitor.source_identifiers)) {
      $subscribers = @(
        Get-EventSubscriber `
          -SourceIdentifier $sourceIdentifier `
          -ErrorAction SilentlyContinue
      )
      if ($subscribers.Count -ne 1) {
        $infrastructureErrors += "error`tmonitor-subscription-missing"
      }
    }
    Start-Sleep -Milliseconds 125
    $Monitor.watcher.EnableRaisingEvents = $false
    $Monitor.watcher.Dispose()
    Start-Sleep -Milliseconds 125
    foreach ($sourceIdentifier in @($Monitor.source_identifiers)) {
      $events += @(
        Get-Event `
          -SourceIdentifier $sourceIdentifier `
          -ErrorAction SilentlyContinue
      )
    }

    $descriptors = @($infrastructureErrors)
    $errorCount = [int]$infrastructureErrors.Count
    foreach ($eventRecord in $events) {
      $eventName = [string]$eventRecord.SourceIdentifier.Split("-")[-1]
      if ($eventName -eq "error") {
        $errorCount += 1
        $exception = $eventRecord.SourceEventArgs.GetException()
        $exceptionType = if ($null -eq $exception) {
          "unknown"
        } else {
          [string]$exception.GetType().FullName
        }
        $descriptors += "error`t$exceptionType"
        continue
      }

      $eventArgs = $eventRecord.SourceEventArgs
      $fullPath = [string]$eventArgs.FullPath
      $oldFullPath = if ($eventName -eq "renamed") {
        [string]$eventArgs.OldFullPath
      } else {
        ""
      }
      $newExcluded = (
        -not [string]::IsNullOrWhiteSpace($fullPath) -and
        (Test-PublicReleaseSourceMutationPathExcluded `
          -FullPath $fullPath `
          -ExcludedRoots @($Monitor.excluded_roots))
      )
      $oldExcluded = (
        -not [string]::IsNullOrWhiteSpace($oldFullPath) -and
        (Test-PublicReleaseSourceMutationPathExcluded `
          -FullPath $oldFullPath `
          -ExcludedRoots @($Monitor.excluded_roots))
      )
      if (
        ($eventName -ne "renamed" -and $newExcluded) -or
        ($eventName -eq "renamed" -and $newExcluded -and $oldExcluded)
      ) {
        continue
      }

      $relativePath = Get-PublicReleaseSourceMutationRelativePath `
        -RepositoryRoot $Monitor.repository_root `
        -FullPath $fullPath
      $oldRelativePath = if ($eventName -eq "renamed") {
        Get-PublicReleaseSourceMutationRelativePath `
          -RepositoryRoot $Monitor.repository_root `
          -FullPath $oldFullPath
      } else {
        $null
      }
      if (
        [string]::IsNullOrWhiteSpace([string]$relativePath) -or
        ($eventName -eq "renamed" -and
          [string]::IsNullOrWhiteSpace([string]$oldRelativePath))
      ) {
        $errorCount += 1
        $descriptors += "error`tpath-outside-repository"
        continue
      }
      if ($eventName -eq "renamed") {
        $descriptors += "renamed`t$oldRelativePath`t$relativePath"
      } else {
        $descriptors += "$eventName`t$relativePath"
      }
    }
    [System.Array]::Sort($descriptors, [System.StringComparer]::Ordinal)
    $descriptorBytes = [System.Text.Encoding]::UTF8.GetBytes(
      ($descriptors -join "`n")
    )
    $eventCount = [int]($descriptors.Count - $errorCount)
    $status = if ($errorCount -gt 0) {
      "error"
    } elseif ($eventCount -gt 0) {
      "mutated"
    } else {
      "passed"
    }
    $evidence = [pscustomobject][ordered]@{
      schema_version = 1
      status = $status
      started_at_utc = [string]$Monitor.started_at_utc
      completed_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      include_subdirectories = $true
      internal_buffer_size_bytes = 65536
      excluded_root_count = [int]@($Monitor.excluded_roots).Count
      event_count = $eventCount
      error_count = [int]$errorCount
      event_sha256 = Get-BytesSha256 -Bytes $descriptorBytes
    }
    $Monitor.evidence = $evidence
    $Monitor.completed = $true
    return $evidence
  } finally {
    foreach ($sourceIdentifier in @($Monitor.source_identifiers)) {
      Remove-Event -SourceIdentifier $sourceIdentifier -ErrorAction SilentlyContinue
      Unregister-Event -SourceIdentifier $sourceIdentifier -ErrorAction SilentlyContinue
    }
    if ($null -ne $Monitor.watcher) {
      $Monitor.watcher.Dispose()
    }
  }
}

function Assert-PublicReleaseSourceMutationMonitorClean {
  param([Parameter(Mandatory = $true)]$Evidence)

  if (
    [int]$Evidence.schema_version -ne 1 -or
    [string]$Evidence.status -eq "error" -or
    [int]$Evidence.error_count -ne 0
  ) {
    throw "Public-release source mutation monitoring failed or overflowed; source stability cannot be proven."
  }
  if (
    [string]$Evidence.status -ne "passed" -or
    [int]$Evidence.event_count -ne 0
  ) {
    throw "An in-scope working-tree mutation event occurred during the public release run."
  }
}

function Normalize-Thumbprint {
  param([string]$Value)
  if ([string]::IsNullOrWhiteSpace($Value)) {
    return ""
  }
  return ($Value -replace '\s', '').ToUpperInvariant()
}

function Assert-PathUnderRoot {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string]$Purpose
  )

  $resolvedPath = [System.IO.Path]::GetFullPath($Path)
  $resolvedRoot = [System.IO.Path]::GetFullPath($Root)
  if (-not $resolvedRoot.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
    $resolvedRoot += [System.IO.Path]::DirectorySeparatorChar
  }
  if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "$Purpose resolved outside the release output directory: $resolvedPath"
  }
}

function Test-PathEquals {
  param(
    [string]$Left,
    [string]$Right
  )

  if (
    [string]::IsNullOrWhiteSpace($Left) -or
    [string]::IsNullOrWhiteSpace($Right)
  ) {
    return $false
  }
  return [string]::Equals(
    [System.IO.Path]::GetFullPath($Left),
    [System.IO.Path]::GetFullPath($Right),
    [System.StringComparison]::OrdinalIgnoreCase
  )
}

function Get-CodeSignatureInfo {
  param([Parameter(Mandatory = $true)][string]$Path)

  $signature = Get-AuthenticodeSignature -LiteralPath $Path
  return [ordered]@{
    checked = $true
    status = [string]$signature.Status
    signer_subject = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Subject } else { $null }
    thumbprint = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Thumbprint } else { $null }
    timestamp_subject = if ($null -ne $signature.TimeStamperCertificate) { [string]$signature.TimeStamperCertificate.Subject } else { $null }
    timestamp_thumbprint = if ($null -ne $signature.TimeStamperCertificate) { [string]$signature.TimeStamperCertificate.Thumbprint } else { $null }
  }
}

function Invoke-PublicReleaseStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [string]$CommandText,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Action
  )

  $startedAt = (Get-Date).ToUniversalTime()
  Write-Host "==> $Name"
  try {
    & $Action
    $script:StepResults += [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $true
    }
    Write-Host "[PASS] $Name"
  } catch {
    $script:StepResults += [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $false
      error = $_.Exception.Message
    }
    throw
  }
}

function Assert-CleanWorktree {
  param([Parameter(Mandatory = $true)]$SourceSnapshot)

  if ($AllowDirtyWorktree) {
    Write-Warning "Dirty worktree allowed for this run. Do not publish this output as a final public release."
    return
  }
  if ([bool]$SourceSnapshot.dirty -or [int64]$SourceSnapshot.status_size_bytes -ne 0) {
    throw "Public release requires a clean git worktree. Commit or stash changes, or pass -AllowDirtyWorktree only for a non-publishable rehearsal."
  }
}

function Get-PublicReleaseSourceSnapshotEvidence {
  param($Snapshot)

  if ($null -eq $Snapshot) {
    return $null
  }
  return [ordered]@{
    schema_version = [int]$Snapshot.schema_version
    head = [string]$Snapshot.head
    status_command = [string]$Snapshot.status_command
    status_encoding = [string]$Snapshot.status_encoding
    status_size_bytes = [int64]$Snapshot.status_size_bytes
    status_sha256 = [string]$Snapshot.status_sha256
    tracked_diff_size_bytes = [int64]$Snapshot.tracked_diff_size_bytes
    tracked_diff_sha256 = [string]$Snapshot.tracked_diff_sha256
    tracked_index_file_count = [int]$Snapshot.tracked_index_file_count
    tracked_index_size_bytes = [int64]$Snapshot.tracked_index_size_bytes
    tracked_index_sha256 = [string]$Snapshot.tracked_index_sha256
    tracked_manifest_file_count = [int]$Snapshot.tracked_manifest_file_count
    tracked_manifest_size_bytes = [int64]$Snapshot.tracked_manifest_size_bytes
    tracked_manifest_sha256 = [string]$Snapshot.tracked_manifest_sha256
    untracked_manifest_file_count =
      [int]$Snapshot.untracked_manifest_file_count
    untracked_manifest_size_bytes =
      [int64]$Snapshot.untracked_manifest_size_bytes
    untracked_manifest_sha256 =
      [string]$Snapshot.untracked_manifest_sha256
    source_read_lease_count = [int]$Snapshot.source_read_lease_count
    source_state_sha256 = [string]$Snapshot.source_state_sha256
    dirty = [bool]$Snapshot.dirty
  }
}

function Get-PublicReleaseSourceMutationMonitorEvidence {
  param($Evidence)

  if ($null -eq $Evidence) {
    return $null
  }
  return [ordered]@{
    schema_version = [int]$Evidence.schema_version
    status = [string]$Evidence.status
    started_at_utc = [string]$Evidence.started_at_utc
    completed_at_utc = [string]$Evidence.completed_at_utc
    include_subdirectories = [bool]$Evidence.include_subdirectories
    internal_buffer_size_bytes = [int]$Evidence.internal_buffer_size_bytes
    excluded_root_count = [int]$Evidence.excluded_root_count
    event_count = [int]$Evidence.event_count
    error_count = [int]$Evidence.error_count
    event_sha256 = [string]$Evidence.event_sha256
  }
}

function Test-PublicReleaseEligibility {
  param([Parameter(Mandatory = $true)][string]$Status)

  return (
    $Status -eq "passed" -and
    -not [bool]$AllowDirtyWorktree -and
    -not [bool]$AllowUnsignedBeta -and
    [bool]$script:SourceStateRevalidated -and
    [bool]$script:SourceMutationMonitorRevalidated -and
    [bool]$script:SourceReadLeaseRevalidated -and
    $null -ne $script:InitialSourceSnapshot -and
    $null -ne $script:FinalSourceSnapshot -and
    $null -ne $script:SourceMutationMonitorEvidence -and
    [string]$script:SourceMutationMonitorEvidence.status -eq "passed" -and
    [int]$script:SourceMutationMonitorEvidence.event_count -eq 0 -and
    [int]$script:SourceMutationMonitorEvidence.error_count -eq 0 -and
    -not [bool]$script:InitialSourceSnapshot.dirty -and
    -not [bool]$script:FinalSourceSnapshot.dirty -and
    [string]$script:FinalSourceSnapshot.head -eq
      [string]$script:InitialSourceSnapshot.head -and
    [string]$script:FinalSourceSnapshot.status_sha256 -eq
      [string]$script:InitialSourceSnapshot.status_sha256 -and
    [string]$script:FinalSourceSnapshot.source_state_sha256 -eq
      [string]$script:InitialSourceSnapshot.source_state_sha256
  )
}

function Assert-PublicReleaseEvidence {
  $candidateSummary = Read-JsonFile -Path (Join-Path $resolvedOutDir "summary.json")
  $manifest = Read-JsonFile -Path (Join-Path $resolvedOutDir "manifest.json")
  $smoke = Read-JsonFile -Path (Join-Path $resolvedOutDir "packaged-smoke-summary.json")

  if ($null -eq $candidateSummary -or [string]$candidateSummary.status -ne "passed") {
    throw "Release candidate gate did not pass."
  }
  $workflowAuthorityProperty = $candidateSummary.PSObject.Properties["workflow_authority_complete"]
  if ($null -eq $workflowAuthorityProperty -or -not [bool]$workflowAuthorityProperty.Value) {
    throw "Public release requires complete workflow authority; a candidate created with -SkipWorkflowGate is rehearsal-only."
  }
  $liveSafetyContractScript = Join-Path $repoRoot "scripts\\public_live_safety_evidence_contract.mjs"
  $liveSafetyContractArgs = @(
    $liveSafetyContractScript,
    "--out-dir", $resolvedOutDir,
    "--candidate-summary-path", (Join-Path $resolvedOutDir "summary.json"),
    "--variant", $Variant
  )
  $liveSafetyContractOutput = (& node @liveSafetyContractArgs 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) {
    throw "Public live-execution safety evidence revalidation failed: $($liveSafetyContractOutput.Trim())"
  }
  $script:LiveSafetyEvidenceVerification = $liveSafetyContractOutput | ConvertFrom-Json
  if (
    [int]$script:LiveSafetyEvidenceVerification.schema_version -ne 1 -or
    [string]$script:LiveSafetyEvidenceVerification.contract -ne "vectorgrid-public-live-safety-evidence-v1" -or
    [string]$script:LiveSafetyEvidenceVerification.status -ne "passed" -or
    [string]$script:LiveSafetyEvidenceVerification.variant -ne $Variant -or
    [string]$script:LiveSafetyEvidenceVerification.authority.execution_safety_gate -ne "passed" -or
    [string]$script:LiveSafetyEvidenceVerification.authority.live_safety_harness -ne "passed" -or
    -not [bool]$script:LiveSafetyEvidenceVerification.authority.authority_eligible -or
    [int]$script:LiveSafetyEvidenceVerification.authority.required_test_count -ne 128
  ) {
    throw "Public live-execution safety evidence revalidation returned incomplete or mismatched authority."
  }
  if ($IncludeOptionsGpuPromotion) {
    $optionsPromotionProperty = $candidateSummary.PSObject.Properties["include_options_gpu_promotion"]
    if ($null -eq $optionsPromotionProperty -or -not [bool]$optionsPromotionProperty.Value) {
      throw "Public options GPU evidence requires a candidate created with -IncludeOptionsGpuPromotion."
    }
    $optionsPromotionStep = @($candidateSummary.steps | Where-Object {
      [string]$_.name -eq "Options GPU promotion authority"
    })
    if ($optionsPromotionStep.Count -ne 1 -or -not [bool]$optionsPromotionStep[0].passed) {
      throw "Public options GPU evidence requires one passing Options GPU promotion authority candidate step."
    }
    $optionsPromotionSummaryPath = Join-Path $resolvedOutDir "options-gpu-promotion-summary.json"
    $optionsPromotionSummary = Read-JsonFile -Path $optionsPromotionSummaryPath
    if ($null -eq $optionsPromotionSummary -or [string]$optionsPromotionSummary.mode -ne "candidate") {
      throw "Public options GPU evidence requires a candidate-mode options promotion summary."
    }
    $correctnessEvidence = @($optionsPromotionSummary.correctness_evidence)
    if (
      $correctnessEvidence.Count -lt 3 -or
      @($correctnessEvidence | Where-Object {
        -not [bool]$_.passed -or [int]$_.passed_test_count -lt 1
      }).Count -gt 0
    ) {
      throw "Public options GPU evidence requires passing CPU options tests plus GPU aggregate and ranking parity tests."
    }
    $requiredCorrectnessNames = @(
      "Options expression CPU test suite",
      "Options expression GPU aggregate parity",
      "Options expression CPU/GPU ranking parity"
    )
    foreach ($requiredCorrectnessName in $requiredCorrectnessNames) {
      if (@($correctnessEvidence | Where-Object { [string]$_.name -eq $requiredCorrectnessName }).Count -ne 1) {
        throw "Public options GPU evidence is missing required correctness evidence '$requiredCorrectnessName'."
      }
    }
    $promotionChecks = @($optionsPromotionSummary.checks)
    $baselineChecks = @($optionsPromotionSummary.baseline_comparison)
    if (
      $promotionChecks.Count -eq 0 -or
      @($promotionChecks | Where-Object { -not [bool]$_.Passed }).Count -gt 0 -or
      $baselineChecks.Count -eq 0 -or
      @($baselineChecks | Where-Object { -not [bool]$_.Passed }).Count -gt 0
    ) {
      throw "Public options GPU evidence requires passing promotion and baseline-regression checks."
    }
  }
  if ($null -eq $manifest) {
    throw "Release manifest is missing."
  }
  if (
    [string]$candidateSummary.variant -ne $Variant -or
    [string]$manifest.variant -ne $Variant
  ) {
    throw "Public release evidence belongs to a different release variant."
  }
  if (-not $AllowUnsignedBeta -and ([bool]$manifest.unsigned_beta -or -not [bool]$manifest.installer_signature_eligible)) {
    throw "Public release requires installer-signature-eligible staging evidence. Re-run without unsigned beta labeling after bundle-time signing."
  }
  if ($AllowUnsignedBeta -and -not [bool]$manifest.unsigned_beta) {
    throw "Unsigned beta rehearsal requires manifest.unsigned_beta=true and beta-labeled artifacts."
  }
  if ([string]$manifest.artifact_source_kind -ne "bundle") {
    throw "Public release requires staged bundle artifacts, not only a raw release executable."
  }
  if ($null -eq $smoke -or [string]$smoke.status -ne "passed" -or [string]$smoke.smoke_mode -ne "bundle") {
    throw "Public release requires a passing bundle-mode packaged smoke."
  }
  if (
    [string]$candidateSummary.desktop_suite -ne "release-proof" -or
    [string]$smoke.desktop_suite -ne "release-proof" -or
    -not [bool]$smoke.exact_package_release_proof -or
    [string]$smoke.release_variant -ne $Variant
  ) {
    throw "Public release requires the exact-package release-proof desktop suite for the current release variant."
  }
  $profileIsolation = $smoke.profile_isolation
  $appAttestation = if ($null -ne $profileIsolation) {
    $profileIsolation.app_attestation
  } else {
    $null
  }
  $expectedIsolatedAppData = if (
    -not [string]::IsNullOrWhiteSpace([string]$smoke.isolated_profile_root)
  ) {
    [System.IO.Path]::GetFullPath(
      (Join-Path ([string]$smoke.isolated_profile_root) "LocalAppData\\dev.vectoralpha.vectorgrid")
    )
  } else {
    ""
  }
  if (
    $null -eq $profileIsolation -or
    [int]$profileIsolation.schema_version -ne 2 -or
    [string]$profileIsolation.status -ne "passed" -or
    -not [bool]$profileIsolation.real_profile_unchanged -or
    -not [bool]$profileIsolation.effective_isolation_proven -or
    [string]$profileIsolation.teardown -ne "completed" -or
    $null -eq $appAttestation -or
    [int]$appAttestation.schema_version -ne 2 -or
    [string]$appAttestation.kind -ne "vectorgrid-qa-profile-attestation" -or
    [string]$appAttestation.run_id -ne [string]$smoke.run_id -or
    [string]$appAttestation.original_identifier -ne "dev.vectoralpha.vectorgrid" -or
    -not (Test-PathEquals -Left ([string]$appAttestation.isolation_root) -Right ([string]$smoke.isolated_profile_root)) -or
    -not (Test-PathEquals -Left ([string]$appAttestation.effective_identifier) -Right $expectedIsolatedAppData) -or
    -not (Test-PathEquals -Left ([string]$appAttestation.app_data_dir) -Right $expectedIsolatedAppData)
  ) {
    throw "Public release requires schema-2 isolated-profile evidence with an exact matching application attestation, completed teardown, and the real profile unchanged."
  }
  if (-not [bool]$smoke.runtime_executed -or [string]$smoke.binary_origin -ne "extracted-bundle") {
    throw "Public release packaged smoke must execute the product binary extracted from the staged installer."
  }
  $releaseProofEvidence = $smoke.release_proof_evidence
  if (
    $null -eq $releaseProofEvidence -or
    [int]$releaseProofEvidence.schema_version -ne 1 -or
    [string]$releaseProofEvidence.contract -ne "vectorgrid-release-proof-evidence-manifest-v1" -or
    [string]$releaseProofEvidence.status -ne "passed" -or
    [string]$releaseProofEvidence.run_id -ne [string]$smoke.run_id -or
    [string]$releaseProofEvidence.release_variant -ne $Variant -or
    [int]$releaseProofEvidence.required_journey_count -le 0 -or
    @($releaseProofEvidence.journey_ids).Count -ne [int]$releaseProofEvidence.required_journey_count
  ) {
    throw "Public release requires one passing, current-run release-proof manifest covering every allowlisted native journey."
  }
  $releaseProofEvidenceRoot = [string]$releaseProofEvidence.evidence_root
  $releaseProofEvidenceManifestPath = [string]$releaseProofEvidence.manifest_path
  if (
    [string]::IsNullOrWhiteSpace($releaseProofEvidenceRoot) -or
    [string]::IsNullOrWhiteSpace($releaseProofEvidenceManifestPath) -or
    -not (Test-Path -LiteralPath $releaseProofEvidenceRoot -PathType Container) -or
    -not (Test-Path -LiteralPath $releaseProofEvidenceManifestPath -PathType Leaf)
  ) {
    throw "Public release-proof journey evidence files are missing."
  }
  Assert-PathUnderRoot -Path $releaseProofEvidenceRoot -Root $resolvedOutDir -Purpose "Release-proof evidence root"
  Assert-PathUnderRoot -Path $releaseProofEvidenceManifestPath -Root $resolvedOutDir -Purpose "Release-proof evidence manifest"
  if (-not (Test-PathEquals -Left (Split-Path -Parent $releaseProofEvidenceManifestPath) -Right $releaseProofEvidenceRoot)) {
    throw "Release-proof evidence manifest must live directly under its recorded evidence root."
  }
  $releaseProofEvidenceRootItem = Get-Item -LiteralPath $releaseProofEvidenceRoot -Force
  $releaseProofEvidenceManifestItem = Get-Item -LiteralPath $releaseProofEvidenceManifestPath -Force
  if (
    ($releaseProofEvidenceRootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
    ($releaseProofEvidenceManifestItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
    [int64]$releaseProofEvidenceManifestItem.Length -ne [int64]$releaseProofEvidence.manifest_size_bytes -or
    (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseProofEvidenceManifestPath).Hash.ToLowerInvariant() -ne [string]$releaseProofEvidence.manifest_sha256
  ) {
    throw "Release-proof evidence manifest identity changed after packaged smoke."
  }
  $releaseProofExtractionRoot = [string]$smoke.bundle_extraction_root
  $releaseProofBinaryPath = [string]$smoke.binary_path
  if (
    [string]::IsNullOrWhiteSpace($releaseProofExtractionRoot) -or
    -not (Test-Path -LiteralPath $releaseProofExtractionRoot -PathType Container) -or
    [string]::IsNullOrWhiteSpace($releaseProofBinaryPath) -or
    -not (Test-Path -LiteralPath $releaseProofBinaryPath -PathType Leaf)
  ) {
    throw "Release-proof packaged-binary evidence is missing."
  }
  Assert-PathUnderRoot -Path $releaseProofExtractionRoot -Root $resolvedOutDir -Purpose "Release-proof package extraction root"
  Assert-PathUnderRoot -Path $releaseProofBinaryPath -Root $releaseProofExtractionRoot -Purpose "Release-proof packaged binary"
  $releaseProofExtractionRootItem = Get-Item -LiteralPath $releaseProofExtractionRoot -Force
  $releaseProofBinaryItem = Get-Item -LiteralPath $releaseProofBinaryPath -Force
  if (
    ($releaseProofExtractionRootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
    ($releaseProofBinaryItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  ) {
    throw "Release-proof packaged-binary evidence may not be read through a symlink or junction."
  }
  $releaseProofContractScript = Join-Path $repoRoot "scripts\\release_proof_evidence_contract.mjs"
  $releaseProofContractArgs = @(
    $releaseProofContractScript,
    "--mode", "verify",
    "--evidence-root", $releaseProofEvidenceRoot,
    "--run-id", ([string]$smoke.run_id),
    "--variant", $Variant,
    "--binary-path", ([string]$smoke.binary_path),
    "--binary-size-bytes", ([string]$smoke.binary_size_bytes),
    "--binary-sha256", ([string]$smoke.binary_sha256),
    "--manifest-path", $releaseProofEvidenceManifestPath
  )
  $releaseProofContractOutput = (& node @releaseProofContractArgs 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) {
    throw "Public release-proof journey evidence revalidation failed: $($releaseProofContractOutput.Trim())"
  }
  $script:ReleaseProofEvidenceVerification = $releaseProofContractOutput | ConvertFrom-Json
  if (
    [string]$script:ReleaseProofEvidenceVerification.manifest.status -ne "passed" -or
    [int]$script:ReleaseProofEvidenceVerification.manifest.required_journey_count -ne [int]$releaseProofEvidence.required_journey_count -or
    [string]$script:ReleaseProofEvidenceVerification.manifest_sha256 -ne [string]$releaseProofEvidence.manifest_sha256
  ) {
    throw "Public release-proof journey evidence revalidation returned incomplete or mismatched authority."
  }

  $artifacts = @($manifest.staged_artifacts)
  $stagedArtifactsDir = [string]$manifest.staged_artifacts_dir
  if (
    [string]::IsNullOrWhiteSpace($stagedArtifactsDir) -or
    -not (Test-Path -LiteralPath $stagedArtifactsDir -PathType Container)
  ) {
    throw "Release manifest staged-artifacts directory is missing."
  }
  Assert-PathUnderRoot -Path $stagedArtifactsDir -Root $resolvedOutDir -Purpose "Staged release artifacts directory"
  $stagingDirItem = Get-Item -LiteralPath $stagedArtifactsDir -Force
  if (($stagingDirItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Staged release artifacts directory must not be a symlink or junction."
  }
  $actualArtifactEntries = @(Get-ChildItem -LiteralPath $stagedArtifactsDir -Force)
  if (@($actualArtifactEntries | Where-Object {
    $_.PSIsContainer -or
    ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  }).Count -gt 0) {
    throw "Staged release artifacts directory may contain only ordinary files."
  }
  $expectedArtifactNames = @($artifacts | ForEach-Object { [string]$_.staged_name } | Sort-Object -Unique)
  $actualArtifactNames = @($actualArtifactEntries | ForEach-Object { [string]$_.Name } | Sort-Object -Unique)
  if (
    $expectedArtifactNames.Count -ne $artifacts.Count -or
    $actualArtifactNames.Count -ne $expectedArtifactNames.Count -or
    @(Compare-Object -ReferenceObject $expectedArtifactNames -DifferenceObject $actualArtifactNames).Count -gt 0
  ) {
    throw "Staged release artifacts directory contains files not declared exactly once by manifest.staged_artifacts."
  }
  $setupArtifacts = @($artifacts | Where-Object {
    [string]$_.source_kind -eq "bundle" -and @(".exe", ".msi") -contains [string]$_.extension
  })
  if ($setupArtifacts.Count -eq 0) {
    throw "Public release manifest contains no staged installer artifact."
  }
  $smokedBundleSha256 = [string]$smoke.bundle_artifact_sha256
  $matchingSmokedBundle = @($setupArtifacts | Where-Object {
    -not [string]::IsNullOrWhiteSpace($smokedBundleSha256) -and
    [string]$_.sha256 -eq $smokedBundleSha256
  })
  if ($matchingSmokedBundle.Count -ne 1) {
    throw "Packaged smoke did not execute content extracted from exactly one staged installer in the release manifest."
  }

  $hashFilePath = [string]$manifest.hash_file
  if ([string]::IsNullOrWhiteSpace($hashFilePath) -or -not (Test-Path -LiteralPath $hashFilePath -PathType Leaf)) {
    throw "Release checksum file is missing."
  }
  Assert-PathUnderRoot -Path $hashFilePath -Root $resolvedOutDir -Purpose "Release checksum file"
  $checksumEntries = @{}
  foreach ($line in @(Get-Content -LiteralPath $hashFilePath)) {
    if ([string]::IsNullOrWhiteSpace($line)) {
      continue
    }
    if ($line -notmatch '^([A-Fa-f0-9]{64}) \*(.+)$') {
      throw "Release checksum file contains a malformed entry: $line"
    }
    $relativePath = $Matches[2].Replace([char]92, [char]47).ToLowerInvariant()
    if ($checksumEntries.ContainsKey($relativePath)) {
      throw "Release checksum file contains a duplicate entry: $relativePath"
    }
    $checksumEntries[$relativePath] = $Matches[1].ToLowerInvariant()
  }
  if ($checksumEntries.Count -ne $artifacts.Count) {
    throw "Release checksum entry count does not match the staged artifact count."
  }

  $finalArtifactHashes = @()
  foreach ($artifact in $artifacts) {
    $artifactPath = [string]$artifact.staged_path
    if ([string]::IsNullOrWhiteSpace($artifactPath) -or -not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
      throw "Staged release artifact is missing: $artifactPath"
    }
    Assert-PathUnderRoot -Path $artifactPath -Root $resolvedOutDir -Purpose "Staged release artifact"
    if (-not (Test-PathEquals -Left (Split-Path -Parent $artifactPath) -Right $stagedArtifactsDir)) {
      throw "Staged release artifact is not an immediate child of manifest.staged_artifacts_dir: $artifactPath"
    }
    $actualSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifactPath).Hash.ToLowerInvariant()
    $manifestSha256 = ([string]$artifact.sha256).ToLowerInvariant()
    $checksumKey = ("artifacts/" + [string]$artifact.staged_name).ToLowerInvariant()
    if ($actualSha256 -ne $manifestSha256) {
      throw "Staged artifact hash changed after manifest generation: $artifactPath"
    }
    if (-not $checksumEntries.ContainsKey($checksumKey) -or $checksumEntries[$checksumKey] -ne $actualSha256) {
      throw "Staged artifact hash does not match SHA256SUMS.txt: $artifactPath"
    }
    $finalArtifactHashes += [ordered]@{
      staged_path = $artifactPath
      staged_name = [string]$artifact.staged_name
      extension = [string]$artifact.extension
      sha256 = $actualSha256
      size_bytes = [int64](Get-Item -LiteralPath $artifactPath).Length
      code_signature = if (@(".exe", ".msi") -contains [string]$artifact.extension) {
        Get-CodeSignatureInfo -Path $artifactPath
      } else {
        $null
      }
    }
  }
  $script:FinalArtifactHashes = $finalArtifactHashes

  $extractionRoot = [string]$smoke.bundle_extraction_root
  $embeddedApplicationPath = [string]$smoke.binary_path
  if (
    [string]::IsNullOrWhiteSpace($extractionRoot) -or
    -not (Test-Path -LiteralPath $extractionRoot -PathType Container) -or
    [string]::IsNullOrWhiteSpace($embeddedApplicationPath) -or
    -not (Test-Path -LiteralPath $embeddedApplicationPath -PathType Leaf)
  ) {
    throw "Packaged smoke extracted-application evidence is missing."
  }
  Assert-PathUnderRoot -Path $extractionRoot -Root $resolvedOutDir -Purpose "Packaged smoke extraction root"
  Assert-PathUnderRoot -Path $embeddedApplicationPath -Root $extractionRoot -Purpose "Packaged smoke embedded application"
  $finalEmbeddedSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $embeddedApplicationPath).Hash.ToLowerInvariant()
  if ($finalEmbeddedSha256 -ne ([string]$smoke.binary_sha256).ToLowerInvariant()) {
    throw "Embedded application hash changed after packaged smoke."
  }
  $finalEmbeddedSignature = Get-CodeSignatureInfo -Path $embeddedApplicationPath
  $script:FinalEmbeddedApplication = [ordered]@{
    path = $embeddedApplicationPath
    sha256 = $finalEmbeddedSha256
    size_bytes = [int64](Get-Item -LiteralPath $embeddedApplicationPath).Length
    code_signature = $finalEmbeddedSignature
  }

  if (-not $AllowUnsignedBeta) {
    $unsignedOrInvalid = @($setupArtifacts | Where-Object {
      $null -eq $_.code_signature -or [string]$_.code_signature.status -ne "Valid"
    })
    if ($unsignedOrInvalid.Count -gt 0) {
      $names = ($unsignedOrInvalid | ForEach-Object { [string]$_.staged_name }) -join ", "
      throw "Public release requires valid Authenticode signatures for installer artifacts. Invalid or unsigned: $names"
    }
    if ($null -eq $smoke.binary_code_signature -or [string]$smoke.binary_code_signature.status -ne "Valid") {
      throw "Public release requires a valid Authenticode signature on the application executable embedded in the installer."
    }
    if ([string]$finalEmbeddedSignature.status -ne "Valid") {
      throw "Final Authenticode recheck failed for the embedded application."
    }
    $innerThumbprint = Normalize-Thumbprint -Value ([string]$finalEmbeddedSignature.thumbprint)
    if ($innerThumbprint -ne $resolvedExpectedSignerThumbprint) {
      throw "Embedded application signer does not match the expected publisher thumbprint."
    }
    if ([string]::IsNullOrWhiteSpace([string]$finalEmbeddedSignature.timestamp_thumbprint)) {
      throw "Public release requires a trusted timestamp on the embedded application signature."
    }
    $finalSetupArtifacts = @($finalArtifactHashes | Where-Object {
      @(".exe", ".msi") -contains [string]$_.extension
    })
    foreach ($artifact in $finalSetupArtifacts) {
      if ($null -eq $artifact.code_signature -or [string]$artifact.code_signature.status -ne "Valid") {
        throw "Final Authenticode recheck failed for installer '$($artifact.staged_name)'."
      }
      $outerThumbprint = Normalize-Thumbprint -Value ([string]$artifact.code_signature.thumbprint)
      if ($outerThumbprint -ne $resolvedExpectedSignerThumbprint -or $outerThumbprint -ne $innerThumbprint) {
        throw "Installer and embedded application signatures must match the expected publisher."
      }
      if ([string]::IsNullOrWhiteSpace([string]$artifact.code_signature.timestamp_thumbprint)) {
        throw "Public release requires a trusted timestamp on every installer signature."
      }
    }
  } else {
    Write-Warning "Unsigned beta mode enabled. Output may be useful for tester rehearsal but is not public-release complete."
  }
}

function Write-PublicReleaseSummary {
  param(
    [string]$Status,
    [string]$ErrorMessage = ""
  )

  $manifest = Read-JsonFile -Path (Join-Path $resolvedOutDir "manifest.json")
  $smoke = Read-JsonFile -Path (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = $Variant
    status = $Status
    out_dir = $resolvedOutDir
    allow_dirty_worktree = [bool]$AllowDirtyWorktree
    allow_unsigned_beta = [bool]$AllowUnsignedBeta
    include_options_gpu_promotion = [bool]$IncludeOptionsGpuPromotion
    public_release_eligible = (Test-PublicReleaseEligibility -Status $Status)
    source_state_revalidated = [bool]$script:SourceStateRevalidated
    source_mutation_monitor_revalidated =
      [bool]$script:SourceMutationMonitorRevalidated
    source_read_lease_revalidated =
      [bool]$script:SourceReadLeaseRevalidated
    source_mutation_monitor = Get-PublicReleaseSourceMutationMonitorEvidence `
      -Evidence $script:SourceMutationMonitorEvidence
    initial_source_state = Get-PublicReleaseSourceSnapshotEvidence `
      -Snapshot $script:InitialSourceSnapshot
    final_source_state = Get-PublicReleaseSourceSnapshotEvidence `
      -Snapshot $script:FinalSourceSnapshot
    expected_signer_thumbprint = if ([string]::IsNullOrWhiteSpace($resolvedExpectedSignerThumbprint)) { $null } else { $resolvedExpectedSignerThumbprint }
    post_build_sign_script = if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { $null } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) }
    error = if ([string]::IsNullOrWhiteSpace($ErrorMessage)) { $null } else { $ErrorMessage }
    candidate_summary = (Join-Path $resolvedOutDir "summary.json")
    manifest = (Join-Path $resolvedOutDir "manifest.json")
    hashes = (Join-Path $resolvedOutDir "SHA256SUMS.txt")
    packaged_smoke = (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
    live_safety = (Join-Path $resolvedOutDir "live-safety-summary.json")
    options_gpu_promotion = if ($IncludeOptionsGpuPromotion) { (Join-Path $resolvedOutDir "options-gpu-promotion-summary.json") } else { $null }
    artifact_source_kind = if ($null -ne $manifest) { [string]$manifest.artifact_source_kind } else { $null }
    manifest_publishable = if ($null -ne $manifest) { [bool]$manifest.publishable } else { $null }
    manifest_installer_signature_eligible = if ($null -ne $manifest) { [bool]$manifest.installer_signature_eligible } else { $null }
    manifest_unsigned_beta = if ($null -ne $manifest) { [bool]$manifest.unsigned_beta } else { $null }
    smoke_mode = if ($null -ne $smoke) { [string]$smoke.smoke_mode } else { $null }
    desktop_suite = if ($null -ne $smoke) { [string]$smoke.desktop_suite } else { $null }
    exact_package_release_proof = if ($null -ne $smoke) { [bool]$smoke.exact_package_release_proof } else { $false }
    profile_isolation_status = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [string]$smoke.profile_isolation.status } else { $null }
    profile_isolation_schema_version = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [int]$smoke.profile_isolation.schema_version } else { $null }
    real_profile_unchanged = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [bool]$smoke.profile_isolation.real_profile_unchanged } else { $false }
    effective_profile_isolation_proven = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [bool]$smoke.profile_isolation.effective_isolation_proven } else { $false }
    profile_app_attestation = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { $smoke.profile_isolation.app_attestation } else { $null }
    profile_isolation_teardown = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [string]$smoke.profile_isolation.teardown } else { $null }
    release_proof_evidence = if ($null -ne $smoke) { $smoke.release_proof_evidence } else { $null }
    release_proof_evidence_revalidated = if ($null -ne $script:ReleaseProofEvidenceVerification) {
      [ordered]@{
        status = [string]$script:ReleaseProofEvidenceVerification.manifest.status
        required_journey_count = [int]$script:ReleaseProofEvidenceVerification.manifest.required_journey_count
        manifest_path = [string]$script:ReleaseProofEvidenceVerification.manifest_path
        manifest_sha256 = [string]$script:ReleaseProofEvidenceVerification.manifest_sha256
      }
    } else {
      $null
    }
    live_safety_evidence_revalidated = $script:LiveSafetyEvidenceVerification
    final_artifact_hashes = @($script:FinalArtifactHashes)
    final_embedded_application = $script:FinalEmbeddedApplication
    staged_artifacts = if ($null -ne $manifest) { @($manifest.staged_artifacts | ForEach-Object { [string]$_.staged_path }) } else { @() }
    steps = $script:StepResults
  }
  Write-JsonUtf8 -Path $summaryJsonPath -Value $summary

  $lines = @(
    "# Public Release Gate",
    "",
    "- Status: $Status",
    "- Variant: $Variant",
    "- Output: $resolvedOutDir",
    "- Dirty worktree allowed: $([bool]$AllowDirtyWorktree)",
    "- Unsigned beta allowed: $([bool]$AllowUnsignedBeta)",
    "- Include options GPU promotion: $([bool]$IncludeOptionsGpuPromotion)",
    "- Source state revalidated: $([bool]$script:SourceStateRevalidated)",
    "- Source mutation monitor revalidated: $([bool]$script:SourceMutationMonitorRevalidated)",
    "- Source read lease revalidated: $([bool]$script:SourceReadLeaseRevalidated)",
    "- Source mutation monitor status: $(if ($null -ne $script:SourceMutationMonitorEvidence) { [string]$script:SourceMutationMonitorEvidence.status } else { 'not completed' })",
    "- Initial source HEAD: $(if ($null -ne $script:InitialSourceSnapshot) { [string]$script:InitialSourceSnapshot.head } else { 'not captured' })",
    "- Final source HEAD: $(if ($null -ne $script:FinalSourceSnapshot) { [string]$script:FinalSourceSnapshot.head } else { 'not captured' })",
    "- Public release eligible: $(Test-PublicReleaseEligibility -Status $Status)",
    "- Expected signer thumbprint: $(if ([string]::IsNullOrWhiteSpace($resolvedExpectedSignerThumbprint)) { 'not configured' } else { $resolvedExpectedSignerThumbprint })",
    "- Post-build signing hook: $(if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { 'env/default or none' } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) })"
  )
  if ($null -ne $manifest) {
    $lines += "- Manifest publishable: $([bool]$manifest.publishable)"
    $lines += "- Installer signature eligible: $([bool]$manifest.installer_signature_eligible)"
    $lines += "- Manifest unsigned beta: $([bool]$manifest.unsigned_beta)"
  }
  if ($null -ne $smoke) {
    $lines += "- Desktop suite: $([string]$smoke.desktop_suite)"
    $lines += "- Exact-package release proof: $([bool]$smoke.exact_package_release_proof)"
    $lines += "- Profile isolation: $(if ($null -ne $smoke.profile_isolation) { [string]$smoke.profile_isolation.status } else { 'missing' })"
  }
  if ($null -ne $script:LiveSafetyEvidenceVerification) {
    $lines += "- Execution safety authority: $([string]$script:LiveSafetyEvidenceVerification.authority.execution_safety_gate)"
    $lines += "- Live safety harness authority: $([string]$script:LiveSafetyEvidenceVerification.authority.live_safety_harness)"
    $lines += "- Live safety required tests: $([int]$script:LiveSafetyEvidenceVerification.authority.required_test_count)"
  }
  if (-not [string]::IsNullOrWhiteSpace($ErrorMessage)) {
    $lines += ""
    $lines += "## Failure"
    $lines += ""
    $lines += "- $ErrorMessage"
  }
  $lines += ""
  $lines += "## Steps"
  $lines += ""
  foreach ($step in $script:StepResults) {
    $statusLabel = if ($step.passed) { "PASS" } else { "FAIL" }
    $lines += "- [$statusLabel] $($step.name)"
    $lines += "  command: $($step.command)"
    if ($step.error) {
      $lines += "  error: $($step.error)"
    }
  }
  Set-Content -Path $summaryMdPath -Encoding UTF8 -Value $lines
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
Assert-FreshOutputDirectory -Path $resolvedOutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null
$summaryJsonPath = Join-Path $resolvedOutDir "public-release-summary.json"
$summaryMdPath = Join-Path $resolvedOutDir "public-release-summary.md"
$script:StepResults = @()
$script:FinalArtifactHashes = @()
$script:FinalEmbeddedApplication = $null
$script:ReleaseProofEvidenceVerification = $null
$script:LiveSafetyEvidenceVerification = $null
$script:InitialSourceSnapshot = $null
$script:FinalSourceSnapshot = $null
$script:SourceStateRevalidated = $false
$script:SourceMutationMonitor = $null
$script:SourceMutationMonitorEvidence = $null
$script:SourceMutationMonitorRevalidated = $false
$script:SourceReadLeaseRevalidated = $false
$script:GitExecutablePath = $null
$script:SourceMutationExcludedRoots =
  Get-PublicReleaseSourceMutationExcludedRoots `
    -RepositoryRoot $repoRoot `
    -ReleaseOutputDirectory $resolvedOutDir
$resolvedExpectedSignerThumbprint = Normalize-Thumbprint -Value $(if (-not [string]::IsNullOrWhiteSpace($ExpectedSignerThumbprint)) {
  $ExpectedSignerThumbprint
} else {
  [string]$env:VECTORGRID_EXPECTED_SIGNER_THUMBPRINT
})

try {
  if (@("gpu-ma", "gpu-kernel") -contains $Variant -and -not $IncludePerf) {
    throw "Public GPU releases require -IncludePerf for the variant-specific optimization performance authority."
  }
  if ($Variant -eq "gpu-kernel" -and -not $IncludeGpuValidationPerf) {
    throw "Public gpu-kernel releases require -IncludeGpuValidationPerf for fixed-candidate GPU validation authority."
  }
  if ($IncludeGpuValidationPerf -and $Variant -ne "gpu-kernel") {
    throw "-IncludeGpuValidationPerf requires -Variant gpu-kernel."
  }
  if ($IncludeOptionsGpuPromotion -and $Variant -ne "gpu-kernel") {
    throw "-IncludeOptionsGpuPromotion requires -Variant gpu-kernel."
  }
  if ($AllowDirtyWorktree -and -not $AllowUnsignedBeta) {
    throw "-AllowDirtyWorktree is rehearsal-only and requires -AllowUnsignedBeta."
  }
  if (-not $AllowUnsignedBeta) {
    if ($resolvedExpectedSignerThumbprint -notmatch '^[A-F0-9]{40}$') {
      throw "Final public release requires -ExpectedSignerThumbprint (or VECTORGRID_EXPECTED_SIGNER_THUMBPRINT) as a 40-character certificate thumbprint."
    }
  }

  Invoke-PublicReleaseStep -Name "Clean worktree check" -CommandText "git -C `"$repoRoot`" status --porcelain" -Action {
    $gitCommand = Get-Command git.exe -CommandType Application -ErrorAction Stop |
      Select-Object -First 1
    $script:GitExecutablePath = [System.IO.Path]::GetFullPath(
      [string]$gitCommand.Source
    )
    if (-not (Test-Path -LiteralPath $script:GitExecutablePath -PathType Leaf)) {
      throw "Git executable was not found: $($script:GitExecutablePath)"
    }
    $script:SourceMutationMonitor =
      Start-PublicReleaseSourceMutationMonitor `
        -RepositoryRoot $repoRoot `
        -ExcludedRoots @($script:SourceMutationExcludedRoots)
    $script:InitialSourceSnapshot = Get-PublicReleaseSourceSnapshot `
      -GitExecutablePath $script:GitExecutablePath `
      -RepositoryRoot $repoRoot `
      -RetainSourceReadLeases
    Assert-PublicReleaseSourceReadLeasesActive `
      -SourceSnapshot $script:InitialSourceSnapshot
    Assert-PublicReleaseSourceMutationExclusionsSafe `
      -SourceSnapshot $script:InitialSourceSnapshot `
      -RepositoryRoot $repoRoot `
      -ExcludedRoots @($script:SourceMutationExcludedRoots)
    Assert-CleanWorktree -SourceSnapshot $script:InitialSourceSnapshot
  }

  $candidateArgs = @(
    "-File", (Join-Path $repoRoot "scripts\\run_release_candidate_gate.ps1"),
    "-Variant", $Variant,
    "-OutDir", $resolvedOutDir,
    "-IncludeDesktopSmoke",
    "-DesktopSuite", "release-proof"
  )
  $candidateArgs += "-IncludeOutputParity"
  $candidateArgs += @("-IncludeExecutionSafetyGate", "-IncludeLiveSafetyHarness")
  if ($IncludePerf) {
    $candidateArgs += "-IncludePerf"
  }
  if ($IncludeGpuValidationPerf) {
    $candidateArgs += "-IncludeGpuValidationPerf"
  }
  if ($IncludeOptionsGpuPromotion) {
    $candidateArgs += @(
      "-IncludeOptionsGpuPromotion",
      "-OptionsGpuArtifactPath", $OptionsGpuArtifactPath,
      "-OptionsGpuBaselinePath", $OptionsGpuBaselinePath
    )
    if (-not [string]::IsNullOrWhiteSpace($OptionsGpuDatasetId)) {
      $candidateArgs += @(
        "-OptionsGpuDatasetId", $OptionsGpuDatasetId,
        "-OptionsGpuAppLocalDataDir", $OptionsGpuAppLocalDataDir
      )
    }
    if (-not [string]::IsNullOrWhiteSpace($OptionsGpuImportCsv)) {
      $candidateArgs += @(
        "-OptionsGpuImportCsv", $OptionsGpuImportCsv,
        "-OptionsGpuImportRootSymbol", $OptionsGpuImportRootSymbol
      )
    }
  }
  if ($IncludeExpandedBrowserQa) {
    $candidateArgs += "-IncludeExpandedBrowserQa"
  }
  if ($IncludeBaselineUi) {
    $candidateArgs += "-IncludeBaselineUi"
  }
  if ($SkipWorkspaceTests) {
    $candidateArgs += "-SkipWorkspaceTests"
  }
  if (-not [string]::IsNullOrWhiteSpace($PostBuildSignScript)) {
    $candidateArgs += @("-PostBuildSignScript", [System.IO.Path]::GetFullPath($PostBuildSignScript))
  }
  if ($AllowUnsignedBeta) {
    $candidateArgs += "-UnsignedBeta"
  }

  Invoke-PublicReleaseStep -Name "Release candidate gate" -CommandText ("powershell " + ($candidateArgs -join " ")) -Action {
    powershell @candidateArgs
    if ($LASTEXITCODE -ne 0) {
      throw "release candidate gate failed with exit code $LASTEXITCODE"
    }
  }

  Invoke-PublicReleaseStep -Name "Public release evidence check" -CommandText "manifest/signature/smoke/live-safety assertions" -Action {
    Assert-PublicReleaseEvidence
  }

  Invoke-PublicReleaseStep -Name "Final source-state revalidation" -CommandText "raw tracked/untracked content plus run-long mutation-monitor comparison" -Action {
    $script:FinalSourceSnapshot = Get-PublicReleaseSourceSnapshot `
      -GitExecutablePath $script:GitExecutablePath `
      -RepositoryRoot $repoRoot
    $script:SourceMutationMonitorEvidence =
      Complete-PublicReleaseSourceMutationMonitor `
        -Monitor $script:SourceMutationMonitor
    Assert-PublicReleaseSourceMutationMonitorClean `
      -Evidence $script:SourceMutationMonitorEvidence
    Assert-PublicReleaseSourceSnapshotStable `
      -Initial $script:InitialSourceSnapshot `
      -Final $script:FinalSourceSnapshot
    Assert-PublicReleaseSourceReadLeasesActive `
      -SourceSnapshot $script:InitialSourceSnapshot
    $script:SourceReadLeaseRevalidated = $true
    $script:SourceMutationMonitorRevalidated = $true
    $script:SourceStateRevalidated = $true
    Close-PublicReleaseSourceReadLeases `
      -SourceSnapshot $script:InitialSourceSnapshot
  }

  $completedStatus = if ($AllowUnsignedBeta) { "rehearsal-passed" } else { "passed" }
  Write-PublicReleaseSummary -Status $completedStatus
  Write-Host "Public release gate completed with status '$completedStatus'."
  Write-Host "Summary written to $summaryJsonPath"
} catch {
  $releaseFailure = $_
  if (
    $null -ne $script:SourceMutationMonitor -and
    -not [bool]$script:SourceMutationMonitor.completed
  ) {
    try {
      $script:SourceMutationMonitorEvidence =
        Complete-PublicReleaseSourceMutationMonitor `
          -Monitor $script:SourceMutationMonitor
    } catch {
      $emptyDigest = Get-BytesSha256 -Bytes ([byte[]]@())
      $script:SourceMutationMonitorEvidence = [pscustomobject][ordered]@{
        schema_version = 1
        status = "error"
        started_at_utc =
          [string]$script:SourceMutationMonitor.started_at_utc
        completed_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        include_subdirectories = $true
        internal_buffer_size_bytes = 65536
        excluded_root_count =
          [int]@($script:SourceMutationExcludedRoots).Count
        event_count = 0
        error_count = 1
        event_sha256 = $emptyDigest
      }
    }
  }
  Close-PublicReleaseSourceReadLeases `
    -SourceSnapshot $script:InitialSourceSnapshot
  Write-PublicReleaseSummary `
    -Status "failed" `
    -ErrorMessage $releaseFailure.Exception.Message
  throw $releaseFailure
}
