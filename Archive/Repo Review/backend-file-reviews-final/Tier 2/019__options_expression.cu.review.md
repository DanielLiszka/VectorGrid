# Final Consolidated Review: `crates/app_core/src/kernels/options_expression.cu`

- Review date: `2026-04-15`
- Source file: [options_expression.cu](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu:1>)
- First-pass packet: [options_expression.cu.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/kernels/options_expression.cu.review.md:1>)
- Second-pass packet: [options_expression.cu.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kernels/options_expression.cu.review.md:1>)
- Third-pass packet: [options_expression.cu.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kernels/options_expression.cu.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/019__crates__app_core__src__kernels__options_expression.cu/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/>
  - <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth-pass packet is valid and materially sharper than the earlier ABI-heavy framing. The live kernel code and the owning Rust pipeline confirm the real contract: these kernels accumulate aggregate metrics in `float`, produce a robust shortlist score using the current four realized lanes, and feed retained GPU-derived aggregates into a later exactify stage that only partially repairs approximation. The Rust owner also confirms that positive-premium validity is already enforced before flattening GPU tensors, that missing `extreme_stress` is intentionally folded into the harsher lane today, and that the combined scorer remains environment-gated with a live legacy per-range fallback.

No wrong `review_fourth_pass.md` is present in this folder. The strengthening needed for implementation is to keep verification options-expression-specific rather than falling back to generic GPU harnesses, and to treat the mixed-precision/exactify boundary as first-class documentation.

## Safe-Now Priority

- Add concise kernel or paired-launch contract comments covering:
  - range ordering invariants
  - positive-premium validity assumptions
  - combined-kernel lane ownership
  - the intentional absence of exact extreme-stress scoring in this kernel family
  - the fact that the current `0.15` harsher weight reflects CPU fallback semantics when `extreme_stress` is absent here
- Document the mixed-precision boundary explicitly:
  - device aggregation in `float`
  - host reconstruction into `f64` result fields
  - later exactify enriching only part of the result surface
- Add or tighten legacy-path parity coverage by running the same small surface with `VECTORGRID_OPTIONS_GPU_DISABLE_COMBINED_SCORING=1`.

## Deferred Work

- Do not switch aggregate math to `double` as a casual cleanup; that would change ranking and persisted metrics.
- Do not push extreme-stress or sealed-stage behavior into this kernel family in the same pass.
- Do not do launch-shape or micro-optimization work without explicit measurement and ranking-baseline review.

## Verification If Changed

- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core gpu_combined_ranking_matches_cpu_topk_on_small_surface --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core exactify_ranked_results_reuses_gpu_summary_metrics_and_only_enriches_medians --features cuda,cuda-backtest-kernel -- --nocapture`
- Add and run a legacy-path regression with `VECTORGRID_OPTIONS_GPU_DISABLE_COMBINED_SCORING=1` if this area is touched.

## Regression And Feature-Preservation Constraints

- Preserve kernel entry names, argument ordering, and lane/output layout unless Rust launch code moves in lockstep.
- Do not add device-side premium guards casually; upstream currently enforces the invariant.
- Do not reinterpret the weight blend as an isolated tuning knob divorced from CPU/GPU consistency.
- Do not assume exactify fully repairs GPU approximation; it does not.

## Final Conclusion

The implementation-facing conclusion is:

- keep `options_expression.cu` in `Small surgical refactor`
- treat the fourth pass as the correct final frame
- prioritize contract documentation and route-specific parity coverage over kernel restructuring
- defer accumulation-type changes, GPU scope expansion, and performance retuning until the project is ready to re-baseline ranking and artifact semantics
