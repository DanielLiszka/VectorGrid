# Fourth-Pass Review: crates/app_core/src/gpu_backtest_kernel.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/gpu_backtest_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/gpu_backtest_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file earned a fourth pass because the third-pass packet moved from broad caution to a very specific recommendation: compact the uploaded dynamic exact path first. That is the kind of claim that is worth challenging in a Tier 1 shared CUDA runtime file.

The live source still makes that path look attractive at first glance. `eval_uploaded_double_ma_dynamic_program_gpu_kernel()` allocates and reuses output by dense fast-row × slow-row shape, keeps `compact_output_i = 0`, synchronizes, copies the full dense device buffer back to host, and then decodes only the combo-selected rows (`crates/app_core/src/gpu_backtest_kernel.rs:2726-2898`). The third-pass packet was right to notice that.

But the fourth-pass question is narrower and more adversarial: is that really the safest next change, or is there still a lower-risk, clearer-return hardening move available first? After re-reading the live source and the repo routing guidance, I do **not** think the uploaded-path compaction change is the best immediate priority. I think the bucket should **stand**, but the implementation priority should shift back toward public-contract hardening and diagnostics.

## Fourth-Pass Research Log

I re-read the live source first, then re-checked the repo guidance only where it changed the risk calculus.

The most important repo-level check was `REPO_MAP.md`, which still describes `DynamicDoubleMa` as a constrained builder/preview surface that “runs through normal Double-MA path” and is “mainly a builder/preview surface, not a separate execution engine” (`REPO_MAP.md:1057-1074, 1223`). That matters because it weakens the third-pass framing that the uploaded dynamic helper path is obviously the highest-ROI place to spend the next small refactor.

I also re-checked the same primary standards the earlier packets were leaning on, but only to keep the recommendations conservative:

- Rust API Guidelines, especially documentation and failure-contract guidance.
- `std::thread_local!` documentation, because this module still exposes thread-local-runtime-backed helper APIs.
- NVIDIA CUDA C++ Best Practices Guide, to avoid recommending casual simplification of pinned-memory, stream-overlap, or staging behavior.
- CUDA Driver / stream-capture documentation, to keep graph/runtime observations grounded and to avoid suggesting speculative cleanup around stateful capture behavior.

Those sources did not change the code-facing findings. They mainly reinforced a conservative posture: public helper validation and contract documentation are safe-now; launch/runtime/staging/perf changes should stay measurement-gated.

## What Fourth Pass Sharpened

The current third-pass bucket should **stand**. This file is still a `Small surgical refactor` candidate, not a rewrite candidate. The third-pass packet also got two important things right:

First, it correctly identified that the uploaded dynamic exact path is doing more work than it needs to for the rows it eventually decodes (`crates/app_core/src/gpu_backtest_kernel.rs:2770-2894`). Second, it correctly kept the raw probe-helper surface in scope; `prepare_cuda_kernel_runtime_for_probe()` still returns a raw `usize` handle backed by thread-local runtime ownership (`crates/app_core/src/gpu_backtest_kernel.rs:1907-1918, 2004-2006`).

What the third pass underweighted is that there are still **clearer, safer, more contract-driven fixes** ahead of compaction.

### 1. The uploaded dynamic-batch public contract is still under-validated, and that is a better safe-now target than output compaction

`upload_double_ma_dynamic_program_batch()` checks nonzero stride, nonempty combos, row-count agreement, and `lr`/`stride` fitting into `i32` (`crates/app_core/src/gpu_backtest_kernel.rs:2610-2656`). But it still does **not** validate the sharpest geometry assumptions that the execution path relies on immediately after:

- `first_valid` is stored as-is and later passed to the kernel unchanged (`crates/app_core/src/gpu_backtest_kernel.rs:2715-2719, 2798-2799`).
- the kernel time axis is driven by `uploaded_batch.d_lr.len()` (`crates/app_core/src/gpu_backtest_kernel.rs:2778-2780`), but the uploaded MA matrices are only validated as “multiple of stride,” not as having `stride == lr.len()`.
- `fast_tm_stride`, `slow_tm_stride`, and `combo_count` are later cast to `i32` without upload-time upper-bound checks (`crates/app_core/src/gpu_backtest_kernel.rs:2782-2788, 2813`).

The sibling exact/precomputed path is stricter. It validates precomputed matrix width against `n` and rejects `first_valid_override >= n` (`crates/app_core/src/gpu_backtest_kernel.rs:3442-3468, 12423-12505`). That is a strong signal that the uploaded helper path is still the looser contract surface.

This is the clearest missed fourth-pass finding: **the best immediate hardening is still input-contract tightening**, not compaction. The minimal, evidence-backed version is:

- reject `first_valid >= lr.len()` to align with the exact path’s “must leave at least one valid index” rule,
- reject uploaded MA matrices whose column count does not equal `lr.len()` rather than merely being a multiple of a caller-provided stride,
- reject period-grid and combo counts that would later overflow the kernel’s `i32`/`u32` launch and argument casts.

That is more conservative than the third-pass proposal and more clearly correctness-oriented.

### 2. The third-pass compaction recommendation is directionally good, but it is overstated as the “next” change

The third-pass packet called uploaded-path compaction the “highest-ROI small fix.” I do not think the live evidence supports that as strongly as stated.

The main reason is scope. `DynamicDoubleMa` is routed in repo guidance as a constrained builder/preview surface, not the core optimization engine (`REPO_MAP.md:1057-1074, 1223`). That does not make the uploaded path unimportant, but it does make it much harder to justify elevating a performance refactor on that path above public-contract hardening on a shared public module.

The second reason is semantic risk. The uploaded path’s pair map is **combo-driven** and may contain duplicate or arbitrary dense pair indices (`crates/app_core/src/gpu_backtest_kernel.rs:2672-2692`). The main exact path’s compact-output machinery is built around valid-pair maps and companion period/local-row buffers (`crates/app_core/src/gpu_backtest_kernel.rs:1928-1988, 10186-10203`). Without the CUDA kernel source in this review bundle, the third-pass packet does not fully prove that simply flipping the uploaded path into compact mode preserves row ordering for duplicate or unsorted combo lists.

The third reason is that compaction only solves one slice of the retention story. The scratch cache is still thread-local and unbounded by shape churn (`crates/app_core/src/gpu_backtest_kernel.rs:2004-2006, 2816-2824`). Compacting one path reduces one retained buffer size, but it does **not** fix the lack of eviction/reset policy. So the third-pass packet slightly overclaims the residency payoff.

My read is: **keep compaction on the list, but demote it from “obvious next fix” to “measured perf candidate after contract hardening.”**

### 3. The accepted dynamic-handle surface is broader than the hard-coded runtime path proves

This is the biggest issue the third-pass packet did not really surface.

`validate_double_ma_dynamic_strategy_handle()` checks only a partial semantic contract: backend, profile, absence of halftrend filter, cross exit mode, exactly two indicator slots, slot indices limited to fast/slow, and ABI operand-source kinds limited to indicator/constant (`crates/app_core/src/gpu_backtest_kernel.rs:2550-2599`).

But `eval_uploaded_double_ma_dynamic_program_gpu_kernel()` does **not** pass uploaded predicate-program buffers into the kernel launch at all. It warms the executable cache, then launches the hard-coded `double_cross_backtest_tm_flex_f32_m7` kernel (`crates/app_core/src/gpu_backtest_kernel.rs:2740-2878`). The `_uploaded_program` value handed into the closure is unused.

That means the safe interpretation of this API is narrower than the public signature suggests. In practice, this path appears to be relying on “the accepted handle is semantically equivalent to the shipped plain crossover DynamicDoubleMa template,” not on “the uploaded GPU predicate program is truly being executed here.”

I am **not** claiming the current in-repo caller is wrong. The repo guidance suggests the current surface is indeed constrained. But as a public helper API, this is under-documented and under-enforced. The safe-now action is documentation:

- document that this helper path is only intended for the constrained, plain-crossover DynamicDoubleMa builder output,
- document that upload-time strategy is not frozen into the uploaded batch,
- document that evaluation-time `strategy`/`handle` compatibility is part of the caller contract.

The larger fix — either true semantic validation of the accepted handle shape or an execution path that actually consumes the uploaded predicate program — is **deferred**, because it is no longer a small refactor once you cross that boundary.

### 4. The split upload/eval API is itself under-documented

There is a smaller contract clue here that the third-pass packet did not call out: `upload_double_ma_dynamic_program_batch()` still takes `_strategy: &StrategyConfig`, but that parameter is unused at upload time (`crates/app_core/src/gpu_backtest_kernel.rs:2618-2620`). The actual kernel flags and risk settings are derived later from the `strategy` passed into evaluation (`crates/app_core/src/gpu_backtest_kernel.rs:2743, 2798-2805`).

That is not necessarily a bug, but it is a public-API sharp edge. A caller could reasonably assume the upload step locks in strategy semantics. The source says otherwise. That needs rustdoc now, and only later — in a breaking-change window — should the signature be reconsidered.

### 5. The first-pass diagnostics and observability concerns still beat speculative runtime cleanup

Two first-pass concerns remain live and were underweighted by the third pass:

- `load_double_crossover_module()` still drops cubin/PTX/JIT failure context and returns only the last bare-PTX error (`crates/app_core/src/gpu_backtest_kernel.rs:1879-1904`). In a CUDA bring-up path, that is low-risk, high-value diagnostic hardening.
- `CampaignVramPrefetchOutcome::AlreadyResident` still conflates “already active resident” and “already staged for promotion,” and that same label flows into progress feedback (`crates/app_core/src/gpu_backtest_kernel.rs:3990-3993, 4269-4282, 12143-12152`). The safe fix is still documentation, not enum-shape churn.

Those are narrower and safer than a performance-motivated layout change.

## Implementation Priority

### Safe-now hardening

The priority order I would use now is:

1. **Harden uploaded-batch geometry validation**
   - reject `first_valid >= lr.len()`,
   - reject uploaded matrix column counts that do not equal `lr.len()`,
   - reject period-grid and combo counts that exceed later kernel cast limits.

2. **Document the public helper contract instead of implying it**
   - `UploadedDoubleMaDynamicProgramBatch` is device-specific and tied to a thread-local CUDA/runtime story,
   - `upload_double_ma_dynamic_program_batch()` is a data-upload step; it does not freeze strategy flags,
   - `eval_uploaded_double_ma_dynamic_program_gpu_kernel()` is only safe for the constrained DynamicDoubleMa/plain-crossover handle surface currently accepted,
   - `prepare_cuda_kernel_runtime_for_probe()` is a raw-handle helper with same-thread/runtime-affinity sharp edges,
   - `CampaignVramPrefetchOutcome::AlreadyResident` currently covers both active-resident and already-staged cases.

3. **Improve loader diagnostics**
   - preserve cubin/PTX attempt context in the final module-load error instead of returning only the last fallback failure.

4. **Add the missing negative tests before any perf refactor**
   - invalid `first_valid`,
   - uploaded-matrix width/stride mismatch,
   - optionally, upper-bound validation for kernel argument casts if that validation is added.

### Larger redesign still deferred

These are still valid, but they should stay deferred or be treated as coordinated perf work rather than the next safe patch:

- compacting the uploaded dynamic exact path,
- narrowing `prepare_cuda_kernel_runtime_for_probe()` visibility or replacing the raw `usize` with an opaque type,
- tightening the accepted DynamicDoubleMa handle semantics beyond the current partial validator,
- adding eviction/reset policy for `DYNAMIC_GPU_METRICS_SCRATCH`,
- splitting this file into smaller modules.

The important fourth-pass correction is that **uploaded-path compaction belongs in this deferred bucket unless and until parity/order tests and a GPU perf gate say otherwise**.

## Verification If Changed

For safe-now contract hardening, I would keep verification focused and local:

- existing exact/dynamic coverage already worth rerunning:
  - `dynamic_double_ma_program_gpu_kernel_matches_cpu_plain_sma_metrics` (`crates/app_core/src/gpu_backtest_kernel.rs:18575+`)
  - `uploaded_dynamic_double_ma_batch_relaunches_consistently` (`crates/app_core/src/gpu_backtest_kernel.rs:18740+`)
  - `uploaded_dynamic_double_ma_batch_rejects_mismatched_handle_executable_shape` (`crates/app_core/src/gpu_backtest_kernel.rs:18841+`)
  - `period_upload_signature_tracks_full_period_contents` (`crates/app_core/src/gpu_backtest_kernel.rs:16170+`)
  - `staged_market_promotes_into_active_resident_cache` (`crates/app_core/src/gpu_backtest_kernel.rs:16199+`)

- add targeted negative tests for:
  - uploaded batch rejects `first_valid >= lr.len()`,
  - uploaded batch rejects MA matrices whose column count does not match `lr.len()`,
  - if handle-contract docs or checks are tightened, add a failure case proving that unsupported DynamicDoubleMa plan shapes are rejected explicitly rather than silently treated as plain crossover.

If the uploaded-path compaction refactor is attempted anyway, the verification bar needs to be higher than the third-pass packet suggested. In addition to the existing tests, add:

- duplicate combo rows,
- non-sorted combo order,
- parity versus current decode ordering,
- GPU perf gate evidence.

Only then should `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` become mandatory for that change.

## Regression And Feature-Preservation Constraints

The highest regression risks here are **public helper compatibility**, **GPU ABI assumptions**, and **row-layout/ordering semantics**.

Do not casually change launch heuristics, stream topology, pinned-memory usage, or graph-capture behavior. The official CUDA guidance still supports treating those as benchmark-sensitive primitives, not hygiene targets.

Do not enable uploaded-path compaction without proving what happens for duplicate and arbitrary-order combo maps. The uploaded helper path is not using the same obviously-structured valid-pair maps as the exact path, so “pair-map semantics remain unchanged” is not something I would claim without new tests.

Do not narrow `prepare_cuda_kernel_runtime_for_probe()` visibility unless the crate owners are willing to treat that as a public-surface change. `gpu_backtest_kernel` is still a public module (`crates/app_core/src/lib.rs:29-30`), and the uploaded dynamic helpers are re-exported directly (`crates/app_core/src/lib.rs:291-295`).

Do not treat uploaded-path compaction as a fix for scratch-lifetime policy. It is a buffer-size improvement, not an eviction design.

Do not over-broaden DynamicDoubleMa acceptance on this path. Right now the validator is only a partial proxy for the handle semantics, and the launch path is still hard-coded crossover execution. Any change there has real GPU ABI and parity risk.

IPC and persistence are not the main risk surfaces here. The main risk surfaces are runtime compatibility, public API sharpness, GPU execution semantics, and preserving existing exact-metric decode behavior.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `Small surgical refactor` is still the right bucket for `crates/app_core/src/gpu_backtest_kernel.rs`.

But the **priority inside that bucket should change**.

The third-pass packet was right that the uploaded dynamic exact path still wastes dense output and retains a larger scratch allocation than it strictly needs. What it got wrong was the confidence level that this should be the next change. On the live source, the safer next move is still:

- tighten the uploaded-batch geometry contract,
- document the sharp public helper semantics,
- preserve richer loader diagnostics,
- keep prefetch outcome semantics truthful without changing enum shape yet.

The uploaded-path compaction idea is still valuable, but it is not the best conservative next step. It should be treated as deferred, measurement-backed performance work with stronger ordering/parity coverage, not as the new default hardening target.
