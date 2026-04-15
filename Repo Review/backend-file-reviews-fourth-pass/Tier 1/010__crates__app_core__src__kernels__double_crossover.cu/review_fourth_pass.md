# Fourth-Pass Review: `crates/app_core/src/kernels/double_crossover.cu`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/kernels/double_crossover.cu`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kernels/double_crossover.cu.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kernels/double_crossover.cu.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kernels/double_crossover.cu.review.md`
- Third-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Fourth-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file warranted a fourth pass because the third-pass packet was directionally correct but not yet sharp enough about where the live risk actually sits.

The third pass correctly re-centered the review on the Rust/CUDA contract rather than on superficial formatting or file-size complaints. But after reopening the live source together with the current Rust launch/runtime code, the more important question is not simply whether the kernel translation unit contains handwritten ABI state. The sharper question is which parts of that ABI are actually load-bearing on the executed hot path today, and which failures would present as silent route/performance degradation instead of clean runtime failure.

That distinction matters here. `double_crossover.cu` still owns a very large, benchmark-sensitive surface: handwritten ABI structs near the top (`double_crossover.cu:38-65`), a strategy-program probe (`double_crossover.cu:69-115`), a dedicated dynamic-strategy backtest kernel (`double_crossover.cu:373-700`), fixed-row metrics-store helpers (`double_crossover.cu:4106-4218`), a wide family of specialized Double-MA kernels (`double_crossover.cu:5392-6230` and beyond), and the late CUB-backed top-k reducers (`double_crossover.cu:8963-10351`). The bucket therefore still stands. What needed correction was the priority of the risks inside that bucket.

## Fourth-Pass Research Log

This pass re-read the live source and the current Rust-side consumers before trusting the earlier packets.

Local source and contract checks performed in this pass:

- `double_crossover.cu`
  - ABI structs and probe: `38-115`
  - operand evaluation and source handling: `220-242`
  - dedicated strategy-program kernel: `373-700`
  - fixed-row zero/store helpers: `4106-4218`
  - `m7` fixed-contract kernel entry: `6187-6230`
  - CUB-backed top-k region: `8963-10351`
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - dynamic-handle validator: `2550-2599`
  - probe uploader/launcher: `2449-2488`
  - uploaded dynamic execution path: `2726-2867`
  - fixed-contract specialization symbol loading: `9191-9305`
  - dispatch selection based on symbol presence: `10279-10360`
  - explicit fallback tests when matching specialized symbols are absent: `18194-18263`
- `crates/app_core/src/strategy_gpu_program.rs`
  - Rust-side numeric encodings: `145-177`
  - executable fingerprint construction: `197-227`
  - ABI compilation: `229-250`
- `crates/app_core/src/strategy_gpu_runtime.rs`
  - upload-time probe enforcement: `270-301`
  - upload-cache availability test: `598-640`

External primary-source recheck performed in this pass:

1. NVIDIA CUDA C++ Programming Guide
   - Reconfirmed that launch configuration, launch-bounds behavior, and low-level kernel contracts are execution-shape-sensitive and should not be “cleaned up” casually.
2. NVIDIA CUDA C++ Best Practices Guide
   - Reconfirmed that hot-kernel changes should be measurement-backed, especially where occupancy, coalescing, and codegen-sensitive helpers are involved.
3. NVIDIA CCCL / CUB documentation
   - Reconfirmed that block-wide radix-sort helpers encode non-trivial block-shape and storage assumptions; they are not just library decoration.
4. Rust Reference: Type Layout
   - Reconfirmed that cross-language layout assumptions need explicit control and explicit checking, not inference.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**.

`double_crossover.cu` is still a benchmark-sensitive, ABI-sensitive, shared CUDA translation unit with a broad blast radius. The fourth pass did not find a clear correctness bug that justifies opening the kernel body for opportunistic cleanup. It did, however, find that the third-pass packet aimed at the wrong primary hardening lane.

### What the third-pass packet got right

The third-pass packet got several important things right.

First, it was correct to keep the file in `Benchmark-sensitive; do not touch yet`. The live source still contains handwritten ABI structs (`double_crossover.cu:38-65`), multiple specialized kernel families, and a CUB-backed top-k tail. That is not a file for cosmetic refactors.

Second, it was right that layout and numeric encodings matter. The Rust-side encoder still hand-maps value sources, condition ops, group modes, and routes to raw integers in `strategy_gpu_program.rs:145-177`, while the CUDA side carries its own ABI struct definitions and operand evaluation logic in `double_crossover.cu:38-65` and `220-242`.

Third, it was right to prefer validation/probe hardening over structural churn. That instinct is correct. The mistake was being too generic about *where* that hardening should happen first.

### What the third-pass packet missed or underweighted

#### 1. Silent specialization loss is a more immediate live risk than raw symbol failure

This is the biggest miss.

The current Rust runtime loads many optimized symbols with `.ok()` rather than as mandatory requirements, including `windowed_m17`, `windowed_m22`, `m17`, `m22`, lockstep variants, and regime/no-risk specializations (`gpu_backtest_kernel.rs:9191-9305`). Dispatch then chooses optimized routes only when those symbol lookups succeed (`gpu_backtest_kernel.rs:10279-10360`). The repo also has explicit unit tests asserting that dispatch falls back to `FixedContractDispatchKind::None` when the matching specialization is unavailable (`gpu_backtest_kernel.rs:18194-18263`).

That means some drift in this CUDA file will not fail as a clean launch error. It will fail open into a slower generic route. For a Tier 1 benchmark-sensitive file, that is the nastier risk.

So the fourth-pass correction is this: the current packet over-focuses on “runtime failure from ABI/symbol drift” and under-focuses on **silent benchmark regression from specialization inventory drift**.

#### 2. The generic dynamic-strategy ABI is less load-bearing on the executed hot path than the third-pass framing suggests

The third-pass packet treats the CUDA half of the dynamic-strategy ABI as though it were the dominant live execution contract. The current code does not support that framing.

The dedicated strategy-program kernel exists in the CUDA file as `strategy_program_double_ma_backtest_f32` (`double_crossover.cu:373-700`). In the supplied Rust-side runtime context, however, the uploaded dynamic execution path does **not** launch that kernel. `eval_uploaded_double_ma_dynamic_program_gpu_kernel` validates the handle, checks the executable fingerprint, ensures an uploaded executable exists in the thread-local cache, and then launches `double_cross_backtest_tm_flex_f32_m7` (`gpu_backtest_kernel.rs:2726-2867`). The argument list passed at launch time contains MA buffers, returns, periods, regime/risk flags, metrics settings, pair maps, and the output buffer, but no uploaded strategy-program ABI pointers (`gpu_backtest_kernel.rs:2829-2867`).

That is not enough to call the current implementation wrong. The same file explicitly narrows the supported plan shape to a Double-MA-only, plain-crossover, two-indicator-slot subset, and it rejects non-indicator/non-constant operands before launch (`gpu_backtest_kernel.rs:2550-2599`). So there is a plausible intentional design here: v1 “dynamic GPU” support is really a constrained lowering to the fixed crossover executor.

But it does mean the third-pass packet overstates how much of the generic predicate-program ABI is actually on the executed hot path today. The live state looks more like a **partly decoupled upload/probe/fingerprint surface** than a fully generic predicate-program executor.

#### 3. The probe hardening opportunity is mostly in Rust-side enforcement, not in `.cu` edits first

The third-pass recommendation to add stronger ABI validation was correct in spirit but too vague in practice.

The CUDA probe writes seven values: ABI version, indicator-slot count, condition count, group count, group-condition-index count, bound-signal count, and a narrow checksum over only the first slot/condition/group/index (`double_crossover.cu:69-115`). But the current Rust-side uploader only checks the first three of those values: `abi_version`, indicator-slot count, and condition count (`strategy_gpu_runtime.rs:270-301`). Group count, group-condition-index count, bound-signal count, and the checksum are generated but not enforced.

That matters because it identifies a truly low-risk hardening lane: tighten the Rust-side upload/probe assertions and tests first. That work can happen without opening the kernel hot path at all.

#### 4. Performance sensitivity is broader than the late CUB top-k section

The earlier packets correctly flagged the CUB top-k region, but they underweighted another codegen-sensitive area: the fixed-row metrics writers.

`store_metrics_row_fixed_f32` already contains explicit packed-store paths for the 17-field and 22-field contracts, using `float4` / `float2` alignment checks and fast paths (`double_crossover.cu:4117-4198`). The repo also carries a dedicated ALMA contract-row perf gate in addition to the generic GPU perf gate. That is a signal that row-contract layout/store behavior is itself benchmark-sensitive, not just the final CUB reducers.

So the fourth-pass correction is that “performance-sensitive” here is not only “don’t rewrite the top-k algorithm.” It also includes “do not casually perturb row-store helpers, metric strides, or contract-width-specific paths.”

#### 5. There is a latent capability mismatch, but it is not the dominant current hot-path hazard

Rust-side ABI compilation still encodes `MaSource => 1` and `Close => 2` (`strategy_gpu_program.rs:145-151`). CUDA-side generic operand evaluation currently only handles source `0` (indicator) and `3` (constant), returning `NAN` otherwise (`double_crossover.cu:220-242`). The current runtime protects itself by rejecting unsupported operand sources before launch (`gpu_backtest_kernel.rs:2587-2598`).

That is real technical debt, but the fourth pass would classify it as a **latent capability inconsistency**, not the main current production hazard. The live runtime is already narrowed away from it.

### What the third-pass packet overstated

The main overstatement is not that the packet was wrong about ABI risk. It is that it treated ABI/layout/encoding drift as the primary sharpened risk **instead of** separating three different categories:

1. active hot-path risk: silent specialization loss and route drift,
2. adjacent contract-validation weakness: under-enforced upload/probe results,
3. deferred design debt: the partly decoupled generic predicate-program surface.

Those are not the same thing, and collapsing them into “ABI drift sharpened” makes the packet less actionable than it should be.

The other overstatement is the practical implication that the safest immediate work is still mostly comments/documentation. Comments are harmless, but on this file they are not the highest-ROI safe-now move. Tightening the Rust-side upload/probe checks and adding route-presence/perf-fallback regression coverage is more valuable and still conservatively scoped.

## Implementation Priority

### Safe-now hardening

The best safe-now hardening items are **adjacent to** this file, not inside its hot loops.

1. **Expand upload-time probe enforcement in `strategy_gpu_runtime.rs`.**
   - Today the uploader only validates `probe[0..=2]` (`strategy_gpu_runtime.rs:285-289`).
   - Low-risk hardening would also compare group count, group-condition-index count, and the existing checksum/bound-signal fields emitted by the CUDA probe (`double_crossover.cu:85-114`).
   - This should not regress kernel throughput because it lives on the upload/probe path, not inside the backtest loop.

2. **Add tests that make the current v1 dynamic-GPU contract explicit.**
   - Today the runtime clearly constrains supported plans (`gpu_backtest_kernel.rs:2550-2599`), uploads/probes the ABI, and then executes via `double_cross_backtest_tm_flex_f32_m7` (`gpu_backtest_kernel.rs:2726-2867`).
   - The safe-now action is not to “fix” that in the kernel. It is to add tests and/or comments at the Rust boundary that state whether this is an intentional lowering strategy.
   - That avoids future reviewers mistaking a future-facing surface for a fully generic executed path.

3. **Add perf-sensitive symbol-presence regression coverage for shipped specialization families.**
   - Because the runtime explicitly falls back when specialized symbols are absent (`gpu_backtest_kernel.rs:18194-18263`), a symbol-inventory regression can become a silent performance regression.
   - Safe-now hardening is to make the expected specialization inventory more explicit in tests/verification for the routes that are supposed to ship fast.
   - This is a safer first move than touching kernel registrations or renaming symbols.

4. **Treat fixed-row store helpers as perf-gated code, not generic cleanup territory.**
   - Any change near `store_metrics_row_fixed_f32` / `zero_metrics_row_fixed_f32` (`double_crossover.cu:4106-4218`) should automatically trigger both the generic GPU perf gate and the specialized ALMA contract-row perf gate.
   - That is a process hardening step, not a source rewrite.

5. **Add an explicit rejection test for latent operand-source widening.**
   - Since Rust-side ABI generation can encode `MaSource` and `Close` but the live runtime/kernel v1 path does not support them end-to-end (`strategy_gpu_program.rs:145-151`, `double_crossover.cu:220-242`, `gpu_backtest_kernel.rs:2587-2598`), it is worth pinning the current rejection behavior in tests until broader execution support really exists.
   - This avoids accidental surface widening that would look harmless in Rust but degrade to `NAN`-driven behavior on the device.

### Larger redesign still deferred

These are real issues, but they still belong in the deferred bucket.

1. **Wiring the dedicated strategy-program kernel into the live runtime, or retiring/renaming that surface.**
   - The dedicated kernel exists (`double_crossover.cu:373-700`), but the live uploaded execution path launches `double_cross_backtest_tm_flex_f32_m7` instead (`gpu_backtest_kernel.rs:2759-2867`).
   - The dedicated kernel also appears to speak a 10-field reduced output contract (`double_crossover.cu:689-699`), while the current Rust-side runtime explicitly manages 10/17/22 metric-row families (`gpu_backtest_kernel.rs:156-158`).
   - That makes this feature work, not cleanup. Treat it as a redesign with parity and perf consequences.

2. **Splitting the translation unit by kernel family.**
   - Still attractive for maintainability.
   - Still too risky to do casually because this file mixes ABI/probe kernels, options-expression work, core Double-MA backtest families, regime builders, row writers, and CUB reducers in one performance-sensitive unit.

3. **Replacing optional specialization probing with an explicit specialization manifest/contract.**
   - Architecturally cleaner.
   - But changing fail-open lookup behavior into hard requirements or a new manifest system will alter route selection, compatibility posture, and possibly benchmark fallback behavior. That is redesign work.

Any of those deferred items could be worthwhile. None of them are “safe-now hardening.”

## Verification If Changed

If anything around this file or its immediate Rust-side launch contracts changes, verification should be stricter than the earlier packet suggested.

Minimum correctness/contract coverage:

- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_backtest_kernel -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel strategy_gpu_program -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel strategy_gpu_runtime -- --nocapture`

Minimum performance coverage:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

Additional required performance coverage when row contracts, packed store helpers, or fixed-contract specialization symbols move:

- `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`

Supplemental/manual evidence worth running for deeper investigation, but not as a replacement for the gates above:

- the app-core benchmark surfaces routed in `REPO_MAP.md`, especially `crates/app_core/benches/double_ma_opt.rs`
- the profiling helpers routed in `REPO_MAP.md`

## Regression And Feature-Preservation Constraints

Preserve the following unless the Rust launch/runtime side is intentionally updated together and the performance gates are rerun:

- kernel entry names and specialization inventory for the currently shipped fast paths,
- Rust/CUDA numeric encodings and ABI layouts,
- current fail-open vs fail-hard route-selection behavior,
- metric-row contract widths and packed-store assumptions,
- current rejection of unsupported operand sources on the dynamic v1 GPU path.

Do **not** treat “the app still runs” as sufficient evidence after touching this file. In the current runtime, missing or renamed specialization symbols can still yield a valid but slower fallback route instead of a clean failure.

Do **not** route a “safe-now hardening” task into the kernel body unless the change is obviously non-semantic. The most defensible immediate work here lives in Rust-side probe enforcement, contract tests, and perf-fallback detection.

If anyone chooses to wire `strategy_program_double_ma_backtest_f32` into the live runtime later, that should be treated as a feature/redesign task with a fresh parity and benchmark plan, not as a cleanup follow-up from this packet.

## Fourth-Pass Conclusion

The current third-pass bucket should **remain unchanged**: `Benchmark-sensitive; do not touch yet`.

What the third-pass packet got right was the overall caution level. What it still missed was the live hierarchy of risk.

The fourth pass would summarize that hierarchy this way:

- **Highest live risk:** silent specialization loss and route/performance fallback when this file’s optional optimized symbols drift.
- **Best safe-now hardening:** stronger Rust-side upload/probe assertions and tests that make the current dynamic-v1 lowering/constraints explicit.
- **Real but deferred redesign debt:** the partly decoupled generic predicate-program kernel surface and any attempt to make it the actual executed path.

So the correct adversarial conclusion is not “the bucket was wrong.” The bucket was right. The packet’s internal emphasis was just still off by one level. The safest near-term work is not kernel cleanup and not comments-only cleanup. It is conservative contract hardening around the Rust/CUDA boundary and conservative regression detection for specialization availability, while keeping the kernel hot path itself frozen until a benchmark-backed redesign is explicitly budgeted.
