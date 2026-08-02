import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  buildValidationReport,
  validateAuthorityMatrix,
  validateMatrixFile,
} from "./check_release_authority_matrix.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const matrixRelativePath = "docs/release_authority_matrix.json";

function readCanonicalMatrix() {
  return JSON.parse(
    fs.readFileSync(path.join(repoRoot, matrixRelativePath), "utf8"),
  );
}

test("canonical release authority matrix passes its contract", () => {
  const { report } = validateMatrixFile({
    repoRoot,
    matrixPath: matrixRelativePath,
  });
  assert.equal(report.status, "passed");
  assert.deepEqual(report.errors, []);
});

test("validation report is deterministic for identical matrix bytes", () => {
  const matrix = readCanonicalMatrix();
  const matrixBytes = fs.readFileSync(path.join(repoRoot, matrixRelativePath));
  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  const first = buildValidationReport({
    matrix,
    matrixBytes,
    matrixRelativePath,
    errors,
  });
  const second = buildValidationReport({
    matrix,
    matrixBytes,
    matrixRelativePath,
    errors,
  });
  assert.deepEqual(second, first);
  assert.equal(JSON.stringify(second), JSON.stringify(first));
});

test("duplicate evidence identifiers fail closed", () => {
  const matrix = readCanonicalMatrix();
  matrix.evidence_sources.splice(
    1,
    0,
    structuredClone(matrix.evidence_sources[0]),
  );
  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  assert.ok(
    errors.some((error) =>
      error.includes("duplicate id alma_contract_rows_gate"),
    ),
  );
});

test("a diagnostic or helper cannot become primary claim authority", () => {
  const matrix = readCanonicalMatrix();
  const claim = matrix.claims.find(
    (entry) => entry.id === "artifact-report-shape",
  );
  claim.primary_authority = ["output_structural_helper"];
  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  assert.ok(
    errors.some(
      (error) =>
        error.includes("claims.artifact-report-shape.primary_authority") &&
        error.includes("forbidden primary class non_authority_helper"),
    ),
  );
});

test("missing command paths fail closed", () => {
  const matrix = readCanonicalMatrix();
  const source = matrix.evidence_sources.find(
    (entry) => entry.id === "repo_quality_gate",
  );
  source.path = "scripts/does-not-exist.ps1";
  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  assert.ok(
    errors.some(
      (error) =>
        error.includes("evidence_sources.repo_quality_gate.path") &&
        error.includes("file does not exist"),
    ),
  );
});

test("public desktop variants cannot omit public-release authority", () => {
  const matrix = readCanonicalMatrix();
  for (const variantId of ["cpu", "gpu-kernel", "gpu-ma"]) {
    const missingClaim = structuredClone(matrix);
    const claimVariant = missingClaim.release_variants.find(
      (entry) => entry.id === variantId,
    );
    claimVariant.required_claims = claimVariant.required_claims.filter(
      (id) => id !== "public-release-eligibility",
    );
    const claimErrors = validateAuthorityMatrix({
      matrix: missingClaim,
      repoRoot,
      matrixRelativePath,
    });
    assert.ok(
      claimErrors.some(
        (error) =>
          error.includes(
            `release_variants.${variantId}.required_claims`,
          ) && error.includes("must require public-release-eligibility"),
      ),
      `expected ${variantId} to require the public-release claim`,
    );

    const missingSource = structuredClone(matrix);
    const sourceVariant = missingSource.release_variants.find(
      (entry) => entry.id === variantId,
    );
    sourceVariant.required_sources = sourceVariant.required_sources.filter(
      (id) => id !== "public_release_gate",
    );
    const sourceErrors = validateAuthorityMatrix({
      matrix: missingSource,
      repoRoot,
      matrixRelativePath,
    });
    assert.ok(
      sourceErrors.some(
        (error) =>
          error.includes(
            `release_variants.${variantId}.required_sources`,
          ) && error.includes("must require public_release_gate"),
      ),
      `expected ${variantId} to require the public-release gate`,
    );
  }
});

test("CPU reliability authority and short-mode boundary cannot disappear", () => {
  const matrix = readCanonicalMatrix();
  matrix.claims = matrix.claims.filter(
    (entry) => entry.id !== "cpu-reliability-operator-authority",
  );
  matrix.contract_wiring = matrix.contract_wiring.filter(
    (entry) => entry.id !== "cpu-reliability-short-fail-closed",
  );
  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  assert.ok(
    errors.some(
      (error) =>
        error.includes("claims") &&
        error.includes(
          "missing required id cpu-reliability-operator-authority",
        ),
    ),
  );
  assert.ok(
    errors.some(
      (error) =>
        error.includes("contract_wiring") &&
        error.includes(
          "missing required id cpu-reliability-short-fail-closed",
        ),
    ),
  );
});

test("CPU reliability claim and source cannot be semantically rewired together", () => {
  const matrix = readCanonicalMatrix();
  const source = matrix.evidence_sources.find(
    (entry) => entry.id === "cpu_reliability_soak_gate",
  );
  source.authority_class = "broad_authoritative_gate";
  source.path = "scripts/run_workflow_gate.ps1";
  source.invocation =
    "powershell -File scripts/run_workflow_gate.ps1 -Mode candidate";
  const artifact = matrix.evidence_sources.find(
    (entry) => entry.id === "cpu_reliability_soak_artifact",
  );
  artifact.producer = "workflow_gate";
  const claim = matrix.claims.find(
    (entry) => entry.id === "cpu-reliability-operator-authority",
  );
  claim.primary_authority = ["workflow_gate"];
  claim.companion_sources = ["readiness_artifact"];
  claim.required_companion_evidence = ["A workflow command passed."];

  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  for (const expected of [
    "pinned_authority_sources.cpu_reliability_soak_gate.authority_class",
    "pinned_authority_sources.cpu_reliability_soak_gate.path",
    "pinned_authority_sources.cpu_reliability_soak_artifact.producer",
    "pinned_claims.cpu-reliability-operator-authority.primary_authority",
    "pinned_claims.cpu-reliability-operator-authority.companion_sources",
    "pinned_claims.cpu-reliability-operator-authority.required_companion_evidence",
  ]) {
    assert.ok(
      errors.some((error) => error.includes(expected)),
      `expected pinned CPU reliability error for ${expected}`,
    );
  }
});

test("public-release claim, artifacts, and gate cannot be semantically rewired together", () => {
  const matrix = readCanonicalMatrix();
  const source = matrix.evidence_sources.find(
    (entry) => entry.id === "public_release_gate",
  );
  source.authority_class = "subsystem_authority";
  source.path = "scripts/run_release_candidate_gate.ps1";
  source.invocation =
    "powershell -File scripts/run_release_candidate_gate.ps1 -Variant <variant>";
  for (const artifactId of [
    "public_release_summary_artifact",
    "release_proof_evidence_manifest_artifact",
  ]) {
    const artifact = matrix.evidence_sources.find(
      (entry) => entry.id === artifactId,
    );
    artifact.producer = "release_candidate_gate";
  }
  const claim = matrix.claims.find(
    (entry) => entry.id === "public-release-eligibility",
  );
  claim.primary_authority = ["release_candidate_gate"];
  claim.companion_sources = ["manifest_artifact"];
  claim.required_companion_evidence = ["A candidate build completed."];

  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  for (const expected of [
    "pinned_authority_sources.public_release_gate.authority_class",
    "pinned_authority_sources.public_release_gate.path",
    "pinned_authority_sources.public_release_summary_artifact.producer",
    "pinned_authority_sources.release_proof_evidence_manifest_artifact.producer",
    "pinned_claims.public-release-eligibility.primary_authority",
    "pinned_claims.public-release-eligibility.companion_sources",
    "pinned_claims.public-release-eligibility.required_companion_evidence",
  ]) {
    assert.ok(
      errors.some((error) => error.includes(expected)),
      `expected pinned public-release error for ${expected}`,
    );
  }
});

test("locally implemented GPU release-authority gaps cannot regress to deferred", () => {
  const matrix = readCanonicalMatrix();
  for (const gapId of [
    "cuda-graphs-timing-boundary",
    "dedicated-gpu-ma-perf-authority",
    "options-gpu-scorer-release-promotion-authority",
  ]) {
    const mutated = structuredClone(matrix);
    const gap = mutated.gaps.find((entry) => entry.id === gapId);
    gap.status = "deferred";
    gap.implemented_by = [];
    const errors = validateAuthorityMatrix({
      matrix: mutated,
      repoRoot,
      matrixRelativePath,
    });
    assert.ok(
      errors.some(
        (error) =>
          error.includes(`gaps.${gapId}.status`) &&
          error.includes("must remain implemented"),
      ),
      `expected ${gapId} to be protected as an implemented gap`,
    );
  }
});

test("full frontend click-to-render work cannot be overstated as implemented", () => {
  const matrix = readCanonicalMatrix();
  const gap = matrix.gaps.find(
    (entry) => entry.id === "full-frontend-click-to-render-latency",
  );
  gap.status = "implemented";
  gap.implemented_by = ["scripts/run_frontend_interaction_latency.ps1"];
  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath,
  });
  assert.ok(
    errors.some(
      (error) =>
        error.includes("gaps.full-frontend-click-to-render-latency.status") &&
        error.includes("must remain research_spike"),
    ),
  );
});
