#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const EXPECTED_SCHEMA_VERSION = 1;
const DEFAULT_MATRIX_PATH = "docs/release_authority_matrix.json";
const ALLOWED_EVIDENCE_KINDS = new Set(["artifact", "ci_check", "command"]);
const ALLOWED_EVIDENCE_STATUSES = new Set(["active", "specialized"]);
const ALLOWED_CLAIM_STATUSES = new Set(["active", "specialized"]);
const ALLOWED_GAP_STATUSES = new Set([
  "blocked",
  "deferred",
  "implemented",
  "research_spike",
]);
const PRIMARY_FORBIDDEN_CLASSES = new Set([
  "companion_diagnostic",
  "non_authority_helper",
]);
const REQUIRED_AUTHORITY_CLASSES = [
  "broad_authoritative_gate",
  "companion_diagnostic",
  "non_authority_helper",
  "release_authority_artifact",
  "required_cpu_ci_mirror",
  "subsystem_authority",
];
const REQUIRED_CLAIMS = [
  "alma-metric-contract-rows",
  "artifact-report-shape",
  "broad-repo-quality",
  "campaign-orchestration-performance",
  "cpu-performance",
  "cpu-reliability-operator-authority",
  "frontend-scoped-warm-mocked-browser-latency",
  "frontend-workflow-behavior",
  "full-local-release-rehearsal",
  "gpu-cuda-performance",
  "gpu-followup-candidate-detail-latency",
  "gpu-ma-performance",
  "gpu-validation-performance-residency",
  "live-execution-safety",
  "ml-signal-quality-native-cpu",
  "options-expression-gpu-promotion",
  "public-release-eligibility",
  "release-readiness",
  "repo-wide-performance-authority-inventory",
  "support-surface-performance",
  "warm-workflow-regression",
];
const REQUIRED_RELEASE_VARIANTS = [
  "cpu",
  "gpu-kernel",
  "gpu-ma",
  "live-execution-enabled",
  "ml-signal-native-cpu",
  "options-gpu-scorer",
];
const PUBLIC_LIVE_EXECUTION_VARIANTS = ["cpu", "gpu-kernel", "gpu-ma"];
const PUBLIC_RELEASE_VARIANTS = ["cpu", "gpu-kernel", "gpu-ma"];
const REQUIRED_PUBLIC_LIVE_SAFETY_SOURCES = [
  "execution_safety_gate",
  "live_safety_harness",
];
const REQUIRED_PUBLIC_RELEASE_SOURCES = [
  "packaged_smoke_artifact",
  "public_release_gate",
  "public_release_summary_artifact",
  "release_proof_evidence_manifest_artifact",
];
const REQUIRED_PUBLIC_LIVE_SAFETY_WIRING = [
  "execution-safety-credential-isolation",
  "execution-safety-public-routing",
  "live-safety-harness-public-evidence-contract",
  "live-safety-harness-public-evidence-quality-contract",
  "live-safety-harness-public-routing",
];
const REQUIRED_PUBLIC_RELEASE_WIRING = [
  "public-release-dirty-unsigned-ineligible",
  "public-release-release-proof-suite",
  "public-release-runtime-executed",
  "public-release-source-state-revalidation",
];
const REQUIRED_CPU_RELIABILITY_WIRING = [
  "cpu-reliability-operator-minimum",
  "cpu-reliability-short-fail-closed",
];
const PINNED_AUTHORITY_SOURCE_CONTRACTS = {
  cpu_reliability_soak_artifact: {
    authority_class: "release_authority_artifact",
    kind: "artifact",
    artifact_name: "cpu-reliability-soak-evidence.json",
    producer: "cpu_reliability_soak_gate",
    status: "active",
  },
  cpu_reliability_soak_gate: {
    authority_class: "subsystem_authority",
    kind: "command",
    path: "scripts/run_cpu_reliability_soak.ps1",
    invocation:
      "powershell -File scripts/run_cpu_reliability_soak.ps1 -Mode operator -Hours 6 -CargoTargetDir <dedicated-path>",
    status: "active",
    script_needles: [
      "$minimumOperatorSeconds = 6 * 60 * 60",
      "$minimumOperatorMeasuredCycles = 30",
      "$maximumOperatorSampleGapMs = 15 * 60 * 1000",
      "fresh_build_required = $Mode -eq \"operator\"",
      "![bool]$evidence.authority.operator_release_eligible",
      "Short soak must not claim operator release authority",
    ],
  },
  packaged_smoke_artifact: {
    authority_class: "release_authority_artifact",
    kind: "artifact",
    artifact_name: "packaged-smoke-summary.json",
    producer: "packaged_smoke_runner",
    status: "active",
  },
  packaged_smoke_runner: {
    authority_class: "subsystem_authority",
    kind: "command",
    path: "scripts/run_packaged_smoke.ps1",
    invocation: "powershell -File scripts/run_packaged_smoke.ps1 <required args>",
    status: "active",
    script_needles: [
      "$isReleaseProof = $DesktopSuite -eq \"release-proof\"",
      "release-proof-evidence-manifest.json",
      "runtime_executed = -not [bool]$StaticInspectionOnly",
      "binary_origin = $binaryOrigin",
    ],
  },
  public_release_gate: {
    authority_class: "broad_authoritative_gate",
    kind: "command",
    path: "scripts/run_public_release_gate.ps1",
    invocation:
      "powershell -File scripts/run_public_release_gate.ps1 -Variant <variant> <required signing args>",
    status: "active",
    script_needles: [
      "public_release_eligible = (Test-PublicReleaseEligibility -Status $Status)",
      "-not [bool]$AllowDirtyWorktree -and",
      "-not [bool]$AllowUnsignedBeta -and",
      "[bool]$script:SourceStateRevalidated -and",
      "[bool]$script:SourceMutationMonitorRevalidated -and",
      "[bool]$script:SourceReadLeaseRevalidated -and",
      "[string]$script:FinalSourceSnapshot.head -eq",
      "[string]$script:FinalSourceSnapshot.status_sha256 -eq",
      "git ls-files -v -z --cached -- .",
      "Get-PublicReleaseRawFileManifest",
      "Assert-PublicReleaseSourceHasSingleLink",
      "Assert-PublicReleaseSourceReadLeasesActive",
      "Start-PublicReleaseSourceMutationMonitor",
      "Assert-PublicReleaseSourceMutationMonitorClean",
      "\"-DesktopSuite\", \"release-proof\"",
      "if (-not [bool]$smoke.runtime_executed -or [string]$smoke.binary_origin -ne \"extracted-bundle\")",
      "Public release requires one passing, current-run release-proof manifest covering every allowlisted native journey.",
      "Final public release requires -ExpectedSignerThumbprint",
      "Public release requires a trusted timestamp on the embedded application signature.",
      "Invoke-PublicReleaseStep -Name \"Final source-state revalidation\"",
      "Assert-PublicReleaseSourceSnapshotStable",
    ],
  },
  public_release_summary_artifact: {
    authority_class: "release_authority_artifact",
    kind: "artifact",
    artifact_name: "public-release-summary.json",
    producer: "public_release_gate",
    status: "active",
  },
  release_proof_evidence_manifest_artifact: {
    authority_class: "release_authority_artifact",
    kind: "artifact",
    artifact_name: "release-proof-evidence-manifest.json",
    producer: "packaged_smoke_runner",
    status: "active",
  },
};
const PINNED_CLAIM_CONTRACTS = {
  "cpu-reliability-operator-authority": {
    label: "CPU reliability operator authority",
    status: "active",
    primary_authority: ["cpu_reliability_soak_gate"],
    companion_sources: ["cpu_reliability_soak_artifact"],
    required_companion_evidence_needles: [
      "fresh release build",
      "at least six hours",
      "at least 30 measured cycles",
      "maximum sample gap",
      "operator_release_eligible=true",
      "short mode",
      "not operator release authority",
    ],
    not_authority_needles: [
      "CPU or GPU performance",
      "Native desktop",
      "public-release behavior",
      "short-mode soak",
    ],
  },
  "public-release-eligibility": {
    label: "Public release eligibility",
    status: "active",
    primary_authority: ["public_release_gate"],
    companion_sources: [
      "packaged_smoke_artifact",
      "public_release_summary_artifact",
      "release_proof_evidence_manifest_artifact",
    ],
    required_companion_evidence_needles: [
      "runtime_executed=true",
      "binary_origin=extracted-bundle",
      "desktop_suite=release-proof",
      "current run",
      "release variant",
      "packaged-binary identity",
      "clean",
      "signed",
      "timestamped",
      "public_release_eligible=true",
      "dirty-worktree",
      "unsigned-beta",
      "rehearsal-only",
      "exact Git HEAD",
      "raw NUL-delimited",
      "match byte-for-byte",
      "canonical raw-byte manifests",
      "nonstandard tracked index flags",
      "single-link source files",
      "retained read leases",
      "run-long filesystem monitor",
      "zero in-scope mutation events",
      "source-state change fails closed",
    ],
    not_authority_needles: [
      "release-candidate rehearsal alone",
      "Static package inspection",
      "raw-binary smoke",
      "dirty-worktree",
      "unsigned-beta rehearsal",
      "branch protection alone",
    ],
  },
};
const REQUIRED_IMPLEMENTED_GAPS = [
  "cuda-graphs-timing-boundary",
  "dedicated-gpu-ma-perf-authority",
  "options-gpu-scorer-release-promotion-authority",
];
const REQUIRED_RESEARCH_SPIKE_GAPS = ["full-frontend-click-to-render-latency"];

function resolveRepoRoot(start = process.cwd()) {
  let cursor = path.resolve(start);
  for (;;) {
    if (
      fs.existsSync(path.join(cursor, "Cargo.toml")) &&
      fs.existsSync(path.join(cursor, "scripts")) &&
      fs.existsSync(path.join(cursor, "docs"))
    ) {
      return cursor;
    }
    const parent = path.dirname(cursor);
    if (parent === cursor) break;
    cursor = parent;
  }
  throw new Error(`could not resolve repo root from ${start}`);
}

function parseArgs(argv) {
  const parsed = {
    matrixPath: DEFAULT_MATRIX_PATH,
    outPath: "",
    quiet: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--matrix") {
      const value = argv[index + 1];
      if (!value) throw new Error("--matrix requires a path");
      parsed.matrixPath = value;
      index += 1;
    } else if (arg === "--out") {
      const value = argv[index + 1];
      if (!value) throw new Error("--out requires a path");
      parsed.outPath = value;
      index += 1;
    } else if (arg === "--quiet") {
      parsed.quiet = true;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function isSafeRepoRelativePath(value) {
  if (typeof value !== "string" || value.length === 0 || path.isAbsolute(value))
    return false;
  const normalized = value.replaceAll("\\", "/");
  return !normalized.split("/").some((part) => part === ".." || part === "");
}

function resolveWithinRepo(repoRoot, relativePath) {
  if (!isSafeRepoRelativePath(relativePath)) {
    throw new Error(`unsafe repo-relative path: ${String(relativePath)}`);
  }
  const resolved = path.resolve(repoRoot, relativePath);
  const rootPrefix = `${path.resolve(repoRoot)}${path.sep}`;
  if (!resolved.startsWith(rootPrefix)) {
    throw new Error(`path escaped repo root: ${relativePath}`);
  }
  return resolved;
}

function addError(errors, location, message) {
  errors.push(`${location}: ${message}`);
}

function validateSortedUniqueIds(items, location, errors) {
  if (!Array.isArray(items)) {
    addError(errors, location, "must be an array");
    return new Map();
  }
  const byId = new Map();
  const ids = [];
  for (let index = 0; index < items.length; index += 1) {
    const item = items[index];
    const itemLocation = `${location}[${index}]`;
    if (!item || typeof item !== "object" || Array.isArray(item)) {
      addError(errors, itemLocation, "must be an object");
      continue;
    }
    if (typeof item.id !== "string" || !/^[a-z0-9][a-z0-9_-]*$/.test(item.id)) {
      addError(
        errors,
        `${itemLocation}.id`,
        "must be a lowercase identifier using letters, numbers, hyphens, or underscores",
      );
      continue;
    }
    if (byId.has(item.id)) {
      addError(errors, `${itemLocation}.id`, `duplicate id ${item.id}`);
      continue;
    }
    byId.set(item.id, item);
    ids.push(item.id);
  }
  const sorted = [...ids].sort((left, right) => left.localeCompare(right));
  if (JSON.stringify(ids) !== JSON.stringify(sorted)) {
    addError(errors, location, "entries must be sorted by id");
  }
  return byId;
}

function validateStringArray(
  value,
  location,
  errors,
  { nonEmpty = false, unique = true } = {},
) {
  if (!Array.isArray(value)) {
    addError(errors, location, "must be an array");
    return [];
  }
  if (nonEmpty && value.length === 0) {
    addError(errors, location, "must not be empty");
  }
  const seen = new Set();
  for (let index = 0; index < value.length; index += 1) {
    const entry = value[index];
    if (typeof entry !== "string" || entry.trim().length === 0) {
      addError(errors, `${location}[${index}]`, "must be a non-empty string");
      continue;
    }
    if (unique && seen.has(entry)) {
      addError(errors, `${location}[${index}]`, `duplicate value ${entry}`);
    }
    seen.add(entry);
  }
  return value;
}

function validateReferenceArray(
  value,
  location,
  knownIds,
  errors,
  options = {},
) {
  const entries = validateStringArray(value, location, errors, options);
  for (let index = 0; index < entries.length; index += 1) {
    if (typeof entries[index] === "string" && !knownIds.has(entries[index])) {
      addError(
        errors,
        `${location}[${index}]`,
        `unknown reference ${entries[index]}`,
      );
    }
  }
  return entries;
}

function validateRequiredIds(actualIds, requiredIds, location, errors) {
  for (const requiredId of requiredIds) {
    if (!actualIds.has(requiredId)) {
      addError(errors, location, `missing required id ${requiredId}`);
    }
  }
}

function validateExactStringArray(value, expected, location, errors) {
  if (
    !Array.isArray(value) ||
    JSON.stringify(value) !== JSON.stringify(expected)
  ) {
    addError(
      errors,
      location,
      `must equal ${JSON.stringify(expected)}`,
    );
  }
}

function validateTextNeedles(text, needles, location, errors) {
  const normalized = typeof text === "string" ? text.toLowerCase() : "";
  for (const needle of needles) {
    if (!normalized.includes(needle.toLowerCase())) {
      addError(errors, location, `missing required authority text: ${needle}`);
    }
  }
}

function validatePinnedAuthoritySourceContracts({
  evidenceSources,
  repoRoot,
  errors,
}) {
  for (const [sourceId, expected] of Object.entries(
    PINNED_AUTHORITY_SOURCE_CONTRACTS,
  )) {
    const location = `pinned_authority_sources.${sourceId}`;
    const source = evidenceSources.get(sourceId);
    if (!source) {
      addError(errors, location, "missing pinned authority source");
      continue;
    }
    for (const field of [
      "authority_class",
      "kind",
      "path",
      "invocation",
      "artifact_name",
      "producer",
      "status",
    ]) {
      if (
        Object.hasOwn(expected, field) &&
        source[field] !== expected[field]
      ) {
        addError(
          errors,
          `${location}.${field}`,
          `must equal ${JSON.stringify(expected[field])}`,
        );
      }
    }
    if (Array.isArray(expected.script_needles)) {
      const scriptPath = resolveWithinRepo(repoRoot, expected.path);
      if (!fs.existsSync(scriptPath) || !fs.statSync(scriptPath).isFile()) {
        addError(
          errors,
          `${location}.script_needles`,
          `pinned script does not exist: ${expected.path}`,
        );
        continue;
      }
      const scriptText = fs.readFileSync(scriptPath, "utf8");
      validateTextNeedles(
        scriptText,
        expected.script_needles,
        `${location}.script_needles`,
        errors,
      );
    }
  }
}

function validatePinnedClaimContracts(claims, errors) {
  for (const [claimId, expected] of Object.entries(PINNED_CLAIM_CONTRACTS)) {
    const location = `pinned_claims.${claimId}`;
    const claim = claims.get(claimId);
    if (!claim) {
      addError(errors, location, "missing pinned authority claim");
      continue;
    }
    for (const field of ["label", "status"]) {
      if (claim[field] !== expected[field]) {
        addError(
          errors,
          `${location}.${field}`,
          `must equal ${JSON.stringify(expected[field])}`,
        );
      }
    }
    validateExactStringArray(
      claim.primary_authority,
      expected.primary_authority,
      `${location}.primary_authority`,
      errors,
    );
    validateExactStringArray(
      claim.companion_sources,
      expected.companion_sources,
      `${location}.companion_sources`,
      errors,
    );
    validateTextNeedles(
      Array.isArray(claim.required_companion_evidence)
        ? claim.required_companion_evidence.join("\n")
        : "",
      expected.required_companion_evidence_needles,
      `${location}.required_companion_evidence`,
      errors,
    );
    validateTextNeedles(
      Array.isArray(claim.not_authority)
        ? claim.not_authority.join("\n")
        : "",
      expected.not_authority_needles,
      `${location}.not_authority`,
      errors,
    );
  }
}

function validateDocumentAnchor(
  documentText,
  label,
  section,
  location,
  errors,
) {
  if (typeof label !== "string" || label.trim().length === 0) {
    addError(errors, `${location}.label`, "must be a non-empty string");
    return;
  }
  if (!documentText.includes(`| ${label} |`)) {
    addError(
      errors,
      `${location}.label`,
      `is not present as a ${section} table row in the policy document`,
    );
  }
}

export function validateAuthorityMatrix({
  matrix,
  repoRoot,
  matrixRelativePath = DEFAULT_MATRIX_PATH,
}) {
  const errors = [];
  if (!matrix || typeof matrix !== "object" || Array.isArray(matrix)) {
    return ["matrix: must be a JSON object"];
  }
  if (matrix.schema_version !== EXPECTED_SCHEMA_VERSION) {
    addError(errors, "schema_version", `must equal ${EXPECTED_SCHEMA_VERSION}`);
  }
  if (matrix.matrix_id !== "vectorgrid-release-verification-authority-v1") {
    addError(errors, "matrix_id", "unexpected matrix identifier");
  }
  if (!isSafeRepoRelativePath(matrixRelativePath)) {
    addError(
      errors,
      "matrix_path",
      "must be repo-relative and may not contain parent traversal",
    );
  }
  if (!isSafeRepoRelativePath(matrix.policy_document)) {
    addError(errors, "policy_document", "must be a safe repo-relative path");
  }

  let policyDocumentText = "";
  if (isSafeRepoRelativePath(matrix.policy_document)) {
    const policyPath = resolveWithinRepo(repoRoot, matrix.policy_document);
    if (!fs.existsSync(policyPath)) {
      addError(
        errors,
        "policy_document",
        `file does not exist: ${matrix.policy_document}`,
      );
    } else {
      policyDocumentText = fs.readFileSync(policyPath, "utf8");
      for (const requiredText of [
        "docs/release_authority_matrix.json",
        "scripts/check_release_authority_matrix.mjs",
        "machine-readable policy index",
      ]) {
        if (!policyDocumentText.includes(requiredText)) {
          addError(
            errors,
            "policy_document",
            `missing required contract text: ${requiredText}`,
          );
        }
      }
    }
  }

  const authorityClasses = validateSortedUniqueIds(
    matrix.authority_classes,
    "authority_classes",
    errors,
  );
  validateRequiredIds(
    authorityClasses,
    REQUIRED_AUTHORITY_CLASSES,
    "authority_classes",
    errors,
  );
  for (const [id, authorityClass] of authorityClasses) {
    if (
      typeof authorityClass.description !== "string" ||
      authorityClass.description.trim().length === 0
    ) {
      addError(
        errors,
        `authority_classes.${id}.description`,
        "must be a non-empty string",
      );
    }
  }

  const evidenceSources = validateSortedUniqueIds(
    matrix.evidence_sources,
    "evidence_sources",
    errors,
  );
  for (const [id, source] of evidenceSources) {
    const location = `evidence_sources.${id}`;
    if (!authorityClasses.has(source.authority_class)) {
      addError(
        errors,
        `${location}.authority_class`,
        `unknown class ${String(source.authority_class)}`,
      );
    }
    if (!ALLOWED_EVIDENCE_KINDS.has(source.kind)) {
      addError(
        errors,
        `${location}.kind`,
        `must be one of ${[...ALLOWED_EVIDENCE_KINDS].join(", ")}`,
      );
    }
    if (!ALLOWED_EVIDENCE_STATUSES.has(source.status)) {
      addError(
        errors,
        `${location}.status`,
        `must be one of ${[...ALLOWED_EVIDENCE_STATUSES].join(", ")}`,
      );
    }
    if (source.kind === "command" || source.kind === "ci_check") {
      if (!isSafeRepoRelativePath(source.path)) {
        addError(
          errors,
          `${location}.path`,
          "must be a safe repo-relative path",
        );
      } else {
        const resolved = resolveWithinRepo(repoRoot, source.path);
        if (!fs.existsSync(resolved) || !fs.statSync(resolved).isFile()) {
          addError(
            errors,
            `${location}.path`,
            `file does not exist: ${source.path}`,
          );
        }
      }
      if (
        typeof source.invocation !== "string" ||
        source.invocation.trim().length === 0
      ) {
        addError(
          errors,
          `${location}.invocation`,
          "must be a non-empty string",
        );
      } else if (
        source.kind === "command" &&
        isSafeRepoRelativePath(source.path) &&
        !source.invocation
          .replaceAll("\\", "/")
          .includes(source.path.replaceAll("\\", "/"))
      ) {
        addError(
          errors,
          `${location}.invocation`,
          `must name command path ${source.path}`,
        );
      }
      if (source.kind === "ci_check") {
        if (!isSafeRepoRelativePath(source.delegate)) {
          addError(
            errors,
            `${location}.delegate`,
            "must be a safe repo-relative path",
          );
        } else {
          const delegatePath = resolveWithinRepo(repoRoot, source.delegate);
          if (
            !fs.existsSync(delegatePath) ||
            !fs.statSync(delegatePath).isFile()
          ) {
            addError(
              errors,
              `${location}.delegate`,
              `file does not exist: ${source.delegate}`,
            );
          } else if (
            isSafeRepoRelativePath(source.path) &&
            fs.existsSync(resolveWithinRepo(repoRoot, source.path)) &&
            !fs
              .readFileSync(resolveWithinRepo(repoRoot, source.path), "utf8")
              .includes(source.delegate.replaceAll("\\", "/"))
          ) {
            addError(
              errors,
              `${location}.delegate`,
              `${source.path} does not invoke ${source.delegate}`,
            );
          }
        }
      }
    } else if (source.kind === "artifact") {
      if (
        typeof source.artifact_name !== "string" ||
        source.artifact_name.trim().length === 0
      ) {
        addError(
          errors,
          `${location}.artifact_name`,
          "must be a non-empty string",
        );
      }
      if (
        typeof source.producer !== "string" ||
        !evidenceSources.has(source.producer)
      ) {
        addError(
          errors,
          `${location}.producer`,
          `unknown producer ${String(source.producer)}`,
        );
      } else if (evidenceSources.get(source.producer)?.kind !== "command") {
        addError(
          errors,
          `${location}.producer`,
          "must reference a command evidence source",
        );
      }
    }
  }
  validatePinnedAuthoritySourceContracts({
    evidenceSources,
    repoRoot,
    errors,
  });

  const claims = validateSortedUniqueIds(matrix.claims, "claims", errors);
  validateRequiredIds(claims, REQUIRED_CLAIMS, "claims", errors);
  for (const [id, claim] of claims) {
    const location = `claims.${id}`;
    if (!ALLOWED_CLAIM_STATUSES.has(claim.status)) {
      addError(
        errors,
        `${location}.status`,
        `must be one of ${[...ALLOWED_CLAIM_STATUSES].join(", ")}`,
      );
    }
    validateDocumentAnchor(
      policyDocumentText,
      claim.label,
      "claim ledger",
      location,
      errors,
    );
    const primary = validateReferenceArray(
      claim.primary_authority,
      `${location}.primary_authority`,
      evidenceSources,
      errors,
      { nonEmpty: true },
    );
    for (const sourceId of primary) {
      const source = evidenceSources.get(sourceId);
      if (source && PRIMARY_FORBIDDEN_CLASSES.has(source.authority_class)) {
        addError(
          errors,
          `${location}.primary_authority`,
          `${sourceId} uses forbidden primary class ${source.authority_class}`,
        );
      }
    }
    validateReferenceArray(
      claim.companion_sources,
      `${location}.companion_sources`,
      evidenceSources,
      errors,
    );
    validateStringArray(
      claim.required_companion_evidence,
      `${location}.required_companion_evidence`,
      errors,
    );
    validateStringArray(
      claim.not_authority,
      `${location}.not_authority`,
      errors,
      {
        nonEmpty: true,
      },
    );
  }
  validatePinnedClaimContracts(claims, errors);

  const releaseVariants = validateSortedUniqueIds(
    matrix.release_variants,
    "release_variants",
    errors,
  );
  validateRequiredIds(
    releaseVariants,
    REQUIRED_RELEASE_VARIANTS,
    "release_variants",
    errors,
  );
  for (const [id, variant] of releaseVariants) {
    const location = `release_variants.${id}`;
    validateDocumentAnchor(
      policyDocumentText,
      variant.label,
      "release variant",
      location,
      errors,
    );
    validateReferenceArray(
      variant.required_claims,
      `${location}.required_claims`,
      claims,
      errors,
      { nonEmpty: true },
    );
    validateReferenceArray(
      variant.required_sources,
      `${location}.required_sources`,
      evidenceSources,
      errors,
      { nonEmpty: true },
    );
    validateReferenceArray(
      variant.conditional_sources,
      `${location}.conditional_sources`,
      evidenceSources,
      errors,
    );
    validateStringArray(variant.invariants, `${location}.invariants`, errors, {
      nonEmpty: true,
    });
    if (PUBLIC_LIVE_EXECUTION_VARIANTS.includes(id)) {
      if (!variant.required_claims.includes("live-execution-safety")) {
        addError(
          errors,
          `${location}.required_claims`,
          "public desktop variants ship live execution and must require live-execution-safety",
        );
      }
      for (const sourceId of REQUIRED_PUBLIC_LIVE_SAFETY_SOURCES) {
        if (!variant.required_sources.includes(sourceId)) {
          addError(
            errors,
            `${location}.required_sources`,
            `public desktop variants ship live execution and must require ${sourceId}`,
          );
        }
      }
    }
    if (PUBLIC_RELEASE_VARIANTS.includes(id)) {
      if (!variant.required_claims.includes("public-release-eligibility")) {
        addError(
          errors,
          `${location}.required_claims`,
          "public desktop variants must require public-release-eligibility for publication",
        );
      }
      for (const sourceId of REQUIRED_PUBLIC_RELEASE_SOURCES) {
        if (!variant.required_sources.includes(sourceId)) {
          addError(
            errors,
            `${location}.required_sources`,
            `public desktop variants must require ${sourceId} for publication`,
          );
        }
      }
    }
  }

  const gaps = validateSortedUniqueIds(matrix.gaps, "gaps", errors);
  validateRequiredIds(gaps, REQUIRED_IMPLEMENTED_GAPS, "gaps", errors);
  validateRequiredIds(gaps, REQUIRED_RESEARCH_SPIKE_GAPS, "gaps", errors);
  for (const [id, gap] of gaps) {
    const location = `gaps.${id}`;
    if (!ALLOWED_GAP_STATUSES.has(gap.status)) {
      addError(
        errors,
        `${location}.status`,
        `must be one of ${[...ALLOWED_GAP_STATUSES].join(", ")}`,
      );
    }
    if (
      REQUIRED_IMPLEMENTED_GAPS.includes(id) &&
      gap.status !== "implemented"
    ) {
      addError(errors, `${location}.status`, "must remain implemented");
    }
    if (
      REQUIRED_RESEARCH_SPIKE_GAPS.includes(id) &&
      gap.status !== "research_spike"
    ) {
      addError(errors, `${location}.status`, "must remain research_spike");
    }
    validateDocumentAnchor(
      policyDocumentText,
      gap.label,
      "gap register",
      location,
      errors,
    );
    if (typeof gap.reason !== "string" || gap.reason.trim().length === 0) {
      addError(errors, `${location}.reason`, "must be a non-empty string");
    }
    const implementationPaths = validateStringArray(
      gap.implemented_by,
      `${location}.implemented_by`,
      errors,
      { nonEmpty: gap.status === "implemented" },
    );
    if (gap.status !== "implemented" && implementationPaths.length > 0) {
      addError(
        errors,
        `${location}.implemented_by`,
        "must be empty unless status is implemented",
      );
    }
    for (let index = 0; index < implementationPaths.length; index += 1) {
      const implementationPath = implementationPaths[index];
      if (!isSafeRepoRelativePath(implementationPath)) {
        addError(
          errors,
          `${location}.implemented_by[${index}]`,
          "must be a safe repo-relative path",
        );
      } else {
        const resolved = resolveWithinRepo(repoRoot, implementationPath);
        if (!fs.existsSync(resolved) || !fs.statSync(resolved).isFile()) {
          addError(
            errors,
            `${location}.implemented_by[${index}]`,
            `file does not exist: ${implementationPath}`,
          );
        }
      }
    }
  }

  const contractWiring = validateSortedUniqueIds(
    matrix.contract_wiring,
    "contract_wiring",
    errors,
  );
  validateRequiredIds(
    contractWiring,
    [
      ...REQUIRED_CPU_RELIABILITY_WIRING,
      ...REQUIRED_PUBLIC_LIVE_SAFETY_WIRING,
      ...REQUIRED_PUBLIC_RELEASE_WIRING,
    ],
    "contract_wiring",
    errors,
  );
  for (const [id, wiring] of contractWiring) {
    const location = `contract_wiring.${id}`;
    if (!isSafeRepoRelativePath(wiring.path)) {
      addError(errors, `${location}.path`, "must be a safe repo-relative path");
      continue;
    }
    if (
      typeof wiring.must_include !== "string" ||
      wiring.must_include.length === 0
    ) {
      addError(
        errors,
        `${location}.must_include`,
        "must be a non-empty string",
      );
      continue;
    }
    const wiringPath = resolveWithinRepo(repoRoot, wiring.path);
    if (!fs.existsSync(wiringPath) || !fs.statSync(wiringPath).isFile()) {
      addError(
        errors,
        `${location}.path`,
        `file does not exist: ${wiring.path}`,
      );
      continue;
    }
    const wiringText = fs.readFileSync(wiringPath, "utf8");
    if (!wiringText.includes(wiring.must_include)) {
      addError(
        errors,
        `${location}.must_include`,
        `${wiring.path} does not contain ${wiring.must_include}`,
      );
    }
  }

  return [...new Set(errors)].sort((left, right) => left.localeCompare(right));
}

export function buildValidationReport({
  matrix,
  matrixBytes,
  matrixRelativePath,
  errors,
}) {
  return {
    schema_version: 1,
    report_kind: "release-authority-matrix-contract",
    matrix_id: typeof matrix?.matrix_id === "string" ? matrix.matrix_id : null,
    matrix_path: matrixRelativePath.replaceAll("\\", "/"),
    matrix_sha256: crypto
      .createHash("sha256")
      .update(matrixBytes)
      .digest("hex"),
    status: errors.length === 0 ? "passed" : "failed",
    counts: {
      authority_classes: Array.isArray(matrix?.authority_classes)
        ? matrix.authority_classes.length
        : 0,
      evidence_sources: Array.isArray(matrix?.evidence_sources)
        ? matrix.evidence_sources.length
        : 0,
      claims: Array.isArray(matrix?.claims) ? matrix.claims.length : 0,
      release_variants: Array.isArray(matrix?.release_variants)
        ? matrix.release_variants.length
        : 0,
      gaps: Array.isArray(matrix?.gaps) ? matrix.gaps.length : 0,
      contract_wiring: Array.isArray(matrix?.contract_wiring)
        ? matrix.contract_wiring.length
        : 0,
      errors: errors.length,
    },
    errors,
  };
}

export function validateMatrixFile({
  repoRoot,
  matrixPath = DEFAULT_MATRIX_PATH,
}) {
  if (!isSafeRepoRelativePath(matrixPath)) {
    throw new Error(`matrix path must be repo-relative: ${matrixPath}`);
  }
  const resolvedMatrixPath = resolveWithinRepo(repoRoot, matrixPath);
  const matrixBytes = fs.readFileSync(resolvedMatrixPath);
  const matrix = JSON.parse(matrixBytes.toString("utf8"));
  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath: matrixPath,
  });
  return {
    matrix,
    report: buildValidationReport({
      matrix,
      matrixBytes,
      matrixRelativePath: matrixPath,
      errors,
    }),
  };
}

function writeReport(outPath, report) {
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const repoRoot = resolveRepoRoot();
  const { report } = validateMatrixFile({
    repoRoot,
    matrixPath: args.matrixPath.replaceAll("\\", "/"),
  });
  if (args.outPath) {
    writeReport(path.resolve(repoRoot, args.outPath), report);
  }
  if (!args.quiet || report.status !== "passed") {
    console.log(
      `Release authority matrix contract: ${report.status} ` +
        `(${report.counts.claims} claims, ${report.counts.release_variants} variants, ` +
        `${report.counts.gaps} gaps)`,
    );
    for (const error of report.errors) {
      console.error(`- ${error}`);
    }
  }
  if (report.status !== "passed") process.exitCode = 1;
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(
      `Release authority matrix contract failed: ${error instanceof Error ? error.message : String(error)}`,
    );
    process.exitCode = 1;
  }
}
