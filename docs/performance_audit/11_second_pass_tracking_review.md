# Second-Pass Performance Tracking Review

Date: 2026-05-07
Status: implemented targeted second-pass fixes and verification

## Scope Snapshot

This document records the follow-up pass after the repo-wide file checklist in
`10_repo_wide_performance_file_checklist.md`.

Inventory references:

- First-pass active checklist rows: `786`
- Second-pass broad `rg --files` snapshot excluding generated/build/output/dependency roots: `2115`
- Second-pass timestamp: `2026-05-07T12:10:02-04:00`
- Included code surfaces changed in this pass: `crates/optimizer`, `crates/app_core`,
  `src-tauri/src`, `scripts`, `web/src`, and `web/scripts`
- Excluded from direct mutation: generated build outputs, benchmark baselines, provider data,
  and profiler-only hotspots without a safe local edit

Second-pass reviewers were split by surface:

- backend crates: optimizer, app core, CUDA-gated code
- Tauri/runtime/scripts: headless bins, reports, benchmark harnesses
- frontend: Vue stores, services, composables, command palette, QA scripts
- tracking/verification: checklist consistency and final ledger shape

## First-Pass Consistency Findings

The first-pass checklist inventory is usable, but its summary counters drifted
after later edits:

- Parsed row count: `786`
- Parsed statuses: `changed=64`, `verified=14`, `reviewed-noop=638`,
  `deferred-profile=70`
- Header summary currently says: `changed=65`, `verified=15`,
  `reviewed-noop=639`, `deferred-profile=71`
- The header summary totals `790`, which is impossible against `786` rows
- `Generated UTC` reflects the inventory creation time, not the later support
  pack and final verification time

Decision: keep the first-pass checklist as the original inventory artifact and
use this document as the second-pass ledger instead of rewriting its history.

## Implemented Second-Pass Work

### Backend And Optimizer

- `crates/optimizer/src/grid.rs`
  - Avoids allocating a full score vector when `top_k == 0`
  - Reuses the cached score vector for both top-k selection and `top_scores`
    when `top_k > 0`

- `crates/optimizer/src/stream.rs`
  - Moves the candidate directly into `best` for `top_k == 0`
  - Avoids cloning `DoubleMaParams` and `Metrics` for improved-best updates
    when no top-k heap is needed

- `crates/optimizer/src/mo.rs`
  - Preserves deterministic equal-crowding tie order in the `select_nth_unstable_by`
    preselection path by adding the original front index as a tie-breaker
  - This fixes a first-pass determinism risk

- `crates/app_core/src/ml_confidence.rs`
  - Precomputes cross-validation group hashes, fold assignments, and fold counts
    once per candidate training pass
  - Avoids repeated `cv_group_id` hashing and repeated fold-count scans across
    logistic and GBDT CV loops

- `crates/app_core/src/dynamic_indicator_adapter.rs`
  - Uses `HashMap::entry` for CUDA prepared-input device state initialization
    and derived source overrides
  - Reduces duplicate hash lookups on repeated CUDA indicator preparation

### Tauri, Reports, And Scripts

- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
  - Replaces collect/sort latest-run discovery with a single `read_dir` pass
  - Preserves latest-modified behavior and explicit equal-time replacement order
  - Parses run artifacts via `File` + `BufReader` + `serde_json::from_reader`

- `src-tauri/src/bin/ta_campaign_perf.rs`
  - Parses per-dataset artifacts with `serde_json::from_reader`
  - Keeps the same JSON paths and route fields while avoiding a full artifact
    string allocation

- `src-tauri/src/commands_reports.rs`
  - Builds the bounded report row slice directly instead of cloning all sorted
    rows before truncation

- `src-tauri/src/validation_common.rs`
  - Adds parity coverage for canonical JSON serialization over nested objects,
    arrays, escaped strings, booleans, numbers, and nulls

- `src-tauri/src/commands_validation.rs`
  - Adds explicit coverage for NaN open/close fallback behavior inside an
    aggregated price-bar bucket

- `scripts/run_vectorgrid_double_ma_faceoff.ps1`
  - Uses async stdout/stderr draining for redirected process output
  - Avoids synchronous `ReadToEnd()` deadlock risk on noisy benchmark output

- `scripts/run_support_perf_gate.ps1`
  - Cleans the disposable `_write_test/perf_authority_support` fixture directory
    before each support-gate run
  - Guards the recursive cleanup so it only applies inside the repo `_write_test`
    tree
  - Fixes repeated-run drift where accumulated option manifests made the
    support fixture slower than its baseline shape

### Frontend

- `web/src/components/CommandPaletteOverlay.vue`
  - Precomputes lowercased command name words, keyword values, shortcut text,
    and original command order in the search index
  - Avoids repeated lowercasing/splitting/map lookups during command palette
    keystrokes and comparator calls

- `web/src/stores/historyStore.ts`
  - Builds a `Map<path, entry>` during saved-run sync
  - Replaces repeated `some` and `find` passes for selected/compare lookup
  - Caches overview compare options by `savedRunsRaw` array identity

- `web/src/services/workflowArtifactsSync.ts`
  - Replaces two sorted JSON stringification comparisons with structural
    stable equality over the same normalized summary fields
  - Avoids allocating large sorted strings for latest-artifact fast-path checks

- `web/src/utils/portfolioWorkspaceModel.ts`
  - Avoids `Object.values` allocation in recursive text lookup
  - Returns candidate array entries directly instead of building an object only
    to call `Object.entries`

- `web/src/composables/useHeadlessMatrixTable.ts`
  - Freezes the shared empty fallback cell so the allocation-saving sentinel
    cannot be mutated by consumers

- `web/scripts/write-local-qa-summary.mjs`
  - Adds script-local exists/stat/listing caches
  - Reduces repeated recursive listing and `statSync` work during QA freshness
    checks

- `web/scripts/write-frontend-qa-dashboard.mjs`
  - Adds the same script-local metadata caches
  - Reduces repeated directory scans for artifact summaries and visual baselines

## Attempted But Deferred

- `crates/app_core/src/options_expression_gpu.rs`
  - Attempted to hoist `module.get_function(...)` outside chunk loops
  - CUDA feature check rejected the edit because the function handle borrows
    `self.module` immutably across later mutable cache/buffer preparation
  - Reverted the hoist; this should wait for a borrow-safe kernel-function cache
    or runtime API change

- `crates/app_core/src/composite_cpu.rs`
  - Local-stability summary sorting could be reduced with nth-selection
  - Deferred because several duplicated strategy paths have subtly different
    missing-base behavior; this needs a focused parity refactor

- GPU perf gate streaming helpers
  - Several GPU PowerShell gates still have output/temp-file and full-sort
    opportunities
  - Deferred because final validation requires the normal GPU machine and
    elevated Nsight/CUDA-capable shell for meaningful gate coverage

- Virtual table render view models
  - `TopWorkspacePanel` and `ValidationWorkspacePanel` can reduce repeated
    cell payload/meta extraction
  - Deferred until a scroll/render trace can prove the win and protect layout
    behavior

- Runtime Playwright invoke-log count helpers
  - Test helper serialization can be reduced by counting in-browser
  - Deferred because it affects test infrastructure only and was not on the
    runtime product hot path for this pass

## Verification Ledger

Passing commands from this second pass:

- `cargo fmt`
- `cargo test -p ta_optimizer --lib`
- `cargo test -p ta_app_core --lib ml_confidence`
- `cargo test -p ta_desktop_demo_app --lib candidate_key_canonical`
- `cargo test -p ta_desktop_demo_app --lib build_price_bars`
- `cargo check -p ta_desktop_demo_app --bin ta_options_expression_feasibility`
- `cargo test -p ta_desktop_demo_app --bin ta_campaign_perf summarize_prefetch_economics -- --nocapture`
- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel --lib`
- `npm --prefix web run test -- src/components/CommandPaletteOverlay.test.ts src/utils/portfolioWorkspaceModel.test.ts src/stores/historyStore.search.test.ts src/stores/historyStore.actions.test.ts src/services/workflowArtifactsSync.test.ts src/services/artifactHistoryPerf.test.ts src/composables/useHeadlessMatrixTable.test.ts`
- `npm --prefix web run typecheck`
- `powershell -NoProfile -Command "[scriptblock]::Create((Get-Content -Raw scripts/run_vectorgrid_double_ma_faceoff.ps1)) | Out-Null"`
- `powershell -NoProfile -Command "[scriptblock]::Create((Get-Content -Raw scripts/run_support_perf_gate.ps1)) | Out-Null"`
- `npm --prefix web run check:qa-static`
- `npm --prefix web run perf:ui:budget`
- `npm --prefix web run test:e2e:action-coverage`
- `node web/scripts/write-local-qa-summary.mjs`
- `node web/scripts/write-frontend-qa-dashboard.mjs`
- `git diff --check`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -NoProfile -ExecutionPolicy Bypass -Command "& './scripts/run_performance_audit_baseline_pack.ps1' -Mode candidate -IncludeSupport -RunWorkflowGate:\`$false"`

Evidence artifacts from the final second-pass authority-pack run:

- `output/performance_audit/2026-05-07/authority_baseline_pack/summary.md`
- `output/performance_audit/2026-05-07/authority_baseline_pack/gate_run_manifest.json`
- `output/performance_audit/2026-05-07/authority_baseline_pack/raw/support_perf_summary.json`
- `output/performance_audit/2026-05-07/authority_baseline_pack/raw/ta_campaign_perf_summary.json`

Verification notes:

- The first run of the QA summary/dashboard scripts failed because existing
  runtime action-coverage reports were stale after web changes.
- `npm --prefix web run test:e2e:action-coverage` refreshed both runtime
  action coverage reports.
- The QA summary and frontend QA dashboard passed after that refresh.
- CUDA compile verification passed with warnings that pre-existed or are outside
  this second-pass scope.
- The first second-pass baseline-pack run failed because the support-gate
  disposable work dir had accumulated option manifests from prior runs. The gate
  now cleans its guarded `_write_test` fixture directory before running, and the
  baseline pack passes after that fix.

## Research Notes

The second pass used current official references for implementation choices:

- Rust `HashMap` entry APIs for single-lookup insertion/update patterns:
  https://doc.rust-lang.org/std/collections/hash_map/
- Rust slice nth-selection APIs for bounded top-k and median-style work:
  https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable_by
- Vue advanced reactivity APIs for raw/immutable large-payload handling:
  https://vuejs.org/api/reactivity-advanced.html
- Microsoft redirected process output guidance for avoiding stdout/stderr
  deadlocks:
  https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.processstartinfo.redirectstandardoutput

## Remaining Policy

The repo-wide performance pass is no longer blocked on obvious local
micro-optimizations. Remaining meaningful wins should be driven by profiler
evidence, especially:

- Nsight-backed GPU residency and kernel launch profiling
- workflow-level CPU perf gate deltas before/after candidate changes
- browser scroll/render traces for virtualized result tables
- artifact/report serialization traces on large real artifacts
- options-expression GPU promotion gates on a CUDA-capable machine
