import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  normalizeDesktopSmokeEvidence,
  normalizeUiPerfBudgetEvidence,
  resolveFrontendQaReportRun,
} from "./qa-evidence-freshness.mjs";

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
  const latencyIndex = latencyBlock.indexOf("run_frontend_interaction_latency.ps1");
  const refreshedBudgetIndex = latencyBlock.indexOf("check:bundle-budget");
  assert.ok(latencyIndex >= 0 && latencyIndex < refreshedBudgetIndex);
});

test("frontend E2E and canonical CPU QA enforce the production bundle budget after build", () => {
  const e2eGate = read("scripts/run_frontend_e2e_gate.ps1");
  assert.match(
    e2eGate,
    /if \(-not \$SkipBuild\) \{\s*Invoke-Step "Web build" \{ & npm\.cmd --prefix web run build \}\s*Invoke-Step "Production bundle budget" \{ & npm\.cmd --prefix web run check:bundle-budget \}\s*\}/s,
  );
  const buildIndex = e2eGate.indexOf("& npm.cmd --prefix web run build");
  const bundleBudgetIndex = e2eGate.indexOf(
    "& npm.cmd --prefix web run check:bundle-budget",
  );
  const qaIndex = e2eGate.indexOf("if ($IncludeQaStatic)");
  assert.ok(buildIndex >= 0 && buildIndex < bundleBudgetIndex);
  assert.ok(bundleBudgetIndex < qaIndex);

  const cpuGate = read("scripts/run_cpu_ci_gate.ps1");
  const browserQaBlock = cpuGate.slice(
    cpuGate.indexOf('Invoke-Step "Full headless Chromium QA"'),
    cpuGate.indexOf('Invoke-Step "Headless JSON/CSV/XLSX output parity"'),
  );
  assert.ok(browserQaBlock.includes("run_frontend_e2e_gate.ps1"));
  assert.ok(browserQaBlock.includes("-IncludeQaReport"));
  assert.ok(!browserQaBlock.includes("-SkipBuild"));
});

test("frontend QA reports bundle budget separately from captured UI performance", () => {
  const bundleChecker = read("web/scripts/check-bundle-budget.mjs");
  const localSummary = read("web/scripts/write-local-qa-summary.mjs");
  const dashboard = read("web/scripts/write-frontend-qa-dashboard.mjs");

  assert.ok(bundleChecker.includes("output', 'qa', 'bundle-budget.json"));
  assert.ok(bundleChecker.includes("status"));
  assert.ok(bundleChecker.includes("observations"));
  for (const writer of [localSummary, dashboard]) {
    assert.ok(writer.includes("output/qa/bundle-budget.json"));
    assert.ok(writer.includes("bundleBudget: bundleBudget"));
    assert.ok(writer.includes("perfBudget: perfBudget"));
    assert.ok(writer.includes("Production Bundle Budget"));
    assert.ok(writer.includes("UI Performance Capture / Baseline Reference"));
    assert.ok(
      writer.includes("Required manifest actions registered with workflow references"),
    );
  }
  assert.ok(dashboard.includes("Required Action Registrations By Priority"));
  assert.ok(dashboard.includes("required actions registered with workflow references"));
  assert.ok(!dashboard.includes("Required covered:"));
});

test("frontend QA evidence freshness distinguishes current lanes from references", () => {
  const reportRun = resolveFrontendQaReportRun(
    {
      VECTORGRID_FRONTEND_QA_RUN_ID: "qa-run-1",
      VECTORGRID_FRONTEND_QA_RUN_STARTED_AT: "2026-07-28T12:00:00.000Z",
    },
    "2026-07-28T12:10:00.000Z",
  );
  assert.equal(reportRun.available, true);

  const historicalDesktop = normalizeDesktopSmokeEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-27T12:00:00.000Z",
      suite: "smoke",
      runId: "old-smoke",
      specCount: 7,
    },
    reportRun,
  );
  assert.equal(historicalDesktop.status, "reference_only");
  assert.equal(historicalDesktop.sourceStatus, "passed");
  assert.equal(historicalDesktop.evidenceScope, "historical_reference");
  assert.equal(historicalDesktop.currentRun, false);

  const noContextDesktop = normalizeDesktopSmokeEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:05:00.000Z",
      suite: "smoke",
      runId: "qa-run-1",
      specCount: 7,
    },
    resolveFrontendQaReportRun({}, "2026-07-28T12:10:00.000Z"),
  );
  assert.equal(noContextDesktop.status, "reference_only");
  assert.equal(noContextDesktop.freshnessReason, "current_report_context_unavailable");

  const overlappingDesktop = normalizeDesktopSmokeEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:05:00.000Z",
      suite: "smoke",
      runId: "concurrent-foreign-run",
      specCount: 7,
    },
    reportRun,
  );
  assert.equal(overlappingDesktop.status, "reference_only");
  assert.equal(overlappingDesktop.runIdentityMatches, false);
  assert.equal(overlappingDesktop.freshnessReason, "run_identity_mismatch");

  const currentDesktop = normalizeDesktopSmokeEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:05:00.000Z",
      suite: "smoke",
      runId: "qa-run-1",
      specCount: 7,
    },
    reportRun,
  );
  assert.equal(currentDesktop.status, "passed");
  assert.equal(currentDesktop.evidenceScope, "current_run");
  assert.equal(currentDesktop.currentRun, true);

  const currentDesktopFailure = normalizeDesktopSmokeEvidence(
    {
      status: "failed",
      generatedAt: "2026-07-28T12:05:00.000Z",
      suite: "smoke",
      runId: "qa-run-1",
      specCount: 7,
    },
    reportRun,
  );
  assert.equal(currentDesktopFailure.status, "failed");
  assert.equal(currentDesktopFailure.currentRun, true);

  const futureDesktop = normalizeDesktopSmokeEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:11:00.000Z",
      suite: "smoke",
      runId: "qa-run-1",
      specCount: 7,
    },
    reportRun,
  );
  assert.equal(futureDesktop.status, "reference_only");
  assert.equal(futureDesktop.freshnessReason, "not_produced_in_current_report");

  const supersededExpanded = normalizeDesktopSmokeEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:04:00.000Z",
      suite: "smoke-expanded",
      runId: "qa-run-1",
      specCount: 10,
    },
    reportRun,
    {
      generatedAt: "2026-07-28T12:05:00.000Z",
      runId: "qa-run-1",
    },
  );
  assert.equal(supersededExpanded.status, "reference_only");
  assert.equal(supersededExpanded.freshnessReason, "superseded_by_primary_desktop_summary");

  const expandedWithForeignPrimary = normalizeDesktopSmokeEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:04:00.000Z",
      suite: "smoke-expanded",
      runId: "qa-run-1",
      specCount: 10,
    },
    reportRun,
    {
      generatedAt: "2026-07-28T12:05:00.000Z",
      runId: "concurrent-foreign-run",
    },
  );
  assert.equal(expandedWithForeignPrimary.status, "passed");
  assert.equal(expandedWithForeignPrimary.primaryIdentityMatches, false);
  assert.equal(expandedWithForeignPrimary.currentRun, true);

  const baselinePerf = normalizeUiPerfBudgetEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:08:00.000Z",
      runId: "qa-run-1",
      source: "scripts/baselines/ui_capture/perf_metrics.json",
      sourceCapture: {
        generatedAt: "2026-05-07T23:31:32.217Z",
        runId: "old-capture",
        fileModifiedAt: "2026-05-07T23:31:32.300Z",
        mode: "shell_bootstrap",
      },
      budgets: {},
      observations: {},
      issues: [],
    },
    reportRun,
  );
  assert.equal(baselinePerf.status, "reference_only");
  assert.equal(baselinePerf.sourceStatus, "passed");
  assert.equal(baselinePerf.evidenceScope, "baseline_reference");
  assert.equal(baselinePerf.evaluatedInCurrentReport, true);
  assert.equal(baselinePerf.capturedInCurrentReport, false);

  const overlappingPerf = normalizeUiPerfBudgetEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:08:00.000Z",
      runId: "qa-run-1",
      sourceCapture: {
        generatedAt: "2026-07-28T12:06:00.000Z",
        runId: "concurrent-foreign-run",
      },
      issues: [],
    },
    reportRun,
  );
  assert.equal(overlappingPerf.status, "reference_only");
  assert.equal(overlappingPerf.captureRunIdentityMatches, false);
  assert.equal(overlappingPerf.freshnessReason, "capture_run_identity_mismatch");

  const foreignEvaluation = normalizeUiPerfBudgetEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:08:00.000Z",
      runId: "concurrent-foreign-run",
      sourceCapture: {
        generatedAt: "2026-07-28T12:06:00.000Z",
        runId: "qa-run-1",
      },
      issues: [],
    },
    reportRun,
  );
  assert.equal(foreignEvaluation.status, "reference_only");
  assert.equal(foreignEvaluation.evaluationRunIdentityMatches, false);
  assert.equal(foreignEvaluation.freshnessReason, "evaluation_run_identity_mismatch");

  const currentPerf = normalizeUiPerfBudgetEvidence(
    {
      status: "passed",
      generatedAt: "2026-07-28T12:08:00.000Z",
      runId: "qa-run-1",
      sourceCapture: {
        generatedAt: "2026-07-28T12:06:00.000Z",
        runId: "qa-run-1",
      },
      issues: [],
    },
    reportRun,
  );
  assert.equal(currentPerf.status, "passed");
  assert.equal(currentPerf.evidenceScope, "current_run_capture");
  assert.equal(currentPerf.currentRun, true);
});

test("frontend QA gate supplies and restores report-run provenance", () => {
  const gate = read("scripts/run_frontend_e2e_gate.ps1");
  const perfChecker = read("web/scripts/check-ui-perf-budget.mjs");
  const localSummary = read("web/scripts/write-local-qa-summary.mjs");
  const dashboard = read("web/scripts/write-frontend-qa-dashboard.mjs");

  for (const marker of [
    "VECTORGRID_FRONTEND_QA_RUN_ID",
    "VECTORGRID_FRONTEND_QA_RUN_STARTED_AT",
    "VECTORGRID_FRONTEND_QA_UI_CAPTURE_RUN_ID",
    "[guid]::NewGuid()",
    "[DateTimeOffset]::UtcNow",
    "Remove-Item Env:VECTORGRID_FRONTEND_QA_RUN_ID",
    "Remove-Item Env:VECTORGRID_FRONTEND_QA_RUN_STARTED_AT",
    "Remove-Item Env:VECTORGRID_FRONTEND_QA_UI_CAPTURE_RUN_ID",
  ]) {
    assert.ok(gate.includes(marker), `missing frontend QA run provenance marker: ${marker}`);
  }
  for (const marker of [
    "status: issues.length > 0 ? 'failed' : 'passed'",
    "sourceCapture",
    "VECTORGRID_FRONTEND_QA_RUN_ID",
    "VECTORGRID_FRONTEND_QA_UI_CAPTURE_RUN_ID",
    "metrics.generated_at",
    "metrics.startup_capture_mode",
    "metrics.note",
  ]) {
    assert.ok(perfChecker.includes(marker), `missing UI baseline provenance marker: ${marker}`);
  }
  for (const writer of [localSummary, dashboard]) {
    assert.ok(writer.includes("resolveFrontendQaReportRun"));
    assert.ok(writer.includes("normalizeUiPerfBudgetEvidence"));
    assert.ok(writer.includes("baseline reference"));
  }
  assert.ok(dashboard.includes("normalizeDesktopSmokeEvidence"));
  assert.ok(dashboard.includes("not run in this report; latest historical reference"));
  assert.ok(dashboard.includes("current-report desktop smoke failed"));
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
