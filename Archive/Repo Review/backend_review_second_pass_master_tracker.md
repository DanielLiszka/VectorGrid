# Backend Review Second-Pass Master Tracker

Last updated: `2026-04-14`

## Purpose

This document is the master tracking index for the canonical backend review second-pass campaign.

The second pass is not a rewrite of the first pass and it is not generic rewording. It exists to perform a deeper, file-specific follow-up review against every existing first-pass backend review packet.

The campaign goal is to revisit each already-reviewed backend file with more targeted web research and a broader search for issues or opportunities that may have been missed during the first-pass packet creation, including:

- edge cases that were not surfaced in the first pass
- correctness or safety gaps that needed deeper source or standards research
- performance considerations that were left undocumented or deferred too broadly
- opportunities to reduce technical debt without changing behavior or harming hot-path performance
- observability, diagnostics, or invariant-clarity gaps that matter for safe maintenance
- ways to strengthen or refine the first-pass action bucket with better evidence

The second pass must remain compatible with the original campaign doctrine:

- preserve current product capability
- avoid regressions, silent semantic drift, and disguised feature loss
- prefer bounded, evidence-backed recommendations over aesthetic churn
- distinguish safe structural hardening from benchmark-sensitive or semantics-sensitive change

This tracker is the source of truth for:

- which first-pass backend review packets are in scope for a second-pass review
- where each second-pass packet should live under `Repo Review/backend-file-reviews-second-pass/`
- per-packet second-pass review status as the campaign progresses

## Scope

The second-pass campaign is one-to-one with the existing canonical first-pass backend review set.

It is intentionally limited to the same owned backend runtime source surfaces as the first-pass campaign:

- `src-tauri/src/**/*.rs`, excluding `src-tauri/src/bin/**`
- `crates/app_core/src/**/*.rs`, excluding `crates/app_core/src/bin/**`
- `crates/app_core/src/kernels/*.cu`
- `crates/strategies/src/**/*.rs`
- `crates/optimizer/src/**/*.rs`

The second-pass unit of work is:

- one existing first-pass review packet
- one corresponding live backend source file
- one new second-pass packet

## Out Of Scope

These surfaces remain intentionally excluded from this tracker:

- frontend code under `web/`
- vendor code under `vendor/`
- generated CUDA/PTX artifacts such as `*.ptx`
- benches, tests, examples, and `build.rs`
- helper binaries under `src-tauri/src/bin/**` and `crates/app_core/src/bin/**`
- historical or research-only folders such as `SASS research/`, `REPO_Improvement_Epics/`, `options_research/`, and `CUDA kernel review/`

## Second-Pass Quality Bar

Every second-pass packet must be written against the following bar:

- it must start from the first-pass packet rather than ignoring it
- it must reopen the live source file rather than treating the first-pass packet as sufficient
- it must use new, targeted web research aimed at gaps, risks, or opportunities specific to that file
- it must prefer primary or official sources over tertiary commentary
- it must expand the review scope only where the evidence supports doing so
- it must clearly separate newly discovered issues from first-pass findings that still stand unchanged
- it must preserve semantics and feature coverage unless a correctness bug is clearly identified
- it must not use “second pass” as an excuse for broader speculative refactor advice

## Second-Pass Doctrine

The default posture of each second-pass packet is:

- validate whether the first-pass packet missed anything material
- resolve open questions or assumptions where possible
- strengthen, narrow, or revise first-pass recommendations when new evidence justifies it
- identify additional low-regression improvements, if any exist
- explicitly say when the first-pass packet remains directionally correct and no material delta was found

The second pass should prefer:

- additive findings over duplicated prose
- deeper contract and edge-case analysis over stylistic commentary
- evidence-backed refinement over bucket inflation
- small, safe opportunities that preserve performance posture
- explicit treatment of benchmark-sensitive paths instead of vague caution

The second pass should avoid:

- restating the first-pass packet without adding new evidence
- inventing issues that are not supported by code or standards
- turning “more research” into broad redesign advice
- relaxing prior regression or feature-preservation constraints

## Scope Summary

- Total in-scope second-pass packets: `116`
- Existing first-pass packets expected: `116`
- `src-tauri/src`: `32`
- `crates/app_core/src`: `74`
- `crates/app_core/src/kernels`: `2`
- `crates/strategies/src`: `4`
- `crates/optimizer/src`: `4`

## Canonical Inventory Rule

The canonical second-pass inventory is defined by the existing first-pass packet set under:

- `Repo Review/backend-file-reviews/`

Each eligible first-pass packet must correspond one-to-one with a live backend source file in the canonical backend source roots.

The second-pass tracker should therefore stay aligned with both:

- the canonical live backend source inventory
- the canonical first-pass review packet inventory

Notes:

- the second pass should not introduce packet rows for files that were out of scope in the first pass
- if the backend source inventory changes later, the first-pass and second-pass trackers must both be audited
- if a first-pass packet is later found stale because the source changed, that staleness must be called out during second-pass work

## Completeness Audit

This tracker was verified against the current first-pass review packet inventory on `2026-04-14`.

Audit result:

- expected first-pass review packets: `116`
- second-pass tracker rows: `116`
- missing from tracker: `0`
- extra in tracker: `0`

The tracker currently exactly matches the existing first-pass review set.

Reproduction command used for the audit:

```powershell
$expected = Get-ChildItem 'Repo Review\backend-file-reviews' -Recurse -File -Filter *.review.md |
  Sort-Object FullName -Unique |
  ForEach-Object {
    $rel = $_.FullName.Replace((Get-Location).Path + '\Repo Review\backend-file-reviews\','') -replace '\\','/'
    "backend-file-reviews-second-pass/$rel"
  }

$doc = Get-Content 'Repo Review\backend_review_second_pass_master_tracker.md'
$listed = foreach ($line in $doc) {
  if ($line -match '^\| `[^`]+` \| `backend-file-reviews/[^`]+` \| `([^`]+)` \| `[^`]+` \|$') {
    $matches[1]
  }
}

Compare-Object -ReferenceObject $expected -DifferenceObject $listed
```

If the comparison produces no rows, the tracker matches the existing first-pass review set exactly.

## Packet Convention

The second pass uses a parallel packet tree rather than modifying first-pass packets in place.

Conventions:

- first-pass packet:
  - `Repo Review/backend-file-reviews/{repo-relative-source-path}`
- second-pass packet:
  - `Repo Review/backend-file-reviews-second-pass/{repo-relative-source-path}`

Example:

- source:
  - `crates/app_core/src/double_ma.rs`
- first pass:
  - `Repo Review/backend-file-reviews/crates/app_core/src/double_ma.rs.review.md`
- second pass:
  - `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma.rs.review.md`

## Mandatory Pre-Packet Workflow

Before writing any second-pass packet, the reviewer must do all of the following:

1. Reopen the live backend source file.
2. Reopen the corresponding first-pass packet.
3. Check whether the source appears changed or materially stale relative to the first-pass packet.
4. Identify what the first-pass packet did not fully resolve:
   - open questions
   - assumptions
   - conservative deferrals
   - missing edge-case analysis
   - lightly treated performance or technical-debt topics
5. Perform targeted web research aimed at those unresolved areas.

The second pass is invalid if it skips the first-pass packet or skips targeted web research.

## Mandatory Second-Pass Research

Before writing each second-pass packet, there must be an explicit web research phase tailored to that specific file and to the specific gaps left by the first pass.

Minimum expectations:

- consult official or primary sources relevant to the file’s missed or weakly-covered concerns
- prefer new sources beyond the first-pass packet when practical
- revisit first-pass sources when a narrower reading helps resolve an open question
- deepen research for unsafe code, FFI, raw-pointer paths, CUDA runtime behavior, kernels, public API surfaces, math/statistics behavior, or performance-sensitive hot paths
- record not just which sources were consulted, but why the second-pass packet needed them

The second-pass research phase should explicitly ask:

- is there a correctness edge case the first pass did not cover?
- is there a standards-backed clarification that could shrink a previous defer bucket?
- is there a safe-now technical-debt reduction opportunity that does not threaten hot-path performance?
- is there a performance-sensitive area that can be narrowed more precisely than “do not touch yet”?
- were there missing diagnostics, test gaps, or observability gaps that the first pass underweighted?

Each second-pass packet should include a research log section with:

- source title
- link
- why it was consulted for the second pass
- what additional constraint, standard, or opportunity it informed

## Baseline Standards Corpus

These remain baseline sources for the second-pass campaign when relevant:

- Rust API Guidelines checklist:
  - `https://rust-lang.github.io/api-guidelines/checklist.html`
- Rust API Guidelines documentation guidance:
  - `https://rust-lang.github.io/api-guidelines/documentation.html`
- Rust API Guidelines naming guidance:
  - `https://rust-lang.github.io/api-guidelines/naming.html`
- Clippy documentation:
  - `https://doc.rust-lang.org/clippy/`
- Rustonomicon:
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

The second pass should also bring in narrower file-specific sources when relevant, such as:

- standard-library docs for data structures or synchronization used in the file
- crate docs for public dependencies the first pass only referenced indirectly
- primary algorithm/statistics papers where the first pass stayed at a high level
- vendor docs for runtime or kernel behavior when the first pass stayed conservative

## Second-Pass Deliverable Standard

Each second-pass packet should be a reusable additive artifact, not a duplicate of the first pass.

At minimum, each second-pass packet should include:

- file identity
- source file path
- first-pass packet path
- a short first-pass recap:
  - prior action bucket
  - main prior concerns
  - known open questions or deferrals
- stale-check note:
  - whether the live source still appears aligned with the first-pass packet
- second-pass research log
- new or refined findings only
- explicit statement on whether the first-pass action bucket changes
- exact verification required if any newly surfaced recommendation were implemented
- explicit regression and feature-preservation constraints

Useful strengthening sections for the second pass:

- `Delta vs first pass`
- `Resolved open questions`
- `New edge cases`
- `New performance clarification`
- `New technical-debt opportunities with low regression risk`
- `Refined do-not-touch-yet boundaries`
- `Additional observability or diagnostics gaps`
- `No material delta from first pass`, when that is the honest conclusion

## Regression And Feature-Preservation Rule

The intended outcome of this second-pass campaign is deeper review coverage without lowering the regression bar.

That means:

- second-pass packets must preserve the same no-regression and no-feature-loss posture as the first pass
- newly discovered improvements must still be bounded and evidence-backed
- a second-pass packet may strengthen a prior defer into a safe-now recommendation only if the new evidence supports that move clearly
- a second-pass packet may also conclude that the first-pass caution was correct and should stand unchanged

## Status Legend

- `Not started`: second-pass packet not yet written
- `In progress`: second-pass packet being written
- `Reviewed`: second-pass packet complete
- `Needs update`: second-pass packet exists but is stale after source or first-pass changes

Recommended optional secondary tags inside second-pass packets:

- `New edge case`
- `Bucket changed`
- `No material delta`
- `Benchmark-sensitive`
- `Semantics-sensitive`

## Maintenance Rule

Whenever the canonical backend source set changes or the first-pass review tree changes materially:

- update the first-pass tracker first if needed
- rerun the first-pass completeness audit if the inventory changed
- rerun the second-pass completeness audit
- update the `Last updated` date in this document
- mark any affected second-pass rows as `Needs update`

## Tracker

### `src-tauri/src` (`32`)

| Source file | First-pass packet | Planned second-pass packet | Status |
| --- | --- | --- | --- |
| `src-tauri/src/commands_analysis.rs` | `backend-file-reviews/src-tauri/src/commands_analysis.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_analysis.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_artifacts.rs` | `backend-file-reviews/src-tauri/src/commands_artifacts.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_artifacts.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_campaign.rs` | `backend-file-reviews/src-tauri/src/commands_campaign.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_campaign.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_data.rs` | `backend-file-reviews/src-tauri/src/commands_data.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_data.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_dynamic.rs` | `backend-file-reviews/src-tauri/src/commands_dynamic.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_dynamic.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_execution.rs` | `backend-file-reviews/src-tauri/src/commands_execution.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_execution.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_meta.rs` | `backend-file-reviews/src-tauri/src/commands_meta.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_meta.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_ml_signal.rs` | `backend-file-reviews/src-tauri/src/commands_ml_signal.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_ml_signal.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_optimize.rs` | `backend-file-reviews/src-tauri/src/commands_optimize.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_optimize.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_options_data.rs` | `backend-file-reviews/src-tauri/src/commands_options_data.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_options_data.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_options_expression.rs` | `backend-file-reviews/src-tauri/src/commands_options_expression.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_options_expression.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports.rs` | `backend-file-reviews/src-tauri/src/commands_reports.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_reports.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_api.rs` | `backend-file-reviews/src-tauri/src/commands_reports_api.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_reports_api.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_csv.rs` | `backend-file-reviews/src-tauri/src/commands_reports_csv.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_reports_csv.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_meta_xlsx.rs` | `backend-file-reviews/src-tauri/src/commands_reports_meta_xlsx.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_reports_meta_xlsx.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_shared.rs` | `backend-file-reviews/src-tauri/src/commands_reports_shared.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_reports_shared.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_validation_common.rs` | `backend-file-reviews/src-tauri/src/commands_reports_validation_common.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_reports_validation_common.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_reports_validation_xlsx.rs` | `backend-file-reviews/src-tauri/src/commands_reports_validation_xlsx.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_reports_validation_xlsx.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_system.rs` | `backend-file-reviews/src-tauri/src/commands_system.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_system.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands_validation.rs` | `backend-file-reviews/src-tauri/src/commands_validation.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands_validation.rs.review.md` | `Reviewed` |
| `src-tauri/src/commands.rs` | `backend-file-reviews/src-tauri/src/commands.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/commands.rs.review.md` | `Reviewed` |
| `src-tauri/src/data_cache.rs` | `backend-file-reviews/src-tauri/src/data_cache.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/data_cache.rs.review.md` | `Reviewed` |
| `src-tauri/src/lib.rs` | `backend-file-reviews/src-tauri/src/lib.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/lib.rs.review.md` | `Reviewed` |
| `src-tauri/src/ma_registry.rs` | `backend-file-reviews/src-tauri/src/ma_registry.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/ma_registry.rs.review.md` | `Reviewed` |
| `src-tauri/src/main.rs` | `backend-file-reviews/src-tauri/src/main.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/main.rs.review.md` | `Reviewed` |
| `src-tauri/src/metric_surface.rs` | `backend-file-reviews/src-tauri/src/metric_surface.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/metric_surface.rs.review.md` | `Reviewed` |
| `src-tauri/src/options_parent_adapters.rs` | `backend-file-reviews/src-tauri/src/options_parent_adapters.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/options_parent_adapters.rs.review.md` | `Reviewed` |
| `src-tauri/src/polygon.rs` | `backend-file-reviews/src-tauri/src/polygon.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/polygon.rs.review.md` | `Reviewed` |
| `src-tauri/src/run_artifacts.rs` | `backend-file-reviews/src-tauri/src/run_artifacts.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/run_artifacts.rs.review.md` | `Reviewed` |
| `src-tauri/src/settings.rs` | `backend-file-reviews/src-tauri/src/settings.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/settings.rs.review.md` | `Reviewed` |
| `src-tauri/src/state.rs` | `backend-file-reviews/src-tauri/src/state.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/state.rs.review.md` | `Reviewed` |
| `src-tauri/src/validation_common.rs` | `backend-file-reviews/src-tauri/src/validation_common.rs.review.md` | `backend-file-reviews-second-pass/src-tauri/src/validation_common.rs.review.md` | `Reviewed` |

### `crates/app_core/src` (`74`)

| Source file | First-pass packet | Planned second-pass packet | Status |
| --- | --- | --- | --- |
| `crates/app_core/src/alloc_policy.rs` | `backend-file-reviews/crates/app_core/src/alloc_policy.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/alloc_policy.rs.review.md` | `Reviewed` |
| `crates/app_core/src/asr.rs` | `backend-file-reviews/crates/app_core/src/asr.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/asr.rs.review.md` | `Reviewed` |
| `crates/app_core/src/asr_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/asr_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/asr_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/carm.rs` | `backend-file-reviews/crates/app_core/src/carm.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/carm.rs.review.md` | `Reviewed` |
| `crates/app_core/src/carm_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/carm_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/carm_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cdmc.rs` | `backend-file-reviews/crates/app_core/src/cdmc.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/cdmc.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cdmc_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/cdmc_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/cdmc_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_cpu.rs` | `backend-file-reviews/crates/app_core/src/composite_cpu.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/composite_cpu.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_gpu_metric_decode.rs` | `backend-file-reviews/crates/app_core/src/composite_gpu_metric_decode.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/composite_gpu_metric_decode.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_mo.rs` | `backend-file-reviews/crates/app_core/src/composite_mo.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/composite_mo.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_profile_route.rs` | `backend-file-reviews/crates/app_core/src/composite_profile_route.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/composite_profile_route.rs.review.md` | `Reviewed` |
| `crates/app_core/src/composite_validate.rs` | `backend-file-reviews/crates/app_core/src/composite_validate.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/composite_validate.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cscv_pbo.rs` | `backend-file-reviews/crates/app_core/src/cscv_pbo.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/cscv_pbo.rs.review.md` | `Reviewed` |
| `crates/app_core/src/csv_compat.rs` | `backend-file-reviews/crates/app_core/src/csv_compat.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/csv_compat.rs.review.md` | `Reviewed` |
| `crates/app_core/src/cuda_device.rs` | `backend-file-reviews/crates/app_core/src/cuda_device.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/cuda_device.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma.rs` | `backend-file-reviews/crates/app_core/src/double_ma.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_backend.rs` | `backend-file-reviews/crates/app_core/src/double_ma_backend.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_backend.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_ma_kind.rs` | `backend-file-reviews/crates/app_core/src/double_ma_ma_kind.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_ma_kind.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_mode.rs` | `backend-file-reviews/crates/app_core/src/double_ma_mode.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_mode.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_result.rs` | `backend-file-reviews/crates/app_core/src/double_ma_result.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_result.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_strategy.rs` | `backend-file-reviews/crates/app_core/src/double_ma_strategy.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_strategy.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_test_support.rs` | `backend-file-reviews/crates/app_core/src/double_ma_test_support.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_test_support.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_validate.rs` | `backend-file-reviews/crates/app_core/src/double_ma_validate.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_validate.rs.review.md` | `Reviewed` |
| `crates/app_core/src/double_ma_vdps.rs` | `backend-file-reviews/crates/app_core/src/double_ma_vdps.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/double_ma_vdps.rs.review.md` | `Reviewed` |
| `crates/app_core/src/dynamic_indicator_adapter.rs` | `backend-file-reviews/crates/app_core/src/dynamic_indicator_adapter.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/dynamic_indicator_adapter.rs.review.md` | `Reviewed` |
| `crates/app_core/src/gpu_backtest_kernel.rs` | `backend-file-reviews/crates/app_core/src/gpu_backtest_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/indicator_catalog.rs` | `backend-file-reviews/crates/app_core/src/indicator_catalog.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/indicator_catalog.rs.review.md` | `Reviewed` |
| `crates/app_core/src/indicator_factory.rs` | `backend-file-reviews/crates/app_core/src/indicator_factory.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/indicator_factory.rs.review.md` | `Reviewed` |
| `crates/app_core/src/iterative_sim.rs` | `backend-file-reviews/crates/app_core/src/iterative_sim.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/iterative_sim.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kqsr.rs` | `backend-file-reviews/crates/app_core/src/kqsr.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/kqsr.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kqsr_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/kqsr_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/kqsr_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kvdf.rs` | `backend-file-reviews/crates/app_core/src/kvdf.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/kvdf.rs.review.md` | `Reviewed` |
| `crates/app_core/src/kvdf_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/kvdf_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/kvdf_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/lib.rs` | `backend-file-reviews/crates/app_core/src/lib.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/lib.rs.review.md` | `Reviewed` |
| `crates/app_core/src/ma_policy.rs` | `backend-file-reviews/crates/app_core/src/ma_policy.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/ma_policy.rs.review.md` | `Reviewed` |
| `crates/app_core/src/matrix_index.rs` | `backend-file-reviews/crates/app_core/src/matrix_index.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/matrix_index.rs.review.md` | `Reviewed` |
| `crates/app_core/src/meta_select.rs` | `backend-file-reviews/crates/app_core/src/meta_select.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/meta_select.rs.review.md` | `Reviewed` |
| `crates/app_core/src/mkcs.rs` | `backend-file-reviews/crates/app_core/src/mkcs.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/mkcs.rs.review.md` | `Reviewed` |
| `crates/app_core/src/mkcs_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/mkcs_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/mkcs_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/ml_confidence.rs` | `backend-file-reviews/crates/app_core/src/ml_confidence.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/ml_confidence.rs.review.md` | `Reviewed` |
| `crates/app_core/src/ml_signal.rs` | `backend-file-reviews/crates/app_core/src/ml_signal.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/ml_signal.rs.review.md` | `Reviewed` |
| `crates/app_core/src/nvtx.rs` | `backend-file-reviews/crates/app_core/src/nvtx.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/nvtx.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_data_store.rs` | `backend-file-reviews/crates/app_core/src/options_data_store.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/options_data_store.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_expression.rs` | `backend-file-reviews/crates/app_core/src/options_expression.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/options_expression.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_expression_gpu.rs` | `backend-file-reviews/crates/app_core/src/options_expression_gpu.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/options_expression_gpu.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_signal_manifest.rs` | `backend-file-reviews/crates/app_core/src/options_signal_manifest.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/options_signal_manifest.rs.review.md` | `Reviewed` |
| `crates/app_core/src/options_types.rs` | `backend-file-reviews/crates/app_core/src/options_types.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/options_types.rs.review.md` | `Reviewed` |
| `crates/app_core/src/prescan_predictive.rs` | `backend-file-reviews/crates/app_core/src/prescan_predictive.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/prescan_predictive.rs.review.md` | `Reviewed` |
| `crates/app_core/src/progress.rs` | `backend-file-reviews/crates/app_core/src/progress.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/progress.rs.review.md` | `Reviewed` |
| `crates/app_core/src/request_normalization.rs` | `backend-file-reviews/crates/app_core/src/request_normalization.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/request_normalization.rs.review.md` | `Reviewed` |
| `crates/app_core/src/rocb.rs` | `backend-file-reviews/crates/app_core/src/rocb.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/rocb.rs.review.md` | `Reviewed` |
| `crates/app_core/src/rocb_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/rocb_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/rocb_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/sharpe_stats.rs` | `backend-file-reviews/crates/app_core/src/sharpe_stats.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/sharpe_stats.rs.review.md` | `Reviewed` |
| `crates/app_core/src/standalone_gpu_runtime.rs` | `backend-file-reviews/crates/app_core/src/standalone_gpu_runtime.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/standalone_gpu_runtime.rs.review.md` | `Reviewed` |
| `crates/app_core/src/stats_common.rs` | `backend-file-reviews/crates/app_core/src/stats_common.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/stats_common.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_dynamic.rs` | `backend-file-reviews/crates/app_core/src/strategy_dynamic.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/strategy_dynamic.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_gpu_program.rs` | `backend-file-reviews/crates/app_core/src/strategy_gpu_program.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/strategy_gpu_program.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_gpu_runtime.rs` | `backend-file-reviews/crates/app_core/src/strategy_gpu_runtime.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/strategy_gpu_runtime.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_ir.rs` | `backend-file-reviews/crates/app_core/src/strategy_ir.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/strategy_ir.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_plan.rs` | `backend-file-reviews/crates/app_core/src/strategy_plan.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/strategy_plan.rs.review.md` | `Reviewed` |
| `crates/app_core/src/strategy_presets.rs` | `backend-file-reviews/crates/app_core/src/strategy_presets.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/strategy_presets.rs.review.md` | `Reviewed` |
| `crates/app_core/src/test_support.rs` | `backend-file-reviews/crates/app_core/src/test_support.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/test_support.rs.review.md` | `Reviewed` |
| `crates/app_core/src/timeframe.rs` | `backend-file-reviews/crates/app_core/src/timeframe.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/timeframe.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_core.rs` | `backend-file-reviews/crates/app_core/src/validation_core.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/validation_core.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_pipeline.rs` | `backend-file-reviews/crates/app_core/src/validation_pipeline.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/validation_pipeline.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_shared.rs` | `backend-file-reviews/crates/app_core/src/validation_shared.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/validation_shared.rs.review.md` | `Reviewed` |
| `crates/app_core/src/validation_stats.rs` | `backend-file-reviews/crates/app_core/src/validation_stats.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/validation_stats.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcc.rs` | `backend-file-reviews/crates/app_core/src/vcc.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/vcc.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcc_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/vcc_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/vcc_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcet.rs` | `backend-file-reviews/crates/app_core/src/vcet.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/vcet.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vcet_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/vcet_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/vcet_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vemr.rs` | `backend-file-reviews/crates/app_core/src/vemr.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/vemr.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vemr_gpu_kernel.rs` | `backend-file-reviews/crates/app_core/src/vemr_gpu_kernel.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/vemr_gpu_kernel.rs.review.md` | `Reviewed` |
| `crates/app_core/src/vram_ma.rs` | `backend-file-reviews/crates/app_core/src/vram_ma.rs.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/vram_ma.rs.review.md` | `Reviewed` |

### `crates/app_core/src/kernels` (`2`)

| Source file | First-pass packet | Planned second-pass packet | Status |
| --- | --- | --- | --- |
| `crates/app_core/src/kernels/double_crossover.cu` | `backend-file-reviews/crates/app_core/src/kernels/double_crossover.cu.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/kernels/double_crossover.cu.review.md` | `Reviewed` |
| `crates/app_core/src/kernels/options_expression.cu` | `backend-file-reviews/crates/app_core/src/kernels/options_expression.cu.review.md` | `backend-file-reviews-second-pass/crates/app_core/src/kernels/options_expression.cu.review.md` | `Reviewed` |

### `crates/strategies/src` (`4`)

| Source file | First-pass packet | Planned second-pass packet | Status |
| --- | --- | --- | --- |
| `crates/strategies/src/composite_cpu/mod.rs` | `backend-file-reviews/crates/strategies/src/composite_cpu/mod.rs.review.md` | `backend-file-reviews-second-pass/crates/strategies/src/composite_cpu/mod.rs.review.md` | `Reviewed` |
| `crates/strategies/src/double_ma/mod.rs` | `backend-file-reviews/crates/strategies/src/double_ma/mod.rs.review.md` | `backend-file-reviews-second-pass/crates/strategies/src/double_ma/mod.rs.review.md` | `Reviewed` |
| `crates/strategies/src/lib.rs` | `backend-file-reviews/crates/strategies/src/lib.rs.review.md` | `backend-file-reviews-second-pass/crates/strategies/src/lib.rs.review.md` | `Reviewed` |
| `crates/strategies/src/records.rs` | `backend-file-reviews/crates/strategies/src/records.rs.review.md` | `backend-file-reviews-second-pass/crates/strategies/src/records.rs.review.md` | `Reviewed` |

### `crates/optimizer/src` (`4`)

| Source file | First-pass packet | Planned second-pass packet | Status |
| --- | --- | --- | --- |
| `crates/optimizer/src/grid.rs` | `backend-file-reviews/crates/optimizer/src/grid.rs.review.md` | `backend-file-reviews-second-pass/crates/optimizer/src/grid.rs.review.md` | `Reviewed` |
| `crates/optimizer/src/lib.rs` | `backend-file-reviews/crates/optimizer/src/lib.rs.review.md` | `backend-file-reviews-second-pass/crates/optimizer/src/lib.rs.review.md` | `Reviewed` |
| `crates/optimizer/src/mo.rs` | `backend-file-reviews/crates/optimizer/src/mo.rs.review.md` | `backend-file-reviews-second-pass/crates/optimizer/src/mo.rs.review.md` | `Reviewed` |
| `crates/optimizer/src/stream.rs` | `backend-file-reviews/crates/optimizer/src/stream.rs.review.md` | `backend-file-reviews-second-pass/crates/optimizer/src/stream.rs.review.md` | `Reviewed` |
