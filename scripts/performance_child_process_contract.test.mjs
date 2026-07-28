import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptsDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.dirname(scriptsDir);
const helperPath = path.join(scriptsDir, "performance_child_process.ps1");
const gatePath = path.join(scriptsDir, "run_gpu_validation_perf_gate.ps1");
const powershell = process.platform === "win32" ? "powershell.exe" : "pwsh";

function psLiteral(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function runPowerShell(source, timeout = 15_000) {
  const encoded = Buffer.from(source, "utf16le").toString("base64");
  return spawnSync(
    powershell,
    ["-NoProfile", "-ExecutionPolicy", "Bypass", "-EncodedCommand", encoded],
    {
      cwd: repoRoot,
      encoding: "utf8",
      timeout,
      windowsHide: true,
    },
  );
}

test("GPU-validation process selection detects active work without false build or shell matches", () => {
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    $completed = @{ "205" = [int64]2050 }
    $snapshot = @(
      [pscustomobject]@{ ProcessName = "cargo"; Id = 201; ParentId = 1; CommandLine = "cargo bench -p ta_app_core"; StartTicks = [int64]2010 },
      [pscustomobject]@{ ProcessName = "cargo"; Id = 202; ParentId = 1; CommandLine = "cargo bench -p ta_app_core --no-run"; StartTicks = [int64]2020 },
      [pscustomobject]@{ ProcessName = "powershell"; Id = 203; ParentId = 1; CommandLine = "powershell criterion --bench"; StartTicks = [int64]2030 },
      [pscustomobject]@{ ProcessName = "double_ma_validation_workflow-abcd"; Id = 204; ParentId = 1; CommandLine = ""; StartTicks = [int64]2040 },
      [pscustomobject]@{ ProcessName = "ncu"; Id = 205; ParentId = 1; CommandLine = ""; StartTicks = [int64]2050 },
      [pscustomobject]@{ ProcessName = "node"; Id = 206; ParentId = 1; CommandLine = "node app.mjs"; StartTicks = [int64]2060 }
    )
    $active = @(Select-VectorGridGpuValidationPerformanceProcesses -Snapshot $snapshot -CompletedProcessStartTicksById $completed)
    [Console]::Out.Write(($active | Select-Object Id, ProcessName | ConvertTo-Json -Compress))
  `);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), [
    { Id: 201, ProcessName: "cargo" },
    { Id: 204, ProcessName: "double_ma_validation_workflow-abcd" },
  ]);
});

test("launched-tree selection includes only verified descendants", () => {
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    $snapshot = @(
      [pscustomobject]@{ ProcessName = "root"; Id = 100; ParentId = 50; CommandLine = ""; StartTicks = 1000 },
      [pscustomobject]@{ ProcessName = "child"; Id = 101; ParentId = 100; CommandLine = ""; StartTicks = 1100 },
      [pscustomobject]@{ ProcessName = "grandchild"; Id = 102; ParentId = 101; CommandLine = ""; StartTicks = 1200 },
      [pscustomobject]@{ ProcessName = "unrelated"; Id = 103; ParentId = 50; CommandLine = ""; StartTicks = 1300 },
      [pscustomobject]@{ ProcessName = "stale"; Id = 104; ParentId = 100; CommandLine = ""; StartTicks = 900 }
    )
    $tree = @(Get-VectorGridLaunchedProcessTree -Snapshot $snapshot -RootProcessId 100 -RootStartTicks 1000)
    [Console]::Out.Write(($tree | Select-Object Id, ParentId, Depth | ConvertTo-Json -Compress))
  `);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), [
    { Id: 100, ParentId: 50, Depth: 0 },
    { Id: 101, ParentId: 100, Depth: 1 },
    { Id: 102, ParentId: 101, Depth: 2 },
  ]);
});

test("bounded child invocation returns output and process identity", () => {
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    $result = Invoke-VectorGridBoundedChildProcess -FilePath "powershell.exe" -ArgumentList @(
      "-NoProfile",
      "-Command",
      "Write-Output bounded-child-ok"
    ) -TimeoutSeconds 5
    [Console]::Out.Write(($result | ConvertTo-Json -Compress))
  `);
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.exit_code, 0);
  assert.ok(parsed.process_id > 0);
  assert.ok(parsed.process_start_ticks > 0);
  assert.deepEqual(parsed.output, ["bounded-child-ok"]);
});

test(
  "timeout cleans only the launched root and child while preserving an unrelated process",
  { skip: process.platform !== "win32" },
  () => {
    const result = runPowerShell(
      `
      . ${psLiteral(helperPath)}
      $pidFile = Join-Path $env:TEMP ("vectorgrid-child-tree-" + [guid]::NewGuid().ToString("N") + ".txt")
      $sleepSource = "Start-Sleep -Seconds 30"
      $sleepEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($sleepSource))
      $unrelated = Start-Process -FilePath "powershell.exe" -ArgumentList @(
        "-NoProfile", "-EncodedCommand", $sleepEncoded
      ) -PassThru -WindowStyle Hidden
      $rootTemplate = @'
$child = Start-Process -FilePath "powershell.exe" -ArgumentList @(
  "-NoProfile", "-EncodedCommand", "__SLEEP_ENCODED__"
) -PassThru -WindowStyle Hidden
[IO.File]::WriteAllText("__PID_FILE__", "$PID,$($child.Id)")
Start-Sleep -Seconds 30
'@
      $escapedPidFile = $pidFile.Replace("'", "''")
      $rootSource = $rootTemplate.
        Replace("__SLEEP_ENCODED__", $sleepEncoded).
        Replace("__PID_FILE__", $escapedPidFile)
      $rootEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($rootSource))
      $providerTemplate = @'
$ids = [IO.File]::ReadAllText("__PID_FILE__").Split(",")
$root = [Diagnostics.Process]::GetProcessById([int]$ids[0])
$child = [Diagnostics.Process]::GetProcessById([int]$ids[1])
$unrelated = [Diagnostics.Process]::GetProcessById(__UNRELATED_ID__)
@(
  [pscustomobject]@{ Name = $root.ProcessName; ProcessId = $root.Id; ParentProcessId = 0; CommandLine = ""; CreationDate = $root.StartTime },
  [pscustomobject]@{ Name = $child.ProcessName; ProcessId = $child.Id; ParentProcessId = $root.Id; CommandLine = ""; CreationDate = $child.StartTime },
  [pscustomobject]@{ Name = $unrelated.ProcessName; ProcessId = $unrelated.Id; ParentProcessId = 0; CommandLine = ""; CreationDate = $unrelated.StartTime }
)
'@
      $providerSource = $providerTemplate.
        Replace("__PID_FILE__", $escapedPidFile).
        Replace("__UNRELATED_ID__", "$($unrelated.Id)")
      $provider = [scriptblock]::Create($providerSource)
      $errorMessage = ""
      $rootId = 0
      $childId = 0
      try {
        Invoke-VectorGridBoundedChildProcess -FilePath "powershell.exe" -ArgumentList @(
          "-NoProfile", "-EncodedCommand", $rootEncoded
        ) -TimeoutSeconds 2 -SnapshotTimeoutMilliseconds 3000 -CleanupTimeoutSeconds 5 -CleanupSnapshotProvider $provider | Out-Null
        throw "bounded child unexpectedly completed"
      } catch {
        $errorMessage = $_.Exception.Message
      }
      if (Test-Path -LiteralPath $pidFile -PathType Leaf) {
        $ids = [IO.File]::ReadAllText($pidFile).Split(",")
        $rootId = [int]$ids[0]
        $childId = [int]$ids[1]
      }
      Start-Sleep -Milliseconds 100
      $rootAlive = $rootId -gt 0 -and $null -ne (Get-Process -Id $rootId -ErrorAction SilentlyContinue)
      $childAlive = $childId -gt 0 -and $null -ne (Get-Process -Id $childId -ErrorAction SilentlyContinue)
      $unrelatedAlive = $null -ne (Get-Process -Id $unrelated.Id -ErrorAction SilentlyContinue)
      $evidence = [ordered]@{
        error = $errorMessage
        root_id = $rootId
        child_id = $childId
        root_alive = $rootAlive
        child_alive = $childAlive
        unrelated_alive = $unrelatedAlive
      }
      try {
        foreach ($ownedId in @($rootId, $childId, $unrelated.Id)) {
          if ($ownedId -gt 0) {
            $owned = Get-Process -Id $ownedId -ErrorAction SilentlyContinue
            if ($null -ne $owned) {
              $owned.Kill()
            }
          }
        }
        Remove-Item -LiteralPath $pidFile -Force -ErrorAction SilentlyContinue
      } catch {
      }
      [Console]::Out.Write(($evidence | ConvertTo-Json -Compress))
    `,
      20_000,
    );
    assert.equal(result.status, 0, result.stderr);
    const parsed = JSON.parse(result.stdout);
    assert.match(parsed.error, /timed out after 2s/i);
    assert.ok(parsed.root_id > 0);
    assert.ok(parsed.child_id > 0);
    assert.equal(parsed.root_alive, false);
    assert.equal(parsed.child_alive, false);
    assert.equal(parsed.unrelated_alive, true);
  },
);

test("GPU-validation gate has no unbounded CIM scan or Start-Process wait", () => {
  const gate = fs.readFileSync(gatePath, "utf8");
  assert.doesNotMatch(gate, /\bGet-CimInstance\b/);
  assert.doesNotMatch(gate, /\bStart-Process\b[\s\S]{0,160}\s-Wait\b/i);
  assert.match(gate, /performance_child_process\.ps1/);
  assert.match(gate, /Invoke-VectorGridBoundedChildProcess/);
  assert.match(gate, /cleanup_scope\s*=\s*"launched_root_and_verified_descendants_only"/);
});

test("GPU-validation plan-only exposes bounded process policy without launching work", () => {
  const tempRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-gpu-validation-plan-"),
  );
  const outPath = path.join(tempRoot, "plan.json");
  try {
    const result = spawnSync(
      powershell,
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        gatePath,
        "-PlanOnly",
        "-OutPath",
        outPath,
      ],
      {
        cwd: repoRoot,
        encoding: "utf8",
        timeout: 10_000,
        windowsHide: true,
      },
    );
    assert.equal(result.status, 0, result.stderr);
    const plan = JSON.parse(
      fs.readFileSync(outPath, "utf8").replace(/^\uFEFF/, ""),
    );
    assert.deepEqual(plan.process_safety, {
      snapshot_timeout_milliseconds: 5000,
      child_process_timeout_seconds: 1800,
      child_cleanup_timeout_seconds: 15,
      snapshot_failure_policy: "fail_closed",
      cleanup_scope: "launched_root_and_verified_descendants_only",
    });
    assert.equal(plan.criterion_enabled, true);
    assert.equal(plan.structured_probe_enabled, true);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("child cleanup never uses a system-wide termination command", () => {
  const helper = fs.readFileSync(helperPath, "utf8");
  assert.doesNotMatch(helper, /\bStop-Process\b|\btaskkill\b/i);
  assert.match(helper, /Get-VectorGridLaunchedProcessTree/);
  assert.match(helper, /Refusing to terminate reused PID/);
});
