# Review Packet: `crates/app_core/src/request_normalization.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/request_normalization.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful internal helper module, but not fully review-ready because the normalization contract is undocumented, sentinel semantics are implicit, and adjacent request-normalization logic remains fragmented across the repo`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Internal-surface`, `Cross-cutting-support`, `Request-contract`, `Semantics-sensitive`, `Low-compute`

## What This File Owns

`request_normalization.rs` owns three small request-shaping helpers reused by the optimization backends:

- path trimming and empty-string elision:
  - `normalize_optional_path`: `crates/app_core/src/request_normalization.rs:1-9`
- top-k / include-all defaulting:
  - `normalize_topk_include_all`: `12-17`
- heatmap-bin upper-bound normalization:
  - `normalize_heatmap_bins`: `19-28`
- file-local unit coverage:
  - `30-58`

It does not own:

- request-builder legality in the frontend
- timeframe normalization helpers repeated in other backend modules
- downstream interpretation of `(top_k, include_all)` or zero heatmap bins

## Classification

- Public surface: no
  - the module is `pub(crate)` from `crates/app_core/src/lib.rs:57`
- Hot path: no
  - called during request setup, not inner-loop compute
- CUDA/runtime-critical: no
- Strategy-local: no
  - shared by Double-MA and composite optimization entry points
- Low-risk support code: only partially
  - compute cost is trivial, but semantics fan out into many request handlers

## Direct Interactions

### Backend interaction map

- Double-MA optimization request shaping:
  - `crates/app_core/src/double_ma.rs:5004`
  - `crates/app_core/src/double_ma.rs:5014`
  - `crates/app_core/src/double_ma.rs:5131`
- Composite optimization request shaping:
  - `crates/app_core/src/composite_cpu.rs:1372`
  - `crates/app_core/src/composite_cpu.rs:2345`
  - `crates/app_core/src/composite_cpu.rs:2349`
  - plus repeated reuse across many composite strategy request paths throughout `composite_cpu.rs`

### Downstream verification impact if changed

- changing path trimming affects export-path handling in Double-MA flows
- changing `(top_k, include_all)` defaults affects result-shortlist behavior in both Double-MA and composite optimizers
- changing zero-versus-nonzero heatmap semantics affects heatmap generation breadth across many composite request entry points

## Standards And Research Log

This packet was written after targeted web research focused on argument-validation posture, behaviorally important documentation for small helpers, and standard-library semantics around `Option` / `Result`-style request normalization.

### Sources consulted

1. Rust API Guidelines dependability guidance, `C-VALIDATE`
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate>
   - Why it was consulted:
     - this file is entirely about normalizing optional user/request inputs
   - Standard informed:
     - helper functions should make malformed, omitted, and intentionally empty inputs distinguishable when the distinction matters

2. Rust API Guidelines documentation guidance, `C-FAILURE`
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why it was consulted:
     - the functions are tiny, but they encode non-obvious sentinel behavior
   - Standard informed:
     - behaviorally important defaulting and fallible normalization should be documented even for internal helpers

3. `std::option::Option`
   - Source: <https://doc.rust-lang.org/std/option/enum.Option.html>
   - Why it was consulted:
     - the file collapses omitted and empty inputs into `None`
   - Standard informed:
     - `Option`-based normalization should be deliberate about when absence and invalidity are treated the same

4. `std::result`
   - Source: <https://doc.rust-lang.org/std/result/>
   - Why it was consulted:
     - `normalize_heatmap_bins` is the only function in the module that reports an actual error
   - Standard informed:
     - the file should make its “default silently” versus “error loudly” split explicit

## Findings

## Correctness And Safety

### 1. The module’s sentinel semantics are entirely implicit

- Evidence:
  - empty or whitespace-only paths become `None`: `1-9`
  - omitted `top_k` becomes `0` and omitted `include_all` becomes `false`: `12-17`
  - omitted `heatmap_bins` also becomes `0`: `19-28`
- Why this matters:
  - these are valid internal conventions, but there is no local statement of what `0` or `None` mean for each caller
  - maintainers must rediscover semantics from downstream consumers
- Smallest justified fix:
  - add rustdoc explaining each helper’s sentinel policy and which values mean “disabled”, “default”, or “keep all”

## API And Naming

### 2. `normalize_topk_include_all` returns a weakly typed pair for a semantics-sensitive policy

- Evidence:
  - the helper returns a bare `(usize, bool)`: `12-17`
  - the result is reused across many large optimization entry points in `double_ma.rs` and `composite_cpu.rs`
- Why this matters:
  - tuple position is cheap, but not self-documenting
  - the relationship between `top_k = 0` and `include_all = true/false` is left to caller convention
- Smallest justified fix:
  - either document the exact meaning of each tuple state or replace it with a tiny named struct in a later pass

## Error Handling

### 3. The file mixes silent normalization and hard rejection without explaining the rule

- Evidence:
  - optional path and top-k helpers never fail: `1-17`
  - `normalize_heatmap_bins` rejects only the upper bound: `19-28`
- Why this matters:
  - callers reading the module cannot infer why oversize heatmap bins are an error while empty paths or zero-valued shortlist requests are treated as valid normalized states
- Smallest justified fix:
  - document that only “out of supported range” is treated as an error here, while omitted/empty values are normalized intentionally

## Module Boundaries And Technical Debt

### 4. Request normalization remains fragmented across the backend

- Evidence:
  - this module handles path, top-k, and heatmap bins
  - separate `normalize_timeframe_label` helpers still live in:
    - `src-tauri/src/commands_meta.rs:2403`
    - `src-tauri/src/commands_campaign.rs:711`
    - `src-tauri/src/commands_ml_signal.rs:21`
    - `src-tauri/src/commands_validation.rs:94`
    - `crates/app_core/src/meta_select.rs:799`
    - `crates/app_core/src/ml_confidence.rs:393`
- Why this matters:
  - the module’s existence suggests a consolidation point, but in practice normalization policy is still scattered
  - a future maintainer could over-generalize this file without realizing the blast radius
- Smallest justified fix:
  - document this file as a narrow helper module for current optimization request primitives
  - defer broader normalization consolidation to a cross-file pass

## Docs And Examples

### 5. The file has no rustdoc despite encoding cross-cutting request behavior

- Evidence:
  - no rustdoc is present on any function: `1-28`
- Why this matters:
  - the compute work is trivial, but the semantic work is not
  - missing documentation increases the risk of silent drift during “cleanup” edits
- Smallest justified fix:
  - add concise rustdoc to all three helpers

## Tests And Missing Verification

### 6. Tests cover the happy path, but not the ambiguous states that matter most

- Evidence:
  - current coverage exists for trimming, defaults, and upper-bound rejection: `35-58`
- Missing coverage:
  - explicit `Some(0)` versus omitted `None` for `top_k`
  - mixed states such as `(Some(0), Some(true))`
  - explicit `Some(0)` heatmap bins as distinct from omitted bins
- Smallest justified fix:
  - add narrow tests around explicit-zero semantics before changing helper signatures or normalization policy

## Performance, Allocation, And CUDA Concerns

### 7. Performance is not the issue here; semantic drift is

- Evidence:
  - the module is tiny and allocation-light except for the intentional trimmed-string path case
- Safe-now conclusion:
  - do not optimize this file for speed
- Real risk:
  - accidental behavior changes in defaulting rules would have a much larger impact than any micro-optimization win

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc
  - document the meaning of zero/sentinel outputs
  - add explicit-zero tests
- Defer:
  - broad normalization consolidation across commands and app-core modules
  - replacing every pair/sentinel with richer request types without a wider coordinated pass

## Technical-Debt Reduction Register

- Safe now:
  - local documentation hardening
  - file-local tests for ambiguous states
- Defer:
  - cross-backend normalization unification
- Do not casually remove:
  - the current upper-bound check on heatmap bins
  - the current whitespace-trimming behavior for optional paths

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - none

## Recommended Action

`Small surgical refactor`

This file is already small and adequate. The useful work here is to make the normalization policy explicit and to lock in the ambiguous zero/default states with tests before a later cleanup pass widens the blast radius.

## Exact Local Verification Required If Changed

1. File-local helper coverage:
   - `cargo test -p ta_app_core request_normalization::tests:: -- --nocapture`
2. Representative Double-MA request path:
   - `cargo test -p ta_app_core double_ma -- --nocapture`
3. Representative composite request path:
   - `cargo test -p ta_app_core composite_cpu -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core request_normalization::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change zero/default semantics casually; they are already consumed by both Double-MA and composite request handlers
- Do not treat explicit empty-path input differently unless export-path callers are verified together
- Do not broaden this file into a global normalization “cleanup” target without coordinating all duplicated normalization helpers first
