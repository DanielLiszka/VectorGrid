import crypto from "node:crypto";
import { spawnSync } from "node:child_process";
import { isDeepStrictEqual } from "node:util";

import {
  BUILD_ATTESTATION_KIND,
  BUILD_ATTESTATION_PROFILE,
  FILE_SET_HASH_FRAMING,
  isSha256,
} from "./frontend-interaction-build-attestation.mjs";

const EXPECTED_SCHEMA_VERSION = 2;
const EXPECTED_REGISTRY_ID = "vectorgrid-frontend-interaction-latency-v2";
const EXPECTED_PROFILE = "scoped-warm-mocked-browser-v1";
const EXPECTED_MOCK_PROFILE = "strict-path-distinct-async-saved-detail-v1";
const EXPECTED_FIXTURE_PROFILE = "path-distinct-saved-run-details-v1";
const EXPECTED_FIXTURE_PATH =
  "web/e2e/browser/fixtures/artifacts/saved-options-artifact-pack.json";
export const EXPECTED_MEASUREMENT_HARNESS_PATHS = [
  "scripts/benchmark_lock.ps1",
  "scripts/run_frontend_interaction_latency.ps1",
  "web/scripts/frontend-interaction-authority-lease.mjs",
  "web/scripts/frontend-interaction-build-attestation.mjs",
  "web/scripts/frontend-interaction-latency-lib.mjs",
  "web/scripts/frontend-interaction-report-validator.mjs",
  "web/scripts/measure-frontend-interaction-latency.mjs",
  "web/scripts/frontend-interaction-static-snapshot.mjs",
  "web/scripts/support/mockedTauriHarness.mjs",
];
export const SCOPED_GIT_STATUS_FORMAT = "git-status-porcelain-v1-z";
export const EXPECTED_AUTHORITY_GIT_PATHS = [
  "docs/release_authority_matrix.json",
  "docs/verification_benchmark_release_authority.md",
  "scripts/benchmark_lock.ps1",
  "scripts/frontend_interaction_latency_policy.json",
  "scripts/run_frontend_interaction_latency.ps1",
  "web",
];
const REQUIRED_CASE_IDS = ["saved-run-detail-open", "workspace-navigation"];
const EMPTY_SHA256 = crypto
  .createHash("sha256")
  .update(Buffer.alloc(0))
  .digest("hex");

export function computeAuthorityScopedWorktreeState(repoRoot) {
  const result = spawnSync(
    "git",
    [
      "status",
      "--porcelain=v1",
      "-z",
      "--untracked-files=all",
      "--",
      ...EXPECTED_AUTHORITY_GIT_PATHS,
    ],
    {
      cwd: repoRoot,
      encoding: null,
      maxBuffer: 8 * 1024 * 1024,
      windowsHide: true,
    },
  );
  if (result.error || result.status !== 0) {
    const detail = Buffer.isBuffer(result.stderr)
      ? result.stderr.toString("utf8").trim()
      : String(result.stderr || "").trim();
    throw new Error(
      `scoped git worktree status probe failed${detail ? `: ${detail}` : ""}`,
      result.error ? { cause: result.error } : undefined,
    );
  }
  const statusBytes = Buffer.isBuffer(result.stdout)
    ? Buffer.from(result.stdout)
    : Buffer.from(result.stdout || "");
  return {
    clean: statusBytes.length === 0,
    format: SCOPED_GIT_STATUS_FORMAT,
    paths: [...EXPECTED_AUTHORITY_GIT_PATHS],
    sha256: crypto.createHash("sha256").update(statusBytes).digest("hex"),
    byte_count: statusBytes.length,
  };
}

function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function rounded(value, digits = 3) {
  return Number(Number(value).toFixed(digits));
}

function validTimestamp(value) {
  return typeof value === "string" && Number.isFinite(Date.parse(value));
}

function safeOutputJsonPath(value) {
  if (
    typeof value !== "string" ||
    value.includes("\\") ||
    !value.startsWith("output/") ||
    !value.endsWith(".json")
  ) {
    return false;
  }
  const segments = value.split("/");
  return (
    segments.length >= 2 &&
    segments.every(
      (segment) => segment !== "" && segment !== "." && segment !== "..",
    )
  );
}

function safeOutputDirectoryPath(value, expectedPrefix) {
  if (
    typeof value !== "string" ||
    value.includes("\\") ||
    !value.startsWith(expectedPrefix) ||
    value.endsWith("/")
  ) {
    return false;
  }
  return value.split("/").every((segment) => {
    return segment !== "" && segment !== "." && segment !== "..";
  });
}

export function buildPathDistinctDetailFixture(fixture, totalCount) {
  const summaries = [];
  const detailsByPath = {};
  const identities = [];
  for (let index = 0; index < totalCount; index += 1) {
    const ordinal = String(index + 1).padStart(3, "0");
    const artifactPath = `C:/runs/latency-detail-${ordinal}.json`;
    const readyFieldValue = `LATENCY_DETAIL_READY::${ordinal}::${artifactPath}`;
    summaries.push({
      ...structuredClone(fixture.savedRunSummaries[0]),
      ts: `2026-04-24T15:${String(index).padStart(2, "0")}:00Z`,
      path: artifactPath,
      config_fingerprint: `double-ma-latency-${ordinal}`,
      tags: ["qa", "latency", `latency-detail-${ordinal}`],
    });
    detailsByPath[artifactPath] = {
      ...structuredClone(fixture.parentRunDetail),
      ts: `2026-04-24T15:${String(index).padStart(2, "0")}:00Z`,
      artifactPath,
      configFingerprint: `double-ma-latency-${ordinal}`,
      paramsText: readyFieldValue,
    };
    identities.push({
      artifactPath,
      readyFieldValue,
      searchToken: `latency-detail-${ordinal}`,
    });
  }
  return { summaries, detailsByPath, identities };
}

export function percentile(values, quantile) {
  const samples = values
    .map(finiteNumber)
    .filter((value) => value != null)
    .sort((left, right) => left - right);
  if (samples.length === 0) return null;
  if (samples.length === 1) return rounded(samples[0]);

  const boundedQuantile = Math.min(1, Math.max(0, Number(quantile)));
  const position = (samples.length - 1) * boundedQuantile;
  const lowerIndex = Math.floor(position);
  const upperIndex = Math.ceil(position);
  if (lowerIndex === upperIndex) return rounded(samples[lowerIndex]);
  const weight = position - lowerIndex;
  return rounded(
    samples[lowerIndex] * (1 - weight) + samples[upperIndex] * weight,
  );
}

export function summarizeSamples(values) {
  const samples = values
    .map(finiteNumber)
    .filter((value) => value != null)
    .sort((left, right) => left - right);
  if (samples.length === 0) {
    return {
      count: 0,
      min_ms: null,
      median_ms: null,
      p90_ms: null,
      p95_ms: null,
      max_ms: null,
      mean_ms: null,
    };
  }
  const total = samples.reduce((sum, value) => sum + value, 0);
  return {
    count: samples.length,
    min_ms: rounded(samples[0]),
    median_ms: percentile(samples, 0.5),
    p90_ms: percentile(samples, 0.9),
    p95_ms: percentile(samples, 0.95),
    max_ms: rounded(samples.at(-1)),
    mean_ms: rounded(total / samples.length),
  };
}

export function validatePolicy(policy) {
  const issues = [];
  if (!policy || typeof policy !== "object" || Array.isArray(policy)) {
    return ["policy must be an object"];
  }
  if (policy.schema_version !== EXPECTED_SCHEMA_VERSION) {
    issues.push(`schema_version must equal ${EXPECTED_SCHEMA_VERSION}`);
  }
  if (policy.registry_id !== EXPECTED_REGISTRY_ID) {
    issues.push(`registry_id must equal ${EXPECTED_REGISTRY_ID}`);
  }
  if (policy.profile !== EXPECTED_PROFILE) {
    issues.push(`profile must equal ${EXPECTED_PROFILE}`);
  }
  if (policy.mock_profile !== EXPECTED_MOCK_PROFILE) {
    issues.push(`mock_profile must equal ${EXPECTED_MOCK_PROFILE}`);
  }
  if (policy.fixture_profile !== EXPECTED_FIXTURE_PROFILE) {
    issues.push(`fixture_profile must equal ${EXPECTED_FIXTURE_PROFILE}`);
  }
  if (
    !Number.isInteger(policy.warmup_samples_per_case) ||
    policy.warmup_samples_per_case < 2
  ) {
    issues.push("warmup_samples_per_case must be an integer >= 2");
  }
  if (
    !Number.isInteger(policy.measured_samples_per_case) ||
    policy.measured_samples_per_case < 10
  ) {
    issues.push("measured_samples_per_case must be an integer >= 10");
  }
  if (
    !Number.isInteger(policy.dom_quiet_window_ms) ||
    policy.dom_quiet_window_ms < 16 ||
    policy.dom_quiet_window_ms > 250
  ) {
    issues.push("dom_quiet_window_ms must be an integer between 16 and 250");
  }
  if (
    !Number.isInteger(policy.interaction_timeout_ms) ||
    policy.interaction_timeout_ms < 1000
  ) {
    issues.push("interaction_timeout_ms must be an integer >= 1000");
  }
  if (
    !Number.isInteger(policy.detail_mock_delay_ms) ||
    policy.detail_mock_delay_ms < 1
  ) {
    issues.push("detail_mock_delay_ms must be an integer >= 1");
  }
  for (const field of [
    "build_attestation_max_age_minutes",
    "report_freshness_max_minutes",
  ]) {
    if (!Number.isInteger(policy[field]) || policy[field] < 1) {
      issues.push(`${field} must be an integer >= 1`);
    }
  }
  if (
    !policy.viewport ||
    !Number.isInteger(policy.viewport.width) ||
    !Number.isInteger(policy.viewport.height) ||
    policy.viewport.width < 1024 ||
    policy.viewport.height < 720
  ) {
    issues.push(
      "viewport must contain integer width >= 1024 and height >= 720",
    );
  }

  const cases = Array.isArray(policy.cases) ? policy.cases : [];
  const ids = cases.map((entry) => String(entry?.id || ""));
  if (JSON.stringify(ids) !== JSON.stringify([...ids].sort())) {
    issues.push("cases must be sorted by id");
  }
  if (new Set(ids).size !== ids.length) {
    issues.push("case ids must be unique");
  }
  for (const requiredId of REQUIRED_CASE_IDS) {
    if (!ids.includes(requiredId))
      issues.push(`missing required case ${requiredId}`);
  }
  for (const entry of cases) {
    const id = String(entry?.id || "<missing>");
    if (!REQUIRED_CASE_IDS.includes(id)) issues.push(`unexpected case ${id}`);
    for (const field of ["label", "boundary", "readiness_contract_id"]) {
      if (
        typeof entry?.[field] !== "string" ||
        entry[field].trim().length === 0
      ) {
        issues.push(`${id}.${field} must be non-empty`);
      }
    }
    const p95Budget = finiteNumber(entry?.p95_budget_ms);
    const maxBudget = finiteNumber(entry?.max_budget_ms);
    if (p95Budget == null || p95Budget <= 0) {
      issues.push(`${id}.p95_budget_ms must be positive`);
    }
    if (maxBudget == null || maxBudget <= 0) {
      issues.push(`${id}.max_budget_ms must be positive`);
    }
    if (p95Budget != null && maxBudget != null && maxBudget < p95Budget) {
      issues.push(`${id}.max_budget_ms must be >= p95_budget_ms`);
    }
    if (
      id === "saved-run-detail-open" &&
      entry.require_unique_target_identity !== true
    ) {
      issues.push(
        "saved-run-detail-open.require_unique_target_identity must be true",
      );
    }
  }
  return [...new Set(issues)].sort();
}

function validateSample({
  sample,
  casePolicy,
  phase,
  expectedSequence,
  policy,
}) {
  const issues = [];
  const location = `${phase}[${expectedSequence}]`;
  if (!sample || typeof sample !== "object" || Array.isArray(sample)) {
    return [`${location} must be an object`];
  }
  if (sample.phase !== phase) {
    issues.push(`${location}.phase must equal ${phase}`);
  }
  if (sample.sequence !== expectedSequence) {
    issues.push(`${location}.sequence must equal ${expectedSequence}`);
  }
  const duration = finiteNumber(sample.duration_ms);
  if (duration == null || duration <= 0) {
    issues.push(`${location}.duration_ms must be a finite positive number`);
  }
  if (sample.click_is_trusted !== true) {
    issues.push(`${location}.click_is_trusted must be true`);
  }
  const quiet = finiteNumber(sample.quiet_window_observed_ms);
  if (quiet == null || quiet < policy.dom_quiet_window_ms) {
    issues.push(
      `${location}.quiet_window_observed_ms must be >= ${policy.dom_quiet_window_ms}`,
    );
  }
  if (sample.ready_checks_satisfied !== true) {
    issues.push(`${location}.ready_checks_satisfied must be true`);
  }
  if (sample.readiness_contract_id !== casePolicy.readiness_contract_id) {
    issues.push(
      `${location}.readiness_contract_id must equal ${casePolicy.readiness_contract_id}`,
    );
  }
  if (
    typeof sample.readiness_target_id !== "string" ||
    sample.readiness_target_id.trim().length === 0
  ) {
    issues.push(`${location}.readiness_target_id must be non-empty`);
  }
  if (
    typeof sample.trigger_identity !== "string" ||
    sample.trigger_identity.trim().length === 0
  ) {
    issues.push(`${location}.trigger_identity must be non-empty`);
  }

  if (casePolicy.id === "saved-run-detail-open") {
    if (sample.trigger_identity !== sample.readiness_target_id) {
      issues.push(
        `${location}.trigger_identity must match the saved-detail readiness_target_id`,
      );
    }
    if (
      typeof sample.ready_field_value !== "string" ||
      !sample.ready_field_value.startsWith("LATENCY_DETAIL_READY::") ||
      !sample.ready_field_value.endsWith(`::${sample.readiness_target_id}`)
    ) {
      issues.push(
        `${location}.ready_field_value must be the unique detail-only field for readiness_target_id`,
      );
    }
    const hydration = sample.async_detail_hydration;
    if (!hydration || typeof hydration !== "object") {
      issues.push(`${location}.async_detail_hydration must be present`);
    } else {
      if (hydration.command !== "load_run_artifact_detail") {
        issues.push(
          `${location}.async_detail_hydration.command must equal load_run_artifact_detail`,
        );
      }
      if (hydration.requested_path !== sample.readiness_target_id) {
        issues.push(
          `${location}.async_detail_hydration.requested_path must match readiness_target_id`,
        );
      }
      if (hydration.argument_matched !== true) {
        issues.push(
          `${location}.async_detail_hydration.argument_matched must be true`,
        );
      }
      if (hydration.matched_argument_name !== "path") {
        issues.push(
          `${location}.async_detail_hydration.matched_argument_name must equal path`,
        );
      }
      if (hydration.matched_argument_value !== sample.readiness_target_id) {
        issues.push(
          `${location}.async_detail_hydration.matched_argument_value must match readiness_target_id`,
        );
      }
      if (hydration.invocation_count_for_path !== 1) {
        issues.push(
          `${location}.async_detail_hydration.invocation_count_for_path must equal 1`,
        );
      }
      if (hydration.delayed_ms !== policy.detail_mock_delay_ms) {
        issues.push(
          `${location}.async_detail_hydration.delayed_ms must equal ${policy.detail_mock_delay_ms}`,
        );
      }
      const hydrationDuration = finiteNumber(hydration.duration_ms);
      if (hydrationDuration == null || hydrationDuration <= 0) {
        issues.push(
          `${location}.async_detail_hydration.duration_ms must be a finite positive number`,
        );
      }
      if (
        typeof hydration.ready_field_value !== "string" ||
        hydration.ready_field_value.trim().length === 0 ||
        hydration.ready_field_value !== sample.ready_field_value
      ) {
        issues.push(
          `${location}.async_detail_hydration.ready_field_value must match the unique ready field`,
        );
      }
      if (hydration.ready_field_identity !== "savedRunDetailParams") {
        issues.push(
          `${location}.async_detail_hydration.ready_field_identity must equal savedRunDetailParams`,
        );
      }
    }
  } else {
    if (sample.readiness_target_id !== "workspace:library") {
      issues.push(
        `${location}.readiness_target_id must equal workspace:library`,
      );
    }
    if (sample.trigger_identity !== "shell.workspace.library") {
      issues.push(
        `${location}.trigger_identity must equal shell.workspace.library`,
      );
    }
    if (sample.ready_field_value != null) {
      issues.push(`${location}.ready_field_value must be null`);
    }
    if (sample.async_detail_hydration != null) {
      issues.push(`${location}.async_detail_hydration must be null`);
    }
  }
  return issues;
}

function strictProfileEligible({
  policy,
  policyPath,
  policySha256,
  source,
  environment,
  run,
  now,
}) {
  const issues = [];
  if (policyPath !== "scripts/frontend_interaction_latency_policy.json") {
    issues.push("policy path is not canonical");
  }
  if (!isSha256(policySha256)) issues.push("policy SHA-256 is invalid");
  if (source?.fixture_profile !== EXPECTED_FIXTURE_PROFILE) {
    issues.push("fixture profile is not authoritative");
  }
  if (source?.fixture_path !== EXPECTED_FIXTURE_PATH) {
    issues.push("fixture path is not canonical");
  }
  for (const field of [
    "fixture_sha256",
    "effective_fixture_sha256",
    "measurement_harness_sha256",
    "measurement_harness_path_manifest_sha256",
    "dist_sha256",
    "served_dist_snapshot_manifest_sha256",
    "served_dist_snapshot_sha256",
    "served_dist_snapshot_path_manifest_sha256",
    "build_input_sha256",
    "build_attestation_sha256",
  ]) {
    if (!isSha256(source?.[field])) {
      issues.push(`source.${field} must be a lowercase SHA-256`);
    }
  }
  if (
    JSON.stringify(source?.measurement_harness_paths) !==
    JSON.stringify(EXPECTED_MEASUREMENT_HARNESS_PATHS)
  ) {
    issues.push("measurement harness path set is not canonical");
  }
  if (
    source?.measurement_harness_hash_framing !== FILE_SET_HASH_FRAMING ||
    source?.measurement_harness_file_count !==
      EXPECTED_MEASUREMENT_HARNESS_PATHS.length ||
    !Number.isInteger(source?.measurement_harness_total_bytes) ||
    source.measurement_harness_total_bytes <= 0
  ) {
    issues.push("measurement harness manifest metadata is not canonical");
  }
  if (
    source?.detail_fixture_count !==
    policy.warmup_samples_per_case + policy.measured_samples_per_case
  ) {
    issues.push("detail fixture count does not match the fixed sample profile");
  }
  if (
    source?.benchmark_lock?.verified !== true ||
    source.benchmark_lock.verified_at_start !== true ||
    source.benchmark_lock.verified_before_publication !== true ||
    !Number.isInteger(source?.benchmark_lock?.owner_pid) ||
    source.benchmark_lock.owner_pid <= 0 ||
    !Number.isInteger(source?.benchmark_lock?.mutex_owner_pid) ||
    source.benchmark_lock.mutex_owner_pid <= 0 ||
    !Number.isInteger(source?.benchmark_lock?.benchmark_mutex_owner_pid) ||
    source.benchmark_lock.benchmark_mutex_owner_pid <= 0 ||
    !validTimestamp(source?.benchmark_lock?.owner_started_at) ||
    !validTimestamp(source?.benchmark_lock?.mutex_owner_started_at) ||
    !validTimestamp(source?.benchmark_lock?.benchmark_mutex_owner_started_at) ||
    typeof source?.benchmark_lock?.context !== "string" ||
    source.benchmark_lock.context.trim().length === 0 ||
    source.benchmark_lock.wrapper_context !==
      "scoped warm mocked-browser click-to-painted-stable latency" ||
    !isSha256(source?.benchmark_lock?.token_sha256) ||
    !isSha256(source?.benchmark_lock?.lease_sha256) ||
    !isSha256(source?.benchmark_lock?.initial_lease_sha256) ||
    source.benchmark_lock.lease_sha256 !==
      source.benchmark_lock.initial_lease_sha256 ||
    !isSha256(source?.benchmark_lock?.nonce_sha256) ||
    source.benchmark_lock.mutex_name !==
      "Local\\ta_desktop_demo_frontend_interaction_authority" ||
    source.benchmark_lock.mutex_ownership_mode !== "acquired" ||
    source.benchmark_lock.mutex_owner_pid !== source.benchmark_lock.owner_pid ||
    source.benchmark_lock.benchmark_mutex_name !==
      "Local\\ta_desktop_demo_benchmark_gate" ||
    source.benchmark_lock.benchmark_mutex_ownership_mode !== "acquired" ||
    source.benchmark_lock.benchmark_mutex_owner_pid !==
      source.benchmark_lock.owner_pid ||
    source.benchmark_lock.mutex_probe_exit_code !== 0 ||
    source.benchmark_lock.benchmark_mutex_probe_exit_code !== 0 ||
    source.benchmark_lock.benchmark_lease_path !==
      "output/qa/benchmark-lock/active.json" ||
    !isSha256(source.benchmark_lock.benchmark_lease_sha256) ||
    typeof source.benchmark_lock.benchmark_lease_identity !== "string" ||
    !/^[a-f0-9]{8}:[a-f0-9]{16}$/i.test(
      source.benchmark_lock.benchmark_lease_identity,
    ) ||
    source.benchmark_lock.benchmark_lease_write_probe_win32_error_code !== 32 ||
    source.benchmark_lock.benchmark_lease_write_probe_exit_code !== 0 ||
    source.benchmark_lock.benchmark_lease_holder_verified !== true ||
    source.benchmark_lock.benchmark_lease_holder_probe_exit_code !== 0 ||
    source.benchmark_lock.write_probe_win32_error_code !== 32 ||
    source.benchmark_lock.write_probe_exit_code !== 0 ||
    !safeOutputJsonPath(source.benchmark_lock.lease_path) ||
    !source.benchmark_lock.lease_path.startsWith(
      "output/qa/frontend-interaction-latency/leases/",
    ) ||
    source.benchmark_lock.lease_path !==
      `output/qa/frontend-interaction-latency/leases/${run?.id}.json`
  ) {
    issues.push("benchmark lock provenance is not authoritative");
  }
  if (source?.build_attestation_verified !== true) {
    issues.push("build attestation was not verified");
  }
  if (
    typeof source?.scoped_worktree_clean !== "boolean" ||
    source?.scoped_worktree_status_format !== SCOPED_GIT_STATUS_FORMAT ||
    JSON.stringify(source?.scoped_worktree_paths) !==
      JSON.stringify(EXPECTED_AUTHORITY_GIT_PATHS) ||
    !isSha256(source?.scoped_worktree_status_sha256) ||
    !Number.isInteger(source?.scoped_worktree_status_bytes) ||
    source.scoped_worktree_status_bytes < 0 ||
    source.scoped_worktree_clean !==
      (source.scoped_worktree_status_bytes === 0) ||
    (source.scoped_worktree_clean &&
      source.scoped_worktree_status_sha256 !== EMPTY_SHA256) ||
    (!source.scoped_worktree_clean &&
      source.scoped_worktree_status_sha256 === EMPTY_SHA256)
  ) {
    issues.push("authority-scoped worktree status provenance is invalid");
  }
  if (source?.build_attestation_kind !== BUILD_ATTESTATION_KIND) {
    issues.push("build attestation kind is not authoritative");
  }
  if (source?.build_profile !== BUILD_ATTESTATION_PROFILE) {
    issues.push("build profile is not authoritative");
  }
  if (!safeOutputJsonPath(source?.build_attestation_path)) {
    issues.push(
      "build attestation path must be a safe JSON path under output/",
    );
  }
  if (!validTimestamp(source?.build_attestation_generated_at)) {
    issues.push("build attestation generated_at is invalid");
  } else {
    const generatedMs = Date.parse(source.build_attestation_generated_at);
    const nowMs = now.getTime();
    if (
      generatedMs > nowMs + 60_000 ||
      nowMs - generatedMs > policy.build_attestation_max_age_minutes * 60_000
    ) {
      issues.push("build attestation timestamp is stale or in the future");
    }
  }
  if (source?.dist_sha256 !== source?.attested_dist_sha256) {
    issues.push("measured dist hash does not match build attestation");
  }
  if (
    source?.served_dist_snapshot_sha256 !== source?.attested_dist_sha256 ||
    source?.served_from_memory_snapshot !== true ||
    !safeOutputDirectoryPath(
      source?.served_dist_snapshot_path,
      "output/qa/frontend-interaction-latency/dist-snapshots/",
    ) ||
    source?.served_dist_snapshot_path !==
      `output/qa/frontend-interaction-latency/dist-snapshots/${run?.id}` ||
    !safeOutputJsonPath(source?.served_dist_snapshot_manifest_path) ||
    source?.served_dist_snapshot_manifest_path !==
      `${source?.served_dist_snapshot_path}/manifest.json` ||
    !Number.isInteger(source?.served_dist_snapshot_file_count) ||
    source.served_dist_snapshot_file_count <= 0 ||
    !Number.isInteger(source?.served_dist_snapshot_total_bytes) ||
    source.served_dist_snapshot_total_bytes <= 0 ||
    source?.server_snapshot_entry_count !==
      source?.served_dist_snapshot_file_count ||
    source?.server_snapshot_total_bytes !==
      source?.served_dist_snapshot_total_bytes
  ) {
    issues.push("served immutable distribution snapshot is not authoritative");
  }
  if (source?.build_input_sha256 !== source?.attested_build_input_sha256) {
    issues.push("build-input hash does not match build attestation");
  }
  if (
    environment?.browser_name !== "chromium" ||
    environment?.headless !== true ||
    environment?.device_scale_factor !== 1 ||
    environment?.locale !== "en-US" ||
    environment?.timezone !== "UTC" ||
    environment?.reduced_motion !== "reduce"
  ) {
    issues.push("browser environment does not match the fixed profile");
  }
  if (
    typeof environment?.browser_version !== "string" ||
    environment.browser_version.trim().length === 0 ||
    typeof environment?.user_agent !== "string" ||
    environment.user_agent.trim().length === 0 ||
    typeof environment?.node_version !== "string" ||
    environment.node_version !== source?.attested_node_version
  ) {
    issues.push("runtime identity does not match the attested fixed profile");
  }
  if (
    source?.measurement_profile !== policy.profile ||
    source?.mock_profile !== policy.mock_profile
  ) {
    issues.push("measurement or mock profile does not match policy");
  }
  if (
    typeof run?.id !== "string" ||
    !/^[a-f0-9]{32}$/.test(run.id) ||
    !validTimestamp(run?.started_at)
  ) {
    issues.push("run identity or start timestamp is invalid");
  } else {
    const nowMs = now.getTime();
    const startedMs = Date.parse(run.started_at);
    if (
      startedMs > nowMs ||
      nowMs - startedMs > policy.report_freshness_max_minutes * 60_000
    ) {
      issues.push("run timestamp is stale or in the future");
    }
  }
  return issues;
}

export function buildLatencyReport({
  policy,
  policyPath,
  policySha256,
  authorityRequested,
  source,
  environment,
  run,
  measurements,
  runtimeIssues = [],
  now = new Date(),
}) {
  const policyIssues = validatePolicy(policy);
  const issues = [
    ...policyIssues,
    ...runtimeIssues.map((issue) => String(issue)),
  ];
  const profileIssues = strictProfileEligible({
    policy,
    policyPath,
    policySha256,
    source,
    environment,
    run,
    now,
  });
  const cases = [];

  for (const casePolicy of Array.isArray(policy?.cases) ? policy.cases : []) {
    const observation = measurements?.[casePolicy.id] ?? {};
    const warmups = Array.isArray(observation.warmup_samples)
      ? observation.warmup_samples
      : [];
    const measured = Array.isArray(observation.measured_samples)
      ? observation.measured_samples
      : [];
    const sampleIssues = [];
    for (let index = 0; index < warmups.length; index += 1) {
      sampleIssues.push(
        ...validateSample({
          sample: warmups[index],
          casePolicy,
          phase: "warmup",
          expectedSequence: index,
          policy,
        }),
      );
    }
    for (let index = 0; index < measured.length; index += 1) {
      sampleIssues.push(
        ...validateSample({
          sample: measured[index],
          casePolicy,
          phase: "measured",
          expectedSequence: index,
          policy,
        }),
      );
    }
    const allSamples = [...warmups, ...measured];
    const targetIds = allSamples.map((sample) => sample?.readiness_target_id);
    const readyFieldValues = allSamples.map(
      (sample) => sample?.ready_field_value,
    );
    const uniqueTargetIdentities =
      casePolicy.require_unique_target_identity !== true ||
      (targetIds.every(
        (value) => typeof value === "string" && value.length > 0,
      ) &&
        new Set(targetIds).size === targetIds.length &&
        readyFieldValues.every(
          (value) => typeof value === "string" && value.length > 0,
        ) &&
        new Set(readyFieldValues).size === readyFieldValues.length);
    if (!uniqueTargetIdentities) {
      sampleIssues.push(
        "saved-run detail target and ready-field identities must be distinct across warmup and measured samples",
      );
    }
    const stats = summarizeSamples(
      measured.map((sample) => sample?.duration_ms),
    );
    const checks = {
      warmup_count_exact:
        warmups.length === Number(policy.warmup_samples_per_case),
      measured_sample_count_exact:
        measured.length === Number(policy.measured_samples_per_case),
      per_sample_evidence_valid: sampleIssues.length === 0,
      target_identity_contract_satisfied: uniqueTargetIdentities,
      p95_within_budget:
        stats.p95_ms != null &&
        stats.p95_ms <= Number(casePolicy.p95_budget_ms),
      max_within_budget:
        stats.max_ms != null &&
        stats.max_ms <= Number(casePolicy.max_budget_ms),
    };
    const failedChecks = Object.entries(checks)
      .filter(([, passed]) => !passed)
      .map(([name]) => name);
    if (failedChecks.length > 0) {
      issues.push(`${casePolicy.id}: failed ${failedChecks.join(", ")}`);
    }
    issues.push(...sampleIssues.map((issue) => `${casePolicy.id}: ${issue}`));
    cases.push({
      id: casePolicy.id,
      label: casePolicy.label,
      boundary: casePolicy.boundary,
      readiness_contract_id: casePolicy.readiness_contract_id,
      warmup_samples: warmups,
      measured_samples: measured,
      stats,
      budgets: {
        p95_ms: Number(casePolicy.p95_budget_ms),
        max_ms: Number(casePolicy.max_budget_ms),
      },
      checks,
      status: failedChecks.length === 0 ? "passed" : "failed",
    });
  }

  const authorityEligible = Boolean(
    authorityRequested &&
    policyIssues.length === 0 &&
    profileIssues.length === 0 &&
    runtimeIssues.length === 0 &&
    cases.length === REQUIRED_CASE_IDS.length &&
    cases.every((entry) => entry.status === "passed"),
  );
  const releaseEvidenceEligible = Boolean(
    authorityEligible && source?.scoped_worktree_clean === true,
  );
  const evidenceScope = authorityEligible
    ? releaseEvidenceEligible
      ? "clean-source-release-evidence-candidate"
      : "dirty-source-local-candidate-rehearsal"
    : "non-authoritative";
  if (authorityRequested && profileIssues.length > 0) {
    issues.push(...profileIssues.map((issue) => `profile: ${issue}`));
  }
  const uniqueIssues = [...new Set(issues)].sort();
  const passed = authorityEligible && uniqueIssues.length === 0;
  const generatedAt = now.toISOString();

  return {
    schema_version: 2,
    report_kind: "scoped-warm-mocked-browser-click-to-painted-stable-latency",
    generated_at: generatedAt,
    status: passed
      ? "passed"
      : authorityRequested
        ? "failed"
        : "diagnostic_only",
    authority_eligible: authorityEligible,
    release_evidence_eligible: releaseEvidenceEligible,
    evidence_scope: evidenceScope,
    registry_id: policy?.registry_id ?? null,
    run: {
      id: run?.id ?? null,
      started_at: run?.started_at ?? null,
      completed_at: generatedAt,
    },
    policy: {
      path: policyPath,
      sha256: policySha256,
      profile: policy?.profile ?? null,
      mock_profile: policy?.mock_profile ?? null,
      fixture_profile: policy?.fixture_profile ?? null,
      warmup_samples_per_case: policy?.warmup_samples_per_case ?? null,
      measured_samples_per_case: policy?.measured_samples_per_case ?? null,
      dom_quiet_window_ms: policy?.dom_quiet_window_ms ?? null,
      interaction_timeout_ms: policy?.interaction_timeout_ms ?? null,
      detail_mock_delay_ms: policy?.detail_mock_delay_ms ?? null,
      viewport: policy?.viewport ?? null,
    },
    measurement_boundary:
      "trusted headless Chromium click capture -> expected mocked-browser state and unique detail hydration field -> DOM quiet window -> two requestAnimationFrame callbacks -> next task after the intervening paint",
    authority_scope:
      "warm built-frontend interactions under strict path-matched mocked Tauri; representative workspace navigation and saved-result detail only",
    exclusions: [
      "full application or end-to-end workflow latency",
      "backend execution and provider/network latency",
      "native Tauri/WebView2 shell overhead",
      "Playwright actionability wait before the DOM click event",
      "cold application startup",
      "production-device and cross-WebView diversity",
    ],
    source,
    environment,
    cases,
    issues: uniqueIssues,
  };
}

export function recomputeLatencyReport({
  report,
  policy,
  policyPath,
  policySha256,
} = {}) {
  const issues = [];
  if (!report || typeof report !== "object" || Array.isArray(report)) {
    return {
      status: "failed",
      issues: ["published latency report must be an object"],
      recomputed: null,
    };
  }
  const generatedAt = new Date(String(report.generated_at || ""));
  if (!Number.isFinite(generatedAt.getTime())) {
    issues.push("published latency report generated_at is invalid");
  }

  const reportCases = Array.isArray(report.cases) ? report.cases : [];
  const measurements = {};
  for (const casePolicy of Array.isArray(policy?.cases) ? policy.cases : []) {
    const matches = reportCases.filter((entry) => entry?.id === casePolicy.id);
    if (matches.length !== 1) {
      issues.push(
        `published latency report must contain exactly one ${casePolicy.id} case`,
      );
      continue;
    }
    measurements[casePolicy.id] = {
      warmup_samples: matches[0].warmup_samples,
      measured_samples: matches[0].measured_samples,
    };
  }

  let recomputed = null;
  if (issues.length === 0) {
    recomputed = buildLatencyReport({
      policy,
      policyPath,
      policySha256,
      authorityRequested: true,
      run: {
        id: report.run?.id,
        started_at: report.run?.started_at,
      },
      source: report.source,
      environment: report.environment,
      measurements,
      runtimeIssues: [],
      now: generatedAt,
    });
    if (!isDeepStrictEqual(report, recomputed)) {
      issues.push(
        "published latency report does not equal the report recomputed from its raw samples and authoritative inputs",
      );
    }
  }
  return {
    status: issues.length === 0 ? "passed" : "failed",
    issues,
    recomputed,
  };
}
