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

function Convert-ToMilliseconds {
    param(
        [double]$Value,
        [string]$Unit
    )

    switch ($Unit) {
        "ns" { return $Value / 1000000.0 }
        "us" { return $Value / 1000.0 }
        "µs" { return $Value / 1000.0 }
        "ms" { return $Value }
        "s"  { return $Value * 1000.0 }
        default { throw "Unsupported Criterion unit '$Unit'." }
    }
}

function Parse-TimeLine {
    param([string]$Text)

    $re = [regex]'time:\s*\[(?<low>[\d\.]+)\s*(?<unit>[a-zµ]+)\s+(?<mid>[\d\.]+)\s*[a-zµ]+\s+(?<high>[\d\.]+)\s*[a-zµ]+\]'
    $m = $re.Match($Text)
    if (-not $m.Success) {
        throw "Unable to parse Criterion time line."
    }

    $unit = $m.Groups["unit"].Value
    return @{
        low_ms = Convert-ToMilliseconds ([double]$m.Groups["low"].Value) $unit
        point_estimate_ms = Convert-ToMilliseconds ([double]$m.Groups["mid"].Value) $unit
        high_ms = Convert-ToMilliseconds ([double]$m.Groups["high"].Value) $unit
    }
}

function Parse-BenchmarkMetadata {
    param([string]$Id)

    $pairs = $null
    $bars = $null

    $pairsMatch = [regex]::Match($Id, '(?<pairs>\d+)pairs')
    if ($pairsMatch.Success) {
        $pairs = [int64]$pairsMatch.Groups["pairs"].Value
    }

    $barsMatch = [regex]::Match($Id, '/(?<bars>\d+)(?<suffix>k|m)(/|$)')
    if ($barsMatch.Success) {
        $base = [int64]$barsMatch.Groups["bars"].Value
        $suffix = $barsMatch.Groups["suffix"].Value
        if ($suffix -eq "k") {
            $bars = $base * 1000
        } elseif ($suffix -eq "m") {
            $bars = $base * 1000000
        }
    }

    return @{
        pair_count = $pairs
        bars = $bars
        pair_bars = if ($pairs -and $bars) { $pairs * $bars } else { $null }
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

$joined = Invoke-LoggedProcess -Exe "cargo" -CmdArgs $cargoArgs -OutputPath $RawOutputPath
$time = Parse-TimeLine -Text $joined
$meta = Parse-BenchmarkMetadata -Id $BenchmarkId

$result = [ordered]@{
    engine = "vectorgrid"
    mode = "gpu_kernel"
    benchmark_id = $BenchmarkId
    strategy = "double_ma"
    ma_case = "alma_alma"
    objective = "sharpe"
    exact_search = $true
    backend = "GpuKernel"
    pair_count = $meta.pair_count
    bars = $meta.bars
    pair_bars = $meta.pair_bars
    runtime = $time
    features = $Features
    sample_size = $SampleSize
    measurement_time_sec = $MeasurementTimeSec
    warmup_time_sec = $WarmUpTimeSec
    raw_output_path = $RawOutputPath
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
}

$json = $result | ConvertTo-Json -Depth 6
Set-Content -Path $OutputJson -Value $json
$json
