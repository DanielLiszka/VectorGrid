# Backend Review Master Tracker

Last updated: 2026-04-14

## Purpose

This document is the master tracking index for the canonical backend review campaign.

The campaign goal is not to produce commentary for its own sake. The goal is to produce review packets that can drive objectively useful backend improvements while preserving existing product capability:

- maximize code quality where the evidence supports doing so
- reduce technical debt when the reduction is small, justified, and does not create hidden regression risk
- improve correctness, safety, maintainability, and clarity first
- improve performance only when the recommendation is supported by code evidence, benchmark evidence, or official guidance
- preserve feature coverage, supported workflows, and backend route availability unless a change is explicitly approved and justified elsewhere
- avoid regressions, avoid silent semantic drift, and avoid removals disguised as cleanup

It is intentionally limited to owned backend runtime source files:

- `src-tauri/src/**/*.rs`, excluding `src-tauri/src/bin/**`
- `crates/app_core/src/**/*.rs`, excluding `crates/app_core/src/bin/**`
- `crates/app_core/src/kernels/*.cu`
- `crates/strategies/src/**/*.rs`
- `crates/optimizer/src/**/*.rs`

This tracker is the source of truth for:

- which backend files are in scope for one-to-one review packets
- where each future packet should live under `Repo Review/backend-file-reviews/`
- per-file review status as packets are created

## Out Of Scope

These surfaces are intentionally excluded from this tracker:

- frontend code under `web/`
- vendor code under `vendor/`
- generated CUDA/PTX artifacts such as `*.ptx`
- benches, tests, examples, and `build.rs`
- backend helper binaries under `src-tauri/src/bin/**` and `crates/app_core/src/bin/**`
- historical or research-only copies under folders such as `SASS research/`, `REPO_Improvement_Epics/`, `options_research/`, and `CUDA kernel review/`

## Campaign Quality Bar

Every sub-review packet must be written against the following bar:

- recommendations must be evidence-backed, not style-churn or speculative preference
- changes should be the smallest justified change that improves the file materially
- file-local debt reduction is welcome, but broad refactors should be deferred unless they are clearly necessary
- performance advice must distinguish between safe structural improvements and benchmark-sensitive changes
- public API or command-surface suggestions must consider downstream consumers and compatibility risk
- CUDA and runtime-critical recommendations must respect the repo's performance-gate and profiling posture
- no review packet should recommend feature loss, behavior narrowing, or unsupported simplification as a default cleanup path

## Improvement Doctrine

The default posture of each packet is:

- identify objective issues
- separate safe-now improvements from defer-later work
- propose the smallest justified fixes
- preserve semantics unless there is a correctness bug or an explicitly approved product decision

The packet should prefer:

- local invariants over cleverness
- explicit contracts over implicit behavior
- tests and verification over assumption
- additive hardening over destabilizing rewrites

The packet should avoid:

- broad “rewrite this module” guidance without a concrete bounded migration path
- performance edits that are not backed by profiling, benchmarks, or official CUDA/Rust guidance
- naming or API churn without a real readability or contract benefit
- debt cleanup that removes useful hooks, visibility, diagnostics, or fallback behavior without proof that they are dead

## Scope Summary

- Total in-scope files: `116`
- Rust source files: `114`
- CUDA source files: `2`
- `src-tauri/src`: `32`
- `crates/app_core/src`: `74`
- `crates/app_core/src/kernels`: `2`
- `crates/strategies/src`: `4`
- `crates/optimizer/src`: `4`

## Canonical Inventory Rule

The canonical backend file inventory for this campaign is defined by the current live source tree, not by hand-maintained assumptions.

In-scope inventory rule:

- include all `*.rs` under `src-tauri/src/`
- exclude anything under `src-tauri/src/bin/`
- include all `*.rs` under `crates/app_core/src/`
- exclude anything under `crates/app_core/src/bin/`
- include handwritten `*.cu` under `crates/app_core/src/kernels/`
- exclude generated `*.ptx`
- include all `*.rs` under `crates/strategies/src/`
- include all `*.rs` under `crates/optimizer/src/`

Notes:

- `main.rs`, `lib.rs`, and internal support modules are in scope when they live under the canonical backend source roots above.
- `test_support.rs` and similar backend-owned support modules remain in scope because they are part of the owned backend source tree even when some paths are `cfg(test)`-gated.
- Helper binaries under `src/bin` are intentionally not part of this runtime-source tracker; they belong in a later secondary review pass if desired.

## Completeness Audit

This tracker was verified against the canonical backend inventory on `2026-04-14`.

Audit result:

- expected canonical backend files: `116`
- tracker rows: `116`
- missing from tracker: `0`
- extra in tracker: `0`

The tracker currently exactly matches the canonical backend source set.

Reproduction command used for the audit:

```powershell
$expected = @(
  Get-ChildItem src-tauri/src -Recurse -File -Include *.rs |
    Where-Object { $_.FullName -notmatch '\\src\\bin\\' }
  Get-ChildItem crates/app_core/src -Recurse -File -Include *.rs,*.cu |
    Where-Object { $_.FullName -notmatch '\\src\\bin\\' -and $_.Extension -ne '.ptx' }
  Get-ChildItem crates/strategies/src -Recurse -File -Include *.rs
  Get-ChildItem crates/optimizer/src -Recurse -File -Include *.rs
) |
  Sort-Object FullName -Unique |
  ForEach-Object { $_.FullName.Replace((Get-Location).Path + '\','') -replace '\\','/' }

$doc = Get-Content 'Repo Review\backend_review_master_tracker.md'
$listed = foreach ($line in $doc) {
  if ($line -match '^\\| `([^`]+)` \\| `backend-file-reviews/[^`]+` \\| `[^`]+` \\|$') {
    $matches[1]
  }
}

Compare-Object -ReferenceObject $expected -DifferenceObject $listed
```

If the comparison produces no rows, the tracker matches the canonical backend inventory exactly.

## Packet Convention

Each file will eventually receive one review packet at:

- `Repo Review/backend-file-reviews/<repo-relative-source-path>.review.md`

Example:

- Source: `crates/app_core/src/double_ma.rs`
- Packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma.rs.review.md`

## Mandatory Pre-Packet Research

Before writing each sub-review packet, there must be an explicit web research phase.

That research phase is required because the packet is expected to assess the file against current best practices, not just repo-local habits.

Minimum expectations for the research phase:

- consult official or primary sources that are relevant to the file's concerns
- prefer official documentation, language references, primary library docs, and vendor guides over tertiary blog posts
- research should be targeted to the file's actual risk profile, not generic copy-paste boilerplate
- the packet must record which external sources were consulted and why they were relevant
- if the file contains unsafe code, FFI, CUDA kernels, GPU runtime orchestration, or public API surface, the research phase must be deeper than the minimum baseline

Each packet should include a short research log section with:

- source title
- link
- why it was consulted for this file
- what standard or constraint it informed

## Baseline Standards Corpus

These are the baseline sources that review packets should consult when relevant:

- Rust API Guidelines checklist:
  - `https://rust-lang.github.io/api-guidelines/checklist.html`
- Rust API Guidelines documentation guidance:
  - `https://rust-lang.github.io/api-guidelines/documentation.html`
- Rust API Guidelines naming guidance:
  - `https://rust-lang.github.io/api-guidelines/naming.html`
- Clippy documentation:
  - `https://doc.rust-lang.org/clippy/`
- Rustonomicon for unsafe and FFI review:
  - `https://doc.rust-lang.org/nomicon/`
- Rust Book error handling:
  - `https://doc.rust-lang.org/book/ch09-00-error-handling.html`
- Rust Book testing:
  - `https://doc.rust-lang.org/book/ch11-00-testing.html`
- Tauri command guidance:
  - `https://v2.tauri.app/develop/calling-rust/`
- NVIDIA CUDA Best Practices Guide:
  - `https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html`
- NVIDIA CUDA Programming Guide:
  - `https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html`

File-specific research expectations:

- `src-tauri/src/**` command and runtime files should consult Tauri guidance when command shape, async behavior, error transport, or runtime ownership is relevant
- public Rust API modules should consult the Rust API Guidelines and Rustdoc guidance when naming, examples, errors, or surface contracts are relevant
- unsafe, allocation, FFI, or raw-pointer paths should consult the Rustonomicon and any relevant standard-library/reference material
- CUDA runtime and kernel files should consult NVIDIA CUDA best-practices and programming guidance, especially for memory behavior, divergence, synchronization, occupancy, and error handling
- strategy/math/statistics files should consult primary references or official docs when numerical behavior or domain methodology is under review

## Packet Deliverable Standard

Each sub-review packet is expected to produce a reusable professional review artifact, not just notes.

At minimum, each packet should include:

- file identity and ownership summary
- direct interactions with other files or modules
- file classification:
  - public surface
  - hot path
  - CUDA/runtime-critical
  - strategy-local
  - support code
- research log and standards consulted
- findings grouped into:
  - correctness and safety
  - API and naming
  - error handling
  - module boundaries and technical debt
  - docs and examples
  - tests and missing verification
  - performance, allocation, and CUDA concerns
- recommended action bucket:
  - accept as-is
  - document only
  - small surgical refactor
  - larger refactor but defer
  - benchmark-sensitive; do not touch yet
- exact local verification required if changed
- explicit statement about regression risk and feature-preservation constraints

To maximize implementation value before any code changes land, packets should also add the following strengthening sections whenever the file merits them:

- explicit cross-file interaction map:
  - which adjacent files are most likely to require coordinated changes
  - which downstream consumers would need verification if this file changes
- code-quality improvement register:
  - bounded readability, naming, invariant-clarity, duplication, or API-shape improvements that are justified by evidence
  - separate local cleanups from broader design changes
- technical-debt reduction register:
  - which debt is safe to reduce now
  - which debt should be deferred because it spans multiple files or product contracts
  - which debt is useful and should not be “cleaned up” casually
- performance opportunity register:
  - identify safe structural performance improvements
  - identify benchmark-sensitive or throughput-sensitive candidates that require measurement first
  - explicitly say when a file is not the right place to chase performance
- implementation staging guidance:
  - `safe now`
  - `coordination required`
  - `defer until later pass`
- change-blast-radius note:
  - local only
  - same subsystem
  - cross-backend / frontend-backend contract
- observability and diagnostics gaps:
  - missing logging, event detail, artifact detail, or error context that materially affects safe maintenance
- open questions or assumptions:
  - anything the packet could not prove locally and that should be treated as an explicit assumption rather than silent certainty

## Regression And Feature-Preservation Rule

The intended outcome of this campaign is backend improvement with no regressions and no feature loss or removal.

That means:

- the default recommendation should preserve current behavior and capability
- if a suggestion would alter semantics, remove flexibility, narrow supported workflows, or change public contracts, the packet must say so explicitly
- any change with non-trivial semantic or performance risk must be clearly marked as defer, benchmark-sensitive, or semantics-sensitive
- correctness fixes may justify behavior change, but the packet must identify the prior behavior as incorrect and specify the verification needed to confirm the fix

## Status Legend

- `Not started`: packet not yet written
- `In progress`: packet being written
- `Reviewed`: packet complete
- `Needs update`: packet exists but is stale after source changes

Recommended optional secondary tags inside each packet:

- `Safe now`
- `Semantics-sensitive`
- `Benchmark-sensitive`
- `Runtime-critical`
- `Public-surface-sensitive`

## Maintenance Rule

Whenever the canonical backend source set changes:

- update the file inventory in this tracker first
- rerun the completeness audit above
- update the `Last updated` date
- mark any affected packet rows as `Needs update` if their source file changed after review

## Review Packet Expectations

Each packet should cover:

- file ownership and direct interactions
- file classification: public surface, hot path, CUDA/runtime-critical, strategy-local, support code
- applicable standards and research references
- findings grouped into correctness/safety, API/naming, error handling, module boundaries, docs/examples, tests, and perf/CUDA concerns
- recommended action bucket
- exact local verification required if changed

The packet should also answer:

- what improvements are objectively useful here
- which improvements are viable now with low regression risk
- which debt should be documented but deferred
- which performance ideas require benchmark or profiling evidence before touching code
- what must not be changed casually because it protects existing behavior or feature coverage

## Tracker

### `src-tauri/src` (`32`)

| Source file | Planned packet | Status |
| --- | --- | --- |
| `src-tauri/src/commands_analysis.rs` | `backend-file-reviews/src-tauri/src/commands_analysis.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_artifacts.rs` | `backend-file-reviews/src-tauri/src/commands_artifacts.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_campaign.rs` | `backend-file-reviews/src-tauri/src/commands_campaign.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_data.rs` | `backend-file-reviews/src-tauri/src/commands_data.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_dynamic.rs` | `backend-file-reviews/src-tauri/src/commands_dynamic.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_execution.rs` | `backend-file-reviews/src-tauri/src/commands_execution.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_meta.rs` | `backend-file-reviews/src-tauri/src/commands_meta.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_ml_signal.rs` | `backend-file-reviews/src-tauri/src/commands_ml_signal.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_optimize.rs` | `backend-file-reviews/src-tauri/src/commands_optimize.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_options_data.rs` | `backend-file-reviews/src-tauri/src/commands_options_data.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_options_expression.rs` | `backend-file-reviews/src-tauri/src/commands_options_expression.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports.rs` | `backend-file-reviews/src-tauri/src/commands_reports.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_api.rs` | `backend-file-reviews/src-tauri/src/commands_reports_api.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_csv.rs` | `backend-file-reviews/src-tauri/src/commands_reports_csv.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_meta_xlsx.rs` | `backend-file-reviews/src-tauri/src/commands_reports_meta_xlsx.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_shared.rs` | `backend-file-reviews/src-tauri/src/commands_reports_shared.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_validation_common.rs` | `backend-file-reviews/src-tauri/src/commands_reports_validation_common.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_validation_xlsx.rs` | `backend-file-reviews/src-tauri/src/commands_reports_validation_xlsx.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_system.rs` | `backend-file-reviews/src-tauri/src/commands_system.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_validation.rs` | `backend-file-reviews/src-tauri/src/commands_validation.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands.rs` | `backend-file-reviews/src-tauri/src/commands.rs.review.md` | `Reviewed` |
| `src-tauri/src/data_cache.rs` | `backend-file-reviews/src-tauri/src/data_cache.rs.review.md` | `Reviewed` |
| `src-tauri/src/lib.rs` | `backend-file-reviews/src-tauri/src/lib.rs.review.md` | `Reviewed` |
| `src-tauri/src/ma_registry.rs` | `backend-file-reviews/src-tauri/src/ma_registry.rs.review.md` | `Reviewed` |
| `src-tauri/src/main.rs` | `backend-file-reviews/src-tauri/src/main.rs.review.md` | `Reviewed` |
| `src-tauri/src/metric_surface.rs` | `backend-file-reviews/src-tauri/src/metric_surface.rs.review.md` | `Reviewed` |
| `src-tauri/src/options_parent_adapters.rs` | `backend-file-reviews/src-tauri/src/options_parent_adapters.rs.review.md` | `Reviewed` |
| `src-tauri/src/polygon.rs` | `backend-file-reviews/src-tauri/src/polygon.rs.review.md` | `Reviewed` |
| `src-tauri/src/run_artifacts.rs` | `backend-file-reviews/src-tauri/src/run_artifacts.rs.review.md` | `Reviewed` |
| `src-tauri/src/settings.rs` | `backend-file-reviews/src-tauri/src/settings.rs.review.md` | `Reviewed` |
| `src-tauri/src/state.rs` | `backend-file-reviews/src-tauri/src/state.rs.review.md` | `Reviewed` |
| `src-tauri/src/validation_common.rs` | `backend-file-reviews/src-tauri/src/validation_common.rs.review.md` | `Reviewed` |

### `crates/app_core/src` (`74`)

| Source file | Planned packet | Status |
| --- | --- | --- |
| `crates/app_core/src/alloc_policy.rs` | `backend-file-reviews/crates/app_core/src/alloc_policy.rs.review.md` | `Reviewed` |
| `crates/app_core/src/asr.rs` | `backend-file-reviews/crates/app_core/src/asr.rs.review.md` | `Reviewed` |
| `crates/app_core/src/asr_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/asr_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/carm.rs` | `backend-file-reviews/crates/app_core/src/carm.rs.review.md` | `Reviewed` |
| `crates/app_core/src/carm_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/carm_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cdmc.rs` | `backend-file-reviews/crates/app_core/src/cdmc.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cdmc_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/cdmc_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_cpu.rs` | `backend-file-reviews/crates/app_core/src/composite_cpu.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_gpu_metric_decode.rs` | `backend-file-reviews/crates/app_core/src/composite_gpu_metric_decode.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_mo.rs` | `backend-file-reviews/crates/app_core/src/composite_mo.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_profile_route.rs` | `backend-file-reviews/crates/app_core/src/composite_profile_route.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_validate.rs` | `backend-file-reviews/crates/app_core/src/composite_validate.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cscv_pbo.rs` | `backend-file-reviews/crates/app_core/src/cscv_pbo.rs.review.md` | `Reviewed` |
| `crates/app_core/src/csv_compat.rs` | `backend-file-reviews/crates/app_core/src/csv_compat.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cuda_device.rs` | `backend-file-reviews/crates/app_core/src/cuda_device.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma.rs` | `backend-file-reviews/crates/app_core/src/double_ma.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_backend.rs` | `backend-file-reviews/crates/app_core/src/double_ma_backend.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_ma_kind.rs` | `backend-file-reviews/crates/app_core/src/double_ma_ma_kind.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_mode.rs` | `backend-file-reviews/crates/app_core/src/double_ma_mode.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_result.rs` | `backend-file-reviews/crates/app_core/src/double_ma_result.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_strategy.rs` | `backend-file-reviews/crates/app_core/src/double_ma_strategy.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_test_support.rs` | `backend-file-reviews/crates/app_core/src/double_ma_test_support.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_validate.rs` | `backend-file-reviews/crates/app_core/src/double_ma_validate.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_vdps.rs` | `backend-file-reviews/crates/app_core/src/double_ma_vdps.rs.review.md` | `Reviewed` |
| `crates/app_core/src/dynamic_indicator_adapter.rs` | `backend-file-reviews/crates/app_core/src/dynamic_indicator_adapter.rs.review.md` | `Reviewed` |
| `crates/app_core/src/gpu_backtest_kernel.rs` | `backend-file-reviews/crates/app_core/src/gpu_backtest_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/indicator_catalog.rs` | `backend-file-reviews/crates/app_core/src/indicator_catalog.rs.review.md` | `Reviewed` |
| `crates/app_core/src/indicator_factory.rs` | `backend-file-reviews/crates/app_core/src/indicator_factory.rs.review.md` | `Reviewed` |
| `crates/app_core/src/iterative_sim.rs` | `backend-file-reviews/crates/app_core/src/iterative_sim.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kqsr.rs` | `backend-file-reviews/crates/app_core/src/kqsr.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kqsr_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/kqsr_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kvdf.rs` | `backend-file-reviews/crates/app_core/src/kvdf.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kvdf_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/kvdf_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/lib.rs` | `backend-file-reviews/crates/app_core/src/lib.rs.review.md` | `Reviewed` |
| `crates/app_core/src/ma_policy.rs` | `backend-file-reviews/crates/app_core/src/ma_policy.rs.review.md` | `Reviewed` |
| `crates/app_core/src/matrix_index.rs` | `backend-file-reviews/crates/app_core/src/matrix_index.rs.review.md` | `Reviewed` |
| `crates/app_core/src/meta_select.rs` | `backend-file-reviews/crates/app_core/src/meta_select.rs.review.md` | `Reviewed` |
| `crates/app_core/src/mkcs.rs` | `backend-file-reviews/crates/app_core/src/mkcs.rs.review.md` | `Reviewed` |
| `crates/app_core/src/mkcs_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/mkcs_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/ml_confidence.rs` | `backend-file-reviews/crates/app_core/src/ml_confidence.rs.review.md` | `Reviewed` |
| `crates/app_core/src/ml_signal.rs` | `backend-file-reviews/crates/app_core/src/ml_signal.rs.review.md` | `Reviewed` |
| `crates/app_core/src/nvtx.rs` | `backend-file-reviews/crates/app_core/src/nvtx.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_data_store.rs` | `backend-file-reviews/crates/app_core/src/options_data_store.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_expression.rs` | `backend-file-reviews/crates/app_core/src/options_expression.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_expression_gpu.rs` | `backend-file-reviews/crates/app_core/src/options_expression_gpu.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_signal_manifest.rs` | `backend-file-reviews/crates/app_core/src/options_signal_manifest.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_types.rs` | `backend-file-reviews/crates/app_core/src/options_types.rs.review.md` | `Reviewed` |
| `crates/app_core/src/prescan_predictive.rs` | `backend-file-reviews/crates/app_core/src/prescan_predictive.rs.review.md` | `Reviewed` |
| `crates/app_core/src/progress.rs` | `backend-file-reviews/crates/app_core/src/progress.rs.review.md` | `Reviewed` |
| `crates/app_core/src/request_normalization.rs` | `backend-file-reviews/crates/app_core/src/request_normalization.rs.review.md` | `Reviewed` |
| `crates/app_core/src/rocb.rs` | `backend-file-reviews/crates/app_core/src/rocb.rs.review.md` | `Reviewed` |
| `crates/app_core/src/rocb_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/rocb_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/sharpe_stats.rs` | `backend-file-reviews/crates/app_core/src/sharpe_stats.rs.review.md` | `Reviewed` |
| `crates/app_core/src/standalone_gpu_runtime.rs` | `backend-file-reviews/crates/app_core/src/standalone_gpu_runtime.rs.review.md` | `Reviewed` |
| `crates/app_core/src/stats_common.rs` | `backend-file-reviews/crates/app_core/src/stats_common.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_dynamic.rs` | `backend-file-reviews/crates/app_core/src/strategy_dynamic.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_gpu_program.rs` | `backend-file-reviews/crates/app_core/src/strategy_gpu_program.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_gpu_runtime.rs` | `backend-file-reviews/crates/app_core/src/strategy_gpu_runtime.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_ir.rs` | `backend-file-reviews/crates/app_core/src/strategy_ir.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_plan.rs` | `backend-file-reviews/crates/app_core/src/strategy_plan.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_presets.rs` | `backend-file-reviews/crates/app_core/src/strategy_presets.rs.review.md` | `Reviewed` |
| `crates/app_core/src/test_support.rs` | `backend-file-reviews/crates/app_core/src/test_support.rs.review.md` | `Reviewed` |
| `crates/app_core/src/timeframe.rs` | `backend-file-reviews/crates/app_core/src/timeframe.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_core.rs` | `backend-file-reviews/crates/app_core/src/validation_core.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_pipeline.rs` | `backend-file-reviews/crates/app_core/src/validation_pipeline.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_shared.rs` | `backend-file-reviews/crates/app_core/src/validation_shared.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_stats.rs` | `backend-file-reviews/crates/app_core/src/validation_stats.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcc.rs` | `backend-file-reviews/crates/app_core/src/vcc.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcc_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/vcc_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcet.rs` | `backend-file-reviews/crates/app_core/src/vcet.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcet_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/vcet_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vemr.rs` | `backend-file-reviews/crates/app_core/src/vemr.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vemr_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/vemr_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vram_ma.rs` | `backend-file-reviews/crates/app_core/src/vram_ma.rs.review.md` | `Reviewed` |

### `crates/app_core/src/kernels` (`2`)

| Source file | Planned packet | Status |
| --- | --- | --- |
| `crates/app_core/src/kernels/double_crossover.cu` | `backend-file-reviews/crates/app_core/src/kernels/double_crossover.cu.review.md` | `Reviewed` |
| `crates/app_core/src/kernels/options_expression.cu` | `backend-file-reviews/crates/app_core/src/kernels/options_expression.cu.review.md` | `Reviewed` |

### `crates/strategies/src` (`4`)

| Source file | Planned packet | Status |
| --- | --- | --- |
| `crates/strategies/src/composite_cpu/mod.rs` | `backend-file-reviews/crates/strategies/src/composite_cpu/mod.rs.review.md` | `Reviewed` |
| `crates/strategies/src/double_ma/mod.rs` | `backend-file-reviews/crates/strategies/src/double_ma/mod.rs.review.md` | `Reviewed` |
| `crates/strategies/src/lib.rs` | `backend-file-reviews/crates/strategies/src/lib.rs.review.md` | `Reviewed` |
| `crates/strategies/src/records.rs` | `backend-file-reviews/crates/strategies/src/records.rs.review.md` | `Reviewed` |

### `crates/optimizer/src` (`4`)

| Source file | Planned packet | Status |
| --- | --- | --- |
| `crates/optimizer/src/grid.rs` | `backend-file-reviews/crates/optimizer/src/grid.rs.review.md` | `Reviewed` |
| `crates/optimizer/src/lib.rs` | `backend-file-reviews/crates/optimizer/src/lib.rs.review.md` | `Reviewed` |
| `crates/optimizer/src/mo.rs` | `backend-file-reviews/crates/optimizer/src/mo.rs.review.md` | `Reviewed` |
| `crates/optimizer/src/stream.rs` | `backend-file-reviews/crates/optimizer/src/stream.rs.review.md` | `Reviewed` |
