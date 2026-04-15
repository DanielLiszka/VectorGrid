# Second-Pass Review: `crates/app_core/src/carm.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/carm.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/carm.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this strategy file as mostly sound but somewhat opaque because a small public parameter surface hides a richer fixed OTTO policy bundle.

## Stale Check

The live source still couples:

- one exposed `otto_threshold` input
- several implied OTTO policy decisions
- mixed CPU/GPU signal assembly

The first-pass packet remains current.

## Second-Pass Research Log

1. VectorTA OTTO docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/otto/>
   - Why consulted in second pass:
     - verify the underlying defaults and parameter surface
   - Additional constraint informed:
     - the strategy still hides more OTTO policy than its public config suggests

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - ground the default-heavy config review
   - Additional constraint informed:
     - the public config remains narrower than the underlying strategy semantics

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/archive/12.6.2/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - reassess the current one-failure-disables-further-CUDA posture
   - Additional constraint informed:
     - the fast-fail fallback policy is operationally reasonable, but should stay observable if it matters to users

## Delta vs First Pass

### 1. `otto_threshold` still drives two different units, which makes the public contract harder to reason about than the field name suggests

- Why this matters:
  - one public value influences both indicator interpretation and normalized regime spread semantics
- Practical implication:
  - the config surface remains a good small-documentation and naming review target

### 2. The current CUDA fast-fail flag changes whole-pipeline behavior after one miss

- Practical implication:
  - preserve the behavior for now, but surface it explicitly if route diagnostics are expanded

## Verification If Changed

- `cargo test -p ta_app_core carm -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm -- --nocapture` if CUDA-path behavior changes

## Regression And Feature-Preservation Constraints

- Preserve current long-only / flip / trade-on-next-bar behavior
- Preserve mixed CPU/GPU signal assembly
- Do not casually retune the one-failure-disables-further-CUDA policy

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens the contract reading: the exposed OTTO input still hides more policy than it appears to, and the CUDA fast-fail flag still changes pipeline behavior in a way that should remain explicit if surfaced.
