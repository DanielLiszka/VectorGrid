#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import { verifyAuthorityLease } from "./frontend-interaction-authority-lease.mjs";
import {
  BUILD_ATTESTATION_KIND,
  BUILD_ATTESTATION_PROFILE,
  FILE_SET_HASH_FRAMING,
  computeCanonicalFileSetSnapshot,
  isSha256,
  readBoundedFilePublication,
  validateBuildAttestation,
  validateImmutableDistSnapshot,
} from "./frontend-interaction-build-attestation.mjs";
import {
  EXPECTED_MEASUREMENT_HARNESS_PATHS,
  buildPathDistinctDetailFixture,
  computeAuthorityScopedWorktreeState,
  recomputeLatencyReport,
} from "./frontend-interaction-latency-lib.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const defaultWebRoot = path.resolve(scriptDir, "..");
const defaultRepoRoot = path.resolve(defaultWebRoot, "..");

function sha256Bytes(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function readStableOrdinaryFile(filePath, expectedSha256 = "") {
  const resolvedPath = path.resolve(filePath);
  const linkStat = fs.lstatSync(resolvedPath);
  if (linkStat.isSymbolicLink() || !linkStat.isFile() || linkStat.nlink !== 1) {
    throw new Error(
      `authority input must be a single-link file: ${resolvedPath}`,
    );
  }
  const handle = fs.openSync(resolvedPath, "r");
  let before;
  let after;
  let bytes;
  try {
    before = fs.fstatSync(handle);
    if (!before.isFile() || before.nlink !== 1) {
      throw new Error(
        `authority input handle must identify a single-link file: ${resolvedPath}`,
      );
    }
    bytes = fs.readFileSync(handle);
    after = fs.fstatSync(handle);
  } finally {
    fs.closeSync(handle);
  }
  const currentLink = fs.lstatSync(resolvedPath);
  const current = fs.statSync(resolvedPath);
  if (
    currentLink.isSymbolicLink() ||
    !currentLink.isFile() ||
    currentLink.nlink !== 1 ||
    before.dev !== after.dev ||
    before.ino !== after.ino ||
    before.size !== after.size ||
    before.mtimeMs !== after.mtimeMs ||
    current.dev !== after.dev ||
    current.ino !== after.ino ||
    current.size !== after.size ||
    current.mtimeMs !== after.mtimeMs
  ) {
    throw new Error(
      `authority input changed while being read: ${resolvedPath}`,
    );
  }
  const sha256 = sha256Bytes(bytes);
  if (expectedSha256 && sha256 !== expectedSha256) {
    throw new Error(
      `authority input SHA-256 mismatch: expected ${expectedSha256}, observed ${sha256}`,
    );
  }
  return { path: resolvedPath, bytes, sha256, stat: after };
}

function same(left, right) {
  return left === right;
}

function bindEqual(issues, label, observed, expected) {
  if (!same(observed, expected)) {
    issues.push(`${label} does not match its authoritative artifact`);
  }
}

export function validateLatencyReportArtifacts({
  report,
  reportPublication,
  expectedReportSha256,
  policy,
  policyPath,
  policySha256,
  attestation,
  attestationPublication,
  buildValidation,
  snapshot,
  lease,
  measurementHarnessSnapshot,
  scopedWorktreeState,
  fixtureSha256,
  effectiveFixtureSha256,
  expectedRunId,
  expectedRunStartedAt,
  repoRoot = defaultRepoRoot,
} = {}) {
  const issues = [];
  if (!isSha256(expectedReportSha256)) {
    issues.push("expected report publication SHA-256 is invalid");
  }
  bindEqual(
    issues,
    "report publication SHA-256",
    reportPublication?.sha256,
    expectedReportSha256,
  );
  bindEqual(
    issues,
    "measurement harness SHA-256",
    report?.source?.measurement_harness_sha256,
    measurementHarnessSnapshot?.sha256,
  );
  bindEqual(
    issues,
    "measurement harness path-manifest SHA-256",
    report?.source?.measurement_harness_path_manifest_sha256,
    measurementHarnessSnapshot?.path_manifest_sha256,
  );
  bindEqual(
    issues,
    "measurement harness file count",
    report?.source?.measurement_harness_file_count,
    measurementHarnessSnapshot?.file_count,
  );
  bindEqual(
    issues,
    "measurement harness byte count",
    report?.source?.measurement_harness_total_bytes,
    measurementHarnessSnapshot?.total_bytes,
  );
  bindEqual(
    issues,
    "measurement harness framing",
    report?.source?.measurement_harness_hash_framing,
    FILE_SET_HASH_FRAMING,
  );
  bindEqual(
    issues,
    "fixture SHA-256",
    report?.source?.fixture_sha256,
    fixtureSha256,
  );
  bindEqual(
    issues,
    "effective fixture SHA-256",
    report?.source?.effective_fixture_sha256,
    effectiveFixtureSha256,
  );
  bindEqual(
    issues,
    "scoped worktree clean state",
    report?.source?.scoped_worktree_clean,
    scopedWorktreeState?.clean,
  );
  bindEqual(
    issues,
    "scoped worktree status format",
    report?.source?.scoped_worktree_status_format,
    scopedWorktreeState?.format,
  );
  bindEqual(
    issues,
    "scoped worktree path set",
    JSON.stringify(report?.source?.scoped_worktree_paths),
    JSON.stringify(scopedWorktreeState?.paths),
  );
  bindEqual(
    issues,
    "scoped worktree status SHA-256",
    report?.source?.scoped_worktree_status_sha256,
    scopedWorktreeState?.sha256,
  );
  bindEqual(
    issues,
    "scoped worktree status byte count",
    report?.source?.scoped_worktree_status_bytes,
    scopedWorktreeState?.byte_count,
  );
  if (report?.status !== "passed" || report?.authority_eligible !== true) {
    issues.push("published report is not authority-eligible and passed");
  }
  const expectedReleaseEvidenceEligible = Boolean(
    report?.authority_eligible === true && scopedWorktreeState?.clean === true,
  );
  bindEqual(
    issues,
    "release-evidence eligibility",
    report?.release_evidence_eligible,
    expectedReleaseEvidenceEligible,
  );
  bindEqual(
    issues,
    "evidence scope",
    report?.evidence_scope,
    report?.authority_eligible !== true
      ? "non-authoritative"
      : expectedReleaseEvidenceEligible
        ? "clean-source-release-evidence-candidate"
        : "dirty-source-local-candidate-rehearsal",
  );
  bindEqual(issues, "run id", report?.run?.id, expectedRunId);
  bindEqual(
    issues,
    "run started_at",
    report?.run?.started_at,
    expectedRunStartedAt,
  );

  const relativePolicyPath = path
    .relative(path.resolve(repoRoot), path.resolve(policyPath))
    .replaceAll("\\", "/");
  bindEqual(issues, "policy path", report?.policy?.path, relativePolicyPath);
  bindEqual(issues, "policy SHA-256", report?.policy?.sha256, policySha256);

  const relativeAttestationPath = attestationPublication?.relative_path;
  bindEqual(
    issues,
    "build attestation path",
    report?.source?.build_attestation_path,
    relativeAttestationPath,
  );
  bindEqual(
    issues,
    "build attestation SHA-256",
    report?.source?.build_attestation_sha256,
    attestationPublication?.sha256,
  );
  bindEqual(
    issues,
    "build attestation kind",
    report?.source?.build_attestation_kind,
    BUILD_ATTESTATION_KIND,
  );
  bindEqual(
    issues,
    "build attestation profile",
    report?.source?.build_profile,
    BUILD_ATTESTATION_PROFILE,
  );
  bindEqual(
    issues,
    "build attestation generated_at",
    report?.source?.build_attestation_generated_at,
    attestation?.generated_at,
  );
  bindEqual(
    issues,
    "attested Node version",
    report?.source?.attested_node_version,
    attestation?.toolchain?.node_version,
  );
  bindEqual(
    issues,
    "build input SHA-256",
    report?.source?.build_input_sha256,
    attestation?.source_inputs?.sha256,
  );
  bindEqual(
    issues,
    "attested build input SHA-256",
    report?.source?.attested_build_input_sha256,
    attestation?.source_inputs?.sha256,
  );
  bindEqual(
    issues,
    "distribution SHA-256",
    report?.source?.dist_sha256,
    attestation?.dist?.sha256,
  );
  bindEqual(
    issues,
    "attested distribution SHA-256",
    report?.source?.attested_dist_sha256,
    attestation?.dist?.sha256,
  );
  if (buildValidation?.status !== "passed") {
    issues.push(
      ...(buildValidation?.issues || []).map(
        (issue) => `build attestation: ${issue}`,
      ),
    );
  }

  bindEqual(
    issues,
    "served snapshot path",
    report?.source?.served_dist_snapshot_path,
    snapshot?.snapshot_root,
  );
  bindEqual(
    issues,
    "served snapshot manifest path",
    report?.source?.served_dist_snapshot_manifest_path,
    snapshot?.manifest_path,
  );
  bindEqual(
    issues,
    "served snapshot manifest SHA-256",
    report?.source?.served_dist_snapshot_manifest_sha256,
    snapshot?.manifest_sha256,
  );
  bindEqual(
    issues,
    "served snapshot SHA-256",
    report?.source?.served_dist_snapshot_sha256,
    snapshot?.snapshot?.sha256,
  );
  bindEqual(
    issues,
    "served snapshot path-manifest SHA-256",
    report?.source?.served_dist_snapshot_path_manifest_sha256,
    snapshot?.snapshot?.path_manifest_sha256,
  );
  bindEqual(
    issues,
    "served snapshot file count",
    report?.source?.served_dist_snapshot_file_count,
    snapshot?.snapshot?.file_count,
  );
  bindEqual(
    issues,
    "served snapshot byte count",
    report?.source?.served_dist_snapshot_total_bytes,
    snapshot?.snapshot?.total_bytes,
  );
  bindEqual(
    issues,
    "server snapshot entry count",
    report?.source?.server_snapshot_entry_count,
    snapshot?.snapshot?.file_count,
  );
  bindEqual(
    issues,
    "server snapshot byte count",
    report?.source?.server_snapshot_total_bytes,
    snapshot?.snapshot?.total_bytes,
  );
  bindEqual(
    issues,
    "snapshot source distribution SHA-256",
    snapshot?.source_dist_sha256,
    attestation?.dist?.sha256,
  );
  if (report?.source?.served_from_memory_snapshot !== true) {
    issues.push("report does not prove serving from a memory snapshot");
  }

  const reportedLease = report?.source?.benchmark_lock;
  for (const field of [
    "lease_path",
    "lease_sha256",
    "nonce_sha256",
    "owner_pid",
    "owner_started_at",
    "mutex_owner_pid",
    "mutex_owner_started_at",
    "mutex_ownership_mode",
    "mutex_name",
    "benchmark_mutex_owner_pid",
    "benchmark_mutex_owner_started_at",
    "benchmark_mutex_ownership_mode",
    "benchmark_mutex_name",
    "benchmark_lease_path",
    "benchmark_lease_sha256",
    "benchmark_lease_identity",
    "benchmark_lease_write_probe_win32_error_code",
    "benchmark_lease_write_probe_exit_code",
    "benchmark_lease_holder_verified",
    "benchmark_lease_holder_probe_exit_code",
    "context",
    "wrapper_context",
    "token_sha256",
    "created_at",
    "write_probe_win32_error_code",
    "write_probe_exit_code",
    "mutex_probe_exit_code",
    "benchmark_mutex_probe_exit_code",
  ]) {
    bindEqual(
      issues,
      `benchmark lease ${field}`,
      reportedLease?.[field],
      lease?.[field],
    );
  }
  bindEqual(
    issues,
    "initial benchmark lease SHA-256",
    reportedLease?.initial_lease_sha256,
    lease?.lease_sha256,
  );
  if (
    lease?.status !== "passed" ||
    lease?.verified !== true ||
    reportedLease?.verified !== true ||
    reportedLease?.verified_at_start !== true ||
    reportedLease?.verified_before_publication !== true
  ) {
    issues.push("benchmark lease is not authoritative at all boundaries");
  }

  const recomputation = recomputeLatencyReport({
    report,
    policy,
    policyPath: relativePolicyPath,
    policySha256,
  });
  issues.push(
    ...recomputation.issues.map((issue) => `recomputed report: ${issue}`),
  );
  return {
    status: issues.length === 0 ? "passed" : "failed",
    issues: [...new Set(issues)].sort(),
    report_sha256: reportPublication?.sha256 ?? null,
    recomputed_status: recomputation.status,
  };
}

function parseArgs(argv) {
  const parsed = {
    repoRoot: defaultRepoRoot,
    reportPath: "",
    reportSha256: "",
    policyPath: "",
    policySha256: "",
    attestationPath: "",
    snapshotRoot: "",
    leasePath: "",
    leaseNonce: "",
    runId: "",
    runStartedAt: "",
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const value = argv[index + 1];
    if (
      ![
        "--repo-root",
        "--report",
        "--report-sha",
        "--policy",
        "--policy-sha",
        "--build-attestation",
        "--dist-snapshot",
        "--lock-lease",
        "--lock-nonce",
        "--run-id",
        "--run-started-at",
      ].includes(arg) ||
      !value
    ) {
      throw new Error(`invalid or incomplete argument: ${arg}`);
    }
    index += 1;
    if (arg === "--repo-root") parsed.repoRoot = value;
    if (arg === "--report") parsed.reportPath = value;
    if (arg === "--report-sha") parsed.reportSha256 = value;
    if (arg === "--policy") parsed.policyPath = value;
    if (arg === "--policy-sha") parsed.policySha256 = value;
    if (arg === "--build-attestation") parsed.attestationPath = value;
    if (arg === "--dist-snapshot") parsed.snapshotRoot = value;
    if (arg === "--lock-lease") parsed.leasePath = value;
    if (arg === "--lock-nonce") parsed.leaseNonce = value;
    if (arg === "--run-id") parsed.runId = value;
    if (arg === "--run-started-at") parsed.runStartedAt = value;
  }
  return parsed;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!isSha256(args.reportSha256) || !isSha256(args.policySha256)) {
    throw new Error(
      "validator requires trusted report and policy SHA-256 values",
    );
  }
  const reportPublication = readBoundedFilePublication({
    repoRoot: args.repoRoot,
    candidatePath: args.reportPath,
    expectedPrefix: "output/",
    expectedSha256: args.reportSha256,
  });
  const report = JSON.parse(reportPublication.bytes.toString("utf8"));
  const policyPublication = readStableOrdinaryFile(
    args.policyPath,
    args.policySha256,
  );
  const policy = JSON.parse(policyPublication.bytes.toString("utf8"));
  const attestationPublication = readBoundedFilePublication({
    repoRoot: args.repoRoot,
    candidatePath: args.attestationPath,
    expectedPrefix: "output/qa/frontend-interaction-latency/",
  });
  const attestation = JSON.parse(attestationPublication.bytes.toString("utf8"));
  const buildValidation = validateBuildAttestation({
    attestation,
    webRoot: path.resolve(args.repoRoot, "web"),
    maxAgeMinutes: policy.build_attestation_max_age_minutes,
  });
  const snapshot = validateImmutableDistSnapshot({
    repoRoot: args.repoRoot,
    snapshotRoot: args.snapshotRoot,
    expectedDistSha256: attestation.dist?.sha256,
  });
  const lease = verifyAuthorityLease({
    repoRoot: args.repoRoot,
    leasePath: args.leasePath,
    expectedNonce: args.leaseNonce,
    expectedRunId: args.runId,
    maxAgeMinutes: policy.report_freshness_max_minutes,
  });
  const measurementHarnessSnapshot = computeCanonicalFileSetSnapshot(
    args.repoRoot,
    EXPECTED_MEASUREMENT_HARNESS_PATHS,
  );
  const scopedWorktreeState = computeAuthorityScopedWorktreeState(
    args.repoRoot,
  );
  const fixturePath = path.resolve(
    args.repoRoot,
    "web/e2e/browser/fixtures/artifacts/saved-options-artifact-pack.json",
  );
  const fixturePublication = readStableOrdinaryFile(fixturePath);
  const fixture = JSON.parse(fixturePublication.bytes.toString("utf8"));
  const detailFixture = buildPathDistinctDetailFixture(
    fixture,
    policy.warmup_samples_per_case + policy.measured_samples_per_case,
  );
  const result = validateLatencyReportArtifacts({
    report,
    reportPublication,
    expectedReportSha256: args.reportSha256,
    policy,
    policyPath: args.policyPath,
    policySha256: args.policySha256,
    attestation,
    attestationPublication,
    buildValidation,
    snapshot,
    lease,
    measurementHarnessSnapshot,
    scopedWorktreeState,
    fixtureSha256: fixturePublication.sha256,
    effectiveFixtureSha256: sha256Bytes(
      Buffer.from(JSON.stringify(detailFixture)),
    ),
    expectedRunId: args.runId,
    expectedRunStartedAt: args.runStartedAt,
    repoRoot: args.repoRoot,
  });
  readStableOrdinaryFile(args.policyPath, args.policySha256);
  const finalHarnessSnapshot = computeCanonicalFileSetSnapshot(
    args.repoRoot,
    EXPECTED_MEASUREMENT_HARNESS_PATHS,
  );
  if (
    JSON.stringify(finalHarnessSnapshot) !==
    JSON.stringify(measurementHarnessSnapshot)
  ) {
    throw new Error("measurement harness changed during post-run validation");
  }
  const finalScopedWorktreeState = computeAuthorityScopedWorktreeState(
    args.repoRoot,
  );
  if (
    JSON.stringify(finalScopedWorktreeState) !==
    JSON.stringify(scopedWorktreeState)
  ) {
    throw new Error(
      "authority-scoped worktree status changed during post-run validation",
    );
  }
  readStableOrdinaryFile(fixturePath, fixturePublication.sha256);
  readBoundedFilePublication({
    repoRoot: args.repoRoot,
    candidatePath: args.reportPath,
    expectedPrefix: "output/",
    expectedSha256: args.reportSha256,
  });
  console.log(JSON.stringify(result));
  if (result.status !== "passed") process.exitCode = 1;
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(
      error instanceof Error ? error.stack || error.message : String(error),
    );
    process.exitCode = 1;
  }
}
