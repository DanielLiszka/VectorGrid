param(
  [Parameter(Mandatory = $true)]
  [ValidateSet("OpenDialog", "SaveDialog", "CancelDialog", "WaitCloseExplorer")]
  [string]$Mode,
  [string]$TargetPath = "",
  [string]$ExpectedDirectory = "",
  [Parameter(Mandatory = $true)]
  [string]$EvidencePath,
  [ValidateRange(5, 120)]
  [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = "Stop"

function Resolve-FullPath {
  param([Parameter(Mandatory = $true)][string]$Path)
  return [System.IO.Path]::GetFullPath($Path)
}

function Assert-PathUnderRoot {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string]$Purpose
  )
  $resolvedPath = Resolve-FullPath -Path $Path
  $resolvedRoot = Resolve-FullPath -Path $Root
  if (-not $resolvedRoot.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
    $resolvedRoot += [System.IO.Path]::DirectorySeparatorChar
  }
  if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "$Purpose escaped its required root: $resolvedPath"
  }
}

function Write-HarnessEvidence {
  param(
    [Parameter(Mandatory = $true)][string]$Status,
    [string]$Message = "",
    [hashtable]$Extra = @{}
  )
  $payload = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    status = $Status
    mode = $Mode
    message = $Message
  }
  foreach ($entry in $Extra.GetEnumerator()) {
    $payload[$entry.Key] = $entry.Value
  }
  $parent = Split-Path -Parent $EvidencePath
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
  $tempPath = "$EvidencePath.tmp"
  $payload | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $tempPath -Encoding UTF8
  Move-Item -LiteralPath $tempPath -Destination $EvidencePath -Force
}

function Get-ExactProductProcessIds {
  param([Parameter(Mandatory = $true)][string]$BinaryPath)
  $resolvedBinary = Resolve-FullPath -Path $BinaryPath
  $ids = @()
  foreach ($process in @(Get-Process -ErrorAction SilentlyContinue)) {
    try {
      if (
        -not [string]::IsNullOrWhiteSpace([string]$process.Path) -and
        [string]::Equals(
          (Resolve-FullPath -Path $process.Path),
          $resolvedBinary,
          [System.StringComparison]::OrdinalIgnoreCase
        )
      ) {
        $ids += [int]$process.Id
      }
    } catch {
      # Access-denied process metadata cannot identify the exact product and is ignored.
    }
  }
  return @($ids | Sort-Object -Unique)
}

function Get-OwnedTopLevelWindows {
  param([int[]]$ProcessIds)
  $root = [System.Windows.Automation.AutomationElement]::RootElement
  $children = $root.FindAll(
    [System.Windows.Automation.TreeScope]::Children,
    [System.Windows.Automation.Condition]::TrueCondition
  )
  $matches = @()
  foreach ($element in $children) {
    try {
      if ($ProcessIds -contains [int]$element.Current.ProcessId) {
        $matches += $element
      }
    } catch {
      # A disappearing window is expected while the dialog is closing.
    }
  }
  return $matches
}

function Find-DescendantByControlType {
  param(
    [Parameter(Mandatory = $true)]$Root,
    [Parameter(Mandatory = $true)]$ControlType
  )
  $condition = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    $ControlType
  )
  return $Root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Invoke-FileDialogAction {
  param(
    [Parameter(Mandatory = $true)]$Dialog,
    [Parameter(Mandatory = $true)][ValidateSet("Open", "Save", "Cancel")][string]$Action,
    [Parameter(Mandatory = $true)][string]$RequestedPath
  )

  if ($Action -eq "Open" -or $Action -eq "Save") {
    $edits = Find-DescendantByControlType `
      -Root $Dialog `
      -ControlType ([System.Windows.Automation.ControlType]::Edit)
    $fileNameEdit = $null
    foreach ($edit in $edits) {
      try {
        $automationId = [string]$edit.Current.AutomationId
        $name = [string]$edit.Current.Name
        if ($automationId -eq "1001" -or $name -match "File name") {
          $fileNameEdit = $edit
          break
        }
      } catch {
        continue
      }
    }
    if ($null -eq $fileNameEdit) {
      foreach ($edit in $edits) {
        try {
          $pattern = $edit.GetCurrentPattern(
            [System.Windows.Automation.ValuePattern]::Pattern
          )
          if ($null -ne $pattern -and $edit.Current.IsEnabled) {
            $fileNameEdit = $edit
          }
        } catch {
          continue
        }
      }
    }
    if ($null -eq $fileNameEdit) {
      throw "The exact product file dialog did not expose a filename field."
    }
    $valuePattern = $fileNameEdit.GetCurrentPattern(
      [System.Windows.Automation.ValuePattern]::Pattern
    )
    $valuePattern.SetValue($RequestedPath)
  }

  $buttons = Find-DescendantByControlType `
    -Root $Dialog `
    -ControlType ([System.Windows.Automation.ControlType]::Button)
  $actionButton = $null
  foreach ($button in $buttons) {
    try {
      $name = [string]$button.Current.Name
      $automationId = [string]$button.Current.AutomationId
      if (
        ($Action -eq "Open" -and ($automationId -eq "1" -or $name -match "^Open")) -or
        ($Action -eq "Save" -and ($automationId -eq "1" -or $name -match "^Save")) -or
        ($Action -eq "Cancel" -and ($automationId -eq "2" -or $name -match "^Cancel"))
      ) {
        $actionButton = $button
        break
      }
    } catch {
      continue
    }
  }
  if ($null -eq $actionButton) {
    throw "The exact product file dialog did not expose its $Action button."
  }
  $invokePattern = $actionButton.GetCurrentPattern(
    [System.Windows.Automation.InvokePattern]::Pattern
  )
  $invokePattern.Invoke()
}

function Convert-ExplorerLocation {
  param([string]$LocationUrl)
  if ([string]::IsNullOrWhiteSpace($LocationUrl)) {
    return ""
  }
  try {
    $uri = [Uri]$LocationUrl
    if (-not $uri.IsFile) {
      return ""
    }
    return Resolve-FullPath -Path $uri.LocalPath
  } catch {
    return ""
  }
}

$repoRoot = Resolve-FullPath -Path (Join-Path $PSScriptRoot "..")
$outputRoot = Resolve-FullPath -Path (Join-Path $repoRoot "output")
$isolationRoot = [string]$env:VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT
$binaryPath = [string]$env:VECTORGRID_DESKTOP_SMOKE_BINARY

try {
  if ([string]::IsNullOrWhiteSpace($isolationRoot)) {
    throw "VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT is required."
  }
  if ([string]::IsNullOrWhiteSpace($binaryPath) -or -not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
    throw "VECTORGRID_DESKTOP_SMOKE_BINARY must identify the exact packaged executable."
  }
  Assert-PathUnderRoot -Path $isolationRoot -Root $outputRoot -Purpose "Isolation root"
  Assert-PathUnderRoot -Path $EvidencePath -Root $outputRoot -Purpose "Evidence path"

  if ($Mode -eq "OpenDialog" -or $Mode -eq "SaveDialog" -or $Mode -eq "CancelDialog") {
    if ([string]::IsNullOrWhiteSpace($TargetPath)) {
      throw "$Mode requires -TargetPath."
    }
    $TargetPath = Resolve-FullPath -Path $TargetPath
    Assert-PathUnderRoot -Path $TargetPath -Root $isolationRoot -Purpose "Native dialog target"
    if ($Mode -eq "SaveDialog") {
      New-Item -ItemType Directory -Force -Path (Split-Path -Parent $TargetPath) | Out-Null
    } elseif ($Mode -eq "OpenDialog" -and -not (Test-Path -LiteralPath $TargetPath -PathType Leaf)) {
      throw "OpenDialog target must be an existing file: $TargetPath"
    }

    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    Write-HarnessEvidence -Status "ready" -Extra @{ target_path = $TargetPath }
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $handledWindow = $null
    while ([DateTime]::UtcNow -lt $deadline -and $null -eq $handledWindow) {
      $processIds = Get-ExactProductProcessIds -BinaryPath $binaryPath
      if ($processIds.Count -eq 1) {
        foreach ($window in @(Get-OwnedTopLevelWindows -ProcessIds $processIds)) {
          try {
            $name = [string]$window.Current.Name
            $className = [string]$window.Current.ClassName
            if (
              $window.Current.IsEnabled -and
              $className -eq "#32770" -and
              (
                $name -match "Open|Save|Select" -or
                (Find-DescendantByControlType `
                  -Root $window `
                  -ControlType ([System.Windows.Automation.ControlType]::Edit
                )).Count -gt 0
              )
            ) {
              $handledWindow = $window
              break
            }
          } catch {
            continue
          }
        }
      } elseif ($processIds.Count -gt 1) {
        throw "Expected one exact packaged product process; found $($processIds.Count)."
      }
      if ($null -eq $handledWindow) {
        Start-Sleep -Milliseconds 100
      }
    }
    if ($null -eq $handledWindow) {
      throw "No owned native file dialog appeared within $TimeoutSeconds seconds."
    }

    $nativeHandle = [int64]$handledWindow.Current.NativeWindowHandle
    $action = if ($Mode -eq "OpenDialog") {
      "Open"
    } elseif ($Mode -eq "SaveDialog") {
      "Save"
    } else {
      "Cancel"
    }
    Invoke-FileDialogAction -Dialog $handledWindow -Action $action -RequestedPath $TargetPath
    Write-HarnessEvidence -Status "passed" -Extra @{
      target_path = $TargetPath
      action = $action
      window_handle = $nativeHandle
    }
    exit 0
  }

  if ([string]::IsNullOrWhiteSpace($ExpectedDirectory)) {
    throw "WaitCloseExplorer requires -ExpectedDirectory."
  }
  $ExpectedDirectory = Resolve-FullPath -Path $ExpectedDirectory
  Assert-PathUnderRoot -Path $ExpectedDirectory -Root $isolationRoot -Purpose "Expected Explorer directory"
  $shell = New-Object -ComObject Shell.Application
  $baselineHandles = @(
    $shell.Windows() |
      ForEach-Object { [int64]$_.HWND } |
      Sort-Object -Unique
  )
  Write-HarnessEvidence -Status "ready" -Extra @{
    expected_directory = $ExpectedDirectory
    baseline_handles = $baselineHandles
  }
  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $matchedWindow = $null
  while ([DateTime]::UtcNow -lt $deadline -and $null -eq $matchedWindow) {
    foreach ($window in @($shell.Windows())) {
      try {
        $handle = [int64]$window.HWND
        if ($baselineHandles -contains $handle) {
          continue
        }
        $location = Convert-ExplorerLocation -LocationUrl ([string]$window.LocationURL)
        if (
          -not [string]::IsNullOrWhiteSpace($location) -and
          [string]::Equals(
            $location,
            $ExpectedDirectory,
            [System.StringComparison]::OrdinalIgnoreCase
          )
        ) {
          $matchedWindow = $window
          break
        }
      } catch {
        continue
      }
    }
    if ($null -eq $matchedWindow) {
      Start-Sleep -Milliseconds 100
    }
  }
  if ($null -eq $matchedWindow) {
    throw "No new contained Explorer window appeared within $TimeoutSeconds seconds."
  }

  $matchedHandle = [int64]$matchedWindow.HWND
  $matchedWindow.Quit()
  $closeDeadline = [DateTime]::UtcNow.AddSeconds(5)
  do {
    Start-Sleep -Milliseconds 100
    $stillOpen = @(
      $shell.Windows() |
        Where-Object { [int64]$_.HWND -eq $matchedHandle }
    ).Count -gt 0
  } while ($stillOpen -and [DateTime]::UtcNow -lt $closeDeadline)
  if ($stillOpen) {
    throw "The contained Explorer window did not close."
  }
  Write-HarnessEvidence -Status "passed" -Extra @{
    expected_directory = $ExpectedDirectory
    window_handle = $matchedHandle
    baseline_handles = $baselineHandles
  }
  exit 0
} catch {
  Write-HarnessEvidence -Status "failed" -Message $_.Exception.Message
  Write-Error $_
  exit 1
}
