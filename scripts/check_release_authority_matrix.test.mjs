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
