# PR-03 Static candidate-space pruning and route cost model

Compiled first-pass, second-pass, and third-pass answers.

Source prompt bundle: fileciteturn31file0

---

## First-pass answer

## `PR-03` Static candidate-space pruning and route cost model

Owned problem:
static Double-MA style pruning and route selection for the non-dynamic path

Source packets:
`MR-04`

Primary ownership files:

- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `src-tauri/src/commands_optimize.rs`

Context files:

- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`

Own these symbols or decisions:

- static route selection heuristics or cost models
- VDPS rung targeting and local-anchor thinning
- predictive prescan economics
- command-boundary route coercion for static optimization requests

Open problem:

- derive a sound pruning and route-selection rule for discrete candidate grids
  that minimizes compute subject to explicit regret constraints on missed
  finalists

I stayed inside the owned PR-03 surface. The current repo already splits the problem into an exact-output region and a reduced-output region: `commands_optimize.rs` forces `Grid` whenever exhaustive grid outputs are requested, while `double_ma_mode.rs` only allows `CoarseToFine` or `GpuVdps` when `include_all=false`, `heatmap_bins=0`, and no export-all CSV path is present; `Auto` moves to `GpuVdps` only for searches strictly larger than 500,000 combos with a viable GPU path and resident-GPU execution preference. GPU exact routes are further constrained by kernel blockers and hybrid capability routing in `double_ma_backend.rs` and `ma_policy.rs`. `prescan_predictive.rs` exposes a predictive prescan as a heuristic, and the real-data plus route-matrix binaries are the evidence surfaces for regret and route behavior. fileciteturn39file6 fileciteturn38file7 fileciteturn37file9 fileciteturn38file8

### 1. Formal problem statement

For each admissible static optimization request `x`, define a finite feasible lattice `G_x`. For each `g ∈ G_x`, let `J_x(g)` be the exact final objective score returned by an exact evaluation path that preserves the metric contract. Let `g_x*` be a true finalist under the deterministic tie discipline.

A static pruning/route policy `π` acts in rounds. At round `t`, it may inspect approximate summaries on unevaluated candidates, choose a subset `Q_t` for exact evaluation, choose an exact backend route `r_t`, update its incumbent best exact candidate, and optionally prune some unevaluated candidates. Its retained exact set is `H_π = ⋃_t Q_t`, and its finalist regret is

`Δ_x(π) = J_x(g_x*) - max_{h∈H_π} J_x(h)`.

The PR-03 problem is to minimize static-route compute cost subject to an explicit regret contract on `Δ_x(π)`.

### 2. Notation

- Search space: `G_x`
- State at stage `t`: `s_t = (E_t, P_t, A_t, b_t, R_t)`
  - `E_t`: exactly evaluated candidates
  - `P_t`: approximate info for unevaluated candidates
  - `A_t`: anchors or survivor seeds
  - `b_t`: current best exact incumbent
  - `R_t`: feasible exact routes under the current request
- Path functional: `Φ_x(π) = (s_0, Q_0, r_0, s_1, …, s_T)`
- Cost functional: `C_x(π) = Σ_t (A_x(s_t,r_t) + Σ_{g∈Q_t} κ_x(g,r_t))`
- Ranking functional: the repo’s deterministic order, i.e. score descending, drawdown ascending, trades descending, then canonical parameter order

Admissible approximations:

1. Exact upper certificate `U_t(g)` with `J_x(g) ≤ U_t(g)`
2. Deterministic envelope `|J_x(g) - Ĵ_t(g)| ≤ ε_t(g)`
3. Probabilistic envelope with tail probability `δ_t(g)`

Only class 1 supports sound pruning. Classes 2 and 3 support bounded-regret pruning. A raw heuristic score with no envelope is not a sound pruning claim.

### 3. Fixed invariants and interface constraints

1. Exhaustive-output coercion: if `include_all`, `heatmap_bins>0`, or a nonblank `export_all_csv_path` is requested, the command boundary normalizes Double-MA mode to `Grid`. fileciteturn39file6
2. Mode admissibility: `CoarseToFine` and `GpuVdps` require reduced-output requests; `Auto` only reaches `GpuVdps` above the strict `500_000` combo threshold, with a viable GPU route and resident-GPU execution preference. fileciteturn38file7
3. Exact GPU-kernel blockers: Kaufman-stop exit, execution overlays, disabled regime-filter support, large `top_k` when `include_all=false`, or unsupported fast/slow MAs. fileciteturn33file10
4. Hybrid route partition: `{GpuKernel, CpuStaged, Unsupported}` plus capability tiers `{GpuKernel, CudaOnly, CpuOnly, Unsupported}`. Speedup preference is advisory, not a proof of route choice. fileciteturn38file8 fileciteturn38file11
5. Current local thinning is heuristic: window slicing, decimation, survivor ranking, mutation, differential injection, and random rescue are operators, not certificates. fileciteturn38file1 fileciteturn38file4
6. The harness binaries are evidence surfaces, not proofs. fileciteturn35file4 fileciteturn35file3

### 4. Strongest plausible positive result

The strongest current positive result is narrow:

- On the exhaustive-output region of request space, the command boundary already forces `Grid`. Therefore `Δ_x(π)=0` there by construction. fileciteturn39file6
- If, on reduced-output requests, a policy maintains exact upper certificates `U_t(g)` for every unevaluated candidate and prunes only candidates with `U_t(g) < L_t`, where `L_t` is the best exact lower bound so far, then pruning is sound and can still achieve `Δ_x(π)=0`.

This exposes the key limitation: the current prescan and current VDPS machinery do not supply such `U_t`.

### 5. Strongest plausible impossibility result

Without certificates or proved structure linking surrogate information to the true objective, there is no universal safe nontrivial pruning theorem. Any policy that evaluates fewer than all candidates can be broken by another admissible workload whose unique winner lies in the pruned region while matching the observed surrogate information on the evaluated region.

So:

- predictive prescan alone is not safe,
- local-anchor window restriction is not safe,
- period decimation is not safe,
- bounded rescue budgets are not safe.

### 6. Candidate theorem or lemma list

1. Command-Boundary Exactness Lemma
2. Mode-Admissibility Lemma
3. Feasible-Route Partition Lemma
4. Tie-Aware Finalist Lemma
5. Certificate Elimination Theorem
6. Deterministic Envelope Regret Lemma
7. Local-Cell Cover Theorem
8. No-Free-Prune Lower Bound
9. Exact-Route Separation Proposition
10. Heuristic-Only Classification Lemma

### 7. One explicit counterexample family

Let `G_n = {(f,s): 1 ≤ f < s ≤ 2n}`. Consider a policy that:

- promotes a small seed set from a coarse scout score,
- searches only bounded windows around those seeds,
- adds only a bounded rescue budget.

Choose the promoted seed near one corner and the true winner near the center, outside every deterministic local window. Define the scout score to rank the corner seed highest while the true objective ranks the hidden center point highest. Then the policy misses the winner deterministically; bounded random rescue has success probability at most `B_n / |G_n \ W_n|`, which vanishes if `B_n = o(|G_n|)`.

### 8. One lower-bound or identifiability angle

Any statistic built only from route capability summaries, predictive-prescan outputs, local-anchor geometry, and exact scores on a proper subset of the lattice is not a sufficient statistic for exact finalist identification unless it carries a valid envelope linking it to the true objective.

Worst-case exact-safe identification complexity therefore equals the number of non-certified points.

### 9. Unresolved questions that block implementation

1. Is any structural regularity on the MA lattice defensible?
2. What is the primary contract: score regret, exact-best rank, top-k overlap, holdout stability?
3. Can any cheap scout produce a valid upper certificate?
4. What exact static cost functional should route optimization minimize?
5. Is route choice request-level or stage-level?
6. What canonical tie schema must all evidence surfaces share?
7. What workload family names a valid probabilistic contract?
8. How exactly should static-route economics exclude dynamic-residency economics?

First-pass conclusion: under the current repo-fit constraints, the only unconditional sound pruning theorem is certificate-based dominance pruning; absent a new certificate or a proved lattice-regularity assumption, nontrivial pruning cannot be claimed sound. Today there are only two mathematically honest postures for PR-03: exact `Grid` with zero regret, or heuristic pruning/route selection with an explicit empirical regret contract.

---

## Second-pass answer

The first-pass formalization only survives after a hard downgrade. It overclaimed finalist preservation, treated route cost too additively, and quietly projected the full candidate lattice down to a period-pair lattice. Under the prompt’s own bar, every pruning claim must either be proved sound or carry an explicit regret contract. fileciteturn39file10

### 1. Explicit failure modes

**F1. Zero score-regret is not the same as preserving the canonical finalist.**  
The prompt’s `Δ` is score-only. But the ranking surfaces are tie-aware. `vdps_rank_candidates` sorts by effective score descending, then max drawdown ascending, then trades descending, then fast length, slow length, fast MA id, slow MA id. So a pruning rule can achieve `Δ=0` and still lose the canonical winner on a score plateau. fileciteturn38file1

**F2. The current scout/thinning machinery does not live on the full candidate identity space.**  
`predictive_prescan_seed_pairs` returns only `(fast, slow)` seeds, while the full candidate identity may include MA types and MA params. Therefore period-pair reasoning does not lift to the full lattice unless types, params, strategy, filters, objective, score mode, and metric contract are fixed. fileciteturn37file17

**F3. Predictive prescan optimizes the wrong object.**  
The current prescan scores sampled pairs against next-close log return. In `long_only`, bearish bars are skipped from the denominator rather than averaged as zero. That surrogate omits the actual strategy path, drawdown, trade count, and many objectives. fileciteturn37file17 fileciteturn37file14

**F4. Prescan has finite-grid aliasing built in.**  
It samples periods by stride, caps sampled periods, and caps time work through `MAX_TIME_STEPS` via `time_stride`. Narrow ridges or parity-sensitive optima can disappear before exact evaluation begins. fileciteturn37file17 fileciteturn37file11

**F5. VDPS local-anchor thinning is structurally unsafe on disconnected or comb-like grids.**  
`vdps_local_anchor_axes` windows around a center, trims around that center, then decimates while pair count remains too large, re-inserting only the center. That does not prove any missed basin is hit. fileciteturn38file4 fileciteturn38file17

**F6. Route feasibility is request-level, not candidate-local.**  
The exact route set depends on request flags, mode admissibility, blockers, and `gpu_execution`, not on candidate-local scores. `should_cpu_stage_indicator` depends directly on the residency policy. `Auto` reaches `GpuVdps` only under the strict large-search plus viable-route plus resident-GPU regime. fileciteturn37file10 fileciteturn38file7

**F7. Route cost is not reliably additive over candidates.**  
`ta_profile_route_matrix.rs` and `ta_app_path_faceoff.rs` both report whole-case runtime/wall summaries and additive staging fields, showing that setup amortization, fallback, and timing-boundary choice matter. So `A + Σ κ(g,r)` is only a local approximation, not a global law. fileciteturn35file3 fileciteturn39file15

### 2. Adversarial constructions or separating examples

1. **Tie-plateau separator.** Two candidates can tie on score while differing on drawdown or trades, so score-only pruning can preserve `Δ=0` but drop the canonical winner.
2. **Projected-lattice separator.** A period pair can map to multiple MA-param settings. A pair-only scout cannot distinguish them.
3. **Next-close-vs-true-objective separator.** A pair with better one-step edge can lose on the true strategy objective.
4. **Comb-grid separator.** Sparse winning residue classes can be erased by center-window plus stride-decimation rules.
5. **Request-level route separator.** Flipping `include_all`, `heatmap_bins`, or export-all output changes admissible modes and routes immediately, even with the same candidate set. fileciteturn39file6 fileciteturn38file7

### 3. Corrected theorem or lemma candidates

1. Score-certificate pruning theorem: exact upper certificates on the true objective imply safe pruning.
2. Strict-gap identity corollary: identity preservation additionally needs a positive score gap.
3. Tie-aware finalist theorem: score-only certificates are insufficient on tie plateaus.
4. No period-pair sufficient-statistic theorem: not under the current admissible request family.
5. Exact-route separability theorem: only inside a request-level exact-equivalence class.
6. Current prescan and current VDPS classification lemma: heuristic, not certified.

### 4. Weakest assumptions under which anything survives

- Exact safe pruning survives only with exact upper bounds on the true objective over the full candidate identity.
- Identity preservation additionally needs a strict positive gap or a tie-aware certificate.
- Pair-lattice reasoning survives only after fixing MA types, MA params, strategy, filters, objective, score mode, and metric contract.
- Route-cost optimization survives only after fixing a request-level exact-route equivalence class and a common timing boundary.

### 5. Distinction between exact, approximate, and heuristic claims

- **Exact:** command-boundary `Grid` coercion on exhaustive outputs; certificate pruning if exact upper bounds exist.
- **Approximate:** anything with an explicit regret or overlap contract on a named workload family.
- **Heuristic:** current predictive prescan, local-anchor thinning, random injection, and Auto’s non-exact promotion logic. fileciteturn37file17 fileciteturn38file4

### 6. Revised time, memory, and retained-state costs

- Route cost should be modeled at batch level, not as a universal per-candidate additive law.
- Prescan time is roughly `O(n(|F_s|+|S_s|) + (n/time_stride)|F_s||S_s|)` with memory `O(n(|F_s|+|S_s|))`.
- VDPS retained state consists of windowed axes, ranked survivors, refill candidates, and rung telemetry. fileciteturn37file17 fileciteturn38file4

### 7. Dependency burden on other PR prompts

1. Candidate identity must be consolidated across optimization, validation, and audit surfaces.
2. Exact-route equivalence must be supplied as an interface, not assumed.
3. A named workload family and primary contract must be chosen for empirical claims.

Second-pass conclusion: the strongest surviving repo-fit result is narrow. The only unconditional sound nontrivial pruning theorem available is certificate pruning on the true objective over the full candidate identity. Current `prescan_predictive.rs` and current VDPS local thinning do not satisfy those requirements; they survive only as heuristics supported by empirical regret and route-evidence surfaces, not as safe pruning rules.

---

## Third-pass answer

Third-pass result: the strongest implementation-ready mathematical result that survives is a **two-tier contract**.

- **Exact tier.** If the request asks for exhaustive grid-style outputs, the command boundary pins the run to `Grid`, so PR-03 has no pruning freedom there and score regret is exactly zero. fileciteturn39file6
- **Reduced-output tier.** If the request stays in the reduced-output region, route selection is only well-posed over a **request-level feasible exact-route equivalence class**. Inside that class, one may optimize measured batch cost. But with the current `prescan_predictive.rs` and `double_ma_vdps.rs`, there is still **no sound nontrivial pruning theorem**: both are heuristic because they provide no upper certificate and no deterministic/probabilistic envelope on the true objective. Any use of them must therefore ship with an **explicit empirical contract**. That is exactly the acceptance bar in the prompt. fileciteturn39file10 fileciteturn37file17 fileciteturn38file4

### 1. Final formalization

Fix a normalized static Double-MA request `x` after command-boundary normalization. Let the request-fixed metadata be

- strategy and filter shape,
- objective and score mode,
- metric contract,
- output semantics,
- requested backend and `gpu_execution` policy.

Let the mutable candidate axes be whatever the request actually sweeps: fast length, slow length, fast MA type, slow MA type, fast MA params, slow MA params, subject to feasibility such as `fast_len < slow_len`.

Define the per-request candidate lattice

`G_x = A_1(x) × … × A_d(x)`

with infeasible points removed.

For `g ∈ G_x`, let `J_x(g)` be the exact objective score returned by any exact-compatible route under the fixed request contract. Let the canonical score order be the repo’s current lexicographic ranking used by `vdps_rank_candidates`: effective score descending, then max drawdown ascending, then trades descending, then fast length ascending, slow length ascending, fast MA id ascending, and slow MA id ascending. fileciteturn38file1

Because the prompt’s regret is score-only, define

`Δ_score(x, π) = J_x(g_x^*) - max_{h∈H_π} J_x(h)`

where `g_x^*` is a score maximizer. Identity preservation is a stronger claim and is only admissible once candidate keys and tie schema are reconciled across all evidence surfaces.

A policy `π` produces a sequence of exact batches `Q_t`, optional pruned sets `P_t`, and routes `r_t`, with retained exact set `H_π = ⋃_t Q_t`. Its total route cost is not modeled as a universal per-candidate sum. The implementation-ready object is the **batch cost**

`C_x(π) = Σ_t c_x(r_t, B_t)`

where `B_t` includes the retained batch, request flags, blocker state, timing boundary, and route setup work. This is the right cost object because the route harnesses record whole-case `runtime_ms` and `wall_ms`, and the app-path harness also decomposes planner/prep/upload/decode fields. fileciteturn35file3 fileciteturn35file6 fileciteturn39file15

For a non-exact policy family, the admissible contract is an explicit empirical tuple on a named workload family `W`:

- `R_score`: best-score regret,
- `O_k`: top-k overlap ratio,
- `r_exact`: exact rank of the approximate winner,
- `R_holdout_sharpe`, `R_holdout_pnl`: holdout regret of the chosen winner,
- `S_runtime`: exact-vs-approx speedup.

This matches the quantities already surfaced by the fidelity and optimizer-eval binaries. fileciteturn35file4 fileciteturn35file15 fileciteturn35file16

### 2. Final theorem or lemma list

**Lemma 1. Command-boundary exactness.**  
If `include_all=true`, `heatmap_bins>0`, or `export_all_csv_path` is nonblank after trimming, then the command boundary forces `OptimizationMode::Grid`. Therefore PR-03 has no admissible pruning action there, and the resulting score regret is exactly zero. fileciteturn39file6

**Lemma 2. Reduced-output admissibility.**  
`CoarseToFine` and `GpuVdps` are only admissible when `include_all=false`, `heatmap_bins=0`, and no export-all CSV path is present. `Auto` reaches `GpuVdps` only when `total_combos > 500_000`, a GPU route is viable, and `gpu_execution` is `PreferGpuResident` or `StrictGpuResident`; otherwise `Auto` selects `CoarseToFine` for large reduced-output runs and `Grid` for the smaller region. fileciteturn38file7

**Lemma 3. Request-level feasible exact-route class.**  
The feasible exact-route set for a request is determined by the blocker list in `double_ma_backend.rs`, the hybrid capability/route plan in `ma_policy.rs`, and the residency decision in `should_cpu_stage_indicator`. This set is request-level, not candidate-local. fileciteturn37file9 fileciteturn37file10 fileciteturn38file8 fileciteturn38file10

**Theorem 4. Exact-route separability.**  
If two routes belong to the same feasible exact-route equivalence class for request `x`, then they induce the same exact objective values and ranking on every retained candidate. Inside that class, route choice separates from candidate identity and reduces to minimizing measured batch cost. Outside that class, route optimization is not well-defined as a pure cost problem.

**Theorem 5. Certificate pruning.**  
Suppose a reduced-output policy maintains exact upper certificates `U_t(g)` on the true objective for every unevaluated candidate. Let `L_t` be the best exact lower bound seen so far. Then pruning any candidate with `U_t(g) < L_t` is sound and preserves zero score regret.

**Corollary 5.1. Strict-gap identity preservation.**  
If the exact score maximizer is unique with positive score gap `γ>0`, then certificate pruning also preserves winner identity. Without such a gap, score exactness does not imply identity exactness.

**Theorem 6. No sufficient statistic for current non-exact finalist identification.**  
Under the current constraints, there is no sufficient statistic built only from predictive-prescan outputs, local-anchor state, exact scores on a proper subset, and request-level route metadata that exactly identifies the final winner over the admissible request family. The reason is twofold: the scout target differs from the true objective, and the current scout state can collapse distinct full-identity candidates onto the same period pair.

**Proposition 7. Current predictive prescan is heuristic.**  
The current prescan computes MA series from `ma_source`, samples periods, scores `(fast, slow)` pairs by average signed next-close log return, skips bearish bars from the denominator in `long_only`, and uses approximate time sampling. It returns seeds, not bounds. Therefore it is a heuristic scout, not a sound pruning rule. fileciteturn37file17 fileciteturn37file14 fileciteturn37file11

**Proposition 8. Current local-anchor thinning is heuristic.**  
`vdps_local_anchor_axes` slices a center window, trims to nearest-to-center periods, repeatedly decimates while pair count remains too large, and re-inserts only the center. This preserves a center anchor, not the true winner. Therefore it is heuristic thinning, not safe pruning. fileciteturn38file4

**Theorem 9. Empirical-contract-only admissibility for the current non-exact path.**  
Because current prescan and current VDPS thinning have no certificate or envelope on the true objective, the only admissible non-exact claim is an empirical contract on a named workload family, expressed in the contract tuple above and verified by the repo’s evidence surfaces. fileciteturn35file4 fileciteturn37file18

### 3. Proof sketch or proof plan

- **Lemma 1:** direct from `normalize_product_double_ma_optimization_mode`, which pins any grid-style output request to `Grid`. fileciteturn39file6
- **Lemma 2:** direct from `resolve_optimization_mode` and `validate_optimization_mode`, including the strict `> 500_000` threshold and reduced-output preconditions. fileciteturn38file7
- **Lemma 3:** direct from the blocker list plus hybrid plan plus residency policy. Kernel blockers include Kaufman-stop exit, execution overlays, disabled regime filters, oversized `top_k`, and unsupported MAs. Cpu staging depends on the `gpu_execution` residency preference. fileciteturn33file10 fileciteturn37file10 fileciteturn38file8
- **Theorem 4:** once exact-equivalent routes are fixed, candidate scores do not change with route, so route choice is a cost-only problem. If exact equivalence is not established, the theorem does not apply.
- **Theorem 5:** standard dominance proof. If `U_t(g) < L_t`, then `g` cannot beat the incumbent exact lower bound.
- **Theorem 6:** adversarial construction. Build two admissible landscapes that agree on all evaluated points, agree on all observed scout/thinning state, yet place the true optimum at different unevaluated candidates. Then no such statistic is sufficient.
- **Proposition 7:** direct from the code path: the prescan target is next-close edge, not the final strategy objective; the time/period sampling is approximate; no envelope is returned. fileciteturn37file17 fileciteturn37file14
- **Proposition 8:** direct from the local-anchor implementation: center-window trim and stride decimation do not certify coverage of disconnected or comb-like optima. fileciteturn38file4
- **Theorem 9:** direct from the prompt’s acceptance bar plus the existing fidelity/quality-check harness outputs. fileciteturn39file10 fileciteturn35file4 fileciteturn37file18

### 4. Sufficient statistics, or proof they do not exist under the stated constraints

There are two different sufficiency questions.

**4A. Finalist identification.**  
No sufficient statistic exists under the current constraints. In particular, no statistic defined only on `(fast_len, slow_len)`, predictive-prescan scores, local-anchor state, and partial exact evaluations can be sufficient for exact finalist identification over the current admissible request family. Current prescan returns only `(fast, slow)` seeds and targets next-close log return; current VDPS local thinning only reshapes period axes. Neither object is sufficient for the full true objective on the full candidate identity. fileciteturn37file17 fileciteturn38file4

**4B. Feasible route-class identification.**  
A sufficient request-level statistic *does* exist for the feasible exact-route class:

`S_route(x) = (normalized output flags, total_combos, gpu_execution, gpu_kernel_viable, gpu_hybrid_viable, blocker list, hybrid route plan)`.

From this, one can determine whether the request lies in the exact-output region, whether reduced-output modes are admissible, and which exact routes are feasible. What it does **not** determine is the universally optimal route cost; that still depends on measured batch economics. fileciteturn39file6 fileciteturn38file7 fileciteturn37file9 fileciteturn38file8

### 5. Exactness conditions and equivalence classes

**Output-exactness class `E_output`.**  
Requests with `include_all=true`, `heatmap_bins>0`, or nonblank trimmed `export_all_csv_path`. These are pinned to `Grid` at the command boundary. Exactness is mandatory here. fileciteturn39file6

**Reduced-output class `E_reduced`.**  
Requests outside `E_output`. Only here are `CoarseToFine` and `GpuVdps` admissible, and only under the additional mode rules. fileciteturn38file7

**Route equivalence class `E_route(x)`.**  
The subset of feasible routes for request `x` that are already known, by interface contract plus verification, to return the same exact candidate scores, tie order, and output semantics on the retained candidate set. Route-cost optimization is legal only inside `E_route(x)`.

**Identity-completeness class `E_id`.**  
The workload family where candidate keys are identity-complete for the active swept axes. If MA params are fixed and not swept, they may collapse out of the key. If param sweeps are active but keys omit params, winner-identity claims are forbidden. Score-regret claims remain admissible.

### 6. Error, regret, or bias bounds for every non-exact claim

**Exact-output tier:**  
`Δ_score = 0` by command-boundary coercion. fileciteturn39file6

**Certificate pruning tier:**  
`Δ_score = 0` whenever pruning uses valid exact upper certificates.

**Current predictive prescan:**  
No finite worst-case regret bound exists on the admissible request family. Its surrogate bias is **unbounded in worst case** because it optimizes one-step signed next-close edge, not the true objective. Therefore the only admissible bound is an empirical one on a named workload family. fileciteturn37file17 fileciteturn37file14

**Current VDPS local-anchor thinning and refill injection:**  
No finite worst-case regret bound exists on the admissible request family. Center-window trim, decimation, and bounded rescue can all miss disconnected winners. Therefore the only admissible bound is an empirical one on a named workload family. fileciteturn38file4 fileciteturn38file17

**Current route-cost claims:**  
There is no universal closed-form optimality bound. The admissible claim is only: inside a verified exact-route equivalence class, the chosen route minimizes a **measured batch-cost objective** on the agreed timing boundary. The route-matrix and app-path harnesses provide the evidence surface for that claim. fileciteturn35file3 fileciteturn39file15

**Empirical contract schema for the current non-exact path:**  
A policy family `P` on workload family `W` may only claim:

- `R_score ≤ ρ_W`,
- `O_k ≥ ω_W`,
- `r_exact ≤ r̄_W`,
- `R_holdout_sharpe ≤ η^S_W`,
- `R_holdout_pnl ≤ η^P_W`,
- `S_runtime ≥ σ_W`,

where the constants are not theorem constants; they are fitted or audited constants produced by the fidelity and evaluation harnesses on `W`. If those constants are not supplied, the claim is incomplete. fileciteturn35file4 fileciteturn35file15 fileciteturn35file16

### 7. Time, memory, and retained-state complexity

**Request normalization and mode/route classification:**  
`O(1)` in request size plus registry lookup overhead, since it depends on a constant-size set of flags and MA ids. fileciteturn39file6 fileciteturn38file7 fileciteturn38file8

**Predictive prescan:**  
Let `n` be series length, `F_s` and `S_s` the sampled fast/slow period sets after stride sampling and capping, and `τ = max(1, floor((n-1)/MAX_TIME_STEPS))`. Then time is

`Θ(n(|F_s| + |S_s|) + floor(n/τ)|F_s||S_s|)`

and memory is

`Θ(n(|F_s| + |S_s|))`.

Retained state after scouting is `Θ(top_k)`. fileciteturn37file17 fileciteturn37file14

**VDPS local-anchor thinning:**  
If `F_w` and `S_w` are the windowed axis sizes, the local-anchor transform is dominated by sorting/trimming and repeated decimation, so it is roughly

`O(|F_w| log |F_w| + |S_w| log |S_w| + D(|F_w| + |S_w|))`

with `D ≤ 128` from the decimation loop. Retained state is

`O(|F_sel| + |S_sel| + K + B + R)`

for selected fast/slow axes, ranked survivors, refill candidates, and rung telemetry. fileciteturn38file4 fileciteturn38file17

**Route cost:**  
This is batch-level, not globally additive per candidate. The retained state for cost evidence is an artifact-sized summary of runtime/wall statistics and per-path additive timing fields. fileciteturn35file3 fileciteturn39file15

### 8. File and symbol impact map

**`src-tauri/src/commands_optimize.rs`**  
Owns the command-boundary exactness split.

- `normalize_product_double_ma_optimization_mode`
- `run_double_ma_optimization`
- `OffsetProgressSink` only affects progress projection, not pruning soundness

This file defines the exact/output tier boundary. fileciteturn39file8 fileciteturn39file6

**`crates/app_core/src/double_ma_mode.rs`**  
Owns reduced-output admissibility.

- `ModeSelectionContext`
- `resolve_optimization_mode`
- `validate_optimization_mode`

This file defines when PR-03 pruning/route choice is even allowed. fileciteturn38file7

**`crates/app_core/src/double_ma_backend.rs`**  
Owns request-level exact route feasibility.

- `gpu_kernel_blockers_for_request`
- `should_cpu_stage_indicator`
- `build_indicator_route_plan_summary`

This file defines blockers and residency-sensitive staging. fileciteturn33file10 fileciteturn37file10

**`crates/app_core/src/ma_policy.rs`**  
Owns the capability/route partition interface.

- `HybridCapabilityTier`
- `HybridRouteTier`
- `classify_hybrid_capability`
- `plan_double_ma_hybrid_route`
- `prefers_gpu_by_speedup_table`

This file supplies the route-equivalence partition inputs but does not itself prove cost optimality. fileciteturn38file8 fileciteturn38file11

**`crates/app_core/src/prescan_predictive.rs`**  
Owns the heuristic scout.

- `PredictivePrescanParams`
- `predictive_prescan_seed_pairs`

This file is currently heuristic-only. fileciteturn37file17 fileciteturn37file14

**`crates/app_core/src/double_ma_vdps.rs`**  
Owns heuristic thinning and tie discipline.

- `vdps_rank_candidates`
- `vdps_local_anchor_axes`
- `vdps_inject_random_seed_pairs`
- `vdps_merge_partial_results`

This file defines the current reduced-output heuristic mechanics and the tie schema used in VDPS ranking. fileciteturn38file1 fileciteturn38file4 fileciteturn38file17

**Context evidence surfaces**  
These files are verification interfaces, not redesign surfaces.

- `real_data_fidelity_audit.rs` for exact-rank/score-regret/top-k/holdout contracts
- `real_data_optimizer_eval.rs` for holdout-positive recovery and evaluation artifacts
- `ta_profile_route_matrix.rs` for route correctness plus runtime/wall summaries
- `ta_app_path_faceoff.rs` for app-path timing-boundary decomposition
- `stage6b_kernel_report.rs` as an additional exact-surface reporting interface listed in the prompt scope

fileciteturn35file4 fileciteturn35file16 fileciteturn35file3 fileciteturn39file15 fileciteturn39file9

### 9. Verification obligations and falsifiers

**Verification obligations**

1. **Command-boundary exactness:** every request in `E_output` must exit the command boundary as `Grid`. fileciteturn39file6
2. **Mode admissibility:** no reduced-output mode may survive validation when `include_all=true`, `heatmap_bins>0`, or export-all CSV is nonblank. fileciteturn38file7
3. **Route-class verification:** any cost optimization over routes must be restricted to a route set already verified as exact-equivalent on the declared timing boundary. Route-matrix artifacts must show stable `route_ok`, route labels, and fallback reasons. fileciteturn35file3 fileciteturn35file6
4. **Non-exact contract emission:** every heuristic reduced-output run must emit the audit tuple, or the policy must be labeled heuristic-only with no contract. The current VDPS telemetry already exposes `quality_check_top_k_overlap`, `quality_check_best_score_regret`, baseline runtime, and baseline pair count. fileciteturn37file18
5. **Holdout verification:** if holdout stability is part of the contract, `real_data_fidelity_audit.rs` style holdout regret must be computed. fileciteturn35file15

**Falsifiers**

- A reduced-output policy that claims sound pruning but supplies no certificate or envelope.
- Any run in `E_output` that does not execute as `Grid`.
- Any candidate-key surface that omits active swept axes while the policy claims winner identity preservation.
- Any route optimization claim made across routes not already proven exact-equivalent.
- Any declared empirical contract violated by measured `R_score`, `O_k`, `r_exact`, or holdout regret on its named workload family.

### 10. Dependencies that must be reconciled during consolidation

1. **Identity-complete candidate keys.**  
Optimization, validation, and audit surfaces must agree on which axes define a candidate identity. Without this, only score-regret claims are safe.

2. **Single tie schema version.**  
The lexicographic order used by reduced-output ranking and by any evaluation/audit harness must be aligned if identity preservation is claimed.

3. **Exact-route equivalence interface.**  
PR-03 may optimize route cost only after another interface states which routes are exact-equivalent for the retained candidate set.

4. **Common timing boundary.**  
Route-cost comparisons must declare whether they are engine runtime, route-matrix wall time, or app-path wall time. The route binaries already distinguish those boundaries. fileciteturn35file3 fileciteturn39file15

5. **Named workload family for empirical contracts.**  
A non-exact contract is meaningless without the family on which it was calibrated and audited.

6. **Non-goal separation.**  
Dynamic-GPU execution selection remains outside PR-03. Static-route economics must stay separated from dynamic execution economics, exactly as the prompt requires. fileciteturn39file10

### 11. Kill criteria

Kill or disable the nontrivial reduced-output pruning claim if any of the following holds:

1. No exact upper certificates or valid error envelopes exist, **and** the run cannot emit an empirical contract.
2. Candidate keys omit active swept axes, but the policy still claims winner identity preservation.
3. Route cost is being optimized across routes with no exact-equivalence proof.
4. The command boundary fails to pin exhaustive-output requests to `Grid`.
5. The named empirical contract is violated on its own verification harness.
6. The route/timing boundary used for optimization is ambiguous or changes across comparisons.

### Strongest surviving implementation-ready result

The final repo-fit result is:

For PR-03, the only unconditional exact theorem currently available is **command-boundary Grid exactness** on exhaustive-output requests plus **certificate pruning** on reduced-output requests, if exact upper certificates on the true objective are available. The current `prescan_predictive.rs` and `double_ma_vdps.rs` do **not** supply such certificates. Therefore the present implementation-ready design is a strict split:

- **Exact path:** exhaustive-output requests are forced to `Grid`; reduced-output requests may optimize cost only over a verified exact-route equivalence class.
- **Heuristic path:** current prescan and current VDPS thinning may be used only with an explicit empirical contract on a named workload family, expressed in score regret, top-k overlap, exact-rank, holdout regret, and speedup metrics.

Anything stronger would overclaim what the current owned PR-03 surface actually proves. fileciteturn39file6 fileciteturn38file7 fileciteturn37file17 fileciteturn38file4 fileciteturn35file4

