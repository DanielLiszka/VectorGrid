<#
.SYNOPSIS
Builds a text-only, redacted support-evidence ZIP from explicitly selected inputs.

.DESCRIPTION
The bundle is intentionally fail closed:

- evidence roots are scanned with a fixed filename allowlist;
- crash logs and reproduction notes must be passed explicitly and use a text extension;
- binary artifacts, settings files, datasets, credentials, and arbitrary files are never copied;
- sensitive assignments, common token forms, environment-provided secret values, private-key
  blocks, and private absolute paths are redacted before any output is written;
- every emitted file is scanned again before the ZIP is created; and
- the ZIP entry list is checked against the redacted staging payload.

The script never launches the desktop application and never reads credential stores.
It also never uploads the generated archive. The caller context and whether that
caller may upload the archive automatically are recorded in the bundle.

.EXAMPLE
powershell -File scripts/build_support_evidence_bundle.ps1 `
  -EvidenceRoot output/release_candidates/20260724T172006Z-cpu-candidate `
  -CrashLogPath "$env:LOCALAPPDATA/VectorGrid/logs/VectorGrid.log" `
  -ReproductionMetadataPath .\reproduction.md
#>

[CmdletBinding()]
param(
  [string[]]$EvidenceRoot = @(),
  [string[]]$CrashLogPath = @(),
  [string[]]$ReproductionMetadataPath = @(),
  [string]$OutDir = "",
  [ValidatePattern('^[a-zA-Z0-9][a-zA-Z0-9._-]{0,79}$')]
  [string]$CallerContext = "manual",
  [switch]$AutomaticUpload,
  [ValidateRange(1024, 10485760)]
  [long]$MaxFileBytes = 2097152,
  [ValidateRange(1, 500)]
  [int]$MaxFiles = 100
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$script:SensitiveEnvironmentValues = @()
$script:PrivateIdentityValues = @()
$script:PathReplacements = @()

function Write-Utf8NoBom {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [AllowEmptyString()]
    [string]$Value
  )

  $parent = Split-Path -Parent $Path
  if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  [System.IO.File]::WriteAllText(
    $Path,
    $Value,
    (New-Object System.Text.UTF8Encoding($false))
  )
}

function Read-TextFile {
  param([Parameter(Mandatory = $true)][string]$Path)

  $bytes = [System.IO.File]::ReadAllBytes($Path)
  $strictUtf8 = New-Object System.Text.UTF8Encoding($false, $true)
  try {
    $text = $strictUtf8.GetString($bytes)
  } catch {
    throw "Refusing evidence input '$([System.IO.Path]::GetFileName($Path))' because it is not strict UTF-8 text."
  }

  if ($text.IndexOf([char]0) -ge 0) {
    throw "Refusing binary-looking evidence input '$([System.IO.Path]::GetFileName($Path))'. Only text evidence is supported."
  }
  if ($text.IndexOf([char]0xfffd) -ge 0) {
    throw "Refusing evidence input '$([System.IO.Path]::GetFileName($Path))' because it contains Unicode replacement characters."
  }

  $disallowedControlCount = 0
  foreach ($character in $text.ToCharArray()) {
    $codePoint = [int][char]$character
    if (
      ($codePoint -lt 32 -and $codePoint -notin @(9, 10, 13)) -or
      ($codePoint -ge 127 -and $codePoint -le 159)
    ) {
      $disallowedControlCount += 1
    }
  }
  if ($disallowedControlCount -gt 2) {
    throw "Refusing binary-looking evidence input '$([System.IO.Path]::GetFileName($Path))' because it contains excessive control characters."
  }

  return $text
}

function Get-RelativeSlashPath {
  param(
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string]$Path
  )

  $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd([char]92, [char]47)
  $pathFull = [System.IO.Path]::GetFullPath($Path)
  $prefix = $rootFull + [System.IO.Path]::DirectorySeparatorChar
  if (-not $pathFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Path '$pathFull' is not contained by '$rootFull'."
  }
  return $pathFull.Substring($prefix.Length).Replace([char]92, [char]47)
}

function Resolve-ExistingDirectory {
  param([Parameter(Mandatory = $true)][string]$Path)

  if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
    throw "Evidence root does not exist or is not a directory."
  }
  $item = Get-Item -LiteralPath $Path -Force
  if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Evidence roots may not be reparse points."
  }
  $full = [System.IO.Path]::GetFullPath($item.FullName)
  $volumeRoot = [System.IO.Path]::GetPathRoot($full).TrimEnd([char]92, [char]47)
  if ($full.TrimEnd([char]92, [char]47) -eq $volumeRoot) {
    throw "A filesystem volume root is too broad for support-evidence collection."
  }
  return $full
}

function Resolve-ExistingTextFile {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string[]]$AllowedExtensions,
    [Parameter(Mandatory = $true)][long]$MaximumBytes
  )

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "Evidence file does not exist or is not a regular file."
  }
  $item = Get-Item -LiteralPath $Path -Force
  if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Evidence files may not be reparse points."
  }
  $extension = $item.Extension.ToLowerInvariant()
  if ($AllowedExtensions -notcontains $extension) {
    throw "Evidence file '$($item.Name)' has unsupported extension '$extension'."
  }
  if ($item.Length -gt $MaximumBytes) {
    throw "Evidence file '$($item.Name)' exceeds the per-file limit of $MaximumBytes bytes."
  }
  return $item
}

function Get-SafeDescendantFiles {
  param([Parameter(Mandatory = $true)][string]$Root)

  $pending = New-Object 'System.Collections.Generic.Stack[string]'
  $pending.Push($Root)
  while ($pending.Count -gt 0) {
    $directory = $pending.Pop()
    foreach ($item in Get-ChildItem -LiteralPath $directory -Force) {
      if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Evidence roots may not contain reparse points."
      }
      if ($item.PSIsContainer) {
        $pending.Push($item.FullName)
      } else {
        Write-Output $item
      }
    }
  }
}

function Assert-SafeOutputLocation {
  param(
    [Parameter(Mandatory = $true)][string]$AllowedRoot,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  $allowedFull = [System.IO.Path]::GetFullPath($AllowedRoot).TrimEnd([char]92, [char]47)
  $destinationFull = [System.IO.Path]::GetFullPath($Destination)
  $prefix = $allowedFull + [System.IO.Path]::DirectorySeparatorChar
  if (-not $destinationFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Support-evidence output escaped its fixed root."
  }

  $current = $allowedFull
  $relative = $destinationFull.Substring($prefix.Length)
  $segments = @($relative.Split(@([char]92, [char]47), [System.StringSplitOptions]::RemoveEmptyEntries))
  $pathsToCheck = @($allowedFull)
  foreach ($segment in $segments) {
    $current = Join-Path $current $segment
    $pathsToCheck += $current
  }
  foreach ($path in $pathsToCheck) {
    if (Test-Path -LiteralPath $path) {
      $item = Get-Item -LiteralPath $path -Force
      if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Support-evidence output may not traverse a reparse point."
      }
    }
  }
}

function Get-AllowlistedEvidenceKind {
  param([Parameter(Mandatory = $true)][string]$Name)

  $lower = $Name.ToLowerInvariant()
  $exact = @{
    "manifest.json" = @("release", "manifest")
    "provenance.json" = @("release", "provenance")
    "provenance.txt" = @("release", "provenance")
    "readiness.json" = @("diagnostics", "readiness")
    "readiness.core.json" = @("diagnostics", "readiness-core")
    "release-secret-hygiene.json" = @("diagnostics", "secret-hygiene")
    "node-reproducibility.json" = @("diagnostics", "node-reproducibility")
    "tauri-command-risk-policy.json" = @("diagnostics", "command-risk-policy")
    "packaged-smoke-summary.json" = @("release", "packaged-smoke")
    "public-release-summary.json" = @("release", "public-release-summary")
    "public-release-summary.md" = @("release", "public-release-summary")
    "package_integrity.md" = @("release", "package-integrity")
    "sha256sums.txt" = @("release", "release-sha256s")
    "clean-machine-smoke-checklist.json" = @("qa", "clean-machine-checklist")
    "clean-machine-smoke-checklist.md" = @("qa", "clean-machine-checklist")
    "gate_run_manifest.json" = @("qa", "gate-run-manifest")
    "workflow_matrix.json" = @("qa", "workflow-matrix")
    "case_health.json" = @("qa", "case-health")
    "missing_authority_actions.json" = @("qa", "missing-authority-actions")
    "command_surface_coverage.json" = @("qa", "command-surface-coverage")
    "audit-results.json" = @("qa", "audit-results")
    "frontend-qa-dashboard.json" = @("qa", "frontend-qa-dashboard")
    "frontend-qa-dashboard.md" = @("qa", "frontend-qa-dashboard")
    "headless-output-parity-manifest.json" = @("qa", "headless-output-parity-manifest")
    "manifest-toolchain-contract.json" = @("qa", "manifest-toolchain-contract")
    "release-authority-matrix-contract.json" = @("qa", "release-authority-matrix-contract")
    "required-cpu-quality-summary.json" = @("qa", "required-cpu-quality-summary")
  }
  if ($exact.ContainsKey($lower)) {
    return [pscustomobject]@{
      category = $exact[$lower][0]
      label = $exact[$lower][1]
    }
  }

  return $null
}

function Assert-ReleaseManifestShape {
  param([Parameter(Mandatory = $true)][string]$Path)

  $text = Read-TextFile -Path $Path
  try {
    $manifest = ConvertFrom-Json -InputObject $text -ErrorAction Stop
  } catch {
    throw "Refusing malformed JSON evidence '$([System.IO.Path]::GetFileName($Path))': $($_.Exception.Message)"
  }
  if ($null -eq $manifest -or $manifest -isnot [pscustomobject]) {
    throw "Refusing release manifest evidence because its root is not a JSON object."
  }

  $allowedRootFields = @(
    "artifact_source_kind",
    "bundle_root",
    "generated_at_utc",
    "hash_file",
    "identifier",
    "product_name",
    "provenance_path",
    "publishable",
    "release_binary",
    "release_binary_path",
    "required_resources",
    "schema_version",
    "signing",
    "staged_artifacts",
    "staged_artifacts_dir",
    "unsigned_beta",
    "variant",
    "version"
  )
  $unexpectedRootFields = @(
    $manifest.PSObject.Properties.Name |
      Where-Object { $allowedRootFields -notcontains $_ }
  )
  if ($unexpectedRootFields.Count -gt 0) {
    throw "Refusing release manifest evidence with fields outside the fixed release schema: $($unexpectedRootFields -join ', ')."
  }
  if (
    [int]$manifest.schema_version -ne 1 -or
    [string]$manifest.product_name -ne "VectorGrid" -or
    [string]$manifest.variant -notin @("cpu", "gpu-ma", "gpu-kernel") -or
    [string]$manifest.artifact_source_kind -notin @("bundle", "release_binary") -or
    [string]::IsNullOrWhiteSpace([string]$manifest.version)
  ) {
    throw "Refusing release manifest evidence that does not match the VectorGrid release identity contract."
  }

  $artifacts = @($manifest.staged_artifacts)
  if ($artifacts.Count -eq 0) {
    throw "Refusing release manifest evidence without staged artifact metadata."
  }
  foreach ($artifact in $artifacts) {
    if (
      $null -eq $artifact -or
      $artifact -isnot [pscustomobject] -or
      [string]::IsNullOrWhiteSpace([string]$artifact.staged_name) -or
      [long]$artifact.size_bytes -lt 0 -or
      [string]$artifact.sha256 -notmatch '^[a-fA-F0-9]{64}$'
    ) {
      throw "Refusing release manifest evidence with malformed staged artifact metadata."
    }
  }

  $pending = New-Object 'System.Collections.Generic.Stack[object]'
  $pending.Push($manifest)
  while ($pending.Count -gt 0) {
    $value = $pending.Pop()
    if ($null -eq $value) {
      continue
    }
    if ($value -is [pscustomobject]) {
      foreach ($property in $value.PSObject.Properties) {
        $compactName = $property.Name.ToLowerInvariant() -replace '[^a-z0-9]', ''
        if ($compactName -match '^(?:marketdata|ohlcv|pricehistory|tradehistory|candles|positions|accountbalances|datasetrows|rawdata)$') {
          throw "Refusing release manifest evidence containing non-release data field '$($property.Name)'."
        }
        if ($null -ne $property.Value) {
          $pending.Push($property.Value)
        }
      }
    } elseif (($value -is [System.Collections.IEnumerable]) -and -not ($value -is [string])) {
      foreach ($item in $value) {
        if ($null -ne $item) {
          $pending.Push($item)
        }
      }
    }
  }
}

function Test-SensitiveFieldName {
  param([Parameter(Mandatory = $true)][string]$Name)

  $compact = ($Name.ToLowerInvariant() -replace '[^a-z0-9]', '')
  if (@(
      "secret",
      "token",
      "password",
      "passwd",
      "pwd",
      "authorization",
      "apikey",
      "apisecret",
      "accesskey",
      "accesskeyid",
      "secretaccesskey",
      "accesstoken",
      "refreshtoken",
      "bearertoken",
      "clientsecret",
      "privatekey",
      "signingkey",
      "secretvalue",
      "credentialvalue"
      "cookie"
      "setcookie"
      "connectionstring"
      "jwt"
    ) -contains $compact) {
    return $true
  }
  return (
    $compact.EndsWith("apikey") -or
    $compact.EndsWith("apisecret") -or
    $compact.EndsWith("accesstoken") -or
    $compact.EndsWith("refreshtoken") -or
    $compact.EndsWith("clientsecret") -or
    $compact.EndsWith("privatekey") -or
    $compact.EndsWith("signingkey") -or
    $compact.EndsWith("password") -or
    $compact.EndsWith("secret") -or
    $compact.EndsWith("token") -or
    $compact.EndsWith("credential") -or
    $compact.EndsWith("cookie") -or
    $compact.EndsWith("connectionstring")
  )
}

function Protect-PrivatePaths {
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)

  $output = $Text
  foreach ($entry in $script:PathReplacements) {
    if ([string]::IsNullOrWhiteSpace([string]$entry.path)) {
      continue
    }
    $output = [regex]::Replace(
      $output,
      [regex]::Escape([string]$entry.path),
      [string]$entry.replacement,
      [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )
  }
  foreach ($value in $script:PrivateIdentityValues) {
    if (-not [string]::IsNullOrWhiteSpace($value)) {
      $output = [regex]::Replace(
        $output,
        [regex]::Escape($value),
        '<local_identity>',
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
      )
    }
  }

  $output = [regex]::Replace(
    $output,
    '(?i)<(?<role>repo_root|source_root|bundle_root|local_path)>(?:[\\/][^\r\n]*)?',
    '<${role}>'
  )

  $output = [regex]::Replace(
    $output,
    '(?i)file:///(?:[a-z]:/|/)[^\r\n]*',
    'file:///<local_path>'
  )
  $output = [regex]::Replace(
    $output,
    '(?i)(?<prefix>["''])(?:[a-z]:[\\/]|\\\\)[^"''\r\n]*(?<suffix>["''])',
    '${prefix}<local_path>${suffix}'
  )
  $output = [regex]::Replace(
    $output,
    '(?im)(?<![a-z0-9])(?:[a-z]:[\\/]|\\\\)[^\r\n]*',
    '<local_path>'
  )
  $output = [regex]::Replace(
    $output,
    '(?im)(?<![a-z0-9])/(?:home|users|private|var/folders|tmp|mnt/[a-z]/users)/[^\r\n]*',
    '<local_path>'
  )
  $output = [regex]::Replace(
    $output,
    '(?im)(?<![:/a-z0-9])/(?!/|<local_path>)[a-z0-9._~-]+(?:/[^\s"''<>|\r\n]+)+',
    '<local_path>'
  )
  $output = [regex]::Replace(
    $output,
    '(?i)(?:%USERPROFILE%|%HOME%|\$env:(?:USERPROFILE|HOME|APPDATA|LOCALAPPDATA|TEMP|TMP)|~)(?:[\\/][^\r\n]*)',
    '<local_path>'
  )
  return $output
}

function Protect-SensitiveText {
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)

  $output = $Text
  foreach ($value in $script:SensitiveEnvironmentValues) {
    if ([string]::IsNullOrWhiteSpace($value)) {
      continue
    }
    $output = [regex]::Replace(
      $output,
      [regex]::Escape($value),
      '[redacted]',
      [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )
  }

  $privateKeyPattern = '(?is)-----BEGIN (?:RSA |DSA |EC |OPENSSH |ENCRYPTED )?PRIVATE KEY-----.*?-----END (?:RSA |DSA |EC |OPENSSH |ENCRYPTED )?PRIVATE KEY-----'
  $output = [regex]::Replace($output, $privateKeyPattern, '[redacted-private-key]')

  $output = [regex]::Replace(
    $output,
    '(?i)\b(?:Bearer|Basic)\s+(?!\[redacted\])[a-z0-9._~+/=-]{4,}',
    '[redacted-authorization]'
  )
  $keyPattern = '(?:[a-z0-9_.-]*(?:api[_-]?key|api[_-]?secret|access[_-]?key(?:[_-]?id)?|secret[_-]?access[_-]?key|access[_-]?token|refresh[_-]?token|client[_-]?secret|password|passwd|private[_-]?key|signing[_-]?key|connection[_-]?string|secret|credential)|authorization|proxy-authorization|x-api-key|apca-api-key-id|apca-api-secret-key|cookie|set-cookie|token)'
  $headerPattern = '(?im)(?<prefix>^\s*(?:Authorization|Proxy-Authorization|X-Api-Key|APCA-API-KEY-ID|APCA-API-SECRET-KEY|Cookie|Set-Cookie)\s*:\s*)[^\r\n]+'
  $output = [regex]::Replace($output, $headerPattern, '${prefix}[redacted]')

  $quotedAssignmentPattern = '(?i)(?<prefix>["'']?' + $keyPattern + '["'']?\s*[:=]\s*["''])[^"''\r\n]*(?<suffix>["''])'
  $output = [regex]::Replace(
    $output,
    $quotedAssignmentPattern,
    '${prefix}[redacted]${suffix}'
  )

  $unquotedAssignmentPattern = '(?im)(?<prefix>\b' + $keyPattern + '\b\s*[:=]\s*)(?!\[redacted(?:-[a-z-]+)?\])(?<value>\[[^\]\r\n]+\]|[^\s,;}\]]+)'
  $output = [regex]::Replace(
    $output,
    $unquotedAssignmentPattern,
    '${prefix}[redacted]'
  )

  $cliArgumentPattern = '(?i)(?<prefix>--(?:api-key|api-secret|access-key|access-token|refresh-token|password|client-secret|token|secret)\s+)(?:"[^"]*"|''[^'']*''|\S+)'
  $output = [regex]::Replace($output, $cliArgumentPattern, '${prefix}[redacted]')

  $urlParameterPattern = '(?i)(?<prefix>[?&](?:api[_-]?key|api[_-]?secret|access[_-]?token|refresh[_-]?token|password|token|secret|sig|signature|x-amz-[a-z0-9-]+|x-ms-[a-z0-9-]+)=)[^&#\s]+'
  $output = [regex]::Replace($output, $urlParameterPattern, '${prefix}[redacted]')

  $output = [regex]::Replace(
    $output,
    '(?i)\b(?:gh[pousr]_[a-z0-9]{20,}|github_pat_[a-z0-9_]{20,}|xox[baprs]-[a-z0-9-]{12,}|sk-(?:live|proj)-[a-z0-9_-]{12,}|sk-[a-z0-9_-]{20,}|AKIA[0-9A-Z]{16})\b',
    '[redacted-token]'
  )
  $output = [regex]::Replace(
    $output,
    '(?i)\beyJ[a-z0-9_-]{8,}\.[a-z0-9_-]{8,}\.[a-z0-9_-]{8,}\b',
    '[redacted-jwt]'
  )
  $output = [regex]::Replace(
    $output,
    '(?i)(?<scheme>[a-z][a-z0-9+.-]*://)[^/\s:@]+:[^/\s@]+@',
    '${scheme}[redacted-userinfo]@'
  )

  return Protect-PrivatePaths -Text $output
}

function Protect-JsonValue {
  param(
    [AllowNull()]
    $Value,
    [string]$PropertyName = ""
  )

  if (-not [string]::IsNullOrWhiteSpace($PropertyName) -and (Test-SensitiveFieldName -Name $PropertyName)) {
    return "[redacted]"
  }
  if ($null -eq $Value) {
    return $null
  }
  if ($Value -is [string]) {
    return Protect-SensitiveText -Text ([string]$Value)
  }
  if ($Value -is [System.Collections.IDictionary]) {
    $dictionary = [ordered]@{}
    foreach ($key in $Value.Keys) {
      $dictionary[[string]$key] = Protect-JsonValue -Value $Value[$key] -PropertyName ([string]$key)
    }
    return $dictionary
  }
  if (($Value -is [System.Collections.IEnumerable]) -and -not ($Value -is [string])) {
    $items = @()
    foreach ($item in $Value) {
      $items += ,(Protect-JsonValue -Value $item)
    }
    return ,$items
  }
  if ($Value -is [pscustomobject]) {
    $object = [ordered]@{}
    foreach ($property in $Value.PSObject.Properties) {
      $object[$property.Name] = Protect-JsonValue -Value $property.Value -PropertyName $property.Name
    }
    return $object
  }
  return $Value
}

function ConvertTo-SanitizedContent {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Extension
  )

  $text = Read-TextFile -Path $Path
  if ($Extension -eq ".json") {
    try {
      $parsed = ConvertFrom-Json -InputObject $text -ErrorAction Stop
    } catch {
      throw "Refusing malformed JSON evidence '$([System.IO.Path]::GetFileName($Path))': $($_.Exception.Message)"
    }
    $protected = Protect-JsonValue -Value $parsed
    return (($protected | ConvertTo-Json -Depth 50) + [Environment]::NewLine)
  }
  return Protect-SensitiveText -Text $text
}

function Get-RedactionLeakFindings {
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)

  $findings = @()
  foreach ($value in $script:SensitiveEnvironmentValues) {
    if (-not [string]::IsNullOrWhiteSpace($value) -and $Text.IndexOf($value, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
      $findings += "sensitive_environment_value"
      break
    }
  }
  if ($Text -match '(?i)-----BEGIN (?:RSA |DSA |EC |OPENSSH |ENCRYPTED )?PRIVATE KEY-----') {
    $findings += "private_key_marker"
  }
  if ($Text -match '(?i)\b(?:Bearer|Basic)\s+(?!\[redacted\])[a-z0-9._~+/=-]{4,}') {
    $findings += "authorization_value"
  }
  if ($Text -match '(?i)\b(?:gh[pousr]_[a-z0-9]{20,}|github_pat_[a-z0-9_]{20,}|xox[baprs]-[a-z0-9-]{12,}|sk-(?:live|proj)-[a-z0-9_-]{12,}|sk-[a-z0-9_-]{20,}|AKIA[0-9A-Z]{16})\b') {
    $findings += "known_token_form"
  }
  if ($Text -match '(?i)\beyJ[a-z0-9_-]{8,}\.[a-z0-9_-]{8,}\.[a-z0-9_-]{8,}\b') {
    $findings += "jwt"
  }
  if ($Text -match '(?im)(?<![a-z0-9])[a-z]:[\\/][^\r\n]+') {
    $findings += "windows_absolute_path"
  }
  if ($Text -match '(?im)(?:^|[\s="'':(])\\\\[a-z0-9][^\r\n]+') {
    $findings += "unc_absolute_path"
  }
  if ($Text -match '(?im)(?<![a-z0-9])/(?:home|users|private|var/folders|tmp|mnt/[a-z]/users)/[^\r\n]+') {
    $findings += "unix_absolute_path"
  }
  if ($Text -match '(?im)(?<![:/a-z0-9])/(?!/|<local_path>)[a-z0-9._~-]+(?:/[^\s"''<>|\r\n]+)+') {
    $findings += "unix_absolute_path"
  }
  if ($Text -match '(?i)file:///(?:[a-z]:/|/)(?!<local_path>)') {
    $findings += "file_uri_path"
  }

  if ($Text -match '(?i)[a-z][a-z0-9+.-]*://[^/\s:@]+:[^/\s@]+@') {
    $findings += "url_userinfo"
  }

  $keyPattern = '(?:[a-z0-9_.-]*(?:api[_-]?key|api[_-]?secret|access[_-]?key(?:[_-]?id)?|secret[_-]?access[_-]?key|access[_-]?token|refresh[_-]?token|client[_-]?secret|password|passwd|private[_-]?key|signing[_-]?key|connection[_-]?string|secret|credential)|authorization|proxy-authorization|x-api-key|apca-api-key-id|apca-api-secret-key|cookie|set-cookie|token)'
  $assignmentPattern = '(?im)\b' + $keyPattern + '\b\s*[:=]\s*(?:"|\s)*(?!\[redacted(?:-[a-z-]+)?\])(?<value>[^\s,;}\]"'']+)'
  foreach ($match in [regex]::Matches($Text, $assignmentPattern)) {
    $value = [string]$match.Groups["value"].Value
    if (
      -not [string]::IsNullOrWhiteSpace($value) -and
      $value -notmatch '^(?:null|true|false|none|not[_ -]?set|missing|unavailable)$'
    ) {
      $findings += "sensitive_assignment"
      break
    }
  }
  return @($findings | Sort-Object -Unique)
}

function Get-SafeSystemMetadata {
  param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)][string]$ProductVersion
  )

  $gitCommit = $null
  $gitDirty = $null
  try {
    $commitText = (& git -C $RepositoryRoot rev-parse HEAD 2>$null | Out-String).Trim()
    if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($commitText)) {
      $gitCommit = $commitText
    }
    $dirtyText = (& git -C $RepositoryRoot status --porcelain 2>$null | Out-String).Trim()
    if ($LASTEXITCODE -eq 0) {
      $gitDirty = -not [string]::IsNullOrWhiteSpace($dirtyText)
    }
  } catch {
    $gitCommit = $null
    $gitDirty = $null
  }

  $os = $null
  $computer = $null
  $processors = @()
  $videoControllers = @()
  try {
    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction Stop |
      Select-Object -First 1 Caption, Version, BuildNumber, OSArchitecture
  } catch {
    $os = $null
  }
  try {
    $computer = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop |
      Select-Object -First 1 Manufacturer, Model
  } catch {
    $computer = $null
  }
  try {
    $processors = @(
      Get-CimInstance Win32_Processor -ErrorAction Stop |
        Select-Object Name, NumberOfCores, NumberOfLogicalProcessors
    )
  } catch {
    $processors = @()
  }
  try {
    $videoControllers = @(
      Get-CimInstance Win32_VideoController -ErrorAction Stop |
        Select-Object Name, DriverVersion
    )
  } catch {
    $videoControllers = @()
  }

  return [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    product = [ordered]@{
      name = "VectorGrid"
      version = $ProductVersion
    }
    source = [ordered]@{
      git_commit = $gitCommit
      git_dirty = $gitDirty
    }
    runtime = [ordered]@{
      process_architecture = [string]$env:PROCESSOR_ARCHITECTURE
      powershell_version = $PSVersionTable.PSVersion.ToString()
      dotnet_runtime = [System.Runtime.InteropServices.RuntimeInformation]::FrameworkDescription
      culture = [System.Globalization.CultureInfo]::CurrentCulture.Name
      time_zone = [System.TimeZoneInfo]::Local.Id
    }
    operating_system = $os
    computer = $computer
    processors = $processors
    video_controllers = $videoControllers
    privacy = [ordered]@{
      computer_name_included = $false
      user_name_included = $false
      environment_dump_included = $false
      credential_store_read = $false
      absolute_paths_allowed = $false
    }
  }
}

function Assert-SafeArchive {
  param(
    [Parameter(Mandatory = $true)][string]$PayloadRoot,
    [Parameter(Mandatory = $true)][string]$ArchivePath
  )

  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $expectedFiles = @(
    Get-ChildItem -LiteralPath $PayloadRoot -Recurse -File
  )
  $expected = @($expectedFiles | ForEach-Object {
      Get-RelativeSlashPath -Root $PayloadRoot -Path $_.FullName
    } | Sort-Object)
  $expectedByPath = @{}
  foreach ($file in $expectedFiles) {
    $relative = Get-RelativeSlashPath -Root $PayloadRoot -Path $file.FullName
    $expectedByPath[$relative.ToLowerInvariant()] = [ordered]@{
      path = $relative
      size_bytes = [int64]$file.Length
      sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName).Hash.ToLowerInvariant()
    }
  }
  $archive = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
  try {
    $fileEntries = @($archive.Entries | Where-Object { -not [string]::IsNullOrWhiteSpace($_.Name) })
    $actual = @($fileEntries | ForEach-Object {
        $_.FullName.Replace([char]92, [char]47)
      } | Sort-Object)
    $caseFolded = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)
    $totalUncompressedBytes = [int64]0
    foreach ($entry in $fileEntries) {
      $entryPath = $entry.FullName.Replace([char]92, [char]47)
      if (
        [System.IO.Path]::IsPathRooted($entryPath) -or
        $entryPath -match '(^|/)\.\.(/|$)' -or
        $entryPath.Contains("\") -or
        $entryPath.Contains(":")
      ) {
        throw "Unsafe ZIP entry was produced: '$entryPath'."
      }
      if (-not $caseFolded.Add($entryPath)) {
        throw "Case-insensitive duplicate ZIP entry was produced: '$entryPath'."
      }
      $expectedEntry = $expectedByPath[$entryPath.ToLowerInvariant()]
      if ($null -eq $expectedEntry) {
        throw "Unexpected ZIP entry was produced: '$entryPath'."
      }
      $totalUncompressedBytes += [int64]$entry.Length
      if (
        [int64]$entry.Length -ne [int64]$expectedEntry.size_bytes -or
        $totalUncompressedBytes -gt 52428800
      ) {
        throw "ZIP size verification failed for '$entryPath'."
      }

      $stream = $entry.Open()
      $memory = New-Object System.IO.MemoryStream
      try {
        $stream.CopyTo($memory)
        $bytes = $memory.ToArray()
      } finally {
        $memory.Dispose()
        $stream.Dispose()
      }
      $sha = [System.Security.Cryptography.SHA256]::Create()
      try {
        $entryHash = ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
      } finally {
        $sha.Dispose()
      }
      if ($entryHash -ne [string]$expectedEntry.sha256) {
        throw "ZIP content hash verification failed for '$entryPath'."
      }
      $strictUtf8 = New-Object System.Text.UTF8Encoding($false, $true)
      try {
        $decoded = $strictUtf8.GetString($bytes)
      } catch {
        throw "ZIP entry '$entryPath' is not strict UTF-8 text."
      }
      $leaks = @(Get-RedactionLeakFindings -Text $decoded)
      if ($leaks.Count -gt 0) {
        throw "ZIP entry '$entryPath' failed the decompressed privacy scan ($($leaks -join ', '))."
      }
    }
  } finally {
    $archive.Dispose()
  }

  if (($expected -join "`n") -ne ($actual -join "`n")) {
    throw "ZIP entry verification failed; the archive does not exactly match the redacted payload."
  }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ") + "-" + ([guid]::NewGuid().ToString("N").Substring(0, 8))
$repoOutputRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "output"))
$allowedOutputRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "output\support-evidence"))
$resolvedOutDir = if ([string]::IsNullOrWhiteSpace($OutDir)) {
  Join-Path $allowedOutputRoot $stamp
} else {
  [System.IO.Path]::GetFullPath($OutDir)
}
$allowedOutputPrefix = $allowedOutputRoot.TrimEnd([char]92, [char]47) + [System.IO.Path]::DirectorySeparatorChar
if (-not $resolvedOutDir.StartsWith($allowedOutputPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "Support-evidence output must be a child of '$allowedOutputRoot'."
}
foreach ($outputAncestor in @($repoRoot, $repoOutputRoot, $allowedOutputRoot)) {
  if (Test-Path -LiteralPath $outputAncestor) {
    $outputAncestorItem = Get-Item -LiteralPath $outputAncestor -Force
    if (($outputAncestorItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Support-evidence output may not traverse a reparse point."
    }
  }
}
Assert-SafeOutputLocation -AllowedRoot $allowedOutputRoot -Destination $resolvedOutDir

$resolvedEvidenceRoots = @()
$allowedEvidencePrefix = $repoOutputRoot.TrimEnd([char]92, [char]47) + [System.IO.Path]::DirectorySeparatorChar
foreach ($root in $EvidenceRoot) {
  if (-not [string]::IsNullOrWhiteSpace($root)) {
    $resolvedRoot = Resolve-ExistingDirectory -Path $root
    if (-not $resolvedRoot.StartsWith($allowedEvidencePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
      throw "Evidence roots must be children of the repository's output directory. Select external crash logs or reproduction notes explicitly."
    }
    $resolvedEvidenceRoots += $resolvedRoot
  }
}
$resolvedEvidenceRoots = @($resolvedEvidenceRoots | Sort-Object -Unique)
if ($resolvedEvidenceRoots.Count -gt 10) {
  throw "At most 10 evidence roots may be selected."
}
foreach ($root in $resolvedEvidenceRoots) {
  $rootPrefix = $root.TrimEnd([char]92, [char]47) + [System.IO.Path]::DirectorySeparatorChar
  $outPrefix = $resolvedOutDir.TrimEnd([char]92, [char]47) + [System.IO.Path]::DirectorySeparatorChar
  if (
    $resolvedOutDir.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
    $root.StartsWith($outPrefix, [System.StringComparison]::OrdinalIgnoreCase)
  ) {
    throw "Support-evidence input and output directories may not contain one another."
  }
}

$candidates = @()
$seenPaths = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)
$skippedNotAllowlisted = 0
foreach ($root in $resolvedEvidenceRoots) {
  foreach ($file in Get-SafeDescendantFiles -Root $root) {
    $kind = Get-AllowlistedEvidenceKind -Name $file.Name
    if ($null -eq $kind) {
      $skippedNotAllowlisted += 1
      continue
    }
    if (($file.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Allowlisted evidence may not be read through a reparse point."
    }
    if ($file.Length -gt $MaxFileBytes) {
      throw "Allowlisted evidence '$($file.Name)' exceeds the per-file limit of $MaxFileBytes bytes."
    }
    if ($file.Name.Equals("manifest.json", [System.StringComparison]::OrdinalIgnoreCase)) {
      Assert-ReleaseManifestShape -Path $file.FullName
    }
    if ($seenPaths.Add($file.FullName)) {
      $candidates += [pscustomobject]@{
        path = $file.FullName
        extension = $file.Extension.ToLowerInvariant()
        category = $kind.category
        label = $kind.label
      }
    }
  }
}

foreach ($path in $CrashLogPath) {
  if ([string]::IsNullOrWhiteSpace($path)) {
    continue
  }
  $file = Resolve-ExistingTextFile -Path $path -AllowedExtensions @(".log", ".txt", ".json", ".xml") -MaximumBytes $MaxFileBytes
  if ($seenPaths.Add($file.FullName)) {
    $candidates += [pscustomobject]@{
      path = $file.FullName
      extension = $file.Extension.ToLowerInvariant()
      category = "crash"
      label = "crash-log"
    }
  }
}

foreach ($path in $ReproductionMetadataPath) {
  if ([string]::IsNullOrWhiteSpace($path)) {
    continue
  }
  $file = Resolve-ExistingTextFile -Path $path -AllowedExtensions @(".json", ".md", ".txt") -MaximumBytes $MaxFileBytes
  if ($seenPaths.Add($file.FullName)) {
    $candidates += [pscustomobject]@{
      path = $file.FullName
      extension = $file.Extension.ToLowerInvariant()
      category = "reproduction"
      label = "reproduction-note"
    }
  }
}

if ($candidates.Count -eq 0) {
  throw "No allowlisted evidence was selected. Provide an evidence root, crash log, or reproduction metadata file."
}
if ($candidates.Count -gt $MaxFiles) {
  throw "Selected evidence count $($candidates.Count) exceeds the limit of $MaxFiles files."
}

$selectedBytes = [int64]0
foreach ($candidate in $candidates) {
  $selectedBytes += [int64](Get-Item -LiteralPath $candidate.path).Length
}
$maximumTotalBytes = [Math]::Min(
  ([int64]$MaxFileBytes * [int64]$MaxFiles),
  [int64]52428800
)
if ($selectedBytes -gt $maximumTotalBytes) {
  throw "Selected evidence exceeds the aggregate size limit."
}

$sensitiveEnvironmentNames = @(
  "POLYGON_KEY",
  "POLYGON_API_KEY",
  "ALPACA_API_KEY",
  "ALPACA_API_SECRET",
  "ALPACA_KEY_ID",
  "ALPACA_SECRET_KEY",
  "APCA_API_KEY_ID",
  "APCA_API_SECRET_KEY",
  "GITHUB_TOKEN",
  "GH_TOKEN",
  "OPENAI_API_KEY",
  "AWS_ACCESS_KEY_ID",
  "AWS_SECRET_ACCESS_KEY",
  "AWS_SESSION_TOKEN",
  "AZURE_CLIENT_SECRET",
  "GOOGLE_API_KEY",
  "NPM_TOKEN"
)
$sensitiveEnvironmentValues = @()
foreach ($name in $sensitiveEnvironmentNames) {
  $value = [Environment]::GetEnvironmentVariable($name)
  if (-not [string]::IsNullOrWhiteSpace($value) -and $value.Length -ge 8) {
    $sensitiveEnvironmentValues += $value
  }
}
$script:SensitiveEnvironmentValues = @($sensitiveEnvironmentValues | Sort-Object Length -Descending -Unique)
$script:PrivateIdentityValues = @(
  $env:USERNAME,
  $env:COMPUTERNAME,
  $env:USERDOMAIN
) | Where-Object {
  -not [string]::IsNullOrWhiteSpace([string]$_) -and ([string]$_).Length -ge 3
} | Sort-Object Length -Descending -Unique

$pathEntries = @()
foreach ($root in $resolvedEvidenceRoots) {
  $pathEntries += [pscustomobject]@{ path = $root.TrimEnd([char]92, [char]47); replacement = "<source_root>" }
}
foreach ($candidate in $candidates) {
  $parent = Split-Path -Parent ([string]$candidate.path)
  if (-not [string]::IsNullOrWhiteSpace($parent)) {
    $pathEntries += [pscustomobject]@{ path = $parent.TrimEnd([char]92, [char]47); replacement = "<source_root>" }
  }
}
$pathEntries += [pscustomobject]@{ path = $repoRoot.TrimEnd([char]92, [char]47); replacement = "<repo_root>" }
$pathEntries += [pscustomobject]@{ path = $resolvedOutDir.TrimEnd([char]92, [char]47); replacement = "<bundle_root>" }
foreach ($environmentPath in @($env:USERPROFILE, $env:HOME, $env:LOCALAPPDATA, $env:APPDATA, $env:TEMP, $env:TMP)) {
  if (-not [string]::IsNullOrWhiteSpace([string]$environmentPath)) {
    $pathEntries += [pscustomobject]@{
      path = ([System.IO.Path]::GetFullPath([string]$environmentPath)).TrimEnd([char]92, [char]47)
      replacement = "<local_path>"
    }
  }
}
$script:PathReplacements = @(
  $pathEntries |
    Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.path) } |
    Sort-Object @{ Expression = { ([string]$_.path).Length }; Descending = $true } -Unique
)

if (Test-Path -LiteralPath $resolvedOutDir) {
  $existing = Get-ChildItem -LiteralPath $resolvedOutDir -Force | Select-Object -First 1
  if ($null -ne $existing) {
    throw "Support-evidence output directory already exists and is not empty: $resolvedOutDir"
  }
} else {
  New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null
}

$payloadRoot = Join-Path $resolvedOutDir "payload"
New-Item -ItemType Directory -Force -Path $payloadRoot | Out-Null

$indices = @{}
$included = @()
foreach ($candidate in ($candidates | Sort-Object category, label, path)) {
  $indexKey = "$($candidate.category)/$($candidate.label)$($candidate.extension)"
  if (-not $indices.ContainsKey($indexKey)) {
    $indices[$indexKey] = 0
  }
  $indices[$indexKey] += 1
  $destinationName = "{0}-{1:d3}{2}" -f $candidate.label, $indices[$indexKey], $candidate.extension
  $destinationRelative = "$($candidate.category)/$destinationName"
  $destinationPath = Join-Path $payloadRoot $destinationRelative.Replace("/", "\")

  $sanitized = ConvertTo-SanitizedContent -Path $candidate.path -Extension $candidate.extension
  $leaks = @(Get-RedactionLeakFindings -Text $sanitized)
  if ($leaks.Count -gt 0) {
    throw "Redaction verification failed for '$($candidate.label)' ($($leaks -join ', ')). No bundle was created."
  }
  Write-Utf8NoBom -Path $destinationPath -Value $sanitized
  $item = Get-Item -LiteralPath $destinationPath
  $included += [ordered]@{
    category = $candidate.category
    evidence_type = $candidate.label
    bundle_path = $destinationRelative
    size_bytes = [int64]$item.Length
    sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $destinationPath).Hash.ToLowerInvariant()
  }
}

$tauriConfigPath = Join-Path $repoRoot "src-tauri\tauri.conf.json"
$productVersion = "unknown"
if (Test-Path -LiteralPath $tauriConfigPath -PathType Leaf) {
  try {
    $productVersion = [string]((Get-Content -Raw -LiteralPath $tauriConfigPath | ConvertFrom-Json).version)
  } catch {
    $productVersion = "unknown"
  }
}
$systemMetadata = Get-SafeSystemMetadata -RepositoryRoot $repoRoot -ProductVersion $productVersion
$systemMetadataText = ($systemMetadata | ConvertTo-Json -Depth 20) + [Environment]::NewLine
$systemMetadataText = Protect-SensitiveText -Text $systemMetadataText
$metadataLeaks = @(Get-RedactionLeakFindings -Text $systemMetadataText)
if ($metadataLeaks.Count -gt 0) {
  throw "Generated system metadata failed redaction verification ($($metadataLeaks -join ', '))."
}
$systemMetadataPath = Join-Path $payloadRoot "reproduction\system-metadata.json"
Write-Utf8NoBom -Path $systemMetadataPath -Value $systemMetadataText
$systemMetadataItem = Get-Item -LiteralPath $systemMetadataPath
$included += [ordered]@{
  category = "reproduction"
  evidence_type = "system-metadata"
  bundle_path = "reproduction/system-metadata.json"
  size_bytes = [int64]$systemMetadataItem.Length
  sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $systemMetadataPath).Hash.ToLowerInvariant()
}

$readmeText = @"
VectorGrid support evidence

This archive is diagnostic-only evidence. It is not release authority. The
builder never uploads this archive or performs any network request.

Caller context: $CallerContext
Caller automatic upload enabled: $([bool]$AutomaticUpload)

$(if ($AutomaticUpload) {
  "The caller indicated that it may automatically upload only the final redacted ZIP and checksum."
} else {
  "The caller did not indicate any automatic upload of the final redacted ZIP and checksum."
})

Only fixed-name text reports plus explicitly selected text crash/reproduction
files are eligible. Binary dumps, settings, datasets, artifacts, credential
stores, and environment dumps are excluded. Known credential forms, private-key
blocks, private identities, and absolute local paths are redacted before the
archive is created. The archive is then reopened, hashed, and scanned again.

Review the redacted payload before sharing it. Free-form third-party logs can
contain a previously unknown secret format; omit any log that is not required
for the support case.
"@
$readmePath = Join-Path $payloadRoot "README.txt"
Write-Utf8NoBom -Path $readmePath -Value ($readmeText + [Environment]::NewLine)

$bundleManifest = [ordered]@{
  schema_version = 1
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  status = "complete"
  product = "VectorGrid"
  policy = [ordered]@{
    input_selection = "fixed evidence-root filename allowlist plus explicit text crash/reproduction paths"
    caller_context = $CallerContext
    binary_inputs_allowed = $false
    reparse_point_inputs_allowed = $false
    credential_store_read = $false
    environment_dump_included = $false
    raw_binary_dumps_included = $false
    absolute_paths_allowed = $false
    redaction_post_scan_required = $true
    builder_uploads_archive = $false
    automatic_upload = [bool]$AutomaticUpload
    automatic_upload_actor = if ($AutomaticUpload) { "caller" } else { "none" }
    release_authority = $false
    review_before_sharing = $true
    max_file_bytes = $MaxFileBytes
    max_files = $MaxFiles
    max_total_bytes = $maximumTotalBytes
  }
  selection = [ordered]@{
    evidence_root_count = $resolvedEvidenceRoots.Count
    explicitly_selected_crash_log_count = @($CrashLogPath | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
    explicitly_selected_reproduction_file_count = @($ReproductionMetadataPath | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
    skipped_not_allowlisted_count = $skippedNotAllowlisted
    included_file_count = $included.Count
  }
  files = $included
}
$bundleManifestPath = Join-Path $payloadRoot "support-bundle-manifest.json"
$bundleManifestText = ($bundleManifest | ConvertTo-Json -Depth 20) + [Environment]::NewLine
Write-Utf8NoBom -Path $bundleManifestPath -Value $bundleManifestText

$checksumLines = @(
  Get-ChildItem -LiteralPath $payloadRoot -Recurse -File |
    Where-Object { $_.FullName -ne $bundleManifestPath } |
    Sort-Object FullName |
    ForEach-Object {
      $relative = Get-RelativeSlashPath -Root $payloadRoot -Path $_.FullName
      $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
      "$hash *$relative"
    }
)
$checksumPath = Join-Path $payloadRoot "SHA256SUMS.txt"
Write-Utf8NoBom -Path $checksumPath -Value (($checksumLines -join [Environment]::NewLine) + [Environment]::NewLine)

foreach ($emitted in Get-ChildItem -LiteralPath $payloadRoot -Recurse -File) {
  $emittedText = Read-TextFile -Path $emitted.FullName
  $emittedLeaks = @(Get-RedactionLeakFindings -Text $emittedText)
  if ($emittedLeaks.Count -gt 0) {
    throw "Final bundle leak scan failed for '$($emitted.Name)' ($($emittedLeaks -join ', '))."
  }
}

$archivePath = Join-Path $resolvedOutDir "vectorgrid-support-evidence.zip"
$partialArchivePath = Join-Path $resolvedOutDir "vectorgrid-support-evidence.partial.zip"
Compress-Archive -Path (Join-Path $payloadRoot "*") -DestinationPath $partialArchivePath -CompressionLevel Optimal
Assert-SafeArchive -PayloadRoot $payloadRoot -ArchivePath $partialArchivePath
Move-Item -LiteralPath $partialArchivePath -Destination $archivePath
$archiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant()
$archiveHashPath = Join-Path $resolvedOutDir "vectorgrid-support-evidence.zip.sha256"
Write-Utf8NoBom -Path $archiveHashPath -Value "$archiveHash *vectorgrid-support-evidence.zip$([Environment]::NewLine)"

Write-Host "Support-evidence bundle created."
Write-Host "  included files: $($included.Count)"
Write-Host "  skipped non-allowlisted files: $skippedNotAllowlisted"
Write-Host "  payload: $payloadRoot"
Write-Host "  archive: $archivePath"
Write-Host "  archive SHA-256: $archiveHash"
