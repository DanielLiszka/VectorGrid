import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptsDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptsDirectory, "..");
const harnessPath = path.join(scriptsDirectory, "run_ml_signal_real_native_qa.ps1");
const baselinePath = path.join(
  scriptsDirectory,
  "baselines",
  "catboost-native-v1.2.10.json",
);

function runHarness(argumentsList) {
  return spawnSync(
    "powershell",
    ["-NoProfile", "-File", harnessPath, ...argumentsList],
    {
      cwd: repoRoot,
      encoding: "utf8",
      windowsHide: true,
    },
  );
}

test("real-native QA contract is parseable, bounded, and explicit about approval limits", () => {
  const result = runHarness(["-ContractOnly"]);
  assert.equal(result.status, 0, result.stderr);
  const contract = JSON.parse(result.stdout);

  assert.equal(contract.contract_version, 1);
  assert.equal(contract.opt_in, true);
  assert.equal(contract.default_max_rows, 4000);
  assert.equal(contract.max_rows_upper_bound, 10000);
  assert.equal(contract.qa_artifact_scope, "repository_output_only");
  assert.equal(
    contract.toolchain_cache_scope,
    "normal_repository_target_and_output_venv",
  );
  assert.equal(contract.source_mutation, "forbidden");
  assert.equal(contract.app_profile_access, "none");
  assert.equal(contract.backend, "native_cpu");
  assert.equal(
    contract.approval_boundary,
    "functional_inference_only_not_model_quality_or_trading_approval",
  );
  assert.deepEqual(contract.required_negative_checks, [
    "missing_native_library",
    "corrupt_native_library",
    "missing_model",
    "corrupt_model",
  ]);
  assert.equal(contract.cleanup_policy, "sentinel_owned_directories_only");
});

test("normal execution remains opt-in and does not create output without SourceCsv", () => {
  const qaRoot = path.join(repoRoot, "output", "qa", "ml-signal-real-native");
  const before = fs.existsSync(qaRoot)
    ? fs.readdirSync(qaRoot, { withFileTypes: true }).map((entry) => entry.name).sort()
    : [];
  const result = runHarness([]);
  const after = fs.existsSync(qaRoot)
    ? fs.readdirSync(qaRoot, { withFileTypes: true }).map((entry) => entry.name).sort()
    : [];

  assert.notEqual(result.status, 0);
  assert.match(`${result.stdout}\n${result.stderr}`, /SourceCsv is required/);
  assert.deepEqual(after, before);
});

test("output containment rejects a path outside repository output before any write", () => {
  const outside = path.join(
    os.tmpdir(),
    `vectorgrid-ml-signal-qa-contract-${process.pid}-${Date.now()}`,
  );
  const result = runHarness([
    "-SourceCsv",
    baselinePath,
    "-OutputDirectory",
    outside,
  ]);

  assert.notEqual(result.status, 0);
  assert.match(`${result.stdout}\n${result.stderr}`, /must remain under/);
  assert.equal(fs.existsSync(outside), false);
});

test("reviewed native baseline stays pinned to the exercised CatBoost release", () => {
  const baseline = JSON.parse(fs.readFileSync(baselinePath, "utf8"));
  assert.equal(baseline.version, "1.2.10");
  assert.equal(baseline.platform, "windows-x86_64");
  assert.equal(
    baseline.asset_sha256.toLowerCase(),
    "668d8867940a0832accaad6004e249f11ecc48eb63eb1d65551cfdd37c1939ad",
  );
  assert.equal(baseline.asset_size_bytes, 16438272);
});
