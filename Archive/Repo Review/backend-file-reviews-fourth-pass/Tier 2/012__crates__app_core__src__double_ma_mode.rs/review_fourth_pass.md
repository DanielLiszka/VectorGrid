# Fourth-Pass Review: crates/app_core/src/double_ma_mode.rs

**Review date:** `2026-04-16`  
**Source file:** `crates/app_core/src/double_ma_mode.rs`  
**First-pass packet:** `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_mode.rs.review.md`  
**Second-pass packet:** `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_mode.rs.review.md`  
**Third-pass packet:** `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_mode.rs.review.md`  
**Third-pass action bucket:** `Document only`  
**Fourth-pass action bucket:** `Safe-now hardening`  
**Packet status:** `Reviewed`

## Why This File Got A Fourth Pass

This file is still small, but it is not throwaway glue. In the provided repo map it is one of the named first-stop policy files for Double-MA optimization routing, not a dead-end helper. Its decisions determine whether the optimizer runs exact grid, coarse-to-fine, or `GpuVdps`, and those decisions then flow into observable result fields such as `BackendOptimizationResult.mode_used`, optional `vdps` telemetry, backend-route notes, and dynamic GPU preflight behavior in `double_ma.rs`.

That means the third-pass framing needed to be challenged on two fronts.

First, the third-pass packet drifted into a generic “serde/default/DTO seam” reading that does not actually match the live source. `double_ma_mode.rs` is `pub(crate)` and has no serde derives of its own. The compatibility sensitivity here is behavioral route selection, not direct wire-shape ownership.

Second, the current `Document only` bucket underweights one bounded hardening opportunity that is visible from the live control flow: the `Auto -> GpuVdps` promotion currently keys off a notion of GPU-route viability that is broader than actual executable GPU availability. That is small enough to fix surgically and important enough to deserve more than documentation.

## Fourth-Pass Research Log

I re-read the live helper first, then traced its only provided call site in `crates/app_core/src/double_ma.rs`, then checked the public result/export surface in `double_ma.rs` and `lib.rs`, and finally reopened the second- and third-pass packets to compare their claims against the code.

Key local evidence points from the bundle:

- `double_ma_mode.rs:16-49`
  - `resolve_optimization_mode` returns `Grid`, `CoarseToFine`, or `GpuVdps`.
- `double_ma_mode.rs:51-79`
  - `validate_optimization_mode` enforces output incompatibilities for `CoarseToFine` and `GpuVdps`.
- `double_ma.rs:5176-5184`
  - the helper is called with `gpu_kernel_viable: kernel_supported_for_mode` and `gpu_hybrid_viable: hybrid_viable_for_mode`.
- `double_ma.rs:5201-5216`
  - actual kernel execution availability is computed later, separately from `kernel_supported_for_mode`.
- `double_ma.rs:5497-5502`
  - if `mode_used == GpuVdps` and the resolved backend is `CpuOnly`, the run errors.
- `double_ma.rs:5138-5140`
  - `export_all_csv_path` already requires `req.mode == Grid` before the helper result is used.
- `double_ma.rs:586-636`
  - `mode_used` is serialized as part of `BackendOptimizationResult`.
- `lib.rs:103-111`
  - the Double-MA request/result types are re-exported from the crate root.

I also checked the provided validation context and did not find a direct `double_ma_mode` consumer there. In the supplied bundle, the meaningful risk surface is optimization-route behavior and the artifacts/results that surface from it, not the validation entrypoint itself.

Background standards consulted while judging what kind of change is justified:

- Rust API Guidelines, Dependability (`C-VALIDATE`): validate inputs and make invalid states explicit.
- Rust API Guidelines, Documentation: document non-obvious behavior and failure conditions proportionally.
- The Rust Book, Chapter 11: small policy modules should have direct unit tests that pin expected branches.

Those standards support narrow invariant hardening and explicit behavior tests here; they do not justify a broad representation refactor.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right about three things.

First, this is still not a broad refactor target. The file is short, low-hotness policy code, and most “cleanup” ideas would add motion without materially reducing risk.

Second, the `500_000` large-search cutoff in `resolve_optimization_mode` is still benchmark-sensitive policy, not a magic number that should be casually tuned because it “looks arbitrary.” Changing it changes algorithm selection and likely performance characteristics.

Third, invariant tests are higher value than representation churn. The best maintenance work here is still branch-policy hardening, not architecture theater.

### What the current third-pass packet overstated

The third-pass packet overstates the file’s direct contract shape when it describes this module as a serde/default/omission seam.

That is not what the live source is. `double_ma_mode.rs` defines a crate-private context struct and two crate-private functions. It does not derive serde, it does not own request deserialization, and it does not define the public optimization result type. The serde-visible surfaces in the provided bundle live in `double_ma.rs`, especially `OptimizationModeResolved` and `BackendOptimizationResult`.

So the real caution is narrower and more concrete: changes here are externally visible because they alter downstream behavior and serialized result content, not because this file itself owns wire shape. That distinction matters. It means the right review question is “does this helper choose the correct mode under the live backend/runtime conditions?” not “should we preserve this helper’s DTO defaults?”

The earlier packet also states, too broadly, that `include_all`, `export_all_csv_path`, or `heatmap_bins > 0` force `Grid`. That is only partially true in the live backend path provided here. `include_all` and `heatmap_bins` do participate directly in Auto resolution inside this helper. But `export_all_csv_path` is already rejected unless `req.mode == Grid` in `double_ma.rs:5138-5140`, which means the helper’s `export_all_csv_path` handling is a defensive backstop in the provided backend flow, not the primary public policy owner.

### What the current packets missed or underweighted

The main missed issue is the asymmetry between “kernel support” and “kernel execution availability.”

In `double_ma_mode.rs:30-39`, `Auto` promotes to `GpuVdps` when the search is large, GPU residency is preferred, and `gpu_kernel_viable || gpu_hybrid_viable` is true. But the call site currently feeds `gpu_kernel_viable` from `kernel_supported_for_mode` at `double_ma.rs:5176-5184`. That value describes request/kernel compatibility, not actual runtime executability. Actual kernel execution availability is only derived later at `double_ma.rs:5201-5216`.

That matters because the downstream flow still rejects `GpuVdps` if backend resolution lands on `CpuOnly` at `double_ma.rs:5497-5502`. Statically, that means a large-search `Auto` request with `gpu_execution=PreferGpuResident` can promote into `GpuVdps` based on theoretical kernel support and then fail later on a no-GPU / no-kernel-executable path, even though `PreferGpuResident` otherwise reads as a soft preference and the codebase reserves `StrictGpuResident` for hard failure semantics.

I did not execute the code, so I am not claiming a reproduced runtime defect. But the control flow is strong enough that this is no longer just “documentation debt.” It is a real bounded hardening candidate.

The packets also underweighted missing negative-path tests. Current unit tests cover the happy branches, but they do not pin:

- the exact `500_000` versus `500_001` boundary,
- the non-promotion case when `gpu_execution` is `BestEffort`,
- the non-promotion or fallback behavior when GPU-resident preference is set but no executable GPU route exists,
- the defensive `export_all_csv_path` incompatibility path inside this helper.

That missing coverage is especially important now that the most plausible issue is a negative-path policy mismatch rather than a basic branch typo.

### Safe-now hardening

The third-pass bucket should change from `Document only` to `Safe-now hardening`.

The bounded change I would prioritize is this:

Treat Auto `GpuVdps` promotion as depending on an executable GPU route, not merely on abstract kernel compatibility. In practice that likely means either passing an execution-available kernel boolean into `ModeSelectionContext`, or renaming/splitting the current booleans so the helper cannot mistake “kernel-capable request shape” for “GPU route can actually run here.”

That recommendation is deliberately narrow. It does **not** require changing explicit manual `OptimizationMode::GpuVdps` handling. It does **not** require changing the `500_000` cutoff. It only closes the gap where a soft Auto preference appears able to escalate into a hard backend error because the helper resolves mode before executable kernel availability is fully represented.

The second safe-now hardening item is test coverage:

- add a direct boundary test for `500_000` and `500_001`,
- add a direct unit or end-to-end test that `Auto + PreferGpuResident` does not silently become an unintended `GpuVdps -> CpuOnly` error on a non-executable GPU path, unless that behavior is declared intentional,
- add a unit test that `BestEffort` does **not** promote to `GpuVdps` even when the route is otherwise viable,
- add a small test or inline comment pinning that `export_all_csv_path` is redundantly guarded here but primarily owned earlier in request validation for the provided backend flow.

The third safe-now item is readability only: name the `500_000` cutoff as a constant. That is not because the number should be tuned now. It is because a named constant makes the tests and the benchmark-sensitive policy easier to review together.

### Larger redesign still deferred

A larger redesign is still deferred.

I would **not** merge mode selection, backend resolution, request-builder legality, and runtime fallback into a single new planner in this pass. The live execution path in `double_ma.rs` already threads mode choice into backend promotion, route diagnostics, dynamic GPU preflight, and serialized result metadata. A “clean” unification would touch much more than this helper and could easily create regressions in performance posture, route notes, artifact diffs, and GPU fallback semantics.

I would also **not** try to unify every threshold that mentions large searches. The repo already contains other thresholds, such as the separate `VDPS_LOCAL_FRONTIER_MIN_FULL_GRID_PAIRS = 750_000` constant inside the `GpuVdps` implementation path. That threshold governs a later intra-VDPS frontier-window decision, not the top-level Auto mode switch. Collapsing them because they are both “large search” numbers would be unjustified.

If a future redesign is taken, it should start from a typed policy object that carries:

- requested optimization mode,
- requested backend,
- exact-output requirements,
- kernel support,
- runtime kernel executability,
- hybrid executability,
- GPU-residency preference strength.

That would be a separate policy pass, not a safe-now cleanup.

## Implementation Priority

1. **Fix the Auto `GpuVdps` promotion input.** Make the helper reason over executable GPU-route availability instead of a mixed “support vs executability” signal.
2. **Add branch-policy tests.** Pin the exact threshold, the soft-preference negative path, and the BestEffort non-promotion case.
3. **Name and comment the threshold.** Keep it benchmark-sensitive, but make the policy easier to audit.
4. **Defer broader planner unification.** It is too wide for the size of the defect surface here.

If maintainers decide the current `PreferGpuResident -> GpuVdps -> CpuOnly error` path is actually intentional, then the bucket can fall back toward documentation. But in that case it still needs an explicit end-to-end test and a much clearer comment, because the current behavior is surprising and not adequately pinned by the existing test set.

## Verification If Changed

At minimum, keep and rerun the existing focused tests:

- `cargo test -p ta_app_core auto_mode_only_adapts_when_grid_outputs_are_disabled -- --nocapture`
- `cargo test -p ta_app_core auto_mode_prefers_vdps_for_gpu_resident_large_runs_when_route_is_viable -- --nocapture`
- `cargo test -p ta_app_core gpu_vdps_rejects_cpu_backend -- --nocapture`
- `cargo test -p ta_app_core double_ma_mode -- --nocapture`

Add targeted coverage for the missed cases before merging any logic change:

- exact threshold boundary: `500_000` stays non-large, `500_001` becomes large,
- `Auto + BestEffort + viable GPU route` stays out of `GpuVdps`,
- `Auto + PreferGpuResident + no executable GPU route` either falls back cleanly or errors intentionally, but the behavior must be explicitly pinned,
- `export_all_csv_path` defensive backstop stays aligned with the earlier `mode=Grid` request check.

Because `mode_used` is serialized in `BackendOptimizationResult`, verification should also inspect observable result fields, not just success/failure. A route-policy fix here can legitimately change `mode_used`, `backend_route`, `backend_fallback_reason`, and `vdps` telemetry presence.

## Regression And Feature-Preservation Constraints

Preserve these behaviors unless there is explicit benchmark-backed approval to change them:

- explicit manual `OptimizationMode::GpuVdps` with `CpuOnly` still rejects,
- `CoarseToFine` and `GpuVdps` remain incompatible with `include_all`, heatmap output, and export-all output,
- the `500_000` Auto large-search cutoff is treated as benchmark-sensitive policy,
- the separate `GpuVdps` internal thresholds remain separate unless there is evidence they should be unified.

Compatibility and workflow cautions:

- this file is not directly a serde surface, but its decisions are externally visible through serialized optimization results,
- changing Auto mode selection can alter persisted artifacts, IPC-visible result fields, and report/history expectations,
- no direct GPU ABI issue was proven here; the risk is route-selection behavior, not binary interface breakage,
- no direct validation-path consumer was proven in the provided validation file, but optimization-route changes can still affect downstream validation workflows indirectly through the saved optimization result and selected finalists.

Recommendations that would likely cause regressions if implemented too aggressively:

- broad planner unification,
- threshold retuning without measurement,
- deduplicating layered output checks without first deciding which layer owns public policy,
- changing `PreferGpuResident` / `StrictGpuResident` semantics together in the same patch.

## Fourth-Pass Conclusion

The current third-pass bucket should **change**.

`Document only` is too conservative for the live source. The third-pass packet correctly resisted broad refactor churn, but it also mislocated the compatibility risk and missed one concrete hardening opportunity: Auto `GpuVdps` promotion currently appears to consume a broader “GPU route viable” signal than the runtime can always execute, and that can plausibly turn a soft preference into a hard backend error.

So the right fourth-pass conclusion is:

- **safe-now hardening:** yes,
- **larger redesign:** still deferred,
- **broad refactor:** still not justified.

If I were handing this off in a public-facing repository review, I would recommend one surgical policy fix, a handful of focused tests, and no deeper redesign in this pass.
