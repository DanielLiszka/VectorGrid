# Fourth-Pass Review: src-tauri/src/commands_validation.rs

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_validation.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally right: this file is not a thin wrapper, it is the semantic boundary between Tauri IPC, loaded runtime state, app-core validation, cross-ticker post-processing, and a small chart-data API. That architectural reading still holds.

What justified a fourth pass was narrower and more adversarial. The third-pass packet’s Implementation Priority reduced safe-now work to documentation and additive clarification (`review__third_pass.md:68-73`). On the live source, that is too narrow. There is at least one bounded code-level hardening step the packet did not elevate, and there are two wrapper-owned semantics that need to be described more precisely before any larger refactor is attempted.

## Fourth-Pass Research Log

This pass re-read the live source file first, then checked the immediately relevant local context: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/validation_common.rs`, `crates/app_core/src/validation_pipeline.rs`, `AGENTS.md`, `REPO_MAP.md`, and the three prior review packets.

I also re-checked the current official Tauri v2 command and state-management guidance, Tokio’s `spawn_blocking` / task docs, and the Rust API Guidelines dependability guidance. That combination still supports the third-pass framing that this module is a public command boundary, that `spawn_blocking` is the correct broad execution shape for CPU-heavy validation work, and that argument validation / silent reinterpretation at the wrapper layer should be treated as public contract behavior.

One important limitation matters for this pass: the bundle does not include a runnable workspace, so historical local test-failure claims cannot be re-executed here. For fourth-pass judgment, the strongest evidence is therefore the live source and the live in-file test suite, not older test-output prose.

## What Fourth Pass Sharpened

### The third-pass bucket should stand, and the packet got the central architecture call right

The file is still exported through `src-tauri/src/commands.rs:143-145` and registered on the Tauri invoke surface in `src-tauri/src/lib.rs:124-125,178`. Inside that public surface it still validates `validation_context` before the app-core call (`src-tauri/src/commands_validation.rs:131-182`), strips `cross_ticker` out of the base request before invoking app-core (`:392-398`, `:632-638`), reapplies cross-ticker behavior in the wrapper (`:430-530`, `:675-789`), and overwrites `runtime_ms` after wrapper-owned post-processing completes (`:423`, `:668`).

So the third-pass packet was right about the big picture: this is a semantics-owning wrapper, not a harmless transport layer, and the duplicated composite / Double-MA orchestration is still real refactor debt.

### But the third-pass claim that safe-now work is limited to documentation is too strong

The live source rebuilds `run_verdict` after cross-ticker filtering in both strategy families by calling `ta_app_core::validation_pipeline::build_run_verdict(...)` with `res.candidates.len()` as the `candidates_tested` argument (`src-tauri/src/commands_validation.rs:516-529`, `:775-788`). In app-core, `build_run_verdict` treats `candidates_tested` as a caller-supplied field and writes it directly into `ValidationRunVerdict.candidates_tested` (`crates/app_core/src/validation_pipeline.rs:437-578`, especially `:562-577`).

That means a public field named `candidates_tested` changes meaning once cross-ticker filtering runs: it stops reflecting the candidate count that was originally validated and starts reflecting the post-filter survivor count. This is not a broad redesign problem. It is a bounded wrapper hardening target.

I would not overstate it into “proven bug” without downstream snapshots, because changing the field is still public-surface-sensitive. But the third-pass packet should not have classified all safe-now work as documentation-only while this field-level semantic drift remains live.

### The wrapper’s “recomputed” run verdict is only partially recomputed

The third-pass packet correctly noted that the wrapper owns post-processing semantics, but it still left the run-verdict behavior too abstract. After cross-ticker filtering, the wrapper rebuilds `run_verdict` from the filtered candidates, but it keeps using `res.cscv_pbo.as_ref()` and `res.spa.as_ref()` from the original base-dataset run (`src-tauri/src/commands_validation.rs:523-528`, `:782-787`).

So the final verdict is hybrid:

- candidate-count, approval-count, recommendation-rank, and top-candidate fields come from the post-cross-ticker candidate slice;
- CSCV/PBO and SPA evidence still come from the original base run.

That may be exactly the right product choice, but it is not the same thing as a full “recompute the verdict after filtering.” The third-pass packet underweighted this distinction. It matters because any “cleanup” that tries to recompute SPA or CSCV/PBO inside the cross-ticker rerun path would change both runtime cost and result semantics.

### Cross-ticker aggregation is success-only, not failure-penalizing

The generic cross-ticker driver only pushes per-dataset metrics into the accumulator when the rerun candidate itself `validation_passed()` (`src-tauri/src/commands_validation.rs:312-319`). Then `finalize_cross_ticker_candidates()` builds `CrossTickerStats.datasets` from the number of accumulated metric rows, not the number of attempted datasets (`src-tauri/src/validation_common.rs:296-335`). Gate evaluation later uses that stats object (`src-tauri/src/validation_common.rs:160-188`).

That is a real contract detail the third-pass packet did not spell out: on the current implementation, a failed cross-ticker rerun becomes missing-dataset evidence, not explicitly negative evidence. In lightweight mode with a permissive gate, that distinction is material. A redesign that changed failures into zeroed or penalized metrics would tighten ranking and filtering behavior in a way the current packet did not acknowledge.

### Strategy-kind validation is strict for dataset/timeframe, but permissive for unknown kinds

`validate_validation_context()` hard-fails dataset and timeframe mismatches (`src-tauri/src/commands_validation.rs:151-179`), but strategy-kind checking goes through `normalize_strategy_kind()`, which defaults every unknown value to `DoubleMa` (`:104-128`, `:140-149`). The third-pass packet was right that this is risky; the sharper point is that it is not just a hypothetical future problem.

The current routing document still lists `DynamicDoubleMa` as a live strategy family that resolves through the normal Double-MA path (`REPO_MAP.md:1057-1074`), but `normalize_strategy_kind()` has no explicit `DynamicDoubleMa` branch. So today’s behavior is already a silent alias question, not only a future-extension question.

That does **not** automatically mean the code should be tightened immediately. The alias may be deliberately relied on. But it does mean the third-pass packet should have elevated an additive regression test or explicit contract note here, rather than leaving the topic entirely in the deferred redesign bucket.

## Implementation Priority

### Safe-now hardening

1. **Preserve the original tested-count semantics when rebuilding `run_verdict`.**
   Capture the pre-cross-ticker tested count before filtering, or preserve `res.run_verdict.candidates_tested`, and feed that value back into `build_run_verdict(...)` instead of `res.candidates.len()`. Then extend the existing preserved-finalist tests in both families (`src-tauri/src/commands_validation.rs:1532-1619`, `:1953-2064`) so they assert `candidates_tested` as well as approval count.

2. **Pin the current strategy-kind alias behavior with a regression test before changing it.**
   The safest immediate move is not a behavior change. It is a test that says whether `DynamicDoubleMa` (or any unknown kind) is intentionally accepted as `DoubleMa` today. Once that is pinned, a later migration can tighten the rule deliberately instead of silently.

3. **Document the hybrid verdict and success-only cross-ticker accumulation rules.**
   The earlier packet was right that the no-overlap alignment fallback still deserves a direct test. But the bigger missing documentation is actually the wrapper’s hybrid run-verdict rebuild and the fact that failed cross-ticker reruns currently drop out of stats instead of contributing negative evidence.

### Larger redesign still deferred

1. **Split wrapper IPC DTO ownership from app-core DTO reuse.**
   The third-pass packet was right to keep this as the main redesign target. `validation_context`, `cross_ticker`, `runtime_ms`, and post-run verdict rebuilding are still wrapper-owned semantics hidden inside reused app-core request/result shapes.

2. **Unify the duplicated composite / Double-MA orchestration only after the semantics above are pinned.**
   The current duplication (`src-tauri/src/commands_validation.rs:341-576` vs `:578-882`) is still real debt, but unifying it before the hybrid-verdict and success-only-aggregation rules are explicit would be regression-prone.

3. **Do not “fix” cross-ticker semantics by rerunning SPA/PBO or by turning failures into penalties without a product decision.**
   Both changes would alter public validation behavior and runtime cost. They are redesign choices, not cleanup.

4. **Do not casually trim the exported headless helper surface.**
   `apply_cross_ticker_validation_double_ma_headless` is re-exported from `src-tauri/src/commands.rs:143-145`. Even apparently-dead parameter cleanup should be treated as consumer-facing coordination work, not opportunistic tidying.

## Verification If Changed

For the bounded hardening above, the minimum verification should be:

- `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- `npm --prefix ./web run test -- src/services/validationService.test.ts src/stores/validationStore.actions.test.ts src/tauri/commands.test.ts`

If the change goes beyond `candidates_tested` / tests / docs and starts altering cross-ticker evidence semantics, then the workflow-level validation surface should also be rechecked with the repo’s broader validation and workflow gates before handoff.

## Regression And Feature-Preservation Constraints

- **Do not tighten unknown strategy-kind handling blindly.** The current fallback may already be carrying `DynamicDoubleMa` or legacy-artifact compatibility.
- **Do not reinterpret failed cross-ticker reruns as negative metrics unless the product decision is explicit.** Today’s code treats them as missing evidence.
- **Do not claim the wrapper fully recomputes run verdict after cross-ticker filtering.** It currently rebuilds counts and recommendation state while still reusing base-run SPA / CSCV-PBO evidence.
- **Do not loosen dataset/timeframe `validation_context` checks casually.** Those checks are the genuinely strict part of the public boundary today.
- **Do not remove exported headless-helper parameters or replace canonical candidate matching with positional matching as a cleanup-only change.** Both are easy places to break headless or replay consumers.

## Fourth-Pass Conclusion

The third-pass `Larger refactor but defer` bucket should stand.

The third-pass packet got the main architectural point right: this file is still a semantics-owning wrapper over app-core validation, and the larger DTO / orchestration split remains redesign work that should not be rushed.

What it missed was narrower but important. Safe-now work is **not** limited to documentation. The live source has a bounded public-field hardening opportunity around `run_verdict.candidates_tested`, and the current packet also under-described two semantics that any future refactor must preserve or consciously migrate: the wrapper’s hybrid post-cross-ticker verdict rebuild, and the success-only nature of cross-ticker metric accumulation.

So the correct fourth-pass posture is:

- keep the larger redesign deferred;
- add one bounded hardening item now if the file is touched;
- and make the current cross-ticker/result semantics explicit before any consolidation work begins.
