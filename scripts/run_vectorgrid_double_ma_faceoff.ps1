param(
    [string]$BenchmarkId = "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs",
    [int]$SampleSize = 10,
    [int]$MeasurementTimeSec = 3,
    [int]$WarmUpTimeSec = 1,
    [string]$Features = "cuda,cuda-backtest-kernel",
    [string]$OutputJson = "target/vectorgrid_double_ma_faceoff.json",
    [string]$RawOutputPath = "target/vectorgrid_double_ma_faceoff.raw.txt"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        $Value,
        [int]$Depth = 12
    )

    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }
    $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Read-JsonFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    return Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}

function Invoke-LoggedProcess {
    param(
        [string]$Exe,
        [string[]]$CmdArgs,
        [string]$OutputPath
    )

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Exe
    $psi.Arguments = [string]::Join(" ", ($CmdArgs | ForEach-Object {
        '"' + ([string]$_).Replace('"', '\"') + '"'
    }))
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.WorkingDirectory = (Get-Location).Path

    $proc = [System.Diagnostics.Process]::new()
    $proc.StartInfo = $psi
    [void]$proc.Start()
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()

    $combined = @($stdout, $stderr) -join ""
    Set-Content -Path $OutputPath -Value $combined -Encoding UTF8

    if ($proc.ExitCode -ne 0) {
        throw "Process failed ($Exe $($CmdArgs -join ' ')) exit=$($proc.ExitCode)`n$combined"
    }

    return $combined
}

function Resolve-CriterionEstimatePathFromBenchmarkId {
    param([Parameter(Mandatory = $true)][string]$Id)

    $normalizedId = ($Id -replace '[\\/]+', '_').Trim('_')
    $matches = @(Get-ChildItem -Path "target/criterion" -Recurse -File -Filter "estimates.json" | Where-Object {
            $newDir = Split-Path -Parent $_.FullName
            $leafDir = Split-Path -Parent $newDir
            $rootDir = Split-Path -Parent $leafDir
            $key = "{0}_{1}" -f (Split-Path -Leaf $rootDir), (Split-Path -Leaf $leafDir)
            $key -eq $normalizedId
        })

    if ($matches.Count -eq 0) {
        throw "Could not resolve Criterion estimates.json for benchmark id '$Id' (normalized '$normalizedId')."
    }
    if ($matches.Count -gt 1) {
        $paths = ($matches | ForEach-Object { $_.FullName }) -join "; "
        throw "Ambiguous Criterion estimates for benchmark id '$Id': $paths"
    }
    return $matches[0].FullName
}

function Parse-BenchmarkMetadata {
    param([string]$Id)

    $segments = @($Id.Split('/', [System.StringSplitOptions]::RemoveEmptyEntries))
    $pairCount = $null
    $bars = $null
    $pairIndex = -1
    $barsIndex = -1

    for ($i = 0; $i -lt $segments.Count; $i += 1) {
        if ($segments[$i] -match '^(?<pairs>\d+)pairs$') {
            $pairCount = [int64]$matches["pairs"]
            $pairIndex = $i
        }
        if ($segments[$i] -match '^(?<bars>\d+)(?<suffix>k|m)$') {
            $base = [int64]$matches["bars"]
            $bars = if ($matches["suffix"] -eq "k") { $base * 1000 } else { $base * 1000000 }
            $barsIndex = $i
        }
    }

    $variantSegments = @()
    if ($pairIndex -ge 0 -and $pairIndex + 1 -lt $segments.Count) {
        $variantSegments = @($segments[($pairIndex + 1)..($segments.Count - 1)])
    }

    return [ordered]@{
        strategy = if ($segments.Count -ge 1) { $segments[0] } else { "" }
        backend = if ($segments.Count -ge 2) { $segments[1] } else { "" }
        ma_case = if ($segments.Count -ge 3) { $segments[2] } else { "" }
        bars_segment = if ($barsIndex -ge 0) { $segments[$barsIndex] } else { "" }
        workload = if ($barsIndex -ge 0 -and $barsIndex + 1 -lt $segments.Count) { $segments[$barsIndex + 1] } else { "" }
        pair_segment = if ($pairIndex -ge 0) { $segments[$pairIndex] } else { "" }
        variant_segments = $variantSegments
        pair_count = $pairCount
        bars = $bars
        pair_bars = if ($null -ne $pairCount -and $null -ne $bars) { $pairCount * $bars } else { $null }
    }
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$targetDir = Split-Path -Parent $OutputJson
if ($targetDir -and -not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir | Out-Null
}
$rawDir = Split-Path -Parent $RawOutputPath
if ($rawDir -and -not (Test-Path $rawDir)) {
    New-Item -ItemType Directory -Path $rawDir | Out-Null
}

$cargoArgs = @(
    "bench",
    "-p", "ta_app_core",
    "--bench", "double_ma_opt",
    "--features", $Features,
    "--", $BenchmarkId,
    "--sample-size", "$SampleSize",
    "--measurement-time", "$MeasurementTimeSec",
    "--warm-up-time", "$WarmUpTimeSec"
)

$null = Invoke-LoggedProcess -Exe "cargo" -CmdArgs $cargoArgs -OutputPath $RawOutputPath
$estimatePath = Resolve-CriterionEstimatePathFromBenchmarkId -Id $BenchmarkId
$criterion = Read-JsonFile -Path $estimatePath
$meta = Parse-BenchmarkMetadata -Id $BenchmarkId

$result = [ordered]@{
    engine = "vectorgrid"
    benchmark_id = $BenchmarkId
    timing_boundary = "Criterion"
    timing_boundary_note = "Criterion bench-profile estimate for a single VectorGrid benchmark id."
    build_profile = "bench"
    strategy = $meta.strategy
    backend = $meta.backend
    ma_case = $meta.ma_case
    bars_segment = $meta.bars_segment
    workload = $meta.workload
    pair_segment = $meta.pair_segment
    variant_segments = $meta.variant_segments
    exact_search = $true
    pair_count = $meta.pair_count
    bars = $meta.bars
    pair_bars = $meta.pair_bars
    criterion = [ordered]@{
        estimates_json = $estimatePath
        mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
        mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
        mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
        median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
    }
    features = $Features
    sample_size = $SampleSize
    measurement_time_sec = $MeasurementTimeSec
    warmup_time_sec = $WarmUpTimeSec
    raw_output_path = $RawOutputPath
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
}

Write-JsonUtf8 -Path $OutputJson -Value $result
Get-Content -Raw -LiteralPath $OutputJson
