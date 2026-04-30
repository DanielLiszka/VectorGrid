# Verification, CI, Benchmarking, And QA Implementation Plan

## Surface

Primary files:

- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_workspace_tests_stable.ps1`
- `scripts/run_frontend_e2e_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_public_release_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `web/package.json` scripts: `check:qa-static`, `test:e2e:*`, `a11y:smoke`, and `perf:ui:budget`
- `web/e2e/*`
- `.github/workflows.disabled/*` currently; `.github/workflows/*` only after CI is intentionally re-enabled

## Current Strengths

- The repo has broad local gate scripts.
- Stable workspace tests exist for Windows.
- CPU and GPU performance gates are separated.
- Frontend static QA and E2E gates already exist.

## Improvement Plan

### QA-01: Simple Mode Public Workflow Gate

Risk: Low.

Problem:
Simple Mode is the main public-facing workflow and should have a dedicated, clean, deterministic gate.

Implementation:

- Add a Simple Mode E2E gate that runs:
  - clean state
  - bundled sample load
  - optimization
  - optional validation
  - artifact visibility
  - no console errors
- Include the result in public release summaries.

Files:

- `web/e2e/browser/qa/workflowMatrix.spec.ts` preferred, or `web/e2e/browser/qa/simpleModeHappyPath.spec.ts` (new with explicit gate wiring)
- `scripts/run_frontend_e2e_gate.ps1`
- `scripts/run_public_release_gate.ps1`

Verification:

- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### QA-02: Action Coverage Waiver Burn-Down

Risk: Low.

Problem:
Action coverage waivers are acceptable only when they are explicit and shrinking.

Implementation:

- Add a waiver ledger with:
  - action location
  - reason
  - owner
  - expiration or planned removal
- Fail on new unowned waivers.
- Prioritize Simple Mode and public release flows first.

Files:

- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/scripts/check-action-coverage-manifest.ts`
- `web/scripts/check-qa-selector-inventory.mjs`

Verification:

- `npm --prefix web run check:qa-static`
- `npm --prefix web run test:e2e:action-coverage`

### QA-03: Desktop Security Boundary Gate

Risk: Low/medium.

Problem:
Release gates should explicitly cover desktop boundary behavior.

Implementation:

- Add a test lane for:
  - path allow/deny
  - export redaction
  - credential redaction
  - open/reveal policy
  - market-data root restrictions
- Include results in public release summary.

Files:

- `src-tauri/src/local_path_policy.rs`
- `src-tauri/src/redaction.rs`
- `src-tauri/src/commands_reports_api.rs`
- `scripts/run_public_release_gate.ps1`

Verification:

- `cargo test -p ta_desktop_demo_app --lib local_path_policy`
- `cargo test -p ta_desktop_demo_app --lib redaction`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### QA-04: Output Parity Release Gate

Risk: Low.

Problem:
Output parity should be promoted from optional confidence to public release evidence.

Implementation:

- Require output parity for public release or require an explicit documented waiver.
- Store parity output path in release manifest.
- Add failure summaries that name the changed output fields.

Files:

- `scripts/run_output_parity_check.mjs`
- `scripts/run_public_release_gate.ps1`
- `scripts/generate_release_manifest.ps1`

Verification:

- `node scripts/run_output_parity_check.mjs`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### QA-05: ML And Stage 5 Quality Gate

Risk: Low.

Problem:
ML confidence and ML Signal have enough evidence semantics that they deserve explicit release gate status.

Implementation:

- Add ML/Stage 5 quality gate output for:
  - lineage tests
  - stale/future model tests
  - strict health tests
  - ML Signal strict cache tests
  - policy sweep tests
- Include status in release candidate summary.

Files:

- `scripts/run_ml_signal_quality_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_public_release_gate.ps1`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`

Verification:

- `powershell -File scripts/run_ml_signal_quality_gate.ps1`
- `cargo test -p ta_app_core --lib ml_confidence`
- `cargo test -p ta_app_core --lib ml_signal`

### QA-06: GPU Performance And Residency Evidence

Risk: Low/medium.

Problem:
GPU claims should be backed by route, residency, and benchmark evidence.

Implementation:

- Add one normalized GPU evidence output for optimization and validation.
- Include CPU fallback and CPU-tail times.
- Serialize all benchmarks and check for existing benchmark processes before starting.
- Keep using `scripts/benchmark_lock.ps1` in gates that already support it; do not run benchmark/profiling lanes concurrently.
- Include GPU evidence in release candidate summaries only when GPU variant is requested.

Files:

- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`

Verification:

- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`

### QA-07: CI Workflow Reactivation Plan

Risk: Medium.

Problem:
Archived or stale CI is weaker than the current local gate scripts.

Implementation:

- Decide which gates must run in CI:
  - Rust stable workspace tests
  - frontend unit/typecheck
  - frontend static QA
  - no CUDA required lane
  - release doc/link checks
- Keep GPU benchmarks local or self-hosted only.
- Add CI documentation stating local gates remain authoritative for GPU and release signing.

Files:

- `.github/workflows.disabled/*` as the source to reactivate, then `.github/workflows/*` after re-enablement
- `docs/RELEASE.md`
- `scripts/run_repo_quality_gate.ps1`

Verification:

- `gh workflow run` or PR check run only after workflows have been moved back into the active workflows directory.
- Local `powershell -File scripts/run_repo_quality_gate.ps1`.

### QA-08: Benchmark Baseline Bootstrap

Risk: Low.

Problem:
Ignored local baselines are useful but can confuse release preparation.

Implementation:

- Add a baseline bootstrap command.
- Store baseline metadata:
  - CPU/GPU model
  - OS
  - date
  - git commit
  - feature flags
  - benchmark version
- Fail candidate performance gate with a clear "baseline missing" message and generation instructions.

Files:

- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `docs/RELEASE.md`

Verification:

- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

### QA-09: Generated Binding Coverage Gate

Risk: Low/medium.

Problem:
Generated Rust/TypeScript bindings reduce Tauri IPC drift, but a small pilot subset can create false confidence if unbound commands are not classified.

Implementation:

- Add a binding coverage report for Tauri commands:
  - generated and checked
  - intentionally manual
  - blocked by unsupported type
  - high-risk and missing
- Fail release mode for high-risk missing commands unless waived.
- Keep `generatedBindings.ts` current through the existing generation/check flow.

Files:

- `src-tauri/src/command_bindings.rs`
- `src-tauri/src/commands_*.rs`
- `src-tauri/tests/specta_bindings_export.rs`
- `web/src/tauri/generatedBindings.ts`
- `web/package.json`

Verification:

- `cargo test -p ta_desktop_demo_app --test specta_bindings_export`
- `npm --prefix web run check:bindings`
- `npm --prefix web run gen:bindings` is the canonical generation command; the raw ignored Cargo export test is only the underlying implementation.

## Acceptance Criteria For This Guide

- QA plans reference existing scripts in `web/package.json` or `scripts/`; new scripts are labeled as new before implementation.
- Simple Mode has a deterministic public workflow gate.
- Frontend static QA uses `npm --prefix web run check:qa-static`.
- Benchmark gates remain serialized through existing benchmark-lock policy.
- CI reactivation starts with CPU-only deterministic lanes and leaves GPU/signing evidence to local or self-hosted release machines.

## Deferred Higher-Risk Work

- Mandatory GPU CI on public runners.
- Running benchmarks concurrently.
- Treating browser visual tests as a replacement for unit and backend tests.
