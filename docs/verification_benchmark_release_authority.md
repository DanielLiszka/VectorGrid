# Verification, Benchmark Fidelity, Release, And Packaging Authority

## Purpose

This document defines which repo-owned evidence can approve which claim. It is an authority map for implementation planning, review, release rehearsal, and GPT-5.5 Pro recommendation promotion.

It does not change script behavior, baselines, release artifacts, CUDA kernels, Tauri configuration, packaging resources, frontend code, or CI state. When this document disagrees with a script, inspect the script before changing the claim.

## Scope

Use this document when a change or recommendation makes a claim about:

- correctness, parity, or artifact/report shape;
- workflow, CPU, GPU, options, ML Signal, or frontend performance;
- benchmark baseline refreshes;
- CUDA, PTX, native binary, or CPU-only release compatibility;
- release readiness, provenance, manifests, hashes, packaged smoke, or rollback;
- whether a GPT-5.5 Pro recommendation is ready to implement.

CPU-only builds must keep working without a CUDA toolchain, NVIDIA driver, or NVIDIA GPU. Any recommendation that violates that invariant is blocked until rescoped.

## Authority Taxonomy

| Authority class | Meaning | Examples |
|---|---|---|
| Broad authoritative gate | Current repo-labeled broad pass/fail gate. | `scripts/run_workflow_gate.ps1`, `scripts/run_cpu_perf_gate.ps1`, `scripts/run_gpu_perf_gate.ps1`. |
| Proposed subsystem authority | Claim-specific authority documented here for subsystem claims. | GPU validation/follow-up gates, options promotion gate, ML Signal quality/native smoke, execution safety, frontend E2E, output parity. |
| Release authority artifact | Evidence artifact that answers a release-specific question. | `readiness.json`, `provenance.json`, `manifest.json`, `SHA256SUMS.txt`, `packaged-smoke-summary.json`. |
| Companion diagnostic evidence | Helps explain mechanism but cannot approve a claim alone. | Nsight, CUPTI, app-path faceoff, campaign prefetch telemetry, memory telemetry. |
| Non-authority helper | Useful helper that must not be used as proof of unrelated claims. | `scripts/check_output_parity.mjs` for semantic claims, `scripts/export_baseline_samples.mjs`, migration freeze helper, disabled CI workflows. |

## Claim-To-Authority Ledger

| Claim type | Primary authority | Required companion evidence | Not authority for this claim |
|---|---|---|---|
| Broad repo quality | `powershell -File scripts/run_repo_quality_gate.ps1`; optional `powershell -File scripts/run_workspace_tests_stable.ps1` | Targeted Rust/web tests for touched surfaces. | Perf/profile scripts. |
| Warm workflow regression | `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` | Output parity if artifact/report shape changes. | Criterion-only benches. |
| CPU performance | `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` | Workflow gate when product workflow path changes. | GPU gates. |
| GPU/CUDA performance | `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` | Route diagnostics, CUDA tests, Nsight as companion. | ALMA contract rows alone. |
| GPU validation performance/residency | `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate` | Workflow gate; output parity if validation artifacts/reports change. | Generic GPU perf gate alone. |
| GPU follow-up/candidate-detail latency | `powershell -File scripts/run_gpu_followup_perf_gate.ps1 -Mode candidate` | Analysis command/frontend tests if UI path changes. | Generic GPU perf gate alone. |
| ALMA metric contract rows | `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` | Main GPU gate if general GPU behavior changes. | Default PR GPU perf authority. |
| Options-expression GPU promotion | `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate` | Phase 05, CPU/GPU ranking parity, options tests. | Options Criterion benches alone. |
| ML Signal quality/native CPU | `powershell -File scripts/run_ml_signal_quality_gate.ps1`; `powershell -File scripts/run_ml_signal_native_smoke.ps1` with required args | ML-specific artifact/provenance checks. | Normal optimizer perf gates. |
| Live execution safety | `powershell -File scripts/run_execution_safety_gate.ps1` | Mock broker/feed tests; frontend execution tests. | Broker success/profitability or perf benchmarks. |
| Frontend workflow behavior | `powershell -File scripts/run_frontend_e2e_gate.ps1` | Static QA checks, a11y, responsive/regression lanes as affected. | Backend perf gates. |
| Artifact/report shape | `node scripts/run_output_parity_check.mjs` | Semantic parity checks when semantic claims change. | Release-candidate gate alone; `scripts/check_output_parity.mjs` alone for semantics. |
| Release readiness | `powershell -File scripts/run_release_doctor.ps1 -Variant <variant>` | `readiness.json`. | Manifest/provenance alone. |
| Full local release rehearsal | `powershell -File scripts/run_release_candidate_gate.ps1 -Variant <variant>` | Readiness, provenance, manifest, hashes, smoke where configured. Add `-IncludeGpuValidationPerf` to `gpu-kernel` rehearsals for GPU validation/residency release claims. | Disabled CI workflows. |

## Benchmark Timing Boundaries

| Evidence | Boundary | Product relevance | Exclusions/limitations |
|---|---|---|---|
| `scripts/run_workflow_gate.ps1` / `src-tauri/src/bin/ta_workflow_regression.rs` | Warm deterministic backend workflow. | Best current workflow regression authority. | Does not prove full frontend click-to-render latency. |
| `src-tauri/src/bin/ta_app_path_faceoff.rs` | AppState plus blocking optimizer call. | Product-adjacent app-state path. | Not full Tauri command path; not full UI workflow. |
| `src-tauri/src/bin/ta_campaign_perf.rs` | Headless campaign runner. | Product-adjacent campaign/preload evidence. | Prefetch economics are heuristic telemetry. |
| Criterion benches | Bench-profile engine calls. | Good scoped regression control. | Not full app or UI latency unless boundary matches claim. |
| Frontend E2E | Browser workflow behavior under mocked Tauri. | User-flow behavior and regression coverage. | Does not prove backend latency/correctness. |
| Packaged smoke | Shell launch evidence. | Package launch/shell proof. | Raw release-binary smoke is not bundle-level packaging proof. |

A performance claim is valid only when the benchmark timing boundary includes the behavior being claimed.

## Baseline Policy

1. Candidate mode never updates baselines.
2. Baseline refresh requires explicit baseline mode or dedicated baseline-refresh tooling.
3. Baseline refresh requires rationale, hardware identity, build profile, timing boundary, schema version, case registry version, and changed workload identity.
4. Candidate comparisons must fail on schema, registry, timing-boundary, build-profile, route, or GPU identity incompatibility.
5. Near-threshold or noisy results require serialized rerun before any baseline update.
6. Baseline refresh is reviewed like a source-code change.
7. Never run multiple benchmarks or profiling sessions at the same time.

`scripts/benchmark_lock.ps1` provides the repo-local benchmark mutex used by the primary benchmark-running gates. It is a guardrail, not a replacement for checking outside benchmark/profiler processes; gates that also scan processes should keep those checks.

In this working tree, `scripts/baselines/` is git-ignored. Baselines generated there are local-machine evidence by default; promoting one to shared authority requires an explicit reviewed change, for example a force-add or an ignore-policy adjustment.

GPU validation perf has two workload profiles. The default `release-authority` profile preserves the current release evidence case registry and baseline contract. The `validation-heavy` profile is for focused CUDA validation-mode development and uses separate default baseline/output paths so validation-heavy experiments do not contaminate release-authority evidence.

GPU validation `-Quick` is a smoke lane, not a full-profile replacement. It compares selected cases against the relevant profile baseline, skips the structured probe, and uses separate default quick summary paths so quick runs do not overwrite full candidate evidence. `-PlanOnly` uses separate default plan paths for the same artifact-hygiene reason.

## Release Evidence Hierarchy

| Release question | Primary authority | Notes |
|---|---|---|
| Is the local machine/repo ready for this variant? | `scripts/run_release_doctor.ps1` and `readiness.json` | Doctor runs Tauri config sync and release-readiness core. It does not build, stage, hash, or smoke a package. |
| Was a candidate locally rehearsed? | `scripts/run_release_candidate_gate.ps1` and `summary.json` / `summary.md` | Summary is broad rehearsal evidence, not a replacement for specific artifacts. |
| What exactly was built? | `provenance.json` first, `provenance.txt` secondary | Includes build identity, toolchain, and environment evidence. |
| Which assets were staged? | `manifest.json` | Current required resources include CatBoost native DLL and `double_crossover.prebuilt.ptx`. |
| What are the asset hashes? | `SHA256SUMS.txt`, interpreted against `manifest.json` | Hash file alone is not variant authority. |
| Did the packaged artifact launch? | `packaged-smoke-summary.json` | Check `smoke_mode`; raw release-binary smoke is not bundle-level packaging proof. |
| Are artifact/report outputs still compatible? | `node scripts/run_output_parity_check.mjs` | Separate from release-candidate gate because output parity is skipped there. |

Current release-candidate caveats:

- `scripts/run_release_candidate_gate.ps1` runs `npm --prefix web run parity`.
- It prints `[SKIP] Output parity sample baselines are local-only in the backend-preview public tree.`
- It only runs packaged desktop smoke when `-IncludeDesktopSmoke` is set and `-SkipPackagedSmoke` is not set.
- It invokes generic GPU perf under `-IncludePerf` for non-CPU variants, but that is not a dedicated GPU MA-sweep authority.
- It invokes dedicated GPU validation perf only when `-IncludeGpuValidationPerf` is set on the `gpu-kernel` variant.
- It references `scripts/run_live_execution_safety_harness.ps1`; review that exact script before making internal live-harness authority claims.

## Release Variant Evidence Matrix

| Variant/surface | Required evidence | Notes |
|---|---|---|
| CPU-only | CPU build; release doctor; release candidate; provenance/manifest/hash. | Must not require CUDA Toolkit, NVCC, NVIDIA GPU, or driver. |
| GPU MA-sweep | `--features cuda`; release doctor/candidate for `gpu-ma`; NVIDIA driver/runtime evidence. | Current RC `-IncludePerf` uses generic GPU perf gate; dedicated MA-sweep authority is a gap. |
| GPU kernel | `--features cuda-backtest-kernel`; `VECTORBT_USE_PREBUILT_PTX=1` for packaged kernel path; manifest PTX evidence; GPU perf; GPU validation perf when validation/residency claims are made. | `cuda-backtest-kernel` implies `cuda`; runtime NVIDIA driver still required. |
| Options GPU scorer | Options promotion gate; CPU/GPU ranking parity; options tests. | Current manifest does not list `options_expression.prebuilt.ptx`; adding that is a later packaging-evidence gap, not current fact. |
| ML Signal native CPU | ML quality gate; native smoke; CatBoost DLL resource in manifest. | Manifest currently requires CatBoost native DLL unless explicitly allowed missing. |
| Live execution enabled | Execution safety gate; no live credentials; safety harness only after exact script review. | Public live release requires manual recovery and emergency-halt posture. |

## GPT-5.5 Pro Recommendation Promotion Checklist

A recommendation is not implementation-ready until it states:

- claim type and subsystem;
- primary authority gate or artifact;
- timing boundary, if performance-related;
- CPU oracle, CPU-only fallback, or explicit no-op/unsupported behavior;
- CUDA route and fallback transparency, if applicable;
- baseline-mode versus candidate-mode policy;
- artifact/report/provenance impact;
- affected release variants;
- companion evidence;
- helpers explicitly excluded as authority;
- kill criteria;
- rollback path;
- optional or missing files required before implementation.

## CPU-Only Fallback And No-Op Rules

| Situation | Required fallback/no-op statement |
|---|---|
| CUDA acceleration only | CPU-only build remains compiling and runnable; CUDA route either degrades to CPU or is explicitly unsupported. |
| GPU kernel route | `cuda-backtest-kernel` claim is separate from `cuda` MA-sweep claim; fallback must be visible in route metadata. |
| Options GPU scorer | CPU evaluator remains oracle; CPU/GPU ranking parity or exactification must be named. |
| ML native library | Missing native DLL must produce explicit unsupported/fallback behavior, not silent broken validation. |
| Live execution | Dry-run/paper safety remains available without live broker credentials; non-dry-run credential behavior stays explicit. |
| Release packaging | Variant label must match actual feature/resource/runtime requirements. |

## Gates And Helpers Not To Overclaim

- `scripts/check_output_parity.mjs` alone is structural parity, not semantic authority.
- `scripts/export_baseline_samples.mjs` is a sample/export helper, not parity authority.
- `scripts/check_migration_freeze_scope.mjs` is a narrow migration helper, not general quality authority.
- Nsight/CUPTI/profiling helpers explain mechanisms; they do not prove correctness, parity, packaging, or release readiness.
- `src-tauri/src/bin/ta_app_path_faceoff.rs` is app-path boundary evidence, not full Tauri/UI latency.
- `src-tauri/src/bin/ta_campaign_perf.rs` prefetch economics are heuristic telemetry unless separately promoted.
- Disabled `.github/workflows.disabled/*` workflows are not active authority until reviewed and reactivated.
- `scripts/run_release_candidate_gate.ps1` alone is not output-parity authority because current script skips output-parity sample baselines.

## Hotfix, Withdrawal, And Rollback Triggers

| Trigger | Required response |
|---|---|
| App fails to launch. | Hotfix or withdraw. |
| Wrong frontend bundle or stale command inventory shipped. | Hotfix or withdraw. |
| Variant label or GPU requirement is misleading. | Withdraw, relabel, or hotfix. |
| Published asset does not match manifest/hash. | Withdraw immediately. |
| CPU-only artifact requires CUDA/NVIDIA runtime. | Withdraw CPU artifact. |
| GPU-kernel artifact lacks required PTX/resource evidence. | Withdraw or hotfix GPU-kernel artifact. |
| Hidden CPU fallback under a GPU claim. | Hotfix route transparency or correct public claim. |
| Output artifact/report provenance breaks. | Hotfix if user decisions could be affected. |
| CatBoost native DLL missing under current manifest policy. | Hotfix package or explicitly disable affected native path. |
| Live execution safety failure or credential leakage. | Withdraw live-enabled build immediately. |

## Gap Register

| Gap | Status | Why blocked/deferred |
|---|---|---|
| Dedicated `gpu-ma` perf authority | Deferred | Current release `-IncludePerf` uses generic GPU perf for non-CPU variants. |
| Options PTX in manifest required resources | Deferred | Current manifest tracks double-crossover PTX only; CUDA source/PTX package change needs exact review. |
| Release-candidate output parity integration | Deferred | Current script intentionally skips output parity; adding it changes release workflow behavior. |
| Machine-readable authority matrix | Deferred | Markdown policy should stabilize before adding JSON/schema/tooling. |
| Live safety harness authority | Blocked | `scripts/run_live_execution_safety_harness.ps1` must be reviewed before internal behavior claims. |
| Disabled CI reactivation | Blocked | `.github/workflows.disabled/ci.yml.disabled` and `.github/workflows.disabled/desktop-smoke.yml.disabled` must be reviewed before authority claims. |
| CUDA Graphs timing boundary | Research spike | Needs graph capture, instantiate, update, replay instrumentation. |
| Full frontend click-to-render latency | Research spike | Existing frontend E2E proves behavior, not the full latency chain. |

## Source Anchors

Use these files as authority anchors before changing this document:

- `docs/REPO_MAP.md`
- `docs/RELEASE.md`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_release_doctor.ps1`
- `scripts/generate_release_manifest.ps1`
- `scripts/run_packaged_smoke.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_output_parity_check.mjs`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_frontend_e2e_gate.ps1`
