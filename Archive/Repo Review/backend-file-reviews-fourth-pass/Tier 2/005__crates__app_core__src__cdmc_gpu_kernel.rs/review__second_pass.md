# Second-Pass Review: `crates/app_core/src/cdmc_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cdmc_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/cdmc_gpu_kernel.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this wrapper as a safe-looking but contract-heavy bridge into CUDA execution, with device ownership and extent assumptions as the main sharp edges.

## Stale Check

The live source still forwards:

- raw device pointers for MAMA/FAMA buffers
- explicit threshold handoff into GPU execution
- CPU/GPU regime semantics coupled to the same strategy contract

The first-pass packet remains current.

## Second-Pass Research Log

1. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - re-check pointer and extent assumptions on device buffers
   - Additional constraint informed:
     - the safe wrapper still relies on unexpressed layout/extent guarantees

2. `cust::Stream`
   - Source: <https://docs.rs/cust/latest/cust/stream/struct.Stream.html>
   - Why consulted in second pass:
     - confirm the required host-sync boundary
   - Additional constraint informed:
     - stream synchronization remains the correct host read barrier

3. VectorTA Correlation Cycle docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/correlation_cycle/>
   - Why consulted in second pass:
     - verify the semantics the kernel path is supposed to mirror
   - Additional constraint informed:
     - any correction to state semantics must land here and in the CPU path together

## Delta vs First Pass

### 1. Device provenance is still dropped when `CdmcDeviceMatrix` turns into raw pointers

- Why this matters:
  - the wrapper cannot validate same-device ownership across all inputs
- Practical implication:
  - documentation or visibility tightening remains the best bounded hardening

### 2. CPU/GPU threshold and state semantics must stay coupled

- Why this matters:
  - this wrapper already mirrors the same absolute-threshold handoff as the CPU path
- Practical implication:
  - semantics fixes belong to the whole CDMC family, not just one side

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_gpu_kernel -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve scratch reuse and resident close/LR reuse
- Keep CPU/GPU threshold semantics aligned with `cdmc.rs`
- Do not widen or narrow the effective device contract without checking current builders

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct for this wrapper. The second pass mainly clarifies that any real semantics correction belongs to the full CDMC family, while the wrapper itself still has the same device-provenance and extent-contract sharp edges.
