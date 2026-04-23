# Second-Pass Review: `crates/app_core/src/dynamic_indicator_adapter.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/dynamic_indicator_adapter.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/dynamic_indicator_adapter.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a public but sharp adapter around prepared CUDA inputs and validated-vs-unchecked paths.

## Stale Check

The live source still exposes:

- `PreparedCudaIndicatorInputs`
- validated and unchecked execution paths
- thread-local aggregate fallback stats

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground contract/error documentation for public validated vs unchecked APIs
   - Additional constraint informed:
     - the dominant issue remains undocumented sharp invariants rather than algorithmic failure

2. The Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why consulted in second pass:
     - re-check unsafe-backed lifetime and invariant expectations around device-row slicing
   - Additional constraint informed:
     - ownership and thread-affinity documentation still matters if the current API breadth remains

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - reassess why silent fallback still needs to stay observable in perf-sensitive code
   - Additional constraint informed:
     - thread-local aggregate stats alone are a weak observability surface

## Delta vs First Pass

### 1. Fallback observability is still too aggregate to attribute a specific CUDA call to a device-path fallback

- Why this matters:
  - normal callers still cannot easily connect a degraded path to one invocation
- Practical implication:
  - remains a bounded visibility improvement target if this surface is touched

### 2. `PreparedCudaIndicatorInputs` exposes a broader public contract than the first pass emphasized

- Why this matters:
  - runtime and device-buffer accessors raise the documentation bar on ownership and thread affinity
- Practical implication:
  - clearer contract docs or narrower visibility still look like the best bounded hardening

## Verification If Changed

- `cargo test -p ta_app_core cpu_adapter_computes_single_output_ma_like_indicator -- --nocapture`
- existing CUDA prepared-input parity tests

## Regression And Feature-Preservation Constraints

- Preserve selective-upload prepared inputs and current keepalive behavior for device-matrix rows
- Preserve validated vs unchecked split unless a broader API redesign is intentional
- Keep fallback observability at least as strong as today

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that public prepared-input breadth and weak per-call fallback observability are still the main risks here.
