# Prioritized Feature Expansion Execution Program

Date: 2026-04-30

## Goal

Turn the facet-specific plans into an implementation order that reduces risk, improves release posture, and avoids large cross-repo rewrites.

The root `README.md` remains the public status authority. This execution program is a release-readiness and feature-expansion plan, not a statement that signed desktop distribution or broker/live workflows are currently public-supported.

## Sequencing Principles

1. Evidence and safety before algorithmic expansion.
2. Additive schemas before behavior changes.
3. CPU fallback before GPU promotion.
4. Public Simple Mode proof before public desktop release expansion.
5. Route metadata before route policy changes.
6. Artifact/report parity before report/export expansion.
7. Local gate stability before CI reactivation.

## Dependency Map

| Expansion | Depends On |
|---|---|
| Simple Mode public polish | data sample readiness, command parity, compact validation summary |
| GPU validation productization | route metadata, validation evidence summary, GPU validation gate |
| Options GPU promotion | CPU oracle, options dataset provenance, options promotion gate, PTX manifest policy |
| Portfolio export | artifact evidence matrix, export schema registry |
| Live execution expansion | execution safety gate, feed lifecycle clarity, risk-policy tests |
| Artifact analytics lab | artifact row flattener, lazy frontend loading, bundle budget |
| Updater enablement | signed release path, release host, private updater key policy, clean-machine update test |
| CI reactivation | stable local gates, artifact retention policy, redaction policy |
| ML/meta evidence expansion | artifact/schema contract, validation evidence labels, model/native provenance |
| Accessibility/support expansion | frontend QA matrix, command risk policy, local data inventory |
| Market calendar service | provider error taxonomy, dataset manifest, release sample contract |

## Batch 1: Release And Trust Boundary Foundation

### Scope

Low-risk, high-leverage release and runtime hygiene.

### Items

1. Complete command risk policy coverage.
2. Tighten Tauri capability drift checks.
3. Add/verify release secret hygiene scanning.
4. Add local data inventory dry run.
5. Add release manifest checks for current CPU/GPU/native resources.

### Main Files

- `src-tauri/src/lib.rs`
- `src-tauri/capabilities/main.json`
- `web/scripts/check-tauri-command-risk-policy.mjs`
- `web/scripts/check-tauri-capabilities.mjs`
- `scripts/check_release_secret_hygiene.mjs`
- `scripts/generate_release_manifest.ps1`
- `docs/PUBLIC_RELEASE.md`

### Verification

- `npm --prefix web run security:desktop-boundary`
- `npm --prefix web run check:release-secret-hygiene`
- `powershell -File scripts/run_release_doctor.ps1 -Variant cpu`

## Batch 2: Artifact Durability And Output Parity

### Scope

Make persisted evidence safer before expanding new surfaces.

### Items

1. Artifact health dashboard.
2. Atomic write helper for JSON settings/artifacts/manifests where needed.
3. Report mutation contract tests.
4. Output parity pairs for any new export shape.

### Main Files

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports*.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `scripts/run_output_parity_check.mjs`

### Verification

- targeted artifact/report tests;
- `node scripts/run_output_parity_check.mjs`;
- `powershell -File scripts/run_repo_quality_gate.ps1`.

## Batch 3: Data And Market-Time Hardening

### Scope

Reduce subtle date/provenance errors before options/live expansion.

### Items

1. Market-time frontend helper.
2. Polygon cache manifest identity fields.
3. Data quality summary after import/download.
4. Options quote-quality summary fields.

### Main Files

- `web/src/utils/marketTime.ts` new
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/commands_options_data.rs`
- `crates/app_core/src/options_data_store.rs`

### Verification

- data and options frontend tests;
- Rust data/options tests;
- repo quality gate.

## Batch 3A: Manifest, Toolchain, And Release Resource Contract

### Scope

Lock down feature flags, npm dependency adoption, Tauri config sync, and release resources before new public claims depend on them.

### Items

1. Add or update a manifest/toolchain contract script.
2. Verify `vector-ta` version and feature forwarding across workspace manifests.
3. Check Windows Tauri config sync for bundled sample/resources.
4. Record PTX/native/sample resources by release variant.
5. Document optional frontend lab dependencies and flag-off behavior.

### Main Files

- `Cargo.toml`
- `src-tauri/Cargo.toml`
- `crates/app_core/Cargo.toml`
- `web/package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `tauri.conf.json`
- `tauri.windows.conf.json`
- `scripts/check_tauri_config_sync.mjs`
- `scripts/generate_release_manifest.ps1`
- `docs/feature_expansion_implementation_plans_2026-04-30/13_manifest_toolchain_dependency_contract.md`

### Verification

- manifest/toolchain contract script once added;
- `scripts/check_tauri_config_sync.mjs`;
- release doctor for the selected variant.

## Batch 4: Simple Mode Public Proof

### Scope

Make the public first-run path demonstrable.

### Items

1. Simple Mode happy-path E2E.
2. Bundled sample readiness check.
3. Compact optimization and optional validation summary.
4. Stable action IDs for Simple path.

### Main Files

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/services/simpleSummaryService.ts`
- `web/src/utils/simpleWorkflowModel.ts`
- `src-tauri/src/commands_simple_summary.rs`
- `web/e2e/browser/qa/*`

### Verification

- `npm --prefix web run test:e2e:qa:built`
- `npm --prefix web run a11y:smoke:built`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`

## Batch 5: Validation Evidence Productization

### Scope

Make validation verdicts and route metadata clear before changing GPU behavior.

### Items

1. Validation evidence summary utility.
2. GPU validation route metadata.
3. Walk-forward/window summary display.
4. Replacement dataset blocked/compatible labels.

### Main Files

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`

### Verification

- validation frontend/Rust tests;
- `node scripts/run_output_parity_check.mjs` if report shape changes;
- GPU validation gate when route/perf claims change.

## Batch 6: Strategy Route Matrix And Optimization Expansion

### Scope

Prevent frontend/backend route drift and create a safer path for new strategies.

### Items

1. Capability matrix tests.
2. Strategy-local expansion template.
3. Dynamic strategy route labels.
4. Optimization evidence fields for search-space size and validation availability.

### Main Files

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma*.rs`

### Verification

- optimize frontend tests;
- app-core route tests;
- workflow gate for product timing changes.

## Batch 7: Options Expression Hardening

### Scope

Improve options acceptance evidence without publicizing options as Simple Mode.

### Items

1. Per-window walk-forward tables.
2. Quote quality and price semantics summary.
3. CPU oracle fixtures.
4. GPU promotion gate follow-through only after parity.

### Main Files

- options app-core modules;
- options Tauri commands;
- `web/src/components/OptionsExpressionPanel.vue`;
- options scripts.

### Verification

- options frontend/Rust tests;
- `powershell -File scripts/run_options_expression_phase05.ps1`;
- options GPU promotion gate when CUDA behavior changes.

## Batch 7A: ML And Meta Evidence Expansion

### Scope

Bring ML/meta workflows into the same evidence and provenance framework before expanding model behavior.

### Items

1. Calibration bucket summaries.
2. Threshold stability fields.
3. Stage 3 evidence-policy summary.
4. Policy-eval detail tables.
5. ML Signal browser workflow fixture and gate.
6. Local model/native bundle registry only after provenance fields stabilize.

### Main Files

- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `scripts/run_ml_signal_quality_gate.ps1`

### Verification

- ML/meta frontend and Rust tests;
- `powershell -File scripts/run_ml_signal_quality_gate.ps1`;
- browser workflow gate after fixture cost is known.

## Batch 8: Portfolio And Execution Safety Expansion

### Scope

Add research/export and local-paper clarity without crossing into public production trading.

### Items

1. Portfolio replay evidence matrix.
2. Portfolio export without orders.
3. Local paper bracket preview.
4. Broker reconciliation event timeline.
5. Live feed/runner lifecycle status.

### Main Files

- portfolio app-core/Tauri/frontend files;
- execution Tauri/frontend files;
- execution safety scripts.

### Verification

- execution and portfolio tests;
- `powershell -File scripts/run_execution_safety_gate.ps1`.

## Batch 8A: Accessibility, Observability, And Support Readiness

### Scope

Make dense UI, diagnostics, and support workflows ready for broader desktop use.

### Items

1. Dense-table accessibility policy.
2. Chart accessibility summaries.
3. Live-region announcement policy.
4. Local diagnostics summary.
5. Redacted support bundle dry run.
6. Observability decision point: local-only, OpenTelemetry, Sentry, or none.

### Main Files

- `web/src/composables/useVirtualTableRows.ts`
- chart model utilities;
- `web/e2e/accessibility-review-matrix.md`
- `web/src/services/settingsService.ts`
- `src-tauri/src/redaction.rs`
- `src-tauri/src/local_path_policy.rs`
- `docs/feature_expansion_implementation_plans_2026-04-30/16_accessibility_observability_support_expansion.md`

### Verification

- focused component tests;
- `npm --prefix web run a11y:smoke:built`;
- support-bundle redaction tests once implemented.

## Batch 9: GPU/CUDA Performance Promotion

### Scope

Promote GPU work only where route metadata, CPU oracle, and profiling evidence are ready.

### Items

1. Resident buffer inventory/probe.
2. GPU validation productization.
3. Profiling artifact metadata.
4. Options PTX release policy only if options GPU becomes release-supported.

### Verification

- app-core tests;
- GPU perf gates;
- Nsight profiles as companion evidence.

## Batch 10: CI Reactivation

### Scope

Reactivate CI only after local checks are stable.

### Items

1. Review disabled workflows.
2. Add CPU-only format/check/frontend lint/typecheck job.
3. Add static QA and command parity jobs.
4. Add artifact retention/redaction policy.
5. Add browser/desktop smoke later.

### Verification

- local repo gate before enabling;
- CI artifact review after first run.

## First Ten Concrete Tasks

1. Complete command risk policy coverage.
2. Add manifest/toolchain/resource contract checks.
3. Add local data inventory dry-run command and UI summary.
4. Add `marketTime.ts` and DST tests.
5. Add artifact health dashboard fields and frontend labels.
6. Add report mutation contract tests.
7. Add Simple Mode happy-path E2E fixture.
8. Add validation evidence summary utility.
9. Add route capability matrix tests.
10. Add options quote-quality summary display.

## Second Ten Concrete Tasks

1. Add portfolio feasibility export-only summary.
2. Add candidate detail local-stability lane.
3. Add validation cancellation/incomplete artifact semantics.
4. Add options finalist dashboard model.
5. Add ML calibration bucket summaries.
6. Add ML Signal browser workflow fixture.
7. Add dense-table accessibility policy tests.
8. Add WebView2 diagnostic check to release doctor.
9. Add Alpaca reconciliation timeline evidence for paper broker.
10. Add GPU residency probe case registry.

## Cross-Cutting Kill Criteria

Stop or rescope a feature if:

- CPU-only build starts requiring CUDA, NVIDIA runtime, or native ML libraries;
- a GPU claim depends on hidden CPU fallback;
- saved artifacts become unreadable without a migration;
- frontend changes require broad selector churn without action registry updates;
- output report shape changes without parity coverage;
- live execution hides or delays emergency controls;
- release evidence cannot prove variant label, signatures, manifest, and hashes;
- provider expansion exceeds Polygon.io without explicit product decision.

## Handoff Template

Every implementation batch should finish with:

- changed files;
- behavior summary;
- compatibility notes;
- evidence/gate output;
- deferred items;
- release impact;
- rollback path.
