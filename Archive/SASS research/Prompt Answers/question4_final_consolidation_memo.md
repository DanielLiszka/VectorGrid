## Question 4 — final consolidation memo

This memo consolidates the first-pass, second-pass, and final-prompt sequence for Question 4 from the uploaded bundle and treats the result as the repo-ready outcome for this thread. fileciteturn5file0 fileciteturn5file1 fileciteturn5file2 fileciteturn5file3 fileciteturn5file17

### Adopted direction

**Close the standalone “batch-synchronous, strategy-agnostic optimizer” direction.**  
The only residue worth keeping is **not** a universal optimizer thesis; it is a **regime definition** that should feed the separate optimizer-portfolio / online algorithm-selection track. In this repo, giant synchronous batches, mixed conditional spaces, and exact warm evaluation are real and useful constraints, but they do not by themselves create a sufficiently novel standalone CS contribution once compared against current mixed-space, batch, delayed-feedback, and massively parallel BO literature. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/ru20a.html))

### Exact problem statement

The only precise formulation that still looks scientifically meaningful is this:

Given a task family \(s \in \mathcal{S}\), each with a mixed discrete/continuous/conditional search space \(X_s\) and an expensive exact evaluator \(f_s : X_s \to \mathbb{R}\), run for \(T\) synchronous rounds. At round \(t\), choose a batch \(B_t = \{x_{t,1}, \ldots, x_{t,q_t}\}\subset X_s\); observe all outcomes only after the full batch completes; and minimize simple regret or wall-clock-to-target under a fixed round, evaluation, or time budget. The only potentially publishable decision layer is **not** “one optimizer for all tasks,” but a **round-level controller** that allocates each giant batch across solver heads or proposal policies and updates only at round boundaries. That controller framing is the part that overlaps most naturally with P3BO-style few-round populations and newer massively parallel portfolio BO. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/angermueller20a/angermueller20a.pdf))

### Why Q4 does not survive prior-work comparison

The broad Q4 thesis is too crowded. CoCaBO already handles mixed continuous/categorical BO and explicitly extends to the batch setting; HyBO targets hybrid discrete/continuous spaces; Bounce is a strong mixed/combinatorial BO baseline; BO-SDF explicitly treats batch BO as a special case of delayed-feedback BO; and SOBER is built around scalable, diversified large-batch BO over discrete and mixed spaces. That means “mixed-space + batch + expensive evaluator” is no longer a clean gap. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/ru20a.html))

The narrowed reformulation is also not empty territory. P3BO already argues for population-style optimization when only a few large experimental rounds are available, and a 2025 JAIR paper now explicitly studies a **portfolio approach to massively parallel Bayesian optimization** aimed at native massive batching. That is exactly why the controller/portfolio version belongs under the separate portfolio-selection track rather than being defended here as a fresh standalone optimizer idea. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/angermueller20a/angermueller20a.pdf))

A reviewer would also have an easy “why synchronous?” attack. PLAyBOOK explicitly reports that asynchronous BO often outperforms synchronous batch BO in both wall-clock time and sample efficiency, especially as batch size grows. And any attempt to rescue Q4 with a “setup-aware” or “batch-coupled state” story runs into nearby prior work on process-constrained BO and transition-constrained BO. ([proceedings.mlr.press](https://proceedings.mlr.press/v97/alvi19a.html))

### Why this repo is still useful

VectorGrid is still a strong **substrate** for this regime. The README describes a warm app-path benchmark where measured trials reuse resident market-series buffers, and the product framing is explicitly “GPU-accelerated exact-grid optimization and validation,” not surrogate benchmarking. AGENTS further states the architecture target for supported GPU strategies: optimization should be GPU-executed end to end within that stage, with prices, derived series, and candidate buffers remaining VRAM-resident until compact results are returned. The profile runner spans `DoubleMa`, `TrendQuality`, `RegimeHybrid`, and `BreakoutMomentum` across `Small` through `Xlarge`, while the repo docs and routing files show that multiple strategy families already share the `DoubleMa` product path and route policies. App-core also already contains an internal budgeted GPU search mode, `GpuVdps`. All of that makes the repo a good **evaluation environment** for round-constrained search, but not by itself a new optimizer thesis. fileciteturn5file11 fileciteturn5file4 fileciteturn5file5 fileciteturn5file6 fileciteturn5file7 fileciteturn5file12

### Deferred directions

The useful residue of Q4 should be deferred into two other tracks.

First, the **optimizer-portfolio / online selection** track: if anything is revived from Q4, it should be a round-level controller that allocates giant synchronous batches across existing solver heads, not a new universal optimizer. That is now the more honest framing, but it should be pursued under the portfolio question rather than under Q4. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/angermueller20a/angermueller20a.pdf))

Second, the **benchmark/platform** track: VectorGrid’s exact anchor, warm resident execution, route-conditional cases, and multiple strategy families may still support a strong benchmark or systems memo. That is conservative, but it is a different claim from “we invented a new optimizer.” fileciteturn5file11 fileciteturn5file4 fileciteturn5file12

### Rejected directions

Rejected outright are three variants.

The first is the original **strategy-agnostic universal optimizer** thesis. It is too saturated relative to current mixed-space, batch, delayed-feedback, and massively parallel BO work. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/ru20a.html))

The second is a **setup-aware / stateful synchronous optimizer** without a much sharper formal model than process-constrained or transition-constrained BO. That story is too collision-prone to justify reopening implementation from this repo alone. ([proceedings.neurips.cc](https://proceedings.neurips.cc/paper/2017/file/1f71e393b3809197ed66df836fe833e5-Paper.pdf))

The third is any thesis where **CUDA carries the novelty**. If the conceptual contribution disappears when the evaluator is swapped for another fast backend, the work is drifting toward implementation novelty only. That is exactly the warning the prompt bundle told us to avoid. fileciteturn5file0 fileciteturn5file2

### Role of the custom CUDA implementation

For any surviving residue here, CUDA is **substrate only** and, at most, a **systems enabler**. In this repo the warm, resident GPU path is essential for exposing the intended regime at realistic scale, and the implementation surfaces in `double_ma.rs` / `gpu_backtest_kernel.rs` are real strengths. But they are not the scientific claim. If replacing CUDA with another fast synchronous evaluator destroys the paper, that is a warning sign, not a success condition. fileciteturn5file11 fileciteturn5file4 fileciteturn5file6 fileciteturn5file7

### Minimum prototype that would be required for any future revival

No implementation should reopen under Q4 as it stands. The **minimum revival threshold** would be:

- a backend-agnostic round controller that allocates each batch across solver heads rather than proposing all points from one optimizer;
- at least one repo-native head (`GpuVdps`) plus pragmatic heads such as random/Sobol and one strong evolutionary baseline;
- updates only at round boundaries, with explicit handling of giant synchronous batches;
- evaluation across multiple strategy families, not just `DoubleMa`. fileciteturn5file6 fileciteturn5file5

Anything weaker than that is implementation-level search plumbing, not a research contribution.

### Mandatory baseline matrix for any revival

Consistent with the final bundle’s baseline discipline, any revival would need this matrix, not a favorite-baseline comparison. fileciteturn5file2

- **Exact baselines:** current exact GPU brute-force `DoubleMa`, plus exact CPU where feasible for correctness/scaling reference. fileciteturn5file11
- **Repo internal search baseline:** existing `GpuVdps`. fileciteturn5file6
- **Pragmatic baselines:** random and Sobol / uniform batched sampling.
- **Classical heuristic baseline:** one strong mixed-variable evolutionary search method.
- **Modern mixed-space BO baselines:** at least two chosen from CoCaBO, HyBO, Bounce, and SOBER. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/ru20a.html))
- **Massive-batch / few-round baseline:** P3BO-style population/portfolio control or the 2025 massively parallel portfolio BO approach. ([proceedings.mlr.press](https://proceedings.mlr.press/v119/angermueller20a/angermueller20a.pdf))
- **Ablations:** fixed solver instead of controller; no diversity/coverage rule; small repeated batches instead of giant rounds; no cross-family transfer.

The measured outputs would have to be wall-clock to target quality, simple regret under fixed budget, anytime incumbent quality, robustness across seeds, and cross-family performance.

### Mandatory benchmark slate for any revival

Again following the final bundle’s own standard, the minimum internal slate is broader than `DoubleMa`. fileciteturn5file2

- **Exact lattice anchor:** `DoubleMa`. fileciteturn5file11
- **Cross-family profile:** `TrendQuality`, `RegimeHybrid`, and `BreakoutMomentum` from the runner surface. fileciteturn5file5
- **Route-conditional case:** at least one ADX-gated or hybrid-route case from the current route policy for `CompositeTrend`, `BreakoutMomentum`, or `RegimeHybrid`. fileciteturn5file4
- **Validation-coupled case:** at least one optimization problem whose value depends on downstream validation cost/behavior, because validation is part of the shipped execution surface. fileciteturn5file11 fileciteturn5file4

If a method only looks good on `DoubleMa`, it is not a broader CS direction.

### Direction-specific kill test

Kill the direction immediately if a simple portfolio of current methods — especially `GpuVdps` plus strong external mixed-space baselines or a modern massively parallel portfolio BO baseline — matches or beats the proposed controller across `DoubleMa`, `TrendQuality`, `RegimeHybrid`, and `BreakoutMomentum` under fixed round and wall-clock budgets. Kill it again if gains vanish once you leave `DoubleMa`, or if the argument collapses when CUDA is treated purely as execution substrate. ([arxiv.org](https://arxiv.org/abs/2110.09334)) fileciteturn5file6 fileciteturn5file5

### Final verdict

**Close the direction.**  
Do **not** reopen implementation for Q4 as a standalone research target. Treat “batch-synchronous giant-batch mixed-space optimization” as a useful regime description that feeds the optimizer-portfolio track and, secondarily, the benchmark/platform track, but not as its own universal-optimizer thesis. Under the bundle’s own go/no-go logic, the broad Q4 framing has a precise problem statement and an obvious evaluation plan, but it does not clear the novelty bar cleanly enough, and its practical distinctiveness leans too heavily on the repo’s execution substrate. fileciteturn5file2

That conclusion also matches the repo’s recent research posture: retain bounded wins, preserve negative results, and stop branches that do not justify promotion. The Phase 2 experiment table explicitly separates retained from rejected branches; the artifact plan freezes unjustified new prototypes; the research findings memo says the correct next move is artifact consolidation when evidence does not justify more code; the CUPTI follow-up ended as research/tooling only; the xlarge cache-reuse probe was a no-go despite better cache metrics; and the TM-native VWMA work was retained only as a tightly bounded pilot. Q4 should be handled the same way. fileciteturn5file13 fileciteturn5file14 fileciteturn5file15 fileciteturn5file16 fileciteturn5file8 fileciteturn5file9 fileciteturn5file10
