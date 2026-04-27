# GPT-5.5 Pro high-ROI future prompt ideas

Status: research-backed prompt expansion plan  
Created: 2026-04-27  
Depends on:

- `docs/gpt55_pro_final_prompt_dossiers_2026-04-27.md`
- `docs/gpt55_pro_hard_problem_atlas_2026-04-27.md`
- `docs/REPO_MAP.md`

## Purpose

This document identifies high-ROI GPT-5.5 Pro prompt ideas that were originally beyond the first 12 active dossiers.

The original 12 dossiers were comprehensive for the current engine, GPU, options, ML, workflow, artifact, campaign, and release surfaces. Dossiers 13-15 have since been promoted into the active prompt pack; candidates 16-18 remain future expansion ideas.

## Method

This pass used:

- repo scans across `src-tauri`, `crates`, `web/src`, `scripts`, and current prompt docs
- current repo routing in `docs/REPO_MAP.md`
- web research on Tauri security/capabilities, desktop app security, AI risk governance, CUDA determinism, options margin, broker order behavior, and credential protection

## Priority Verdict

Add these first if expanding the prompt pack:

1. Dossier 13: Desktop Trust Boundary, Secrets, Capabilities, And Local Security
2. Dossier 14: Portfolio Construction, Capital Allocation, Margin, And Execution Realism
3. Dossier 15: Numerical Reproducibility, Determinism, And Cross-Backend Scientific Audit

Promotion status: dossiers 13-15 have been promoted into the final prompt dossier document and upload packs.

Add these second:

4. Dossier 16: Data Lifecycle, Local Retention, Privacy, And Disk-Growth Policy
5. Dossier 17: Native Binary, Model Bundle, PTX, And Supply-Chain Integrity
6. Dossier 18: Research ROI, Feature Triage, And Model-Assisted Decision Governance

Do not add more CUDA novelty prompts before these. Tensor cores and RT cores are already covered by prove-or-kill dossiers; the uncovered risk is mostly product correctness, security, reproducibility, and finance realism.

## Candidate 13: Desktop Trust Boundary, Secrets, Capabilities, And Local Security

ROI: very high  
Reason: VectorGrid stores/uses broker and market-data credentials, exposes a Tauri command surface, writes artifacts/reports/logs, and can bridge research into broker actions. Current dossiers cover pieces of this, but not as one trust-boundary threat model.

### Hard Question

What is the minimum secure-by-default desktop trust boundary for VectorGrid's local app, Tauri command bridge, credentials, artifacts, logs, live execution, provider downloads, and report/open/reveal actions?

### Specific Questions

1. Which Tauri commands are safety-critical, data-exfiltration-capable, or broker-action-capable?
2. Are current `main.json` and `dev.json` capabilities minimal enough for a single-window desktop app?
3. Which command families require additional backend authorization checks beyond Tauri capability gating?
4. How should Polygon and Alpaca credentials be stored, migrated, redacted, cleared, and excluded from artifacts, logs, reports, and broker intent records?
5. Is JSON-file credential fallback acceptable, and what warning/fail-closed behavior should it imply?
6. Which local paths are sensitive: `runs`, `reports`, Polygon cache, options datasets, logs, settings, model bundles, native DLLs, PTX?
7. What CSP, remote-content, shell/open/reveal, log redaction, and file-picker rules are required?
8. What should security tests prove before live broker support expands?

### Required File Pack

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/commands_system.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/desktop_menu.rs`
- `src-tauri/capabilities/main.json`
- `src-tauri/capabilities/dev.json`
- `src-tauri/tauri.conf.json`
- `tauri.conf.json`
- `web/src/tauri/commands.ts`
- `web/src/tauri/invoke.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/services/executionService.ts`
- `web/src/services/dataService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/settingsService.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/stores/executionStore.ts`
- `web/src/stores/runtimeStore.ts`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/scripts/check-tauri-capabilities.mjs`
- `web/scripts/check-tauri-parity.mjs`
- `web/scripts/check-known-tauri-commands.mjs`
- `src-tauri/tests/command_runtime_wiring.rs`
- `scripts/run_execution_safety_gate.ps1`

### External Context

- Tauri capabilities and security boundaries: https://v2.tauri.app/security/capabilities/
- Tauri runtime authority: https://v2.tauri.app/security/runtime-authority/
- Tauri CSP guidance: https://v2.tauri.app/security/csp/
- OWASP Desktop App Security Top 10: https://owasp.org/www-project-desktop-app-security-top-10/
- NIST CSF 2.0: https://www.nist.gov/publications/nist-cybersecurity-framework-csf-20
- Microsoft DPAPI/CNG DPAPI: https://learn.microsoft.com/en-us/windows/win32/seccng/cng-dpapi

### Expected Output

- trust-boundary diagram in text form
- command risk classification table
- credential lifecycle policy
- local sensitive-data inventory
- capability/CSP recommendations
- test and redaction plan
- explicit "do not build" list for security theater

## Candidate 14: Portfolio Construction, Capital Allocation, Margin, And Execution Realism

ROI: very high  
Reason: VectorGrid optimizes strategy candidates and options tuples, but the product-level finance problem is how to allocate capital across validated strategies, symbols, options structures, and live execution constraints. This is not solved by single-strategy Sharpe, robust score, or validation alone.

### Hard Question

Should VectorGrid add a portfolio-level decision layer that turns validated candidates into capital allocation, risk-budget, margin, and execution-realism recommendations?

### Specific Questions

1. What is the correct portfolio objective: expected utility, robust Sharpe, drawdown control, tail-risk minimization, capital efficiency, or multi-objective frontier?
2. How should correlated strategies, overlapping symbols, shared market regimes, and crowding be penalized?
3. Which constraints are required: max gross exposure, net exposure, per-symbol exposure, sector/asset-class exposure, drawdown budget, turnover, margin, buying power, liquidity, and open-order risk?
4. Can current validation artifacts provide enough covariance/correlation and scenario data, or is a new portfolio artifact needed?
5. How should options structures account for strategy-based margin, portfolio margin approximations, assignment/exercise, and broker option-trading levels?
6. What execution model is sufficient for non-HFT desktop decisions: spread, slippage, market/limit order tradeoffs, stale quotes, partial fills, and bracket limitations?
7. What would be dangerous to automate live versus safe to present as an offline research recommendation?

### Required File Pack

- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/strategies/src/records.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/execution_policy.rs`
- `src-tauri/src/execution_quantity.rs`
- `src-tauri/src/broker_paper.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/metaService.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/services/executionService.ts`
- `web/src/utils/executionRisk.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/executionStore.ts`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_workflow_gate.ps1`

### External Context

- Cboe strategy-based margin: https://www.cboe.com/en/markets/us/options/margin/strategy-based-margin/
- FINRA margin accounts and portfolio margin overview: https://www.finra.org/rules-guidance/key-topics/margin-accounts
- OCC options disclosure document page: https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document
- Alpaca order behavior and order-type documentation: https://docs.alpaca.markets/docs/trading/orders/

### Expected Output

- portfolio artifact schema proposal
- allocation objective and constraint model
- options margin approximation policy
- execution-realism model
- offline-vs-live boundary
- smallest implementation slice, preferably read-only/offline first
- kill criteria if current artifacts cannot support decision-grade allocation

## Candidate 15: Numerical Reproducibility, Determinism, And Cross-Backend Scientific Audit

ROI: high  
Reason: Current dossiers discuss CPU/GPU parity locally, but there is no single repo-wide reproducibility policy covering tie ordering, non-finite values, seeds, floating-point reductions, generated PTX, benchmark baselines, artifacts, and release-to-release replay.

### Hard Question

What repo-wide reproducibility contract should VectorGrid enforce across CPU, CUDA, frontend request builders, artifacts, reports, benchmarks, and releases?

### Specific Questions

1. Which outputs must be bitwise stable, order stable, tolerance stable, or only distributionally stable?
2. What is the canonical policy for NaN, infinity, invalid candidates, score ties, stable sorting, and tie-break fields?
3. Which seeds and random/sampler controls must be persisted into artifacts and reports?
4. How should CPU/GPU parity tolerances differ between rank, score, metrics, equity curves, validation summaries, and reports?
5. Should CUDA reduction determinism features or reproducible accumulators be considered, and where would the performance tradeoff be acceptable?
6. How should generated PTX, CUDA toolkit version, GPU model, driver, and benchmark environment fingerprinting affect reproducibility claims?
7. What output parity gate should become authority for replay reproducibility?

### Required File Pack

- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_refactor_snapshots.rs`
- `scripts/run_output_parity_check.mjs`
- `scripts/check_output_parity.mjs`
- `scripts/check_output_semantic_parity.mjs`
- `scripts/output_parity_pairs.json`
- `scripts/output_parity_semantic_pairs.json`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/capture_release_provenance.ps1`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/historyService.ts`
- `web/src/utils/core/artifactIdentity.ts`

### External Context

- NVIDIA CUDA floating-point guide: https://docs.nvidia.com/cuda/floating-point/contents.html
- NVIDIA CCCL floating-point determinism: https://developer.nvidia.com/blog/controlling-floating-point-determinism-in-nvidia-cccl/
- CUDA Programming Guide: https://docs.nvidia.com/cuda/cuda-programming-guide/

### Expected Output

- reproducibility tier taxonomy
- deterministic ordering and non-finite policy
- parity tolerance table
- artifact/report/release fingerprint contract
- benchmark reproducibility policy
- CUDA determinism spike or kill recommendation

## Candidate 16: Data Lifecycle, Local Retention, Privacy, And Disk-Growth Policy

ROI: medium-high  
Reason: VectorGrid now has market-data roots, Polygon cache migration, imported options datasets, artifacts, reports, logs, localStorage, and retention settings. Dossiers 3 and 8 cover provenance and replay, but not the operational lifecycle of local data.

### Hard Question

What local data lifecycle should VectorGrid enforce so the app remains auditable without unbounded disk growth, stale hidden state, or accidental privacy/credential leakage?

### Specific Questions

1. What are all durable local data classes and roots?
2. Which data is user-owned research evidence versus disposable cache?
3. Which retention policies should be count-based, age-based, size-based, or explicit-user-delete only?
4. How should market-data-root migration handle partial moves, rollback, external paths, and old cache references?
5. How should reports and artifacts behave when source data has been pruned?
6. What privacy-sensitive data can appear in logs, artifacts, reports, localStorage, settings, cache manifests, or run provenance?
7. What deletion semantics are required: delete artifact only, delete report, delete source dataset, delete cache, clear credentials, clear logs?

### Required File Pack

- `src-tauri/src/settings.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/commands_system.rs`
- `crates/app_core/src/options_data_store.rs`
- `web/src/stores/settingsStore.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/settingsService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/dataService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/utils/storage.ts`
- `web/src/utils/core/storageKeys.ts`
- `web/src/utils/core/artifactIdentity.ts`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `scripts/run_output_parity_check.mjs`

### External Context

- NIST CSF 2.0: https://www.nist.gov/publications/nist-cybersecurity-framework-csf-20
- OWASP Desktop App Security Top 10: https://owasp.org/www-project-desktop-app-security-top-10/
- Tauri file-system and permission guidance if adding frontend filesystem access: https://v2.tauri.app/plugin/file-system/

### Expected Output

- local data inventory
- retention/deletion matrix
- cache-vs-evidence policy
- migration failure and rollback policy
- privacy redaction policy
- UI wording and test authority

## Candidate 17: Native Binary, Model Bundle, PTX, And Supply-Chain Integrity

ROI: medium-high  
Reason: Release packaging currently covers correctness and smoke evidence, but GPT-5.5 Pro has not been asked to reason specifically about native DLL/model bundle/PTX trust, tamper detection, dependency provenance, and local execution of bundled binaries.

### Hard Question

What integrity and trust contract should govern VectorGrid's generated PTX, native CatBoost DLLs, model bundles, embedded resources, dependencies, and packaged release artifacts?

### Specific Questions

1. Which files are executable or executable-adjacent: PTX, DLLs, sidecars, scripts, model bundles, generated command schemas?
2. Which artifacts need hashes, signatures, manifests, provenance, or runtime compatibility checks?
3. How should app startup handle a missing, replaced, or mismatched PTX/native library/model bundle?
4. What dependency and build provenance belongs in release artifacts?
5. What is the rollback/hotfix trigger for a bad native binary or model bundle?
6. Should bundled native ML execution be treated as a trust boundary distinct from normal Rust code?

### Required File Pack

- `docs/RELEASE.md`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `tauri.conf.json`
- `tauri.windows.conf.json`
- `src-tauri/src/release_readiness.rs`
- `src-tauri/src/bin/ta_release_readiness.rs`
- `crates/app_core/build.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`
- `configs/ml_signal_catboost_default.json`
- `scripts/build_catboost_native_windows.ps1`
- `scripts/run_ml_signal_native_smoke.ps1`
- `scripts/run_release_doctor.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/generate_release_manifest.ps1`
- `scripts/capture_release_provenance.ps1`
- `scripts/run_packaged_smoke.ps1`
- `web/scripts/check-tauri-capabilities.mjs`

### External Context

- Tauri distribution and security documentation: https://v2.tauri.app/
- NIST CSF 2.0 supply-chain and governance framing: https://www.nist.gov/publications/nist-cybersecurity-framework-csf-20
- NVIDIA CUDA deployment compatibility documentation: https://docs.nvidia.com/cuda/

### Expected Output

- executable-adjacent artifact inventory
- manifest/signature/hash policy
- runtime integrity check plan
- release gate additions
- native model-bundle trust boundary
- rollback criteria

## Candidate 18: Research ROI, Feature Triage, And Model-Assisted Decision Governance

ROI: medium  
Reason: The prompt pack is now large. A meta-prompt can help decide which dossier to run first, how to avoid novelty traps, and how to convert GPT-5.5 Pro recommendations into implementation tickets without treating the model as authority.

### Hard Question

Given the current VectorGrid architecture and product goals, which hard-problem prompts deserve engineering investment, which should remain research-only, and which should be killed?

### Specific Questions

1. Rank dossiers 1-17 by expected value, implementation cost, verification cost, safety risk, and likelihood of being killed by evidence.
2. Which recommendations would improve actual user decisions versus only adding technical novelty?
3. Which prompts depend on uncertain external facts that must be rechecked immediately before implementation?
4. Which prompts require real credentials, GPU availability, broker sandbox evidence, options data entitlements, or release packaging evidence?
5. What is the smallest 30/60/90-day roadmap that avoids parallel high-risk work?
6. What local triage process should convert a GPT-5.5 Pro response into accepted, spike, defer, or kill?

### Required File Pack

- `docs/REPO_MAP.md`
- `docs/ARCHITECTURE_NOTES.md`
- `docs/RELEASE.md`
- `docs/gpt55_pro_final_prompt_dossiers_2026-04-27.md`
- `docs/gpt55_pro_hard_problem_atlas_2026-04-27.md`
- `docs/gpt55_pro_hard_problem_meta_research_plan_2026-04-27.md`
- `docs/gpt55_pro_high_roi_future_prompt_ideas_2026-04-27.md`
- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_release_doctor.ps1`
- `web/package.json`
- root `Cargo.toml`

### External Context

- NIST AI RMF Generative AI Profile: https://www.nist.gov/publications/artificial-intelligence-risk-management-framework-generative-artificial-intelligence
- NIST CSF 2.0: https://www.nist.gov/publications/nist-cybersecurity-framework-csf-20

### Expected Output

- ranked prompt ROI table
- recommended sequencing
- stop-doing list
- evidence prerequisites
- implementation decision rubric
- "ask for more context" triggers

## Lower-ROI Or Already-Covered Ideas

These should not become new prompt packs yet:

- RT-core/OptiX general-purpose compute: already covered by Dossier 6.
- Tensor-core novelty without a dense workload: already covered by Dossier 5.
- Generic frontend redesign: Dossier 10 already covers workflow/evidence UX.
- Generic app architecture rewrite: too broad and conflicts with the strategy-local architecture policy.
- Cloud/distributed execution: not enough current repo surface; revisit only after campaign resumability and verification authority mature.
- HFT/microstructure execution: likely misaligned with a desktop research app and current Alpaca/Polygon constraints.

## Recommended Next Step

Candidates 13-15 are now promoted into full dossiers and upload packs. The remaining next step is to consider Candidates 16-18 only after the active 15-dossier pack has produced decisions or implementation plans.
