# Options Optimization Depth Direction — Final

Date: 2026-04-09
Status: final consolidation for Prompt 5 (`Optimization-Depth Expansion Inside The Current Pass`)

## 1. Executive summary

The repo does **not** need another broad hot-path metric expansion as the default next step for Prompt 5.

The strongest remaining optimization-depth improvement is narrower:

**add one compact, tuple-coherent leader/disagreement summary for artifact-list, reopened-artifact, and compare surfaces, derived from the already-saved retained tuple rows, while keeping per-tuple role chips and richer delta context client-side.**

That is the highest-ROI next move because the repo already ships rich tuple-level optimization and embedded-validation depth, but `OptionsExpressionArtifactSummary` still reports lane-wise maxima that can come from different tuples. That makes summary-only surfaces less trustworthy than live row inspection. The fix should therefore improve **summary coherence**, not widen GPU state, not widen the hot path, and not rewrite ranking math.

## 2. Current shipped optimization-depth surface

The current options lane is already much deeper than a minimal v1 options follow-up workflow.

Already shipped:

- artifact-driven options-expression optimization and saved artifact reopen
- tuple drilldown, row-level event inspection, and event-path replay
- holdout, sealed-finalist, and multi-lane stress reporting
- first-wave robustness-frontier and validation-frontier surfaces
- validation verdicts, validation-winners frontier reporting, and cross-artifact meta-analysis
- saved-artifact provenance diffing, realism-lane labeling, and backend-route labeling
- compiled-tensor CUDA ranking with retained-finalist CPU exactification

At the tuple level, the backend already stores a large lane surface on each retained result row:

- `selection_metrics`
- `holdout_metrics`
- `sealed_evaluation_metrics`
- `stress_metrics`
- `sealed_stress_metrics`
- `harsher_stress_metrics`
- `sealed_harsher_stress_metrics`
- `extreme_stress_metrics`
- `sealed_extreme_stress_metrics`
- `validation_verdict`
- `robust_score`
- `pre_sealed_rank`
- `sealed_final_rank`

The base metric struct is also already non-trivial:

- `valid_trades`
- `invalid_events`
- `invalid_rate`
- `expectancy_per_trade`
- `expectancy_per_premium`
- `median_return_on_premium`
- `win_rate`
- `total_pnl_per_contract`

The current ranking split is deliberate and should be treated as stable:

- broad retained-tuple ranking is driven by `robust_score`
- sealed metrics are intentionally excluded from broad first-pass tuple ranking
- sealed finalists are reranked separately by sealed-specific ordering

The current artifact/list summary layer is where the remaining ambiguity still lives. `OptionsExpressionArtifactSummary` records maxima such as:

- `best_robust_score`
- `best_selection_expectancy_per_premium`
- `best_holdout_expectancy_per_premium`
- `best_sealed_expectancy_per_premium`
- the various stress-lane maxima

Those maxima can come from different tuples. So list and compare surfaces can still imply a synthetic “best of everything” tuple that never actually existed.

## 3. Final recommendation

### Adopted direction

Add a **compact retained-tuple leader/disagreement summary** to artifact summary payloads.

This should be computed from the already-saved `selection_metrics` rows during artifact summarization and used primarily by:

- artifact library rows
- reopened-artifact headers
- compare tables
- summary-only surfaces that do not yet have a selected tuple expanded

The new summary should be explicitly scoped to the **retained tuple set stored in the artifact**, not the full compiled grid.

### What the compact summary should contain

Keep it small and identity-oriented rather than metric-heavy. A good first version is:

- retained search leader identity
- retained robust leader identity
- retained holdout leader identity, when present
- retained sealed leader identity, when present
- retained stress leader identity, when present
- retained distinct-leader count
- `search_vs_robust_disagree`
- `robust_vs_holdout_disagree`, when holdout exists
- `robust_vs_sealed_disagree`, when sealed exists
- optional finalist movement facts for the robust leader, such as:
  - whether it reached the sealed-finalist set
  - its `sealed_final_rank`, when present

Use tuple index or retained rank, not copied full metric payloads.

### Why this is the right Prompt 5 answer

It solves the one obvious remaining trust problem in current summary-only surfaces:

- the repo already has deep row-level optimization and validation information
- the repo already has frontier-style live views and richer inspection
- the missing piece is that saved-artifact summary rows still collapse different leaders into unrelated maxima

So the next step should be **better tuple-coherent summarization of what already exists**, not another hot-loop metric family.

### Why per-tuple persisted context is not the winner

Do **not** add a backend-owned persisted per-tuple context bundle as the default next step.

Once a run or artifact is loaded, the frontend already has the full retained tuple rows and can derive row-level chips, deltas, and badges locally. Persisting extra per-row context would add schema weight without solving the main remaining problem, which is summary-only ambiguity.

## 4. What remains unchanged

Keep these contracts unchanged:

- `robust_score` remains the broad retained-tuple ranking scalar
- sealed metrics stay out of broad first-pass ranking
- sealed finalists keep their separate rerank
- current GPU scorer shape stays intact
- current `OptionsGpuTupleAggregate` stays unchanged for this work
- current finalist exactification boundary stays unchanged
- current workflow stays artifact-driven and inside the unified options host
- current structure-family, parent-family, and realism-lane scope freeze stays unchanged

Also keep one important interpretability rule unchanged:

- deterministic search and validation remain the first-class decision engine

## 5. Deferred ideas

These are plausible later, but they are not the best next Prompt 5 move.

### Defer

- richer one-pass metric expansion that widens GPU aggregate state
- restoring exact or approximate `median_return_on_premium` on the GPU path
- shrinkage-aware or lower-confidence-bound ranking-math changes
- a backend-owned per-tuple persisted disagreement/context bundle
- full retained-frontier tables embedded into artifact summaries
- larger finalist-acceptance dashboards or broader adequacy math beyond current needs
- globally valid frontier summaries beyond the retained tuple set

The common reason for deferment is the same: these ideas either belong more naturally to the Prompt 9 / Prompt 10 / Prompt 11 line of work, or they add more state and reporting weight than the current trust gap justifies.

## 6. Rejected ideas

Reject these as the default next Prompt 5 direction:

- widening `OptionsGpuTupleAggregate` just to improve summary-only surfaces
- adding a new exactification or replay pass for report-only fields
- rewriting `robust_score` as part of this work
- mixing more lane maxima into artifact summaries and calling that “optimization depth”
- persisting large per-tuple context bundles when loaded artifacts already expose retained rows
- adding new workflow stages, new realism lanes, or new strategy families under the Prompt 5 label

The key rejection principle is:

**do not spend hot-path or schema complexity to solve a problem that is fundamentally a summary-coherence problem.**

## 7. Concrete code touch points by file

### Required touch points

#### `src-tauri/src/commands_options_expression.rs`

Primary implementation file.

Add the compact leader/disagreement fields to `OptionsExpressionArtifactSummary`, then derive them inside `summarize_options_expression_artifact(...)`.

Recommended implementation shape:

- add a small nested summary struct or a compact set of scalar/optional fields
- compute leader identities from `artifact.selection_metrics`
- keep the computation explicitly scoped to the saved retained tuple set
- preserve existing maxima fields for backward compatibility and quick sorting, but stop treating them as the main decision-grade summary

#### `web/src/services/optionsExpressionService.ts`

Extend the artifact-summary transport shape so the new retained-leader/disagreement fields arrive in the frontend cleanly.

#### `web/src/stores/optionsExpressionStore.ts`

Expose the new artifact-summary fields for:

- filtered artifact lists
- compare state
- reopened-artifact header state

Keep richer per-row role chips derived client-side from `activeRankedResults`, not persisted server-side.

#### `web/src/components/OptionsExpressionPanel.vue`

Use the new summary fields for:

- leader chips on artifact rows
- disagreement badges on compare surfaces
- compact “fragmented leadership” or “same tuple leads across lanes” messaging

### Optional touch point

#### `crates/app_core/src/options_types.rs`

Only touch this file if you want the compact leader/disagreement summary to become a shared Rust-side contract type rather than a Tauri-command-local summary shape.

### Intentionally untouched files

These should stay unchanged for the adopted direction unless a helper extraction becomes irresistible:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`

That boundary is intentional. This is a summary/reporting coherence change, not a scorer or ranking-math change.

## 8. Verification order

1. Add narrow unit coverage for artifact-summary derivation.
   - synthetic artifact with one tuple leading search, another leading holdout, another leading sealed
   - assert leader identities, disagreement booleans, and distinct-leader count are correct
   - assert summaries are explicitly based on retained rows only

2. Run backend verification.
   - `cargo check -q -p ta_desktop_demo_app`
   - if shared Rust-side types/helpers move into `ta_app_core`, also run:
     - `cargo test -q -p ta_app_core options_ --lib`

3. Run CUDA build verification if any shared contract crosses the CUDA build boundary.
   - `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`

4. Run frontend verification.
   - `npm run typecheck`
   - `npm run test`
   - `npm run build`

5. Run one saved-artifact manual compare check.
   - choose two artifacts whose maxima are known to be fragmented across different tuples
   - confirm the compare/list/header surfaces now show explicit leader identity and disagreement instead of only mixed maxima

6. Run one reopened-artifact inspection check.
   - open the artifact
   - confirm the summary chips match the actual retained tuple rows visible after load

## 9. Cost-risk checklist

This work is acceptable only if all of the following remain true:

- derived in **O(retained tuple count)** from already-saved rows
- no new GPU tensor width
- no new replay or exactification pass
- no new dependence on quote-native semantics or new realism lanes
- no meaningful artifact bloat beyond a compact set of scalar/optional fields
- no claim that the summary is globally valid outside the saved retained tuple set
- no duplication of full frontier tables inside artifact summaries
- no change to ranking behavior

Main risks to watch:

- **false scope expansion:** summary work drifting into ranking-math or GPU-state changes
- **retained-vs-global confusion:** presenting retained-tuple leaders as though they represent the entire compiled grid
- **schema creep:** adding too many fields because each one looks cheap in isolation
- **frontend duplication:** persisting row-level context that the loaded artifact already lets the frontend derive

## 10. Stop conditions

Stop or defer this work if any of these becomes true:

1. The proposed summary cannot be computed entirely from existing retained artifact rows.
2. The implementation starts requiring wider GPU aggregate state or scorer changes.
3. The implementation requires a new exactification or replay pass.
4. The summary cannot be labeled clearly as “within retained tuples,” creating a trust problem.
5. The field set grows from a compact identity/disagreement summary into a large secondary frontier payload.
6. Compare/list/header surfaces already derive equivalent tuple-coherent leadership cleanly from loaded artifacts, making the new summary redundant.
7. The real product need turns out to be globally valid adequacy/frontier statistics across the full compiled grid rather than better retained-summary coherence.

If any of those conditions appears, the correct move is to stop and reclassify the work as Prompt 6 or Prompt 9 territory instead of forcing it through Prompt 5.

## Source basis

This consolidation is based on the attached repo materials, especially:

- `options_expression_gpt54_pro_prompt_pack_2026-04-08.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `src-tauri/src/commands_options_expression.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_types.rs`
- `web/src/stores/optionsExpressionStore.ts`
