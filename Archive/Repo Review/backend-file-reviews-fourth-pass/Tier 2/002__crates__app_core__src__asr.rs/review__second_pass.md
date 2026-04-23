# Second-Pass Review: `crates/app_core/src/asr.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/asr.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/asr.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this strategy file as mostly sound but contract-sensitive because late-bound selector values and route-dependent behavior affect both correctness and observability.

## Stale Check

The live source still relies on:

- z-score selector values like `z_ma_type` and `z_devtype`
- TTM Squeeze-derived warmup behavior
- route-dependent CUDA/CPU execution

The first-pass packet remains current.

## Second-Pass Research Log

1. VectorTA Z-Score docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/zscore/>
   - Why consulted in second pass:
     - tighten the contract around MA type, deviation mode, and `nbdev`
   - Additional constraint informed:
     - selector values here are discrete config contract, not free-form payload

2. VectorTA TTM Squeeze docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/ttm_squeeze/>
   - Why consulted in second pass:
     - confirm the warmup/state behavior behind `compute_asr_start_t`
   - Additional constraint informed:
     - any shortcut start-time optimization would be risky unless it still keys off fully computed signal validity

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - reassess the default-heavy public config surface
   - Additional constraint informed:
     - invalid selector values still deserve local rejection rather than late behavior drift

## Delta vs First Pass

### 1. Local validation for z-score selector values remains weaker than the public config surface deserves

- Why this matters:
  - route selection and output semantics depend on discrete selector meaning
- Practical implication:
  - this remains a bounded contract-hardening target

### 2. CUDA route availability is materially controlled by public selector values and should stay observable

- Why this matters:
  - `z_devtype == 0` plus supported MA type determines whether direct CUDA sweeps are even available
- Practical implication:
  - if diagnostics evolve, surface this as explicit route policy rather than opaque fallback

## Verification If Changed

- `cargo test -p ta_app_core asr -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel asr -- --nocapture` if CUDA routing changes

## Regression And Feature-Preservation Constraints

- Preserve current ASR CPU and CUDA route availability
- Preserve current metric field coverage
- Preserve start-time safety based on fully computed signals

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens the config-contract reading: selector values remain more structured than the current local validation makes explicit, and those same values materially control CUDA route availability.
