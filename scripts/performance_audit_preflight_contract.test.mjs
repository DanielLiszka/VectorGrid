import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptsDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.dirname(scriptsDir);
const helperPath = path.join(scriptsDir, "performance_audit_preflight.ps1");
const runnerPath = path.join(
  scriptsDir,
  "run_performance_audit_baseline_pack.ps1",
);
const registryPath = path.join(
  scriptsDir,
  "performance_audit_case_registry.json",
);
const appCoreManifestPath = path.join(
  repoRoot,
  "crates",
  "app_core",
  "Cargo.toml",
);
const powershell = process.platform === "win32" ? "powershell.exe" : "pwsh";

function psLiteral(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function runPowerShell(source, timeout = 10_000) {
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

test("bounded native snapshot detects known active performance processes", () => {
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    $rows = @(Get-VectorGridActivePerformanceProcesses -TimeoutMilliseconds 3000 -ExcludeProcessId @() -SnapshotProvider {
      @(
        [pscustomobject]@{ ProcessName = "node"; Id = 101 },
        [pscustomobject]@{ ProcessName = "cargo"; Id = 102 },
        [pscustomobject]@{ ProcessName = "double_ma_opt-deadbeef"; Id = 103 },
        [pscustomobject]@{ ProcessName = "ncu.exe"; Id = 104 },
        [pscustomobject]@{ ProcessName = "composite_mo_opt-1234"; Id = 105 },
        [pscustomobject]@{ ProcessName = "options_expression_gpu_score-abcd"; Id = 106 }
      )
    })
    [Console]::Out.Write(($rows | ConvertTo-Json -Compress))
  `);
  assert.equal(result.status, 0, result.stderr);
  const rows = JSON.parse(result.stdout);
  assert.deepEqual(
    rows.map((row) => [row.id, row.name]),
    [
      [102, "cargo"],
      [103, "double_ma_opt-deadbeef"],
      [104, "ncu.exe"],
      [105, "composite_mo_opt-1234"],
      [106, "options_expression_gpu_score-abcd"],
    ],
  );
});

test("native process-name detection covers every app-core Criterion bench", () => {
  const manifest = fs.readFileSync(appCoreManifestPath, "utf8");
  const benchNames = [
    ...manifest.matchAll(/\[\[bench\]\]\s+name\s*=\s*"([^"]+)"/g),
  ].map((match) => match[1]);
  assert.ok(benchNames.length > 0);
  const simulatedRows = benchNames
    .map(
      (name, index) =>
        `[pscustomobject]@{ ProcessName = ${psLiteral(`${name}-deadbeef`)}; Id = ${200 + index} }`,
    )
    .join(",\n");
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    $rows = @(Get-VectorGridActivePerformanceProcesses -TimeoutMilliseconds 3000 -ExcludeProcessId @() -SnapshotProvider {
      @(
        ${simulatedRows}
      )
    })
    [Console]::Out.Write(($rows | ConvertTo-Json -Compress))
  `);
  assert.equal(result.status, 0, result.stderr);
  const detected = JSON.parse(result.stdout).map((row) => row.name);
  assert.deepEqual(
    detected,
    benchNames.map((name) => `${name}-deadbeef`),
  );
});

test("snapshot timeout fails closed within a bounded interval", () => {
  const started = Date.now();
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    try {
      Get-VectorGridActivePerformanceProcesses -TimeoutMilliseconds 100 -SnapshotProvider {
        Start-Sleep -Seconds 5
        @()
      } | Out-Null
      exit 0
    } catch {
      [Console]::Error.Write($_.Exception.Message)
      exit 42
    }
  `);
  assert.equal(result.status, 42, result.stderr);
  assert.match(result.stderr, /failed closed.*timed out after 100ms/i);
  assert.ok(
    Date.now() - started < 4_000,
    "the simulated hung provider must not control the outer runtime",
  );
});

test("snapshot provider failure fails closed", () => {
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    try {
      Get-VectorGridActivePerformanceProcesses -TimeoutMilliseconds 3000 -SnapshotProvider {
        throw "simulated provider failure"
      } | Out-Null
      exit 0
    } catch {
      [Console]::Error.Write($_.Exception.Message)
      exit 43
    }
  `);
  assert.equal(result.status, 43, result.stderr);
  assert.match(result.stderr, /failed closed.*simulated provider failure/i);
});

test("release-baseline policy reports missing files without inventing them", () => {
  const result = runPowerShell(`
    . ${psLiteral(helperPath)}
    $registry = [pscustomobject]@{
      cases = @(
        [pscustomobject]@{
          id = "present"
          release_required_baseline = $true
          baseline_path = "Cargo.toml"
        },
        [pscustomobject]@{
          id = "missing"
          release_required_baseline = $true
          baseline_path = "scripts/baselines/__must_not_exist__.json"
        },
        [pscustomobject]@{
          id = "not-release-required"
          release_required_baseline = $false
          baseline_path = "scripts/baselines/__ignored__.json"
        }
      )
    }
    $rows = @(Get-VectorGridReleaseBaselineHealth -Registry $registry -RepoRoot ${psLiteral(repoRoot)})
    [Console]::Out.Write(($rows | ConvertTo-Json -Compress))
  `);
  assert.equal(result.status, 0, result.stderr);
  const rows = JSON.parse(result.stdout);
  assert.deepEqual(
    rows.map((row) => [row.case_id, row.exists]),
    [
      ["present", true],
      ["missing", false],
    ],
  );
  assert.equal(
    fs.existsSync(
      path.join(repoRoot, "scripts", "baselines", "__must_not_exist__.json"),
    ),
    false,
  );
});

test("GPU-MA is an explicit release-required baseline and strict inventory can enforce it", () => {
  const registry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
  const gpuMaCases = registry.cases.filter(
    (entry) => entry.id === "gpu.optimization_residency_ma_sweep_route",
  );
  assert.equal(gpuMaCases.length, 1);
  assert.equal(gpuMaCases[0].release_required_baseline, true);
  assert.equal(
    gpuMaCases[0].baseline_path,
    "scripts/baselines/gpu_ma_perf_baseline.json",
  );

  const runner = fs.readFileSync(runnerPath, "utf8");
  assert.match(runner, /\[switch\]\$FailOnMissingReleaseBaseline/);
  assert.match(runner, /Release-required performance baselines are missing/);
  assert.match(runner, /release_baseline_health\.json/);
});

test("preflight never terminates an observed operating-system process", () => {
  const helper = fs.readFileSync(helperPath, "utf8");
  assert.doesNotMatch(helper, /\bStop-Process\b|\btaskkill\b|\.Kill\s*\(/i);
});
