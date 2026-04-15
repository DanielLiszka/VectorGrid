# Second-Pass Review: `crates/app_core/src/double_ma.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as one of the highest-value shared runtime owners in the repo, with route policy, cache identity, and public DTO behavior making even small changes semantics-sensitive.

## Stale Check

The live source still owns:

- public Double-MA execution and route selection
- follow-up/drilldown/local-stability flows
- cache materialization and preferred-device propagation

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - re-check public argument validation and unsurprising behavior expectations
   - Additional constraint informed:
     - preferred-device rewriting and follow-up request shaping are public contract behavior, not only internal plumbing

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground fallback and cache-behavior expectations
   - Additional constraint informed:
     - cache identity and device-preference propagation need to be truthful and documented if they are kept

3. `thread_local!` and `OnceLock`
   - Sources:
     - <https://doc.rust-lang.org/std/macro.thread_local.html>
     - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - review follow-up-cache ownership and lifecycle assumptions
   - Additional constraint informed:
     - any follow-up cache tightening should preserve thread-local ownership rather than turning it into a shared stale-data source

4. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep GPU cache/materialization changes benchmark-aware
   - Additional constraint informed:
     - route and cache cleanup remains performance-sensitive

## Delta vs First Pass

### 1. Explicit non-`Auto` GPU `device_id` still looks vulnerable to accidental overwrite during preferred-device propagation

- Why this matters:
  - `with_preferred_cuda_device()` is still called before route resolution and can rewrite stronger caller intent than intended
- Practical implication:
  - this remains a real bounded contract bug candidate if touched

### 2. Follow-up APIs still look easier to misuse than the resolved parameter type suggests

- Why this matters:
  - drilldown/local-stability still consult top-level MA params even though `DoubleMaParamsResolved` carries nested copies that appear sufficient
- Practical implication:
  - follow-up request shaping remains a real API footgun even if the runtime still works for normal paths

## Verification If Changed

- targeted `double_ma.rs` tests for explicit device preservation
- targeted drilldown/local-stability round-trip tests built directly from `DoubleMaParamsResolved`
- targeted stale follow-up-cache invalidation tests
- `cargo test -p ta_app_core double_ma -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve explicit non-`Auto` GPU `device_id` selections
- Keep `normalize_optional_path` and exact-output forcing aligned with the rest of the stack
- Strengthen follow-up cache identity without turning it into a stale-data source
- Preserve public DTO compatibility unless a broader API revision is intentional

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens two still-real issues: explicit GPU device choice can still be overwritten too early, and the follow-up APIs still expose a request shape that is easier to misuse than the resolved parameter type suggests.
