# Second-Pass Review: `crates/app_core/src/double_ma_validate.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_validate.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_validate.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a broad, contract-heavy validation owner and deferred larger cleanup because wrapper-layer responsibilities and app-core responsibilities are still interleaved.

## Stale Check

The live source still owns:

- app-core validation execution
- route-sensitive cross-ticker behavior
- validation-context and runtime-ms shaping

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - reassess public DTO argument validation
   - Additional constraint informed:
     - direct app-core callers still see a stricter cross-ticker contract than wrapper-mediated callers

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground error and ownership semantics
   - Additional constraint informed:
     - `validation_context` and `runtime_ms` still carry split semantics across layers

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep route gating and fallback policy benchmark-aware
   - Additional constraint informed:
     - route-sensitive capability gates should remain explicit and measured

## Delta vs First Pass

### 1. The cross-ticker contract is still stricter for direct app-core callers than for wrapper callers

- Why this matters:
  - wrapper layers sanitize or meaningfully apply cross-ticker later, while the app-core path still rejects on presence
- Practical implication:
  - this remains a true layer-boundary design issue, not small cleanup

### 2. `runtime_ms` continues to have split meaning across core and wrapper layers

- Why this matters:
  - Tauri-side cross-ticker work can overwrite or extend the runtime attribution
- Practical implication:
  - preserve the defer bucket

## Verification If Changed

- `cargo test -p ta_app_core double_ma_validation -- --nocapture`
- direct tests for app-core `cross_ticker` handling
- direct tests for `validation_context` semantics
- direct tests for `runtime_ms` meaning across wrapper-applied cross-ticker flows

## Regression And Feature-Preservation Constraints

- Preserve existing DTO shapes until a deliberate layer split happens
- Do not break wrapper-owned cross-ticker application
- Preserve candidate-level MA-param override behavior

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly clarifies that cross-ticker and runtime attribution are still split across layers in a way that resists safe local cleanup.
