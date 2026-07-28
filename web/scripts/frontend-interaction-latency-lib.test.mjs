import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  BUILD_ATTESTATION_KIND,
  BUILD_ATTESTATION_PROFILE,
} from "./frontend-interaction-build-attestation.mjs";
import {
  EXPECTED_AUTHORITY_GIT_PATHS,
  SCOPED_GIT_STATUS_FORMAT,
  buildLatencyReport,
  percentile,
  recomputeLatencyReport,
  summarizeSamples,
  validatePolicy,
} from "./frontend-interaction-latency-lib.mjs";
import { validateLatencyReportArtifacts } from "./frontend-interaction-report-validator.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..");
const policyPath = path.resolve(
  repoRoot,
  "scripts",
  "frontend_interaction_latency_policy.json",
);
const now = new Date("2026-07-28T12:00:00.000Z");

function canonicalPolicy() {
  return JSON.parse(fs.readFileSync(policyPath, "utf8"));
}

function sampleFor({ casePolicy, policy, phase, sequence, absoluteIndex }) {
  const detail = casePolicy.id === "saved-run-detail-open";
  const targetId = detail
    ? `C:/runs/latency-detail-${String(absoluteIndex + 1).padStart(3, "0")}.json`
    : "workspace:library";
  const readyFieldValue = detail
    ? `LATENCY_DETAIL_READY::${absoluteIndex + 1}::${targetId}`
    : null;
  return {
    phase,
    sequence,
    duration_ms: 100 + sequence,
    click_is_trusted: true,
    quiet_window_observed_ms: policy.dom_quiet_window_ms + 4,
    ready_checks_satisfied: true,
    readiness_contract_id: casePolicy.readiness_contract_id,
    readiness_target_id: targetId,
    trigger_identity: detail ? targetId : "shell.workspace.library",
    ready_field_value: readyFieldValue,
    async_detail_hydration: detail
      ? {
          command: "load_run_artifact_detail",
          requested_path: targetId,
          matched_argument_name: "path",
          matched_argument_value: targetId,
          argument_matched: true,
          invocation_count_for_path: 1,
          delayed_ms: policy.detail_mock_delay_ms,
          duration_ms: policy.detail_mock_delay_ms + 1,
          ready_field_identity: "savedRunDetailParams",
          ready_field_value: readyFieldValue,
        }
      : null,
  };
}

function measurementsFor(policy) {
  return Object.fromEntries(
    policy.cases.map((casePolicy) => {
      const warmupSamples = Array.from(
        { length: policy.warmup_samples_per_case },
        (_unused, sequence) =>
          sampleFor({
            casePolicy,
            policy,
            phase: "warmup",
            sequence,
            absoluteIndex: sequence,
          }),
      );
      const measuredSamples = Array.from(
        { length: policy.measured_samples_per_case },
        (_unused, sequence) =>
          sampleFor({
            casePolicy,
            policy,
            phase: "measured",
            sequence,
            absoluteIndex: sequence + policy.warmup_samples_per_case,
          }),
      );
      return [
        casePolicy.id,
        {
          warmup_samples: warmupSamples,
          measured_samples: measuredSamples,
        },
      ];
    }),
  );
}

function authoritativeSource(policy) {
  return {
    scoped_worktree_clean: true,
    scoped_worktree_status_format: SCOPED_GIT_STATUS_FORMAT,
    scoped_worktree_paths: [...EXPECTED_AUTHORITY_GIT_PATHS],
    scoped_worktree_status_sha256:
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    scoped_worktree_status_bytes: 0,
    measurement_profile: policy.profile,
    mock_profile: policy.mock_profile,
    fixture_profile: policy.fixture_profile,
    fixture_path:
      "web/e2e/browser/fixtures/artifacts/saved-options-artifact-pack.json",
    fixture_sha256: "a".repeat(64),
    effective_fixture_sha256: "f".repeat(64),
    detail_fixture_count:
      policy.warmup_samples_per_case + policy.measured_samples_per_case,
    benchmark_lock: {
      verified: true,
      verified_at_start: true,
      verified_before_publication: true,
      owner_pid: 1234,
      owner_started_at: "2026-07-28T11:57:00.000Z",
      mutex_owner_pid: 1234,
      mutex_owner_started_at: "2026-07-28T11:57:00.000Z",
      mutex_ownership_mode: "acquired",
      mutex_name: "Local\\ta_desktop_demo_frontend_interaction_authority",
      mutex_probe_exit_code: 0,
      benchmark_mutex_owner_pid: 1234,
      benchmark_mutex_owner_started_at: "2026-07-28T11:57:00.000Z",
      benchmark_mutex_ownership_mode: "acquired",
      benchmark_mutex_name: "Local\\ta_desktop_demo_benchmark_gate",
      benchmark_mutex_probe_exit_code: 0,
      benchmark_lease_path: "output/qa/benchmark-lock/active.json",
      benchmark_lease_sha256: "5".repeat(64),
      benchmark_lease_identity: "01234567:89abcdef01234567",
      benchmark_lease_write_probe_win32_error_code: 32,
      benchmark_lease_write_probe_exit_code: 0,
      benchmark_lease_holder_verified: true,
      benchmark_lease_holder_probe_exit_code: 0,
      write_probe_win32_error_code: 32,
      write_probe_exit_code: 0,
      context: "scoped warm mocked-browser latency",
      wrapper_context:
        "scoped warm mocked-browser click-to-painted-stable latency",
      token_sha256: "8".repeat(64),
      lease_path:
        "output/qa/frontend-interaction-latency/leases/0123456789abcdef0123456789abcdef.json",
      lease_sha256: "1".repeat(64),
      initial_lease_sha256: "1".repeat(64),
      nonce_sha256: "2".repeat(64),
    },
    measurement_harness_paths: [
      "scripts/benchmark_lock.ps1",
      "scripts/run_frontend_interaction_latency.ps1",
      "web/scripts/frontend-interaction-authority-lease.mjs",
      "web/scripts/frontend-interaction-build-attestation.mjs",
      "web/scripts/frontend-interaction-latency-lib.mjs",
      "web/scripts/frontend-interaction-report-validator.mjs",
      "web/scripts/measure-frontend-interaction-latency.mjs",
      "web/scripts/frontend-interaction-static-snapshot.mjs",
      "web/scripts/support/mockedTauriHarness.mjs",
    ],
    measurement_harness_sha256: "9".repeat(64),
    measurement_harness_path_manifest_sha256: "6".repeat(64),
    measurement_harness_file_count: 9,
    measurement_harness_total_bytes: 10_000,
    measurement_harness_hash_framing:
      "uint64be-length-prefixed-path-size-sha256-manifest-v1",
    dist_sha256: "b".repeat(64),
    attested_dist_sha256: "b".repeat(64),
    served_dist_snapshot_path:
      "output/qa/frontend-interaction-latency/dist-snapshots/0123456789abcdef0123456789abcdef",
    served_dist_snapshot_manifest_path:
      "output/qa/frontend-interaction-latency/dist-snapshots/0123456789abcdef0123456789abcdef/manifest.json",
    served_dist_snapshot_manifest_sha256: "3".repeat(64),
    served_dist_snapshot_sha256: "b".repeat(64),
    served_dist_snapshot_path_manifest_sha256: "4".repeat(64),
    served_dist_snapshot_file_count: 2,
    served_dist_snapshot_total_bytes: 100,
    served_from_memory_snapshot: true,
    server_snapshot_entry_count: 2,
    server_snapshot_total_bytes: 100,
    build_input_sha256: "c".repeat(64),
    attested_build_input_sha256: "c".repeat(64),
    build_attestation_sha256: "d".repeat(64),
    build_attestation_verified: true,
    build_attestation_kind: BUILD_ATTESTATION_KIND,
    build_attestation_path:
      "output/qa/frontend-interaction-latency/build-attestation.json",
    build_attestation_generated_at: "2026-07-28T11:58:00.000Z",
    build_profile: BUILD_ATTESTATION_PROFILE,
    attested_node_version: process.version,
  };
}

function buildReport(policy, measurements, overrides = {}) {
  return buildLatencyReport({
    policy,
    policyPath: "scripts/frontend_interaction_latency_policy.json",
    policySha256: "e".repeat(64),
    authorityRequested: true,
    source: authoritativeSource(policy),
    environment: {
      browser_name: "chromium",
      headless: true,
      device_scale_factor: 1,
      locale: "en-US",
      timezone: "UTC",
      reduced_motion: "reduce",
      browser_version: "140.0.0.0",
      user_agent: "HeadlessChrome/140.0.0.0",
      node_version: process.version,
    },
    run: {
      id: "0123456789abcdef0123456789abcdef",
      started_at: "2026-07-28T11:59:00.000Z",
    },
    measurements,
    now,
    ...overrides,
  });
}

test("canonical scoped interaction policy is valid and explicit", () => {
  const policy = canonicalPolicy();
  assert.deepEqual(validatePolicy(policy), []);
  assert.equal(policy.profile, "scoped-warm-mocked-browser-v1");
  assert.equal(
    policy.mock_profile,
    "strict-path-distinct-async-saved-detail-v1",
  );
  assert.deepEqual(
    policy.cases.map((entry) => entry.id),
    ["saved-run-detail-open", "workspace-navigation"],
  );
});

test("sample statistics reject null coercion and use stable percentiles", () => {
  assert.equal(percentile([1, 2, 3, 4, 5], 0.5), 3);
  assert.equal(percentile([10, 20], 0.95), 19.5);
  assert.deepEqual(summarizeSamples([null, 4, 2, 3, 1]), {
    count: 4,
    min_ms: 1,
    median_ms: 2.5,
    p90_ms: 3.7,
    p95_ms: 3.85,
    max_ms: 4,
    mean_ms: 2.5,
  });
});

test("authority report passes with strict provenance and per-sample evidence", () => {
  const policy = canonicalPolicy();
  const report = buildReport(policy, measurementsFor(policy));
  assert.equal(report.status, "passed");
  assert.equal(report.authority_eligible, true);
  assert.equal(report.release_evidence_eligible, true);
  assert.equal(
    report.evidence_scope,
    "clean-source-release-evidence-candidate",
  );
  assert.deepEqual(report.issues, []);
  assert.ok(report.cases.every((entry) => entry.status === "passed"));
  assert.equal(
    report.cases[0].measured_samples.length,
    policy.measured_samples_per_case,
  );
});

test("published report recomputation rejects altered derived statistics", () => {
  const policy = canonicalPolicy();
  const report = buildReport(policy, measurementsFor(policy));
  const exact = recomputeLatencyReport({
    report,
    policy,
    policyPath: "scripts/frontend_interaction_latency_policy.json",
    policySha256: "e".repeat(64),
  });
  assert.equal(exact.status, "passed");

  const altered = structuredClone(report);
  altered.cases[0].stats.p95_ms = 0.001;
  const result = recomputeLatencyReport({
    report: altered,
    policy,
    policyPath: "scripts/frontend_interaction_latency_policy.json",
    policySha256: "e".repeat(64),
  });
  assert.equal(result.status, "failed");
  assert.ok(result.issues.some((issue) => issue.includes("recomputed")));
});

test("post-run validation binds publication bytes and sealed artifact totals", () => {
  const policy = canonicalPolicy();
  const report = buildReport(policy, measurementsFor(policy));
  const source = report.source;
  const reportSha256 = "7".repeat(64);
  const attestation = {
    attestation_kind: BUILD_ATTESTATION_KIND,
    build_profile: BUILD_ATTESTATION_PROFILE,
    generated_at: source.build_attestation_generated_at,
    source_inputs: { sha256: source.attested_build_input_sha256 },
    dist: { sha256: source.attested_dist_sha256 },
    toolchain: { node_version: source.attested_node_version },
  };
  const snapshot = {
    snapshot_root: source.served_dist_snapshot_path,
    manifest_path: source.served_dist_snapshot_manifest_path,
    manifest_sha256: source.served_dist_snapshot_manifest_sha256,
    source_dist_sha256: source.attested_dist_sha256,
    snapshot: {
      sha256: source.served_dist_snapshot_sha256,
      path_manifest_sha256: source.served_dist_snapshot_path_manifest_sha256,
      file_count: source.served_dist_snapshot_file_count,
      total_bytes: source.served_dist_snapshot_total_bytes,
    },
  };
  const lease = {
    ...source.benchmark_lock,
    status: "passed",
    verified: true,
  };
  const args = {
    report,
    reportPublication: { sha256: reportSha256 },
    expectedReportSha256: reportSha256,
    policy,
    policyPath,
    policySha256: report.policy.sha256,
    attestation,
    attestationPublication: {
      relative_path: source.build_attestation_path,
      sha256: source.build_attestation_sha256,
    },
    buildValidation: { status: "passed", issues: [] },
    snapshot,
    lease,
    measurementHarnessSnapshot: {
      sha256: source.measurement_harness_sha256,
      path_manifest_sha256: source.measurement_harness_path_manifest_sha256,
      file_count: source.measurement_harness_file_count,
      total_bytes: source.measurement_harness_total_bytes,
    },
    scopedWorktreeState: {
      clean: source.scoped_worktree_clean,
      format: source.scoped_worktree_status_format,
      paths: source.scoped_worktree_paths,
      sha256: source.scoped_worktree_status_sha256,
      byte_count: source.scoped_worktree_status_bytes,
    },
    fixtureSha256: source.fixture_sha256,
    effectiveFixtureSha256: source.effective_fixture_sha256,
    expectedRunId: report.run.id,
    expectedRunStartedAt: report.run.started_at,
    repoRoot,
  };
  assert.equal(validateLatencyReportArtifacts(args).status, "passed");

  const substitutedPublication = validateLatencyReportArtifacts({
    ...args,
    reportPublication: { sha256: "6".repeat(64) },
  });
  assert.ok(
    substitutedPublication.issues.some((issue) =>
      issue.includes("report publication SHA-256"),
    ),
  );

  const alteredTotals = validateLatencyReportArtifacts({
    ...args,
    snapshot: {
      ...snapshot,
      snapshot: { ...snapshot.snapshot, file_count: 3 },
    },
  });
  assert.ok(
    alteredTotals.issues.some((issue) =>
      issue.includes("served snapshot file count"),
    ),
  );

  const alteredHarness = validateLatencyReportArtifacts({
    ...args,
    measurementHarnessSnapshot: {
      ...args.measurementHarnessSnapshot,
      sha256: "0".repeat(64),
    },
  });
  assert.ok(
    alteredHarness.issues.some((issue) =>
      issue.includes("measurement harness SHA-256"),
    ),
  );

  const alteredFixture = validateLatencyReportArtifacts({
    ...args,
    fixtureSha256: "0".repeat(64),
  });
  assert.ok(
    alteredFixture.issues.some((issue) => issue.includes("fixture SHA-256")),
  );

  const alteredWorktreeStatus = validateLatencyReportArtifacts({
    ...args,
    scopedWorktreeState: {
      ...args.scopedWorktreeState,
      sha256: "0".repeat(64),
    },
  });
  assert.ok(
    alteredWorktreeStatus.issues.some((issue) =>
      issue.includes("scoped worktree status SHA-256"),
    ),
  );

  const overstatedReleaseEligibility = structuredClone(report);
  overstatedReleaseEligibility.release_evidence_eligible = false;
  const alteredClassification = validateLatencyReportArtifacts({
    ...args,
    report: overstatedReleaseEligibility,
  });
  assert.ok(
    alteredClassification.issues.some((issue) =>
      issue.includes("release-evidence eligibility"),
    ),
  );
});

test("null duration cannot become a zero-millisecond authority sample", () => {
  const policy = canonicalPolicy();
  const measurements = measurementsFor(policy);
  measurements["workspace-navigation"].measured_samples[0].duration_ms = null;
  const report = buildReport(policy, measurements);
  assert.equal(report.status, "failed");
  assert.equal(report.authority_eligible, false);
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("duration_ms must be a finite positive number"),
    ),
  );
});

test("untrusted click, short quiet window, and readiness drift fail closed", () => {
  const policy = canonicalPolicy();
  const measurements = measurementsFor(policy);
  const sample = measurements["workspace-navigation"].measured_samples[0];
  sample.click_is_trusted = false;
  sample.quiet_window_observed_ms = policy.dom_quiet_window_ms - 1;
  sample.readiness_contract_id = "wrong-readiness";
  sample.ready_checks_satisfied = false;
  const report = buildReport(policy, measurements);
  assert.equal(report.status, "failed");
  assert.equal(report.authority_eligible, false);
  for (const expected of [
    "click_is_trusted must be true",
    "quiet_window_observed_ms",
    "readiness_contract_id",
    "ready_checks_satisfied",
  ]) {
    assert.ok(report.issues.some((issue) => issue.includes(expected)));
  }
});

test("detail hydration must match its unique requested path and ready field", () => {
  const policy = canonicalPolicy();
  const measurements = measurementsFor(policy);
  const sample = measurements["saved-run-detail-open"].measured_samples[0];
  sample.async_detail_hydration.requested_path = "C:/runs/wrong-detail.json";
  sample.async_detail_hydration.argument_matched = false;
  sample.async_detail_hydration.matched_argument_value =
    "C:/runs/wrong-detail.json";
  sample.async_detail_hydration.ready_field_value = "wrong-field";
  const report = buildReport(policy, measurements);
  assert.equal(report.status, "failed");
  assert.equal(report.authority_eligible, false);
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("requested_path must match readiness_target_id"),
    ),
  );
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("argument_matched must be true"),
    ),
  );
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("matched_argument_value must match"),
    ),
  );
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("ready_field_value must match"),
    ),
  );
});

test("reused saved-detail target identity is rejected", () => {
  const policy = canonicalPolicy();
  const measurements = measurementsFor(policy);
  const rows = measurements["saved-run-detail-open"];
  rows.measured_samples[0].readiness_target_id =
    rows.warmup_samples[0].readiness_target_id;
  rows.measured_samples[0].async_detail_hydration.requested_path =
    rows.measured_samples[0].readiness_target_id;
  const report = buildReport(policy, measurements);
  assert.equal(report.status, "failed");
  assert.equal(
    report.cases.find((entry) => entry.id === "saved-run-detail-open").checks
      .target_identity_contract_satisfied,
    false,
  );
});

test("reused detail-only ready field is rejected even for distinct paths", () => {
  const policy = canonicalPolicy();
  const measurements = measurementsFor(policy);
  const rows = measurements["saved-run-detail-open"];
  const first = rows.measured_samples[0];
  const second = rows.measured_samples[1];
  second.ready_field_value = first.ready_field_value;
  second.async_detail_hydration.ready_field_value = first.ready_field_value;
  const report = buildReport(policy, measurements);
  assert.equal(report.status, "failed");
  assert.equal(report.authority_eligible, false);
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("ready_field_value must be the unique detail-only field"),
    ),
  );
});

test("profile, build attestation, worktree provenance, and timestamp drift prevent eligibility", () => {
  const policy = canonicalPolicy();
  const measurements = measurementsFor(policy);
  const source = authoritativeSource(policy);
  source.mock_profile = "loose-mock";
  source.build_attestation_verified = false;
  source.scoped_worktree_clean = false;
  source.scoped_worktree_status_bytes = 0;
  const report = buildReport(policy, measurements, {
    source,
    run: {
      id: "0123456789abcdef0123456789abcdef",
      started_at: "2026-07-27T00:00:00.000Z",
    },
  });
  assert.equal(report.status, "failed");
  assert.equal(report.authority_eligible, false);
  assert.ok(report.issues.some((issue) => issue.includes("mock profile")));
  assert.ok(report.issues.some((issue) => issue.includes("not verified")));
  assert.ok(report.issues.some((issue) => issue.includes("worktree")));
  assert.ok(
    report.issues.some((issue) => issue.includes("stale or in the future")),
  );
});

test("exact dirty-source snapshots retain local candidate authority but cannot become release evidence", () => {
  const policy = canonicalPolicy();
  const source = authoritativeSource(policy);
  source.scoped_worktree_clean = false;
  source.scoped_worktree_status_sha256 = "7".repeat(64);
  source.scoped_worktree_status_bytes = 73;

  const report = buildReport(policy, measurementsFor(policy), { source });

  assert.equal(report.status, "passed");
  assert.equal(report.authority_eligible, true);
  assert.equal(report.release_evidence_eligible, false);
  assert.equal(report.evidence_scope, "dirty-source-local-candidate-rehearsal");
  assert.deepEqual(report.issues, []);
});

test("served snapshot or lease drift prevents authority eligibility", () => {
  const policy = canonicalPolicy();
  const source = authoritativeSource(policy);
  source.served_dist_snapshot_sha256 = "0".repeat(64);
  source.benchmark_lock.verified_before_publication = false;
  const report = buildReport(policy, measurementsFor(policy), { source });
  assert.equal(report.authority_eligible, false);
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("served immutable distribution snapshot"),
    ),
  );
  assert.ok(
    report.issues.some((issue) => issue.includes("benchmark lock provenance")),
  );
});

test("noncanonical fixture, attestation path, and runtime identity prevent eligibility", () => {
  const policy = canonicalPolicy();
  const measurements = measurementsFor(policy);
  const source = authoritativeSource(policy);
  source.fixture_path = "web/e2e/browser/fixtures/other.json";
  source.build_attestation_path = "../attestation.json";
  source.build_attestation_kind = "untrusted-attestation";
  source.build_attestation_generated_at = "2026-07-27T00:00:00.000Z";
  source.detail_fixture_count = 1;
  source.benchmark_lock.verified = false;
  source.measurement_harness_paths = [
    "web/scripts/measure-frontend-interaction-latency.mjs",
  ];
  const report = buildReport(policy, measurements, {
    source,
    environment: {
      browser_name: "chromium",
      headless: true,
      device_scale_factor: 1,
      locale: "en-US",
      timezone: "UTC",
      reduced_motion: "reduce",
      browser_version: "140.0.0.0",
      user_agent: "HeadlessChrome/140.0.0.0",
      node_version: "v0.0.0",
    },
  });
  assert.equal(report.status, "failed");
  assert.equal(report.authority_eligible, false);
  for (const expected of [
    "fixture path is not canonical",
    "attestation path",
    "attestation kind",
    "attestation timestamp",
    "detail fixture count",
    "benchmark lock provenance",
    "measurement harness path set",
    "runtime identity",
  ]) {
    assert.ok(report.issues.some((issue) => issue.includes(expected)));
  }
});
