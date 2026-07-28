# Verification, Benchmark Fidelity, Release, And Packaging Authority

## Purpose

This document defines which repo-owned evidence can approve which claim. It is an authority map for implementation planning, review, release rehearsal, and GPT-5.5 Pro recommendation promotion.

`docs/release_authority_matrix.json` is the canonical machine-readable policy index for the authority classes, evidence sources, claim mappings, release variants, and open gaps described here. `node scripts/check_release_authority_matrix.mjs` validates its schema, references, repo paths, human-document anchors, and quality/release/CI wiring without executing any benchmark or release workload. The JSON records declared authority; it does not prove that an underlying gate passed.

This document and matrix do not change benchmark behavior, baselines, release artifacts, CUDA kernels, Tauri configuration, packaging resources, or frontend code. When either policy surface disagrees with an executable script, inspect the script before changing the claim, then update both policy surfaces together.

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
| Broad authoritative gate | Current repo-labeled broad pass/fail gate. | `scripts/run_workflow_gate.ps1`, `scripts/run_cpu_perf_gate.ps1`, `scripts/run_gpu_perf_gate.ps1`, `scripts/run_gpu_ma_perf_gate.ps1`. |
| Required CPU CI mirror | Merge enforcement that delegates to repo-owned CPU/static/headless gates without becoming package or public-release authority. | `.github/workflows/cpu-quality.yml` / `Required CPU quality`, via `scripts/run_cpu_ci_gate.ps1`. |
| Proposed subsystem authority | Claim-specific authority documented here for subsystem claims. | GPU validation/follow-up gates, options promotion gate, ML Signal quality/native smoke, execution safety, frontend E2E, output parity. |
| Release authority artifact | Evidence artifact that answers a release-specific question. | `readiness.json`, `provenance.json`, `manifest.json`, `SHA256SUMS.txt`, `packaged-smoke-summary.json`. |
| Companion diagnostic evidence | Helps explain mechanism but cannot approve a claim alone. | Nsight, CUPTI, app-path faceoff, campaign prefetch telemetry, memory telemetry. |
| Non-authority helper | Useful helper that must not be used as proof of unrelated claims. | `scripts/check_output_parity.mjs` for semantic claims, `scripts/export_baseline_samples.mjs`, migration freeze helper, archived workflows under `.github/workflows.disabled/`. |

## Claim-To-Authority Ledger

| Claim type | Primary authority | Required companion evidence | Not authority for this claim |
|---|---|---|---|
| Broad repo quality | `powershell -File scripts/run_repo_quality_gate.ps1`; optional `powershell -File scripts/run_workspace_tests_stable.ps1`; required CI mirror `Required CPU quality` | Targeted Rust/web tests for touched surfaces. | Perf/profile scripts. |
| Warm workflow regression | `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` | Output parity if artifact/report shape changes. | Criterion-only benches. |
| CPU performance | `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` | Workflow gate when product workflow path changes. | GPU gates. |
| GPU kernel/CUDA performance | `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate -WorkloadProfile gpu-kernel` | Route diagnostics, CUDA tests, Nsight as companion. | GPU MA-sweep benchmarks or ALMA contract rows alone. |
| GPU MA-sweep performance | `powershell -File scripts/run_gpu_ma_perf_gate.ps1 -Mode candidate` | CUDA tests and route diagnostics for the `cuda`-only `GpuOnly` path. | The GPU-kernel case registry or baseline. |
| GPU validation performance/residency | `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate` | Workflow gate; output parity if validation artifacts/reports change. | Generic GPU perf gate alone. |
| GPU follow-up/candidate-detail latency | `powershell -File scripts/run_gpu_followup_perf_gate.ps1 -Mode candidate` | Analysis command/frontend tests if UI path changes. | Generic GPU perf gate alone. |
| ALMA metric contract rows | `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` | Main GPU gate if general GPU behavior changes. | Default PR GPU perf authority. |
| Options-expression GPU promotion | `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate` | The gate now runs the CPU options test suite, exact aggregate and ranking CPU/GPU parity tests, Phase 05 backend comparison, promotion checks, and candidate-baseline checks as one fail-closed authority. | Options Criterion benches or packaged PTX alone. |
| Support-surface performance | `powershell -File scripts/run_support_perf_gate.ps1 -Mode candidate` | `scripts/run_performance_audit_baseline_pack.ps1 -Mode candidate -IncludeSupport`; output parity when report/artifact shape changes. | Credentialed provider stream start or live broker/API behavior. |
| Campaign orchestration performance | `powershell -File scripts/run_campaign_perf_gate.ps1 -Mode candidate` | Workflow gate when campaign behavior changes; campaign JSON summary. | Strategy-kernel-only benchmarks. |
| Repo-wide performance authority inventory | `powershell -File scripts/run_performance_audit_baseline_pack.ps1 -FailOnCoverageGap -FailOnActivePerfProcess -FailOnMissingReleaseBaseline` | Candidate pack runs for changed surfaces. | Baseline refresh, CUDA residency proof, credentialed live/provider behavior. |
| ML Signal quality/native CPU | `powershell -File scripts/run_ml_signal_quality_gate.ps1`; `powershell -File scripts/run_ml_signal_native_smoke.ps1` with required args | ML-specific artifact/provenance checks. | Normal optimizer perf gates. |
| Live execution safety | `powershell -File scripts/run_execution_safety_gate.ps1`; `powershell -File scripts/run_live_execution_safety_harness.ps1` | Mock broker/feed tests; frontend execution tests; a harness summary with `passed=true` and `authority_eligible=true`. | Broker connectivity, broker success/profitability, manual recovery drills, or perf benchmarks. |
| Scoped warm mocked-browser frontend latency | `powershell -File scripts/run_frontend_interaction_latency.ps1` | Exact source/dist build attestation; a fresh no-hardlink sealed distribution whose exact buffers are preloaded and served from memory; a wrapper-held lease independently bound to the live named-mutex owner, ancestry/start identities, token, and context; hashed measurement-harness and scoped Git-status identities; strict path-distinct asynchronous detail mocks; trusted-click and retained per-sample state evidence; browser/effective-fixture identity; and a unique no-replace `output/playwright/frontend-interaction-latency/<run>/summary.json`. A byte-bound dirty source snapshot may produce local candidate authority, but it is labeled `dirty-source-local-candidate-rehearsal` and has `release_evidence_eligible=false`. | Full application or native-shell latency, backend/provider latency, cold startup, cross-device behavior, ordinary frontend E2E pass/fail, or dirty-worktree publication authority. |
| Frontend workflow behavior | `powershell -File scripts/run_frontend_e2e_gate.ps1` | Static QA checks, a11y, responsive/regression lanes as affected. | Backend perf gates. |
| Artifact/report shape | `node scripts/run_output_parity_check.mjs` | Semantic parity checks when semantic claims change. | Release-candidate gate alone; `scripts/check_output_parity.mjs` alone for semantics. |
| Release readiness | `powershell -File scripts/run_release_doctor.ps1 -Variant <variant>` | `readiness.json`. | Manifest/provenance alone. |
| Full local release rehearsal | `powershell -File scripts/run_release_candidate_gate.ps1 -Variant <variant>` | Readiness, provenance, manifest, hashes, smoke where configured. Add `-IncludeGpuValidationPerf` to `gpu-kernel` rehearsals for GPU validation/residency claims. Add `-IncludeOptionsGpuPromotion` plus explicit artifact and dataset/import identity for options GPU claims. | Required CPU CI alone or archived workflows. |

## Benchmark Timing Boundaries

| Evidence | Boundary | Product relevance | Exclusions/limitations |
|---|---|---|---|
| `scripts/run_workflow_gate.ps1` / `src-tauri/src/bin/ta_workflow_regression.rs` | Warm deterministic backend workflow. | Best current workflow regression authority. | Does not prove full frontend click-to-render latency. |
| `src-tauri/src/bin/ta_app_path_faceoff.rs` | AppState plus blocking optimizer call. | Product-adjacent app-state path. | Not full Tauri command path; not full UI workflow. |
| `src-tauri/src/bin/ta_campaign_perf.rs` | Headless campaign runner. | Product-adjacent campaign/preload evidence. | Prefetch economics are heuristic telemetry. |
| `scripts/run_support_perf_gate.ps1` / `src-tauri/src/bin/ta_perf_authority_support.rs` | Release-mode support-surface command timing. | Non-credential timing authority for data/cache-adjacent, options-data, reports, artifacts, portfolio, dynamic/status, and local-paper execution scaffolding. | Provider stream start, credentialed broker/API behavior, or core optimizer/validator throughput. |
| `scripts/run_campaign_perf_gate.ps1` / `src-tauri/src/bin/ta_campaign_perf.rs` | Deterministic two-dataset synthetic campaign run. | Campaign orchestration candidate regression authority. | Strategy-kernel-only claims or broad UI latency. |
| `scripts/run_performance_audit_baseline_pack.ps1` | Inventory and optional serialized gate runner. | Coverage proof, authority manifest, and bundled changed-surface candidate evidence. | A replacement for subsystem-specific perf gates or live/provider manual checks. |
| `scripts/run_cuda_graph_timing_probe.ps1` / `profile_gpu_dynamic_runner` | Opt-in monotonic host timing split into stream capture plus node recording, `cuGraphInstantiate_v2`, explicit immutable-topology update non-applicability, and asynchronous `cuGraphLaunch` submission. | Mechanism-level evidence for CUDA Graph construction/reuse overhead. | Replay device completion, kernel duration, correctness, parity, or broad GPU performance authority. |
| `scripts/run_cuda_graph_multitile_parity.ps1` / `profile_gpu_dynamic_runner` | One deterministic constrained-VRAM large Double MA run with graphs off and on. | Specialized correctness regression for immutable CUDA Graph tile-pointer capture: repeated coarse-identical full-tile proof, exact winner/integer parity, tolerance-bounded continuous metrics, and successful pointer-aware capture/instantiate/replay counts. | Broad GPU correctness, GPU performance, other strategies or MA pairs, device-duration timing, or release readiness. |
| Criterion benches | Bench-profile engine calls. | Good scoped regression control. | Not full app or UI latency unless boundary matches claim. |
| Frontend E2E | Browser workflow behavior under mocked Tauri. | User-flow behavior and regression coverage. | Does not prove backend latency/correctness. |
| `scripts/run_frontend_interaction_latency.ps1` / `web/scripts/measure-frontend-interaction-latency.mjs` | Warm trusted headless-Chromium click event over an attested per-run distribution preloaded into a private in-memory map, through expected mocked-browser state, argument-matched asynchronous saved-detail hydration where applicable, 64 ms DOM quiet, two animation-frame callbacks, and the task after the intervening paint. | Scoped built-frontend authority for representative workspace navigation and saved optimization-result detail under the fixed mocked-Tauri profile. | Full application/workflow latency, Playwright actionability wait before the click event, cold startup, native WebView2/Tauri overhead, real backend/provider latency, and production-device diversity. |
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
8. Candidate output paths must not point at the active baseline path.

`scripts/benchmark_lock.ps1` uses the fixed `output/qa/benchmark-lock/active.json` file lease as the primary workspace-wide serialization primitive. Its live owner keeps a read-share/write-exclusive handle open, so the lease works across Windows sessions that can access the workspace; the historical `Local\ta_desktop_demo_benchmark_gate` mutex remains secondary same-session compatibility evidence. Inherited gates fail closed unless the fixed lease has exact stable contents and file identity, a write probe returns Windows sharing violation 32, the claimed PID/start identity is a current ancestor, and a bounded Windows Restart Manager query identifies that same process as the actual file holder. ACL denial, stale content, split-holder spoofing, hardlinks, reparse aliases, PID reuse, and released handles are not authority. The stale ordinary file intentionally remains after release and is safely rewritten only by the next exclusive owner. This guardrail does not replace outside-process checks; gates that scan processes should keep them. The frontend interaction lane adds its run-specific wrapper lease and independently revalidates both leases at child and final-publication boundaries. If a prior process crashes while holding the secondary mutex, the next gate may recover it and prints a warning.

Frontend latency authority separates reproducible local candidate evidence from release publication eligibility. A dirty scoped worktree is not itself a measurement-integrity failure: the production build inputs, distribution bytes, measurement harness, policy, fixture, raw samples, and report are independently hashed and revalidated, while canonical NUL-delimited Git status is also recorded and checked. Such a passing run keeps `authority_eligible=true` for local comparison but is explicitly classified as `dirty-source-local-candidate-rehearsal` with `release_evidence_eligible=false`. Only a clean scoped source state can produce `clean-source-release-evidence-candidate`; that flag is still only an input to release review, never standalone publication approval. `scripts/run_public_release_gate.ps1` remains the final boundary and already makes `-AllowDirtyWorktree -AllowUnsignedBeta` runs rehearsal-only with `public_release_eligible=false`.

In this working tree, `scripts/baselines/` is git-ignored. Baselines generated there are local-machine evidence by default; promoting one to shared authority requires an explicit reviewed change, for example a force-add or an ignore-policy adjustment.

Strict inventory treats every registry case marked `release_required_baseline` as fail-closed. It writes `release_baseline_health.json`, reports every absent path, and never creates or copies a timing file. The bounded active-process snapshot uses native process enumeration in a disposable helper job; a timeout or provider failure aborts the inventory instead of assuming the machine is idle.

GPU optimizer performance has separate workload profiles and baseline namespaces. `scripts/run_gpu_perf_gate.ps1` defaults to the existing `gpu-kernel` registry and preserves `scripts/baselines/gpu_perf_baseline.json`. `scripts/run_gpu_ma_perf_gate.ps1` selects the `cuda`-only `gpu-ma` registry and uses `scripts/baselines/gpu_ma_perf_baseline.json` plus `output/benchmarks/gpu_ma_perf_summary.json`. Both the desktop release-candidate gate and backend release gate select that dedicated script for `gpu-ma`. Candidate mode checks that its profile-specific baseline exists before compiling or running a benchmark. The historical kernel baseline cannot be migrated safely: its case IDs are `gpu_sma_small` / `gpu_alma_large`, its filters are `double_ma/gpu_kernel/*`, and the benchmark requests `Backend::GpuKernel`; the new authority uses `gpu_ma_*`, `double_ma/gpu_sweep/*`, `Backend::GpuOnly`, and a different CUDA feature contract. Reusing those timings would relabel a different execution boundary as evidence. This wiring change therefore deliberately does not synthesize a `gpu-ma` baseline; a maintainer must establish and review it with an explicit serialized `-Mode baseline` run before `gpu-ma -IncludePerf` can pass.

GPU validation perf has two workload profiles. The default `release-authority` profile preserves the current release evidence case registry and baseline contract. The `validation-heavy` profile is for focused CUDA validation-mode development and uses separate default baseline/output paths so validation-heavy experiments do not contaminate release-authority evidence.

GPU validation `-Quick` is a smoke lane, not a full-profile replacement. It compares selected cases against the relevant profile baseline, skips the structured probe, and uses separate default quick summary paths so quick runs do not overwrite full candidate evidence. `-PlanOnly` uses separate default plan paths for the same artifact-hygiene reason. Process discovery is a bounded fail-closed Win32 snapshot, and every Cargo/benchmark/probe child has a hard runtime limit. On timeout, cleanup is restricted to the launched root plus descendants whose parent chain and process-start identity were captured from that root; the gate never issues a system-wide process termination command.

CUDA Graph timing is opt-in through `VECTORBT_KERNEL_CUDA_GRAPH_TIMING=1` (or the runner's `--cuda-graph-timing on`) and is inert by default. `scripts/run_cuda_graph_timing_probe.ps1` forces the compatible reduced-output graph route, holds the repository benchmark mutex, verifies that capture, instantiation, and replay all succeeded, and writes `output/benchmarks/cuda_graph_timing_summary.json`. Before launching work it replaces only a bounded immediate-child JSON destination under `output/benchmarks` (or requires a fresh destination elsewhere) with a non-passing `running` marker, so a failed rerun cannot leave stale `passed` evidence at the requested path. The implementation does not call `cuGraphExecUpdate`: tile graphs have an immutable captured topology, and their reuse signature includes every tile-specific valid-pair-map device pointer captured into the kernel nodes; a pointer change forces recapture. Update is therefore machine-readably declared not applicable with zero attempts and zero elapsed time. Replay timing is the host duration of `cuGraphLaunch`, not device completion; use serialized Nsight/CUDA-event work if a future claim needs device execution duration.

`scripts/run_cuda_graph_multitile_parity.ps1` is the focused correctness companion to that timing probe. It builds the same release runner once, holds the repository benchmark mutex, forces a bounded large Double MA grid into multiple full tiles with `VECTORBT_KERNEL_VRAM_BUDGET_MB`, and records the tile selector's exact geometry. The gate independently reconstructs valid-pair counts and rejects the run unless at least two full tiles have the same former coarse reuse signature. It then compares graph-off and graph-on results, requires exact best lengths and trade-count fields, applies explicit absolute-plus-relative tolerances to every emitted continuous best metric, and requires all capture, instantiation, and replay attempts to succeed. Because every cached valid-pair map owns tile-specific device allocations, the immutable pointer-aware route must report one successful capture per successful replay; this equality makes the historical stale-pointer reuse failure deterministic even when a corrupted later tile would not have changed the winning candidate. Existing output paths are rejected; a fresh path is atomically reserved with a run-owned non-passing marker and only that marker may be atomically promoted to passed evidence. The lane is a specialized GPU correctness regression, not broad performance or release authority.

The scoped warm mocked-browser interaction-latency authority is deliberately opt-in. Its wrapper acquires the repository benchmark lock before checking concurrency and building, then holds the lock through evidence validation; the runner rejects authority mode unless it can verify the live lock ownership chain. The tracked policy fixes three warmups and twenty measured trusted clicks per case, a 1600x1000 headless Chromium viewport, reduced motion, a 64 ms DOM-quiet window, and absolute p95/max budgets. A fresh v2 attestation binds a canonical length-prefixed `{path,size,sha256}` manifest for the exact frontend build inputs and built distribution; `-SkipBuild` is fail-closed unless that attestation is current and unchanged. Each run copies the attested assets into flat content-addressed, read-only object files, validates them, preloads independent byte copies into memory, and serves only those bytes, so live `web/dist` changes and nested junction swaps cannot alter the measured app. The exact wrapper/runner/evaluator/validator/mock-harness file set and effective path-distinct fixture are hashed, checked again after sampling, and retained in evidence. Saved-detail samples use argument-matched asynchronous mock responses and a unique visible detail-only ready field. The runner prints the SHA-256 of the no-replace report it published; the wrapper requires those exact bytes, recomputes eligibility/statistics from every retained raw sample, and binds the policy, attestation, lease, flat snapshot manifest/hash/count/bytes, and server totals before acceptance. The helper is injected only by the Playwright harness; no production component, store, or runtime instrumentation is enabled. Use `-IncludeInteractionLatency` on `scripts/run_frontend_e2e_gate.ps1` or `-IncludeFrontendInteractionLatency` on `scripts/run_performance_audit_baseline_pack.ps1` only when that scoped evidence is wanted; required CPU CI and normal frontend E2E do not run the timing lane. This lane does not close the full frontend/native click-to-render gap.

## Release Evidence Hierarchy

| Release question | Primary authority | Notes |
|---|---|---|
| Is the local machine/repo ready for this variant? | `scripts/run_release_doctor.ps1` and `readiness.json` | Doctor runs Tauri config sync and release-readiness core. It does not build, stage, hash, or smoke a package. |
| Was a candidate locally rehearsed? | `scripts/run_release_candidate_gate.ps1` and `summary.json` / `summary.md` | Summary is broad rehearsal evidence, not a replacement for specific artifacts. |
| What exactly was built? | `provenance.json` first, `provenance.txt` secondary | Includes build identity, toolchain, and environment evidence. |
| Which assets were staged? | `manifest.json` | Current required resources include CatBoost native DLL and license, bundled synthetic sample CSV, `double_crossover.prebuilt.ptx`, and `options_expression.prebuilt.ptx`. |
| What are the asset hashes? | `SHA256SUMS.txt`, interpreted against `manifest.json` | Hash file alone is not variant authority. |
| Did the packaged artifact launch? | `packaged-smoke-summary.json` | Check `smoke_mode`; raw release-binary smoke is not bundle-level packaging proof. |
| Are artifact/report outputs still compatible? | `node scripts/run_output_parity_check.mjs` | Also runs by default in the release-candidate gate; the direct command remains the fastest focused authority. |
| Are shipped live-execution controls covered? | `scripts/run_execution_safety_gate.ps1`, authority-eligible `live-safety-summary.json`, and `scripts/public_live_safety_evidence_contract.mjs` | All public compute variants expose Alpaca live execution. The public wrapper requires both authorities; broker connectivity and manual recovery remain separate evidence. |

Current release-candidate caveats:

- `scripts/run_release_candidate_gate.ps1` runs `npm --prefix web run parity`.
- It runs the tracked structural and semantic output-parity pack by default.
- It only runs packaged desktop smoke when `-IncludeDesktopSmoke` is set and `-SkipPackagedSmoke` is not set.
- Under `-IncludePerf`, `gpu-ma` invokes `scripts/run_gpu_ma_perf_gate.ps1` and `gpu-kernel` invokes the kernel profile in `scripts/run_gpu_perf_gate.ps1`; the two profiles cannot share a baseline.
- It invokes dedicated GPU validation perf only when `-IncludeGpuValidationPerf` is set on the `gpu-kernel` variant.
- `-IncludeOptionsGpuPromotion` is an explicit `gpu-kernel`-only opt-in. It requires an existing artifact path, exactly one explicit dataset id or import CSV, an import root symbol for CSV input, and an existing candidate baseline. The release summary records the promotion artifact; the public wrapper additionally verifies the three correctness-evidence records, promotion checks, and baseline-regression checks.
- `-IncludeExecutionSafetyGate` and `-IncludeLiveSafetyHarness` opt ordinary candidate rehearsals into the two reviewed live-execution authorities. The harness uses one locked, offline Cargo invocation with explicit CPU features, clears production credential environment variables, applies an external-HTTP proxy trap, serializes tests, hard-pins all 107 current `commands_execution` tests plus 21 execution-policy, mock, quantity, and settings contracts, and fails if its hashed source contract changes during the run.
- `live-safety-summary.json` is authority evidence only when `status` is `passed` and `authority_eligible` is `true`. `-ContractSimulation` accepts an injected executable solely for deterministic harness testing and always emits `authority_eligible=false`.
- The harness is unit-contract authority for its enumerated invariants. It does not prove broker connectivity or writes, OS-level network isolation, proxy-bypassing clients, profitability, end-to-end command behavior, or manual disaster recovery.
- Every public `cpu`, `gpu-ma`, and `gpu-kernel` desktop ships the Alpaca-live command and UI surface. `scripts/run_public_release_gate.ps1` therefore invokes both authorities unconditionally and uses `scripts/public_live_safety_evidence_contract.mjs` to reject missing, stale, diagnostic, custom-Cargo, source-drifted, or incomplete evidence.

## Release Variant Evidence Matrix

| Variant/surface | Required evidence | Notes |
|---|---|---|
| CPU-only | CPU build; release doctor; release candidate; provenance/manifest/hash; execution safety gate; authority-eligible live-safety harness. | Must not require CUDA Toolkit, NVCC, NVIDIA GPU, or driver. The package still exposes Alpaca live execution. |
| GPU MA-sweep | `--features cuda`; release doctor/candidate for `gpu-ma`; `scripts/run_gpu_ma_perf_gate.ps1 -Mode candidate`; NVIDIA driver/runtime evidence; both live-execution safety authorities. | Uses the dedicated `double_ma/gpu_sweep/*` registry and `gpu_ma_perf_baseline.json`; it does not consume the GPU-kernel baseline. |
| GPU kernel | `--features cuda-backtest-kernel`; `VECTORBT_USE_PREBUILT_PTX=1` for packaged kernel path; manifest PTX evidence; GPU perf; GPU validation perf when validation/residency claims are made; both live-execution safety authorities. | `cuda-backtest-kernel` implies `cuda`; runtime NVIDIA driver still required. |
| Options GPU scorer | `gpu-kernel` release/public gate with `-IncludeOptionsGpuPromotion`, explicit artifact and dataset/import identity, CPU options tests, exact CPU/GPU parity, promotion checks, and baseline comparison. | The manifest tracks `options_expression.prebuilt.ptx`; that packaging evidence is not enough to promote options GPU scorer claims. |
| ML Signal native CPU | ML quality gate; native smoke; CatBoost DLL resource in manifest. | Manifest currently requires CatBoost native DLL unless explicitly allowed missing. |
| Live execution enabled | Execution safety gate; reviewed live-safety harness with authority-eligible evidence; no production credentials. | Public live release still requires manual recovery and emergency-halt posture; the harness does not contact a broker. |

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
- `scripts/run_cuda_graph_timing_probe.ps1` is mechanism-level companion evidence; its host replay-submission duration is not GPU-kernel execution time or a release-performance gate.
- `scripts/run_cuda_graph_multitile_parity.ps1` is one constrained Double MA correctness regression; it is not broad CUDA parity, performance, or release authority.
- The active `Required CPU quality` check is merge enforcement for CPU/static/headless scope, not installer, signing, GPU, native-desktop, or clean-machine release authority.
- Archived `.github/workflows.disabled/*` definitions are not active authority and must not be copied back as current workflow contracts.
- `scripts/run_release_candidate_gate.ps1` records the same tracked output-parity pack as the focused command when the step passes.

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
| Dedicated `gpu-ma` perf authority | Implemented | `gpu-ma -IncludePerf` selects the dedicated CUDA-only MA-sweep registry, baseline, and summary paths through `scripts/run_gpu_ma_perf_gate.ps1`; the existing GPU-kernel baseline remains unchanged. |
| Options GPU scorer release-promotion authority | Implemented | Candidate and public gates expose a `gpu-kernel`-only explicit opt-in and fail closed unless fixed workload inputs, CPU options tests, exact CPU/GPU parity, promotion checks, and candidate-baseline checks all pass. |
| Release-candidate output parity integration | Implemented | Candidate rehearsal now runs the tracked JSON/CSV/XLSX structural and semantic parity pack by default; public release also requires it. |
| Machine-readable authority matrix | Implemented | `docs/release_authority_matrix.json` is validated deterministically by `scripts/check_release_authority_matrix.mjs`; repo quality, release doctor, and the required CPU CI chain fail closed on drift. |
| Live safety harness authority | Implemented | The reviewed harness runs one locked/offline explicit-CPU Cargo process, removes production credential environment variables, traps ordinary external HTTP proxy traffic, hard-pins the complete 107-test `commands_execution` namespace plus supporting execution contracts, verifies pre/post source hashes, separates simulation from authority eligibility, and is protected by positive and negative repo-quality contracts. |
| Required CPU CI enforcement | Implemented | `.github/workflows/cpu-quality.yml` delegates to `scripts/run_cpu_ci_gate.ps1`; branch protection must require `Required CPU quality`. |
| GPU/native/clean-machine CI lanes | Deferred | Require suitable GPU or desktop runners, foreground ownership, signing/release infrastructure, or clean-VM orchestration; they are intentionally outside required CPU CI. |
| CUDA Graphs timing boundary | Implemented | Opt-in microsecond instrumentation and a serialized machine-readable probe separate capture, instantiate, immutable-topology update non-applicability, and asynchronous replay submission without changing the default runtime path. |
| Full frontend click-to-render latency | Research spike | The scoped warm mocked-browser lane covers two representative built-frontend interactions. Full authority still needs native Tauri/WebView2, real backend/provider boundaries, cold paths, complete workflow coverage, and representative device evidence. |

## Source Anchors

Use these files as authority anchors before changing this document:

- `docs/REPO_MAP.md`
- `docs/release_authority_matrix.json`
- `docs/RELEASE.md`
- `scripts/check_release_authority_matrix.mjs`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_release_doctor.ps1`
- `scripts/generate_release_manifest.ps1`
- `scripts/run_packaged_smoke.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_ma_perf_gate.ps1`
- `scripts/run_cuda_graph_timing_probe.ps1`
- `scripts/cuda_graph_timing_contract.test.mjs`
- `scripts/run_cuda_graph_multitile_parity.ps1`
- `scripts/cuda_graph_multitile_parity_contract.test.mjs`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/run_public_release_gate.ps1`
- `scripts/run_output_parity_check.mjs`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_live_execution_safety_harness.ps1`
- `scripts/live_execution_safety_harness_contract.test.mjs`
- `scripts/run_frontend_e2e_gate.ps1`
- `scripts/run_frontend_interaction_latency.ps1`
- `web/scripts/frontend-interaction-authority-lease.mjs`
- `web/scripts/frontend-interaction-build-attestation.mjs`
- `web/scripts/frontend-interaction-report-validator.mjs`
- `web/scripts/frontend-interaction-static-snapshot.mjs`
- `web/scripts/measure-frontend-interaction-latency.mjs`
- `scripts/run_performance_audit_baseline_pack.ps1`
- `scripts/run_cpu_ci_gate.ps1`
- `.github/workflows/cpu-quality.yml`
- `docs/CI.md`
