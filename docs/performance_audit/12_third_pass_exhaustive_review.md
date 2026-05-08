# Third-Pass Exhaustive Performance Review

Date: 2026-05-07
Status: implemented and verified

## Scope

This is the third repo-wide performance pass after the initial file checklist and second-pass tracking review. The goal was not to add new product features. The goal was to use the existing audit program to find remaining low-risk performance and evidence-integrity opportunities across backend, Tauri/runtime, frontend, scripts, and documentation.

This pass intentionally did not claim that every possible rewrite is complete. It distinguishes:

- implemented low-risk changes with targeted tests or gate coverage;
- evidence/gate hardening that prevents false performance claims;
- deferred changes that need profiling, GPU hardware, or behavior-sensitive review.

## Review Model

Four read-only reviewers split the repo into bounded surfaces:

| Surface | Focus | Result |
|---|---|---|
| Backend crates | `crates/app_core`, `crates/strategies`, `crates/optimizer` | Identified low-risk allocation/hash/loop opportunities and deferred behavior-sensitive hot-loop rewrites. |
| Tauri/runtime/scripts | `src-tauri`, benchmark scripts, audit runners | Identified support/campaign gate baseline enforcement, support-case validation, process timeout/deadlock protections, and artifact-catalog opportunities. |
| Frontend | `web/src`, `web/scripts` | Identified comparator/cache hardening and command-palette ranking memoization. |
| Docs/verification | `docs`, performance audit docs, authority docs | Identified stale tracking language and missing authority-map entries for the new support/campaign/baseline-pack gates. |

The main implementation then applied the changes that were both local and verifiable in this branch.

## Implemented Changes

### Performance Evidence Integrity

- `scripts/run_support_perf_gate.ps1`
  - Default candidate baseline is now `scripts/baselines/support_perf_baseline.json`.
  - Candidate mode now fails if that baseline is missing.
  - Subset case runs compare only the requested cases against baseline membership and timings instead of requiring exact full-case equality.

- `scripts/run_campaign_perf_gate.ps1`
  - Default candidate baseline is now `scripts/baselines/ta_campaign_perf_baseline.json`.
  - Candidate mode now fails if that baseline is missing.

- `src-tauri/src/bin/ta_perf_authority_support.rs`
  - Rejects unknown `--case` values with a hard error.
  - Normalizes known case names before selection.
  - Adds unit coverage for unknown and normalized case handling.

- `scripts/run_performance_audit_baseline_pack.ps1`
  - Adds `-GateTimeoutSec` for child gate runs.
  - Kills timed-out child processes and records exit code `124` instead of waiting indefinitely.

- `scripts/capture_optimization_memory.ps1`
  - Starts asynchronous stdout/stderr drain tasks immediately after process start to avoid redirected-pipe deadlocks during memory sampling.

### Backend Hot Path And Allocation Work

- `crates/app_core/src/ml_confidence.rs`
  - `cv_group_fold_plan` no longer stores a full intermediate hash vector.
  - It first counts unique groups and then builds fold IDs directly with `Vec::with_capacity(rows.len())`.

- `crates/app_core/src/options_expression.rs`
  - Session-index builders now use `HashMap::entry` to avoid double lookup while preserving first-seen ordered session output.

- `crates/app_core/src/double_ma_validate.rs`
  - Slice-return sum generation now iterates slice outer loops and bar inner loops, removing per-bar integer division.
  - Benchmark slice return generation now returns immediately for flat benchmarks and uses slice-outer loops for buy-and-hold.

- `crates/app_core/src/composite_validate.rs`
  - Composite and desired-position slice-return sum generation now uses slice-outer loops.
  - Benchmark slice return generation now returns immediately for flat benchmarks and uses slice-outer loops for buy-and-hold.
  - Removed unnecessary `mut` from CUDA-only walk-forward summary parameters.

- `crates/optimizer/src/grid.rs`
  - Removed unnecessary explicit `.into_iter()` in a `zip`.

- `crates/optimizer/src/mo.rs`
  - Replaced repeated manual `weights` fill loops with `weights.fill(1.0)`.

- `src-tauri/src/commands_validation.rs`
  - Replaced a small `map_or` branch with `is_none_or`.

- `src-tauri/src/run_artifacts.rs`
  - Reduced unnecessary cloning and repeated summary derivation in run-artifact detail construction.
  - Uses catalog-backed run listing where a valid catalog is available.
  - Narrows meta artifact loading to meta-named files before JSON hydration.

### Frontend Workbench Efficiency

- `web/src/composables/useHeadlessMatrixTable.ts`
  - Reuses a frozen empty-cell fallback.
  - Caches resolved column metadata by key in a computed map.
  - Precomputes sortable cell keys before sorting.
  - Keeps missing/blank cells sorted last for both ascending and descending directions.

- `web/src/composables/useHeadlessMatrixTable.test.ts`
  - Adds coverage for blank sortable cells staying last in both directions.

- `web/src/components/CommandPaletteOverlay.vue`
  - Precomputes lowercase shortcut/name/keyword search fields in the command search index.
  - Computes each ranking bucket once per filtered computed evaluation and reuses it in prefix/fuzzy ordering.

- `web/src/services/analysisService.ts`
  - Extends top-row sorted-model cache keys with objective, score mode, and score-observation context.
  - Prevents stale score ordering when the same run request/result objects are mutated in place.

- `web/src/services/analysisService.test.ts`
  - Adds a regression test for objective mutation with cached score sorting.

- `web/src/stores/optionsExpressionStore.ts`
  - Keeps primitives out of `markRaw` and only raw-marks object/function payloads.

## Deferred Work

These were explicitly reviewed and not changed in this pass:

- `double_ma_validate.rs` walk-forward window start lookup via a `HashMap` in additional paths beyond already scoped changes. This can help large window sets but may regress small sets through allocation and hashing overhead.
- `crates/strategies/src/records.rs` pre-count allocation. It adds a second scan and needs call-site evidence before changing.
- Larger composite/double-MA iterator rewrites in hot loops. Many are style-equivalent but need behavior tests and timing evidence.
- Frontend heatmap quantile/range semantics. That changes visual interpretation, not just performance.
- Deep raw-marking of entire Pinia stores. That can break replacement reactivity.
- Broker/provider live-start performance. These remain manual or credentialed exclusions.
- GPU/Nsight validation. CUDA profiling remains optional unless the final claim includes CUDA residency or GPU timing.

## Verification Ledger

Passing commands from the third pass:

- `git diff --check`
- `cargo fmt`
- `cargo test -p ta_optimizer --lib`
- `cargo test -p ta_app_core --lib ml_confidence`
- `cargo test -p ta_app_core --lib options_expression`
- `cargo test -p ta_app_core --lib composite_validate`
- `cargo test -p ta_app_core --lib double_ma_validate`
- `cargo test -p ta_desktop_demo_app --bin ta_perf_authority_support`
- `cargo test -p ta_desktop_demo_app --lib build_price_bars`
- `cargo test -p ta_desktop_demo_app --lib list_run_artifacts_blocking_uses_valid_catalog_snapshot`
- `cargo test -p ta_desktop_demo_app --lib load_meta_artifacts_with_paths_filters_to_meta_file_prefixes`
- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel --lib`
- `npm exec vitest run src/composables/useHeadlessMatrixTable.test.ts src/services/analysisService.test.ts src/components/CommandPaletteOverlay.test.ts src/stores/optionsExpressionStore.test.ts` from `web/`
- `npm --prefix web run typecheck`
- PowerShell parser checks for:
  - `scripts/run_support_perf_gate.ps1`
  - `scripts/run_campaign_perf_gate.ps1`
  - `scripts/run_performance_audit_baseline_pack.ps1`
  - `scripts/capture_optimization_memory.ps1`
  - `scripts/run_gpu_perf_gate.ps1`
  - `scripts/run_gpu_validation_perf_gate.ps1`
  - `scripts/run_gpu_followup_perf_gate.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_performance_audit_baseline_pack.ps1 -FailOnCoverageGap`
- `powershell -NoProfile -ExecutionPolicy Bypass -Command "& './scripts/run_performance_audit_baseline_pack.ps1' -Mode candidate -IncludeSupport -RunWorkflowGate:\`$false"`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_support_perf_gate.ps1 -Mode candidate -Case dynamic -OutPath output/benchmarks/support_perf_dynamic_third_pass.json`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_campaign_perf_gate.ps1 -Mode candidate -OutPath output/benchmarks/ta_campaign_perf_third_pass.json`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- `node scripts/run_output_parity_check.mjs`
- `cargo clippy -p ta_desktop_demo_app --bin ta_perf_authority_support --tests --no-deps`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`

Verification notes:

- `node scripts/run_output_parity_check.mjs` initially failed semantic parity because the sample baseline expected the old sparse 24x24 heatmap artifact and no validation evidence pack. The deterministic sample baseline was refreshed from the current headless output, then semantic parity passed.
- The first direct `run_performance_audit_baseline_pack.ps1 -Mode candidate -IncludeSupport -RunWorkflowGate:$false` shell invocation failed before running gates because PowerShell parsed the boolean argument as a string. The rerun through `powershell -Command` with escaped `$false` passed.
- CUDA feature compile passed with existing warnings outside this pass.
- CPU perf gate passed its threshold checks. Criterion reported statistically slower samples for the two CPU Double-MA benches and dynamic strategy microbenchmarks, but the repo gate thresholds still passed: `criterion_case_cpu_sma_small_not_regressed_15pct`, `criterion_case_cpu_alma_large_not_regressed_15pct`, `workflow_runtime_not_regressed_50pct`, and `cpu_dynamic_strategy_ratio_not_regressed_20pct`.

## Evidence Outputs To Record

When the final gates are run, record:

- `output/performance_audit/2026-05-07/authority_baseline_pack/summary.md`
- `output/performance_audit/2026-05-07/authority_baseline_pack/gate_run_manifest.json`
- `output/performance_audit/2026-05-07/authority_baseline_pack/raw/support_perf_summary.json`
- `output/performance_audit/2026-05-07/authority_baseline_pack/raw/ta_campaign_perf_summary.json`
- `output/benchmarks/support_perf_summary.json`
- `output/benchmarks/ta_campaign_perf_summary.json`
- `output/benchmarks/support_perf_dynamic_third_pass.json`
- `output/benchmarks/ta_campaign_perf_third_pass.json`
- `output/benchmarks/cpu_perf_summary.json`
- `output/benchmarks/workflow_regression_summary.json`

## Research Notes

This pass used current primary documentation for the implementation choices:

- Microsoft .NET redirected process output docs note that waiting before draining redirected output can deadlock when the child fills the pipe: https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.standardoutput
- Microsoft .NET `Process.WaitForExit(Int32)` documents timeout behavior for child processes: https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.waitforexit
- Rust `HashMap` entry API supports single-lookup occupied/vacant insertion/update patterns: https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html
- Rust `HashMap` docs describe the entry API as part of the standard map implementation: https://doc.rust-lang.org/std/collections/struct.HashMap.html
- Vue advanced reactivity docs describe `markRaw` for object-level opt-out from proxy conversion and large immutable payload performance use cases: https://vuejs.org/api/reactivity-advanced.html
