# Second-Pass Review: `crates/app_core/src/cdmc.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cdmc.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/cdmc.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already suspected this strategy had a semantics-sensitive regime-mapping issue, but kept the bucket small pending a tighter read of the upstream indicator contract.

## Stale Check

The live source still:

- maps Correlation Cycle state into trend/reversion regime behavior
- hardcodes the correlation-cycle threshold
- shares CPU/GPU semantics with sibling kernel paths

The second pass found enough evidence to raise the bucket.

## Second-Pass Research Log

1. VectorTA Correlation Cycle docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/correlation_cycle/>
   - Why consulted in second pass:
     - pin down the intended meaning of `state`
   - Additional constraint informed:
     - documented `state` values are `+1` up, `-1` down, `0` cycling/indeterminate

2. VectorTA MAMA docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/mama/>
   - Why consulted in second pass:
     - verify whether MAMA parameter handling is the sharper issue
   - Additional constraint informed:
     - MAMA validation already looks aligned; regime interpretation is the bigger risk

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - reassess whether hidden fixed strategy constants deserve public-surface attention
   - Additional constraint informed:
     - the hardcoded threshold is a real hidden strategy policy choice

## Delta vs First Pass

### 1. Regime interpretation now looks like a probable core semantics issue, not just a small cleanup

- Evidence:
  - the live mapping treats `state >= 0` as trend regime and only `< 0` as reversion regime
  - the upstream docs describe `0` as cycling/indeterminate, not trending
- Why this matters:
  - this can alter actual strategy behavior, not only diagnostics or labels
- Practical implication:
  - the bucket should move to `Larger refactor but defer`

### 2. The fixed `threshold = 9.0` choice is now part of the same review surface

- Practical implication:
  - any semantics fix likely needs to decide whether that threshold is deliberate product policy or a missing public knob

## Verification If Changed

- `cargo test -p ta_app_core cdmc -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve CPU/GPU parity across `cdmc.rs`, `cdmc_gpu_kernel.rs`, and `double_crossover.cu`
- Do not silently change regime mapping without targeted parity review and product signoff
- Treat threshold exposure or reinterpretation as semantics-sensitive product policy

## Second-Pass Conclusion

`First-pass bucket changed`

The second pass upgrades `cdmc.rs` from `Small surgical refactor` to `Larger refactor but defer`. The main reason is no longer speculative: the current mapping appears to treat the upstream indeterminate `state == 0` case as trending behavior, which is likely a real strategy-semantics issue.
