# Final Consolidated Review: `crates/app_core/src/vcet_gpu_kernel.rs`

- Review date: `2026-04-15`
- Source file: [vcet_gpu_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/vcet_gpu_kernel.rs:1>)
- First-pass packet: [vcet_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/vcet_gpu_kernel.rs.review.md:1>)
- Second-pass packet: [vcet_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcet_gpu_kernel.rs.review.md:1>)
- Third-pass packet: [vcet_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcet_gpu_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/026__crates__app_core__src__vcet_gpu_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Targeted safe-now hardening is justified; larger benchmark-sensitive redesign still deferred`
- External research anchors:
  - <https://docs.rs/cust/latest/cust/memory/struct.DevicePointer.html>
  - <https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__UNIFIED.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth pass is the right final packet for this file. Earlier passes were correct that `vcet_gpu_kernel.rs` is a hot CUDA-facing wrapper whose cache, launch, and scratch behavior should not be casually rearranged. What they underweighted is that one narrow contract gap is both real and cheaply fixable: a prepared batch currently materializes device-owned state on one device, while `eval_metrics()` still accepts an arbitrary device id later.

That mismatch is the best safe-now hardening slice because it adds a deterministic prelaunch error path without touching kernel bodies, cache policy, or metric decode semantics. The rest of the redesign ideas remain too benchmark-sensitive to treat as opportunistic cleanup.

## Safe-Now Priority

- Add `prepared_device_id` to `PreparedVcetGpuKernelBatch` and reject mismatched `eval_metrics(device_id, ...)` calls before launch.
- Replace raw ABI flag literals with named constants aligned to the shared kernel ABI surface.
- Add explicit contract docs on:
  - same-device/current-context expectations
  - owner-lifetime requirements for device-row tables
  - the fact that non-zero device pointers are trusted as resident inputs
- Add a narrow overwrite-before-read comment only at the explicit local uninitialized allocation site.

## Deferred Work

- Defer thread-local runtime/cache redesign.
- Defer genericizing this wrapper into shared infrastructure.
- Defer launch-shape retuning, scratch-policy changes, or public-API reshaping.
- Defer stronger provenance checking that would require deeper CUDA pointer-attribute handling or multi-device policy work.

## Verification If Changed

- `cargo test -p ta_app_core vcet_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- Re-run representative VCET GPU orchestration and parity coverage in `composite_cpu`
- If runtime-path code changes beyond docs/constants/guardrails, run:
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression And Feature-Preservation Constraints

- Preserve the current strategy-local ownership model for device rows and keepalive-backed matrices.
- Preserve current runtime-cache and launch behavior unless parity and performance are being measured together.
- Treat public raw-pointer DTO cleanup as compatibility-sensitive work, not as free ergonomics.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- upgrade `vcet_gpu_kernel.rs` from pure defer to `Targeted safe-now hardening is justified; larger benchmark-sensitive redesign still deferred`
- make the prepare/eval device-lockstep guard the first implementation priority
- allow only named-constant, contract-doc, and narrowly scoped guardrail work beyond that
- defer cache, launch, ownership-model, and genericization work until CUDA parity and performance are in the verification plan
