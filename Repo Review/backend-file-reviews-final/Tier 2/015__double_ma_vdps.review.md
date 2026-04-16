# Final Consolidated Review: `crates/app_core/src/double_ma_vdps.rs`

- Review date: `2026-04-15`
- Source file: [double_ma_vdps.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_vdps.rs:1>)
- First-pass packet: [double_ma_vdps.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/double_ma_vdps.rs.review.md:1>)
- Second-pass packet: [double_ma_vdps.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_vdps.rs.review.md:1>)
- Third-pass packet: [double_ma_vdps.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_vdps.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/015__crates__app_core__src__double_ma_vdps.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth-pass packet is the right governing review for this file. The first and second passes already identified the two real defects, and the fourth pass correctly repairs the third-pass drift into generic “tolerant policy surface” language. The live source confirms both core findings: all three refill helpers compare `seen.len()` against a moving `current.len() + target_count`, so the stop condition can overshoot badly once inserts start succeeding; and `vdps_merge_partial_results()` still reranks with `score_mode = None`, which can prune the wrong rows before the caller’s later rerank. The live `top_k == 0` path also leaves `best_params` unrepaired after that wrong merge contract.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the source file and its claims are materially aligned with the live code. The only strengthening needed for implementation is to make the verification list exact and to treat telemetry merge hardening as secondary to the refill and `score_mode` fixes.

## Safe-Now Priority

- Fix the three refill-helper loop guards by comparing against a stable target size captured before mutation begins.
- Preserve the requested `score_mode` through `vdps_merge_partial_results()` and ensure the `top_k == 0` path still returns a winner ranked under that same contract.
- Add focused tests for each helper’s exact unique-add budget, for non-`None` `score_mode` preservation during partial merge, and for the final `top_k == 0` result path.
- Keep sorted-period hardening lightweight: concise docs and, at most, debug-only assertions around the `partition_point` precondition.
- If a caller-side clamp is added in `double_ma.rs`, keep it strictly defensive and avoid changing telemetry semantics more than necessary.

## Deferred Work

- Do not retune mutation radii, refill ratios, decimation policy, or local-anchor heuristic behavior in the same patch.
- Do not redesign `VdpsTelemetry` aggregation or serialized shape in the same change.
- Do not chase container-choice or micro-allocation optimizations before the live contract bugs are fixed and re-measured.

## Verification If Changed

- `cargo test -p ta_app_core vdps_merge_partial_results_keeps_global_best -- --nocapture`
- `cargo test -p ta_app_core vdps_local_anchor_axes_include_center_and_trim_candidates -- --nocapture`
- `cargo test -p ta_app_core gpu_vdps_quality_check_emits_overlap_and_regret -- --nocapture`
- `cargo test -p ta_app_core gpu_vdps_deterministic_same_seed_reproducible -- --nocapture`
- Add and run file-local regression tests for:
  - exact refill-count stopping in `vdps_inject_random_seed_pairs`
  - exact refill-count stopping in `vdps_inject_elite_mutation_pairs`
  - exact refill-count stopping in `vdps_inject_differential_seed_pairs`
  - non-`None` `score_mode` preservation through partial merge
  - non-`None` `score_mode` with `top_k == 0`
- Because workload shape can change after the refill fix, run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` on CUDA-enabled trees before handoff.

## Regression And Feature-Preservation Constraints

- Preserve the `GpuVdps` route itself; fix helper contracts without redesigning the mode.
- Treat refill-budget repair as semantics-sensitive and benchmark-sensitive because it changes survivor-set size.
- Preserve default-score behavior for `score_mode = None`.
- Keep sorted-period assumptions as caller invariants rather than new release-build panics.

## Final Conclusion

The implementation-facing conclusion is:

- keep `double_ma_vdps.rs` in `Small surgical refactor`
- treat the fourth pass as the correct final frame
- prioritize the refill-budget and `score_mode` defects over telemetry or heuristic cleanup
- add direct regression tests before touching any perf-adjacent helper logic
