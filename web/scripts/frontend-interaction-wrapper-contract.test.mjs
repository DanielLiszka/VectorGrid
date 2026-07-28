import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..");

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

test("Vitest excludes Node-native latency contract files", () => {
  const viteConfig = read("web/vite.config.ts");
  assert.ok(viteConfig.includes("'scripts/**/*.test.mjs'"));
});

test("wrapper holds the benchmark lock across build, measurement, and report validation", () => {
  const wrapper = read("scripts/run_frontend_interaction_latency.ps1");
  const lockIndex = wrapper.indexOf(
    "$lock = Enter-BenchmarkLock -Context $benchmarkContext",
  );
  const leaseIndex = wrapper.indexOf("$authorityLease = New-AuthorityLease");
  const buildIndex = wrapper.indexOf("& npm.cmd --prefix web run build");
  const runnerIndex = wrapper.indexOf("& node $runner");
  const reportValidationIndex = wrapper.indexOf(
    "$stableReportRead = Read-StableAuthorityJson -Path $resolvedOut",
  );
  const leaseDisposeIndex = wrapper.lastIndexOf(
    "$authorityLease.stream.Dispose()",
  );
  const unlockIndex = wrapper.lastIndexOf("Exit-BenchmarkLock $lock");
  for (const index of [
    lockIndex,
    leaseIndex,
    buildIndex,
    runnerIndex,
    reportValidationIndex,
    leaseDisposeIndex,
    unlockIndex,
  ]) {
    assert.ok(index >= 0, "expected wrapper lifecycle marker");
  }
  assert.ok(lockIndex < buildIndex);
  assert.ok(lockIndex < leaseIndex);
  assert.ok(leaseIndex < buildIndex);
  assert.ok(buildIndex < runnerIndex);
  assert.ok(runnerIndex < reportValidationIndex);
  assert.ok(reportValidationIndex < leaseDisposeIndex);
  assert.ok(leaseDisposeIndex < unlockIndex);
  assert.ok(reportValidationIndex < unlockIndex);
});

test("wrapper fails closed on relevant concurrency, stale output, and unattested SkipBuild", () => {
  const wrapper = read("scripts/run_frontend_interaction_latency.ps1");
  for (const marker of [
    "cargo.exe",
    "node.exe",
    "vite",
    "playwright",
    "chrome.exe",
    "return ,$ids",
    "Invoke-VectorGridBoundedProcessSnapshot",
    "-TimeoutMilliseconds 5000",
    "Resolve-BoundedOutputPath -Path $OutPath -MustNotExist",
    "-SkipBuild requires a fresh build attestation",
    "--expected-source-sha",
    "--max-age-minutes",
    "Path must not be a reparse-point target",
    "Output root must be an ordinary directory",
    "Path must not be a hardlink target",
    "inspect-output",
  ]) {
    assert.ok(wrapper.includes(marker), `missing wrapper guard: ${marker}`);
  }
});

test("authority runner verifies lease, ancestry, mutex, and snapshot at both boundaries", () => {
  const runner = read("web/scripts/measure-frontend-interaction-latency.mjs");
  for (const marker of [
    "verifyAuthorityLease",
    "initialLockEvidence",
    "finalLockEvidence",
    "verified_at_start",
    "verified_before_publication",
    "validateImmutableDistSnapshot",
    "startStaticServerFromSnapshot",
    "served_from_memory_snapshot: true",
  ]) {
    assert.ok(
      runner.includes(marker),
      `missing runner authority guard: ${marker}`,
    );
  }
});

test("post-run validator recomputes raw samples and binds every authority artifact", () => {
  const validator = read(
    "web/scripts/frontend-interaction-report-validator.mjs",
  );
  for (const marker of [
    "recomputeLatencyReport",
    "expectedReportSha256",
    "build_attestation_sha256",
    "served_dist_snapshot_manifest_sha256",
    "served_dist_snapshot_path_manifest_sha256",
    "served_dist_snapshot_file_count",
    "served_dist_snapshot_total_bytes",
    "initial_lease_sha256",
    "measurement_harness_path_manifest_sha256",
    "measurement_harness_file_count",
    "measurement_harness_total_bytes",
    "effective_fixture_sha256",
    "attested_node_version",
    "computeAuthorityScopedWorktreeState",
    "scoped_worktree_status_sha256",
    "release_evidence_eligible",
    "dirty-source-local-candidate-rehearsal",
    "computeCanonicalFileSetSnapshot",
    "buildPathDistinctDetailFixture",
    "validateBuildAttestation",
    "validateImmutableDistSnapshot",
    "verifyAuthorityLease",
  ]) {
    assert.ok(
      validator.includes(marker),
      `missing post-run validator binding: ${marker}`,
    );
  }
});

test("frontend E2E latency opt-in delegates to the locked attested build", () => {
  const e2eGate = read("scripts/run_frontend_e2e_gate.ps1");
  const latencyBlock = e2eGate.slice(
    e2eGate.indexOf("if ($IncludeInteractionLatency)"),
    e2eGate.indexOf("if ($IncludeQaReport)"),
  );
  assert.ok(latencyBlock.includes("run_frontend_interaction_latency.ps1"));
  assert.ok(!latencyBlock.includes("-SkipBuild"));
});

test("measurement runner guarantees server cleanup even if browser cleanup fails", () => {
  const runner = read("web/scripts/measure-frontend-interaction-latency.mjs");
  assert.match(
    runner,
    /finally\s*\{\s*try\s*\{\s*await browser\?\.close\(\);\s*\}\s*finally\s*\{\s*await closeStaticSnapshotServer\(server\);/s,
  );
});

test("wrapper creates unique no-replace attestation, lease, and dist snapshot paths", () => {
  const wrapper = read("scripts/run_frontend_interaction_latency.ps1");
  for (const marker of [
    "build-attestations/$runId.json",
    "leases/$runId.json",
    "dist-snapshots/$runId",
    "[System.IO.FileMode]::CreateNew",
    "[System.IO.FileShare]::Read",
    "Local\\ta_desktop_demo_frontend_interaction_authority",
    "$authorityMutex = Enter-FrontendAuthorityMutex",
    "Exit-FrontendAuthorityMutex -Mutex $authorityMutex",
    "--dist-snapshot $resolvedDistSnapshot",
    "--lock-lease $authorityLease.path",
    "--lock-nonce $authorityLease.nonce",
  ]) {
    assert.ok(
      wrapper.includes(marker),
      `missing unique authority path: ${marker}`,
    );
  }
});

test("frontend authority rejects inherited ownership and binds exact runner bytes", () => {
  const wrapper = read("scripts/run_frontend_interaction_latency.ps1");
  for (const marker of [
    "Frontend interaction authority refuses inherited benchmark-lock markers",
    "-not [bool]$LockHandle.acquired",
    'benchmark_mutex_ownership_mode = "acquired"',
    "^Evidence-SHA256: [a-f0-9]{64}$",
    "$runnerPublicationSha256",
    "$attestationRead.publication.sha256",
    "$snapshotManifestRead.publication.sha256",
    "$snapshotManifestRead.report.dist.path_manifest_sha256",
    "$snapshotManifestRead.report.dist.file_count",
    "$snapshotManifestRead.report.dist.total_bytes",
    "Read-BenchmarkLockLease",
    "Get-BenchmarkLockLeaseWriteProbeCode",
    "benchmark_lease_identity",
    "benchmark_lease_write_probe_win32_error_code",
    "Recompute and bind complete frontend latency evidence",
    "--report-sha $runnerPublicationSha256",
    "--policy-sha $policySha256",
    "$report.release_evidence_eligible",
    "dirty-source-local-candidate-rehearsal",
    "ineligible for release publication",
  ]) {
    assert.ok(
      wrapper.includes(marker),
      `missing direct-ownership/report binding: ${marker}`,
    );
  }
  const rejectIndex = wrapper.indexOf(
    "Frontend interaction authority refuses inherited benchmark-lock markers",
  );
  const leaseIndex = wrapper.indexOf("$authorityLease = New-AuthorityLease");
  assert.ok(rejectIndex >= 0 && rejectIndex < leaseIndex);
});

test("performance pack delegates only frontend latency to direct mutex ownership", () => {
  const pack = read("scripts/run_performance_audit_baseline_pack.ps1");
  const executionStart = pack.indexOf('if ($Mode -ne "inventory")');
  const execution = pack.slice(executionStart);
  for (const marker of [
    '"$($run.id)" -eq "frontend_interaction_latency"',
    "-not [bool]$lock.acquired",
    "cannot delegate inherited benchmark-lock authority",
    "Exit-BenchmarkLock -Handle $lock",
    "$lock = $null",
    "finally {",
    "$lock = Enter-BenchmarkLock",
  ]) {
    assert.ok(
      execution.includes(marker),
      `missing pack direct-ownership handoff: ${marker}`,
    );
  }
  assert.doesNotMatch(
    execution,
    /TA_DESKTOP_DEMO_BENCHMARK_LOCK_(?:HELD|OWNER_PID|CONTEXT|TOKEN)\s*=/,
  );
  const directIdCount = (execution.match(/frontend_interaction_latency/g) || [])
    .length;
  assert.equal(
    directIdCount,
    2,
    "execution exception must name only the exact frontend latency id and its error",
  );
  const releaseIndex = execution.indexOf("Exit-BenchmarkLock -Handle $lock");
  const invokeIndex = execution.indexOf(
    "$runResult = Invoke-GateRun -Run $run",
  );
  const finallyIndex = execution.indexOf("finally {", invokeIndex);
  const reacquireIndex = execution.indexOf(
    "$lock = Enter-BenchmarkLock",
    finallyIndex,
  );
  assert.ok(releaseIndex < invokeIndex);
  assert.ok(invokeIndex < finallyIndex);
  assert.ok(finallyIndex < reacquireIndex);
});

test("saved-detail readiness uses the explicit visible detail-only field", () => {
  const runner = read("web/scripts/measure-frontend-interaction-latency.mjs");
  assert.ok(runner.includes('"Request snapshot"'));
  assert.ok(
    runner.includes('data-vta-latency-ready-field="savedRunDetailParams"'),
  );
  assert.ok(runner.includes("requestDetails.open = true"));
});
