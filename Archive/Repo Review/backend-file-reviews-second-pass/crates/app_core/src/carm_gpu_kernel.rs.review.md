# Second-Pass Review: `crates/app_core/src/carm_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/carm_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/carm_gpu_kernel.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this wrapper as a safe API over raw device-pointer and output-layout contracts, with reuse behavior worth preserving but documenting more clearly.

## Stale Check

The live source still exposes:

- per-device runtime reuse
- resident close/LR reuse
- raw pointer transport plus separate `device_id`

The first-pass packet remains current.

## Second-Pass Research Log

1. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - re-check the uninitialized and buffer-ownership contract
   - Additional constraint informed:
     - output correctness still depends on the kernel fully writing every slot before host decode

2. `cust::Stream`
   - Source: <https://docs.rs/cust/latest/cust/stream/struct.Stream.html>
   - Why consulted in second pass:
     - verify the synchronization boundary before host reads
   - Additional constraint informed:
     - stream synchronization remains the critical host-safety barrier

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground the public-API review for a safe wrapper over sharp contracts
   - Additional constraint informed:
     - the dominant risk remains undocumented invariants, not algorithmic behavior

## Delta vs First Pass

### 1. Device provenance is still weaker than the signature suggests because raw pointers plus separate `device_id` cannot prove same-device ownership

- Why this matters:
  - the API looks safer than the actual contract
- Practical implication:
  - documentation or visibility tightening remains the cleanest bounded fix

### 2. `combo_count <= mixed_rows` is still the highest-value safe-now hardening

- Practical implication:
  - keep this ahead of broader cache or topology changes if the wrapper is touched

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_gpu_kernel -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve per-device runtime reuse and uploaded-combo signature reuse
- Preserve resident close/LR reuse
- Do not add expensive re-keying or remove reuse paths without profiling

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that the safe-looking API still cannot prove device provenance, and that bounded count validation remains the best small hardening target.
