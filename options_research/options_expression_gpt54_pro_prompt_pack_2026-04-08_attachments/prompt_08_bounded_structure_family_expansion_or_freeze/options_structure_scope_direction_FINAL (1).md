# options_structure_scope_direction_FINAL

## 1. Executive summary

VectorGrid should **freeze structure-family breadth now** at the currently shipped set:

- `long_single`
- `debit_vertical`
- `long_pair`

No fourth fixed-geometry family should be implemented in the current cycle.

This is the strongest repo-fit direction for three reasons.

First, the current future roadmap explicitly changed the question from “what other structures should we add?” to “how do we deepen the current lane?” It now says new structure families are **not** active next steps, and that the default direction is deeper optimization quality, deeper embedded validation quality, stronger realism/robustness interpretation, and preserved performance rather than wider expression-family breadth. (Source: `options_expression_future_scope_roadmap_2026-04-07.md:332-360`.)

Second, the live code still treats structure family as a **closed, explicitly branched surface**, not as a generic leg-graph. `OptionsStructureFamily` is a three-variant enum, and the evaluator hard-switches on those variants in entry pricing, exit pricing, path-range construction, candidate selection, and event-path replay. That makes every added family a real surface-area increase rather than a cheap enum extension. (Sources: `options_types.rs:156-162`; `options_expression.rs:124-127`; `options_expression.rs:660-716`; `options_expression.rs:758-778`; `options_expression.rs:946-1006`; `options_expression.rs:3498-3518`.)

Third, the current provider-backed Phase 0.5 evidence already shows that the existing three-family lane is viable enough to justify depth work. On the canonical SPY DTE7 compiled-tensor slice, candidate density is about `0.5247`, invalid-slot rate about `0.4753`, structural and summary cache reuse are both `1.0`, and measured GPU scorer speedup is about `5.32x` versus CPU. That means the current blocker is **not** the absence of a fourth family. (Sources: `spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json:62-75`; `spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json:334-346`; `options_expression_optimizer_viable_plan_2026-04-02.md:1826-1847`.)

The right outcome is therefore a **hard freeze for now**, with only a dormant reopen envelope for a future same-expiry fixed-ratio two-leg ladder if — and only if — the repo deliberately revisits structure breadth after the higher-ROI depth backlog is substantially exhausted.

## 2. Current shipped structure-family surface

The shipped options-expression lane is already real and already bounded.

The future roadmap lists the current baseline as an artifact-driven options-expression workflow with a unified History follow-up host, current parent-family support, direct bounded `1m` aggregate provider download, quote-aware import lanes, CPU reference scoring, CUDA compiled-tensor scoring, holdout/sealed/stress reporting, artifact persistence, compare, drilldown, and path replay. Inside that baseline, the currently shipped structure families are exactly `long_single`, `debit_vertical`, and `long_pair`. (Source: `options_expression_future_scope_roadmap_2026-04-07.md:90-114`.)

The viable-plan tracker says the same thing in repo-fit implementation terms: v1 stays scoped to long single-leg calls/puts plus bounded same-expiry fixed-geometry two-leg families, and the current shipped two-leg families are `debit_vertical` and `long_pair`. Broader ladders or other bundles are explicitly post-v1. (Source: `options_expression_optimizer_viable_plan_2026-04-02.md:153-169`.)

At the type level, `OptionsStructureFamily` is a three-variant enum:

- `LongSingle`
- `DebitVertical`
- `LongPair`

(Source: `options_types.rs:156-162`.)

At runtime, the evaluator shape is still fundamentally “primary leg plus optional companion leg.” `SelectedCandidate` is literally modeled as:

- `primary_leg`
- `companion_leg: Option<SelectedLeg>`

(Source: `options_expression.rs:124-127`.)

That runtime shape flows into explicit per-family branches for:

- net entry pricing (`position_entry_prices`) (`options_expression.rs:660-691`)
- net exit pricing (`position_exit_prices`) (`options_expression.rs:693-724`)
- path-range construction (`position_path_range_for_rows`) (`options_expression.rs:758-778`)
- candidate selection and companion-leg alignment (`select_candidate`) (`options_expression.rs:946-1006`)
- event-path replay netting (`build_event_path_points`) (`options_expression.rs:3498-3518`)

The current behavior is also verified by family-specific tests for `debit_vertical` and `long_pair`, including tuple inspection and event-path replay. (Source: `options_expression.rs:5400-5528`.)

On the Tauri/artifact side, `commands_options_expression.rs` persists `request_structure_family` in artifact summaries, keeps the options-expression artifact kind bounded and explicit, and surfaces the selected structure family as first-class provenance. (Sources: `commands_options_expression.rs:64-80`; `commands_options_expression.rs:788-796`; `commands_options_expression.rs:1140-1150`; `commands_options_expression.rs:1695-1742`.)

On the frontend/navigation side, REPO_MAP treats options expression as its own follow-up workflow surface and points contributors at `OptionsExpressionPanel.vue`, the options store/service, and the options command boundary rather than a separate options platform branch. (Source: `REPO_MAP.md:101-123`.) The current panel exposes a structure-family selector, but that selector should remain bounded to the current shipped family set unless the repo deliberately reopens the question. (Source: `OptionsExpressionPanel.vue:2795-2802`.)

## 3. Final recommendation

### Adopted direction

Adopt a **hard freeze** on structure-family breadth for the current cycle.

Concretely:

- keep `long_single`, `debit_vertical`, and `long_pair`
- do **not** add a fourth family now
- do **not** carry a named fourth family as active near-term roadmap work
- keep current effort focused on optimization depth, embedded validation depth, realism/robustness interpretation, and performance preservation inside the existing workflow

For the current follow-through tracker, see [options_structure_scope_freeze_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_08_bounded_structure_family_expansion_or_freeze/options_structure_scope_freeze_implementation_guide_2026-04-10.md).

This is the most faithful reading of the current roadmap, the viable-plan tracker, and the code.

The roadmap is explicit that new structure families are no longer active next steps and that breadth is not the default direction anymore. (Source: `options_expression_future_scope_roadmap_2026-04-07.md:332-360`.) The prompt pack also frames Prompt 8 as a lower-priority scope question, not a primary next-step implementation lane. (Source: `options_expression_gpt54_pro_prompt_pack_2026-04-08.md:1489-1505`.)

### Why freeze beats a bounded addition right now

A narrow same-expiry fixed-ratio ladder is the only future family that still looks structurally plausible from the current repo shape. The viable-plan document explicitly preserves that as the first conceivable post-v1 expansion order if the currently shipped two-leg families continue to prove performant enough. (Source: `options_expression_optimizer_viable_plan_2026-04-02.md:215-226`.)

But that is **not** the same thing as an active recommendation.

Adding that family now would still widen every place that already special-cases `OptionsStructureFamily`, including candidate selection, companion-leg resolution, pricing, replay, artifact provenance, UI request handling, and tests. Because the current runtime is not generic, each new family still costs real maintenance and verification budget. The roadmap now says that budget should go elsewhere first. (Sources: `options_expression.rs:660-716`; `options_expression.rs:758-778`; `options_expression.rs:946-1006`; `commands_options_expression.rs:64-80`; `OptionsExpressionPanel.vue:2795-2802`.)

### What should remain unchanged

The following constraints should remain unchanged:

- one structure family per job
- fixed-geometry structure compilation
- same-expiry first-wave multi-leg scope
- deterministic strike-offset geometry from a base selector
- same entry timestamp across all legs in a bundle
- no rolling
- no assignment/exercise modeling
- no generic multi-leg scripting
- no dedicated options workflow branch
- CPU fallback/oracle behavior remains intact
- the History follow-up host remains the default product surface

These are all aligned with the repo’s current product shape and with the future roadmap’s performance-first doctrine. (Sources: `options_expression_optimizer_viable_plan_2026-04-02.md:223-229`; `options_expression_optimizer_viable_plan_2026-04-02.md:1733-1749`; `options_expression_future_scope_roadmap_2026-04-07.md:332-360`.)

## 4. Deferred ideas

These ideas are **deferred**, not adopted.

### 4.1 Dormant future reopen candidate

If the repo ever deliberately reopens structure breadth, the only plausible first candidate is a **same-expiry fixed-ratio two-leg ladder beyond `long_pair`**.

This is deferred because:

- the viable plan preserves it only as a future expansion envelope, not as active roadmap work
- the current runtime still branches family-by-family
- current repo priorities now point to depth, not breadth

(Source: `options_expression_optimizer_viable_plan_2026-04-02.md:215-226`.)

### 4.2 Additional same-expiry fixed debit families

These remain even further back in the queue than the ladder-style reopen candidate. The viable plan lists them behind the ladder-family reopen idea. They should remain dormant until the first reopen candidate, if ever pursued, proves that density, cache reuse, and GPU economics survive. (Source: `options_expression_optimizer_viable_plan_2026-04-02.md:217-221`.)

### 4.3 Explicit freeze codification in docs/UI text only

If the repo wants to make the freeze more visible, the only immediate change worth considering is **documentation or labeling clarification**, not implementation of another structure family.

That means small clarifications in:

- roadmap or viable-plan wording
- options-panel helper text
- artifact comparison labels if needed

This is optional polish, not a new feature lane.

## 5. Rejected ideas

These ideas should be treated as rejected for the current cycle.

### 5.1 Add a fourth family now

Rejected because the roadmap explicitly says new structure families are no longer active next steps and because the current provider-backed evidence does not show a missing-family bottleneck. Existing families already clear viability and GPU-economics gates on the canonical slice. (Sources: `options_expression_future_scope_roadmap_2026-04-07.md:332-360`; `spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json:62-75`; `spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json:334-346`.)

### 5.2 Broader bounded-family expansion program

Rejected because reopening broader bounded-family work would spend engineering budget in the exact area the roadmap now says to deprioritize. It would also increase the number of explicit branches across a still-closed evaluator surface. (Sources: `options_expression_future_scope_roadmap_2026-04-07.md:334-360`; `options_expression.rs:660-716`; `options_expression.rs:946-1006`.)

### 5.3 Any drift toward generic multi-leg strategy breadth

Rejected, including:

- arbitrary leg counts
- mixed-expiry calendars/diagonals
- rolling logic
- assignment/exercise modeling
- generic event scripting
- turning the lane into a general options platform

Those are explicitly out of scope in the roadmap and would break the current compiled-tensor, GPU-first product shape. (Source: `options_expression_future_scope_roadmap_2026-04-07.md:140-152`.)

### 5.4 Reframing family expansion as UI or workflow work

Rejected. The current repo already routes options expression through the existing follow-up workflow, and the roadmap explicitly says the default direction is to deepen the current workflow rather than create new workflow branches. (Sources: `REPO_MAP.md:101-123`; `options_expression_future_scope_roadmap_2026-04-07.md:349-360`.)

## 6. Concrete code touch points by file

Because the recommendation is **freeze, not expand**, the primary guidance is “do not widen these surfaces.”

### `crates/app_core/src/options_types.rs`

Keep `OptionsStructureFamily` unchanged.

Relevant symbol:

- `OptionsStructureFamily`

Current surface:

- `LongSingle`
- `DebitVertical`
- `LongPair`

(Source: `options_types.rs:156-162`.)

### `crates/app_core/src/options_expression.rs`

Do **not** add new structure-family branches unless the repo deliberately reopens the question.

Key symbols that would need widening if breadth were reopened:

- `SelectedCandidate`
- `position_entry_prices`
- `position_exit_prices`
- `position_path_range_for_rows`
- `select_candidate`
- `build_event_path_points`

These are the real cost centers for family expansion because they still encode structure semantics explicitly. (Sources: `options_expression.rs:124-127`; `options_expression.rs:660-716`; `options_expression.rs:758-778`; `options_expression.rs:946-1006`; `options_expression.rs:3498-3518`.)

Also preserve the current family-specific tests as the guardrail for the existing bounded set:

- `tuple_inspection_returns_companion_leg_for_debit_vertical`
- `tuple_inspection_returns_companion_leg_for_long_pair`
- `event_path_replay_returns_net_position_points_for_debit_vertical`
- `event_path_replay_returns_net_position_points_for_long_pair`

(Source: `options_expression.rs:5400-5528`.)

### `src-tauri/src/commands_options_expression.rs`

Do not widen artifact or summary plumbing for another family unless the reopen criteria are met.

Key symbols/fields:

- `OptionsExpressionArtifactSummary.request_structure_family`
- `summarize_options_expression_artifact(...)`
- artifact-kind handling for `vectorgrid_options_expression_v1`

(Source: `commands_options_expression.rs:64-80`; `commands_options_expression.rs:788-796`; `commands_options_expression.rs:1140-1150`; `commands_options_expression.rs:1695-1742`.)

### `web/src/components/OptionsExpressionPanel.vue`

Keep the structure-family selector bounded to the current shipped options unless the repo deliberately reopens structure breadth.

Relevant surface:

- `Structure family` selector bound to `OPTIONS_STRUCTURE_FAMILY_OPTIONS`

(Source: `OptionsExpressionPanel.vue:2795-2802`.)

### `options_research/options_expression_future_scope_roadmap_2026-04-07.md`

If any repo-visible clarification is desired, this is the canonical place to keep the freeze explicit.

Relevant section:

- `## 8. Scope Freeze For This Roadmap`

(Source: `options_expression_future_scope_roadmap_2026-04-07.md:332-360`.)

### `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`

If the dormant reopen envelope needs clarification, keep it here and keep it clearly subordinate to the freeze.

Relevant sections:

- shipped scope and post-v1 family note (`1733-1749`)
- preserved future expansion order (`215-226`)
- current CUDA/viability gates (`1923-1934`)

## 7. Verification order

Because the recommendation is **do not add a family now**, verification is mostly about preserving the current bounded surface and avoiding accidental drift.

### If no code changes are made

1. Treat the freeze as a repo decision only.
2. No new benchmark run is required solely to support the freeze.
3. Keep current family-specific Rust tests and artifact-summary expectations passing.

### If only docs or UI copy are changed

1. Run the relevant frontend tests that touch the options panel.
2. Run targeted Rust tests for options-expression artifact summary and existing structure-family behavior.
3. Confirm that artifact reopen, tuple inspection, and event-path replay still behave identically for:
   - `long_single`
   - `debit_vertical`
   - `long_pair`

### If someone later proposes reopening the question

Run a dedicated feasibility spike **before** implementing a family broadly.

Minimum verification order:

1. **Type/runtime fit check**
   - prove the candidate still fits the current fixed-geometry, same-expiry, deterministic structure model
   - prove it does not require generic leg scripting

2. **Candidate-density and invalid-slot check**
   - use representative provider-backed jobs
   - require density and invalid-rate to stay inside the current promotion envelope

3. **Cache-reuse and replay-cost check**
   - require strong immediate rerun reuse
   - prove preprocessing does not drown the scorer economics

4. **GPU-economics check**
   - prove the compiled tensors remain GPU-worthy under the current product gates

5. **Decision-value check**
   - show that the added family produces materially useful robust finalists or validation winners that the current three-family set is not already providing

The current documented promotion gates are still the right baseline for that reopen feasibility work:

- valid-slot density at least `25%`
- invalid-slot rate no worse than `75%`
- immediate rerun structural and summary cache reuse at least `90%`
- measured GPU scorer at least `2.75x` faster than CPU on representative large compiled-tensor jobs
- scoring remains a meaningful share of rerun wall time

(Source: `options_expression_optimizer_viable_plan_2026-04-02.md:1923-1934`.)

## 8. Reopen criteria

The structure-family question should stay closed unless **all** of the following are true.

### 8.1 Priority criterion

The current higher-ROI backlog in optimization depth, embedded validation depth, realism/robustness interpretation, and performance preservation has been substantially exhausted first.

This is required because the roadmap explicitly says those are the default next-step areas rather than broader structure-family work. (Source: `options_expression_future_scope_roadmap_2026-04-07.md:349-360`.)

### 8.2 Shape criterion

The candidate family must remain:

- fixed-geometry
- same-expiry in first-wave support
- deterministic from a base selector
- one structure family per job
- same entry timestamp across all legs
- free of rolling, assignment, and exercise semantics

(Source: `options_expression_optimizer_viable_plan_2026-04-02.md:223-229`.)

### 8.3 Runtime-fit criterion

The candidate must fit the current evaluator shape without forcing a generic leg-engine rewrite.

In practice that means one of two things:

- it still fits the current `primary_leg + optional companion_leg` structure directly, or
- any extension beyond that is genuinely local and does not metastasize into generic multi-leg scripting

(Source: `options_expression.rs:124-127`.)

### 8.4 Economics criterion

A dedicated feasibility spike must show that the candidate preserves the repo’s current promotion envelope:

- valid-slot density at least `25%`
- invalid-slot rate no worse than `75%`
- structural and summary cache reuse at least `90%`
- GPU scorer at least `2.75x` faster than CPU on representative compiled-tensor jobs

(Source: `options_expression_optimizer_viable_plan_2026-04-02.md:1923-1934`.)

### 8.5 Decision-value criterion

The candidate must show material decision value beyond cosmetic breadth.

That means evidence such as:

- materially different robust finalists
- materially different holdout or sealed winners
- materially different useful stress/validation outcomes

If it cannot clear that bar, it is only adding maintenance surface.

### 8.6 Explicit stop conditions

Keep the freeze in place if any of the following happen during a reopen spike:

- the candidate pushes the runtime toward generic multi-leg scripting
- density or cache reuse degrades materially
- GPU economics fall below the current gates
- artifact/UI complexity rises without clearly better decision-grade outcomes
- the new family duplicates behavior the current three-family set already captures well enough

## Bottom line

The current repo should **freeze structure-family breadth now**.

The next implementation dollar belongs in the already-declared depth lanes, not in adding a fourth structure family. The only family worth keeping alive as a dormant future reopen candidate is a same-expiry fixed-ratio two-leg ladder beyond `long_pair`, and even that should remain closed by default until the explicit reopen criteria above are met.
