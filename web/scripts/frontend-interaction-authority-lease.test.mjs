import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const leaseModule = path.join(
  scriptDir,
  "frontend-interaction-authority-lease.mjs",
);
const hostScript = path.join(
  scriptDir,
  "support",
  "frontend-interaction-authority-lease-host.ps1",
);

function runLeaseHost(mode) {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), `vectorgrid-authority-lease-${mode}-`),
  );
  const runId = "0123456789abcdef0123456789abcdef";
  const nonce = "fedcba9876543210fedcba9876543210";
  const leasePath = path.join(
    repoRoot,
    "output",
    "qa",
    "frontend-interaction-latency",
    "leases",
    `${runId}.json`,
  );
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      hostScript,
      "-Mode",
      mode,
      "-RepoRoot",
      repoRoot,
      "-LeasePath",
      leasePath,
      "-Nonce",
      nonce,
      "-RunId",
      runId,
      "-LeaseModule",
      leaseModule,
    ],
    {
      encoding: "utf8",
      timeout: 20_000,
      windowsHide: true,
    },
  );
  const lines = String(result.stdout || "")
    .trim()
    .split(/\r?\n/)
    .filter(Boolean);
  const evidence = JSON.parse(lines.at(-1) || "{}");
  return { result, evidence };
}

test(
  "live direct-parent lease, file-share hold, ancestry, and mutex prove authority",
  { skip: process.platform !== "win32" },
  () => {
    const { result, evidence } = runLeaseHost("positive");
    assert.equal(result.status, 0, result.stderr);
    assert.equal(evidence.status, "passed");
    assert.equal(evidence.verified, true);
    assert.equal(evidence.mutex_probe_exit_code, 0);
    assert.equal(evidence.write_probe_win32_error_code, 32);
    assert.equal(evidence.write_probe_exit_code, 0);
    assert.equal(
      evidence.benchmark_lease_path,
      "output/qa/benchmark-lock/active.json",
    );
    assert.equal(evidence.benchmark_lease_write_probe_win32_error_code, 32);
    assert.equal(evidence.benchmark_lease_write_probe_exit_code, 0);
    assert.equal(evidence.benchmark_lease_holder_verified, true);
    assert.equal(evidence.benchmark_lease_holder_probe_exit_code, 0);
  },
);

test(
  "well-formed environment and live parent cannot spoof an unlocked lease",
  { skip: process.platform !== "win32" },
  () => {
    const { result, evidence } = runLeaseHost("spoof");
    assert.notEqual(result.status, 0);
    assert.equal(evidence.status, "failed");
    assert.ok(
      evidence.issues.some((issue) => issue.includes("sharing violation")),
    );
    assert.ok(evidence.issues.some((issue) => issue.includes("mutex")));
  },
);

test(
  "ACL write denial without a live handle is not accepted as sharing proof",
  { skip: process.platform !== "win32" },
  () => {
    const { result, evidence } = runLeaseHost("acl-denied");
    assert.notEqual(result.status, 0);
    assert.equal(evidence.status, "failed");
    assert.notEqual(evidence.write_probe_win32_error_code, 32);
    assert.ok(
      evidence.issues.some((issue) => issue.includes("sharing violation")),
    );
  },
);

test(
  "stale lease fails even while its direct parent holds both resources",
  { skip: process.platform !== "win32" },
  () => {
    const { result, evidence } = runLeaseHost("stale");
    assert.notEqual(result.status, 0);
    assert.ok(evidence.issues.some((issue) => issue.includes("stale")));
  },
);

test(
  "dead or unrelated claimed mutex owner fails ancestry and environment binding",
  { skip: process.platform !== "win32" },
  () => {
    const { result, evidence } = runLeaseHost("dead-owner");
    assert.notEqual(result.status, 0);
    assert.ok(evidence.issues.some((issue) => issue.includes("live ancestor")));
  },
);

test(
  "mutex release after lease creation fails the child probe",
  { skip: process.platform !== "win32" },
  () => {
    const { result, evidence } = runLeaseHost("released-mutex");
    assert.notEqual(result.status, 0);
    assert.ok(evidence.issues.some((issue) => issue.includes("mutex")));
  },
);

test(
  "benchmark token mutation fails exact environment binding",
  { skip: process.platform !== "win32" },
  () => {
    const { result, evidence } = runLeaseHost("token-mismatch");
    assert.notEqual(result.status, 0);
    assert.ok(evidence.issues.some((issue) => issue.includes("environment")));
  },
);
