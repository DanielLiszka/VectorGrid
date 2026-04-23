# OX-03 Sequential compute allocation and stopping-value math

## Context note

I only found the OX-03 overlay brief in `PROMPT.md`, not the attached core `PR-*` prompt. So all three passes below keep the same repo boundaries and treat this as an overlay review over:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_campaign.rs`

---

## First pass

The repo already has several local compute-budget mechanisms, but they do not yet compose into a single sequential stopping policy.

`CompositeMo` already has a real staged budget shape: optional `gpu_prescan` narrows the search domain to allowed MA pairs and records pair-density telemetry, then the search runs, and then every surviving Pareto point is unconditionally enriched with final CPU `Trade22` metrics. The telemetry explicitly says `actual_evaluations` counts only search-phase work and excludes that final enrichment pass, so search budget and end-to-end compute are already separated in the code.

`DoubleMa` also already contains two useful budgeted patterns: a cheap-search / expensive-replay split for shortlist runs, and a runged `GpuVdps` allocator that sets survivor counts, opens local windows around survivor anchors, and can run a baseline quality check on small enough grids.

So the missing idea is not “introduce budgeting”; it is “carry the budgeting lens past search into admission, validation, and holdout.”

### Where OX-03 changes the first-pass review

1. **Campaign admission is diversity-aware, not VOI-aware.**

   `commands_campaign.rs` builds validation-admission rows from optimizer output. For `CompositeMo`, `Auto` resolves to `ParetoFrontier`, and dense frontiers are capped by feasible-first, evenly spaced index selection with the note `pareto_diversity_trim`. That is a reasonable coverage heuristic, but it is not a compute-allocation policy: it ignores which retained point is most likely to change the downstream winner after validation.

2. **Validation is throughput-optimized, not sequential.**

   `validation_pipeline.rs` evaluates admitted candidates in parallel and then performs probability metrics, CSCV/PBO, hard gates, sorting, and final verdict construction. Under the OX-03 lens, this is the biggest missed opportunity: once an incumbent is already strong enough, the pipeline still spends near-fixed work on challengers that may have no realistic path to winning.

3. **Meta-selection exposes budget knobs without yet owning the policy.**

   `MetaSelectionRequest` already has `holdout_eval_top_n`, `holdout_eval_explore_n`, and `holdout_eval_rank_by`, but `meta_select_from_artifacts` explicitly owns only the train-stage shortlist/grouping/`meta_score` pass. The interface already advertises selective holdout spend, but this file does not yet exploit those knobs.

### New math the core prompt would likely miss

**Lemma candidate: safe pruning under residual-value bounds.**

Let `F_i` be candidate `i`’s eventual final score after all remaining validation or holdout work. After stage `t`, maintain bounds `[ℓ_i^(t), u_i^(t)]` for the work already done, plus an admissible upper bound `R_i^(t)` on all remaining uncomputed contribution. If

`u_i^(t) + R_i^(t) < max_j ℓ_j^(t)`

then candidate `i` can be pruned without changing the final argmax.

That is the missing stopping lemma for `validation_pipeline.rs` and the later holdout owner. Right now the code has hard gates and verdict logic, but not this kind of “best possible finish is already too weak” rule.

**Counterexample family: dense frontier with a thin downstream knee.**

Take a Pareto frontier of size `n` whose visible maximize/minimize tradeoff looks smooth, except for one interior point with much better downstream validation survival because it sits in a stable parameter basin. The current even-spacing trim can keep the endpoints and still discard that interior knee. So “frontier diversity” does not imply “good spend of validation compute.”

**Bound candidate.**

If a frontier of size `n` is trimmed to `k` evenly spaced indices and the downstream-best point’s index is effectively unmodeled, then under a uniform prior over its index the retention probability is only `k/n`.

### First-pass implementation direction

The highest-leverage first-pass work is not another optimizer-side micro-heuristic. It is staged continuation across admission, validation, and holdout.

In `commands_campaign.rs`, keep the current admission ownership, but replace pure even-spacing as the last trim rule with a tiny VOI proxy such as `novelty × uncertainty / estimated_validation_cost`.

In `validation_pipeline.rs`, split work into cheap and expensive phases. Use cheap phases first to tighten incumbent bounds, then run CSCV / SPA / reopt only for candidates that survive the pruning lemma.

In the later holdout owner, make `holdout_eval_top_n` and `holdout_eval_explore_n` real. Spend holdout budget on the current top groups plus a small exploration set chosen from train-stage uncertainty proxies that `meta_select.rs` already computes.

In `CompositeMo`, treat final `Trade22` enrichment as another sequential budget stage. Right now every Pareto point is enriched after search. Under OX-03, the better policy is to enrich only the points still competitive for admission or meta-ranking, then lazily expand if uncertainty remains.

### First-pass verdict

This overlay changes the priority order of the review:

1. `commands_campaign.rs` and `validation_pipeline.rs`
2. the later holdout owner behind `meta_select.rs`
3. the raw optimizers

The repo already knows how to spend compute selectively inside search. The missing theorem and the missing implementation are both downstream: once candidates are admitted, compute is still allocated too uniformly.

---

## Second pass

Using the same owned surfaces as the OX-03 overlay, the second-pass answer is that the overlay changes the meaning of exactness. The right target is no longer “every admitted candidate gets the full expensive pipeline.” It is “the final recommendation, winner set, and required global tests are unchanged under an asymmetric budget policy.”

That fits the current ownership split because:

- `double_ma.rs` already has staged cheap→expensive evaluation,
- `validation_pipeline.rs` owns post-search candidate statistics and run verdicts,
- `commands_campaign.rs` owns admission caps,
- `meta_select.rs` explicitly stops at train-stage ranking,
- `composite_mo.rs` already separates search from final `Trade22` enrichment.

### How the overlay changes exactness

In `validation_pipeline.rs`, the second-pass change is to separate **candidate-local exactness** from **run-level exactness**.

Candidate-local quantities are already monotone: `compute_probability_metrics` assigns PSR/DSR before CSCV, and later `fold_cscv_metric(base, oos)` is a one-sided fold `min(base, oos)`, so CSCV can only lower a candidate’s robust score, never raise it.

But the same file also computes CSCV/PBO from the full retained set and `build_run_verdict` can change the final recommendation based on SPA and PBO. So a theorem that preserves ranking alone is not enough to preserve the full validation verdict.

That yields the strongest new lemma candidate in this pass:

Let `U_i` be candidate `i`’s current `robust_score` before CSCV fold, and let `L*` be the best fully evaluated post-CSCV score among incumbents. If `i` already fails a requested PSR or DSR gate after `compute_probability_metrics`, or if `U_i < L*`, then `i` cannot become the final robust-score winner after CSCV, because later CSCV only decreases score and does not repair PSR/DSR.

This is an exact pruning rule for candidate ranking and candidate-level approval, provided SPA and global PBO are preserved on a protected witness set.

### New adversarial construction

Candidate A can have the highest robust-score upper bound, while candidate B looks weaker on robust score but has the largest slice-return mean difference against the benchmark. `compute_spa_validation_stats` explicitly chooses the SPA witness by best mean difference / t-stat across the candidate set, and `build_run_verdict` can downgrade the whole run when SPA is not significant.

So pruning by robust-score bounds alone can preserve the ranking winner and still change the final run verdict.

The same caution applies to max-PBO gating, because CSCV/PBO is built from the retained set as a whole rather than candidate by candidate.

### New bounds and counterexamples by owner

**`commands_campaign.rs`**

OX-03 changes the standard from “cover the Pareto frontier” to “do not drop the most downstream-valuable candidate under the validation cap.” The current `CompositeMo` path sorts rows feasible-first, then maximize descending, then minimize ascending, and if the frontier is dense it keeps evenly spaced indices and records `pareto_diversity_trim`.

That preserves frontier coverage, but it has no winner-preservation theorem for downstream validation. In the worst case, the validation-best point can sit in an omitted interior slot, so the deterministic guarantee is zero. Under a flat prior over winner location, keeping `k` of `n` frontier points gives only the trivial `k/n` retention bound.

**`meta_select.rs`**

The request already has `holdout_eval_top_n`, `holdout_eval_explore_n`, and `holdout_eval_rank_by`, but the file’s own ownership comment says `meta_select_from_artifacts` owns only the train-stage shortlist/grouping/`meta_score` pass, while a later holdout stage owns `holdout` and any `final_score` rewrite.

So exact selective holdout cannot be proved inside this owner unless a bound on unseen holdout uplift is supplied externally.

**`composite_mo.rs`**

Its telemetry explicitly says `actual_evaluations` counts search-phase work only and excludes the final CPU `Trade22` Pareto enrichment pass, yet the code then reevaluates every Pareto point with `evaluate_composite_cpu_trade22_metrics` and returns a `trade22` surface.

So end-to-end compute already satisfies a simple lower bound: search work plus one extra CPU enrichment call per Pareto survivor.

### The positive control

`double_ma.rs` is the positive control for this overlay. It already does a cheap global sweep first and, when rich return stats were not needed for the search objective, recomputes richer metrics only for survivors after aggregation. Its `GpuVdps` path allocates budget rung by rung, promotes survivor seeds, opens anchored local windows, and can run a baseline quality check that reports overlap, regret, baseline runtime, and baseline pair count.

That is the exact pattern OX-03 wants elsewhere: early approximate stages are acceptable when they end in exact finalist replay and are audited by explicit regret-style telemetry.

### Second-pass verdict

The priority order sharpens in this pass.

- `validation_pipeline.rs` is still the main target, but now for a more precise reason: it already contains enough monotonicity for exact candidate-local pruning, while also containing the global CSCV/PBO and SPA hooks that make naive pruning invalid.
- `commands_campaign.rs` is next because `pareto_diversity_trim` has no downstream winner-preservation bound.
- The later holdout owner behind `meta_select.rs` is third because selective holdout needs a holdout-uplift bound before it can claim exactness.
- `composite_mo.rs` then needs end-to-end budget accounting for Pareto enrichment.
- `double_ma.rs` is mostly the model to imitate rather than the main gap.

---

## Third pass

Using the same assumption as before — only the OX-03 overlay is present, not the attached core `PR-*` prompt — the third-pass answer is implementation-facing rather than theorem-facing.

The overlay’s main implication is now this:

**do not try to force a single global stopping rule across all five files.**

Instead, implement one exact fast path where the current math already supports it, add one honest accounting fix where compute is currently hidden, and keep the remaining heuristics clearly labeled as heuristics.

### 1. `validation_pipeline.rs`: split candidate-local fold from set-level CSCV/PBO

The second-pass lemma looked strongest here, but the third-pass implementation detail is narrower than it first appeared.

Why: `fold_cscv_metric(base, oos)` is candidate-local and monotone decreasing, but `finalize_with_probability_and_optional_cscv` currently computes CSCV/PBO from the whole candidate matrix, and `build_run_verdict` can still depend on global PBO and SPA.

So the exact patch is **not** “just prune candidates mid-CSCV.”

The exact patch is:

- keep the current full path for any run that needs global PBO or SPA-sensitive reporting,
- extract a candidate-local CSCV fold helper from the current all-candidate CSCV machinery,
- only then add a guarded sequential fast path.

Concretely, the guarded fast path should be enabled only when all of the following are true:

- CSCV is being used only to fold candidate-local robust scores,
- `max_pbo` is not requested,
- no CSCV-based hard gate is requested,
- the caller does not require SPA over the full candidate set.

Inside that guarded path, the implementation can be exact:

1. run `compute_probability_metrics`,
2. reject any candidate that already fails PSR/DSR,
3. sort the survivors by current `robust_score` descending,
4. evaluate candidate-local CSCV fold in that order,
5. maintain `best_folded`,
6. stop evaluating new candidates once `next_base_score < best_folded`.

That is exact because the fold is one-sided and cannot raise a challenger above its current base score.

The important engineering consequence is that **the split comes first**. Without splitting candidate-local fold from set-level PBO construction, sequential CSCV pruning is not exact in the current design.

### 2. `validation_pipeline.rs`: do not prune SPA yet

The adversarial example from pass two becomes an implementation rule here.

`compute_spa_validation_stats` chooses the best benchmark-relative witness by maximum t-stat / mean difference across candidates. The current file does not expose an admissible upper bound on that witness statistic for unevaluated candidates.

So third-pass guidance is simple:

- do not add SPA-based pruning yet,
- treat SPA as a full-set obligation,
- keep the current all-candidate path whenever SPA is enabled.

This keeps exactness honest and avoids a subtle false “approved” verdict.

### 3. `commands_campaign.rs`: fix the interior-knee failure without changing ownership

The second-pass counterexample says the current `pareto_diversity_trim` can drop the validation-best interior knee.

The implementation-facing implication is not a new subsystem. It is a small local change inside `build_pareto_frontier_validation_admission_selection`:

- always keep the feasible frontier endpoints,
- reserve one slot for the strongest local knee / curvature point,
- fill the remaining slots by the current even-spacing rule,
- add a provenance warning when `max_candidates / frontier_size` is very small.

This does **not** make admission exact. But it directly addresses the counterexample while preserving the current owner, return type, and provenance flow.

So the honest third-pass label for this patch is:

- **high leverage, low surface area, still heuristic**.

### 4. `composite_mo.rs`: fix end-to-end compute accounting before changing policy

`CompositeMo` already separates search from Pareto enrichment in spirit, but it does not yet report end-to-end cost in the same object. Search telemetry explicitly excludes the final CPU `Trade22` enrichment pass, while the implementation still enriches every Pareto point.

The third-pass implication is to fix accounting first:

- add optional telemetry for `pareto_enrichment_points`,
- add optional telemetry for `pareto_enrichment_runtime_ms`,
- add an end-to-end total or clearly expose `search_evaluations + enrichment_points`.

That patch is exact and backward-compatible if added as optional fields.

Only after that accounting is in place should the repo consider lazy Pareto enrichment. Otherwise the system will keep under-reporting the very budget stage OX-03 is supposed to reason about.

### 5. `meta_select.rs`: keep holdout execution out of this owner

The third-pass implication of the holdout bound is mostly a restraint, not a rewrite.

`meta_select_from_artifacts` already computes exactly the train-stage signals a later holdout owner needs: `meta_score`, `pass_rate`, `tail_gap`, `datasets_covered`, and `evidence_flags`, while the request already exposes `holdout_eval_top_n`, `holdout_eval_explore_n`, and `holdout_eval_rank_by`.

So the right implementation move is:

- **do not** push holdout execution into this file,
- let the later owner consume the train-stage ranking and evidence already emitted here,
- if anything is added locally, make it a deterministic rank helper for later holdout selection rather than holdout evaluation itself.

That respects the current ownership comment and still makes OX-03 actionable.

### 6. `double_ma.rs`: use it as the template, not the main target

The clearest implementation lesson is that `double_ma.rs` already shows the shape the other files should emulate.

- shortlist confidence / union / slice rerank modes are guarded so they only run on exact GPU-kernel-compatible routes,
- `GpuVdps` allocates rung budgets, promotes survivors, opens local anchor windows, and audits itself with overlap / regret telemetry.

That is the model to copy:

- exact fast paths should be capability-guarded,
- approximate early stages should end in exact survivor replay,
- any approximate allocator should ship with regret-style telemetry.

### Third-pass implementation order

If this were queued as actual work, the order should be:

1. **`validation_pipeline.rs`** — split candidate-local fold from set-level CSCV/PBO, then add the guarded exact fast path.
2. **`composite_mo.rs`** — fix end-to-end enrichment accounting.
3. **`commands_campaign.rs`** — replace pure even-spacing with endpoint + knee + spacing and add trim-ratio provenance.
4. **later holdout owner using `meta_select.rs` output** — consume `holdout_eval_top_n`, `holdout_eval_explore_n`, and evidence-based exploration.
5. **`double_ma.rs`** — mostly documentation / pattern reuse, not a major redesign target.

### Third-pass verdict

The third pass changes the recommendation from “add sequential stopping everywhere” to something more precise:

- one exact sequential path is available in `validation_pipeline.rs`, but only after separating candidate-local fold from set-level CSCV/PBO,
- one exact accounting fix is needed in `composite_mo.rs`,
- one high-value heuristic fix is needed in `commands_campaign.rs`,
- holdout selectivity should stay outside `meta_select.rs`’s current owner,
- `double_ma.rs` is the template the rest of the stack should imitate.

That is the strongest implementation-facing reading of OX-03 that stays inside the current file boundaries and does not pretend the global-test problem is solved when it is not.
