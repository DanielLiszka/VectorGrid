# GPT-5.4 Pro Prompt Pack For Options-Expression Research

Last revised: 2026-04-08

Purpose: a trimmed, implementation-oriented prompt pack for handing the remaining worthwhile `options_expression` full-GPU research questions to a stronger model such as GPT-5.4 Pro.

This file intentionally replaces the broader April 8 prompt list. That earlier format still encouraged menu-style brainstorming. The only prompts kept here are the ones most likely to produce a defensible recommendation about what to implement or change next in this repo.

Current repo evidence already says:

- the artifact-driven options-expression workflow is already shipped
- supported parent families already include `DoubleMa`, `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- supported structure families already include `long_single`, `debit_vertical`, and `long_pair`
- the compiled-tensor GPU scorer path is real, benchmarked, and already usable on bounded canonical workloads
- two bounded provider-backed aggregate slices (`SPY` DTE7 and `QQQ` DTE7) now clear the live auto-GPU gates in code
- the remaining uncertainty is mostly about next-step ROI inside the current full-GPU lane, especially the GPU/CPU finalist boundary, not about whether the feature exists at all

## How To Use This Pack

- Use one primary prompt per chat or thread.
- Attach the shared baseline files first.
- Then attach the prompt-specific files.
- The primary prompt decides a direction.
- The `Second-Pass Prompt` and `Final Consolidation Prompt` are optional follow-ups inside the same thread. They are not separate required threads.
- The primary prompts are not strict serial dependencies on one another. Start with Prompt 1. Run Prompt 2 or Prompt 3 only if Prompt 1 says that topic is still the best unresolved question.
- Ask the model to stay repo-specific and to distinguish:
  - what the repo already completed
  - what is still genuinely open
  - what should be rejected as low ROI

Important instruction for every thread:

- full GPU execution remains the target
- do not widen this into a general options platform or LEAN replacement
- preserve the current artifact-driven workflow
- do not reopen parent-family or structure-family expansion unless the prompt explicitly asks for it
- keep CPU fallback correctness intact
- treat live code and latest local benchmark artifacts as authoritative when older docs differ

## Shared Baseline Files

Attach these to every prompt:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`

Note on file lists:

- `Required` means the minimum file set that should be attached for a grounded answer.
- `Optional` means useful expansion context, not mandatory context.
- Do **not** interpret `Required` as “every conceivably relevant file in the repo.”
- When in doubt, prefer the smaller required set first; only add optional files if the prompt needs more implementation detail.

Recommended shared framing for every thread:

- the current shipped slice already includes saved artifacts, tuple drilldown, event replay, holdout/sealed/stress reporting, and a compiled-tensor GPU scorer
- the current GPU route is still hybrid at the finalist boundary: broad tuple ranking happens on GPU, then `top_k` exactification, sealed evaluation, and extreme-stress outputs still flow through CPU replay
- direct provider download is still bounded to `1m` aggregate datasets; quote-aware and derived-mid realism lanes are currently CSV-import paths
- live code now uses more permissive auto-GPU gates than some older docs; if docs and code disagree, prefer current code plus latest local artifacts
- the latest provider-backed evidence is no longer single-root: use both the shipped `SPY` and bounded `QQQ` phase-0.5 outputs before declaring a GPU direction "proven"
- the latest canonical artifacts show useful GPU viability, but still only middling compiled density and materially non-zero invalid-slot rates
- the dominant recurring invalid reasons across the current provider-backed artifacts are `entry_past_cutoff`, `no_strike_in_tolerance`, and slice-overlap / eligibility failures such as `contract_not_eligible_as_of_event`

## Shared Answer Shape

For any prompt in this pack, ask the model to keep the answer concrete and implementation-oriented.

Minimum answer shape to request when you want a full-quality response:

1. What the repo already completed
2. What remains genuinely unresolved
3. The single best remaining direction, if any
4. What should explicitly remain unchanged
5. Concrete code touch points by file and symbol
6. A staged benchmark and validation order
7. Rejection criteria and failure modes

If you need more structure, also ask for:

- an `implement now / defer / reject` table
- a short artifact plan
- a note explaining which previously plausible ideas should now be considered closed

## Consolidated Short Route

If you want broad coverage without running a large serial tree of prompts, use this compressed route instead.

This is the recommended route when:

- you want to stay under practical upload/time limits
- you want current-scope coverage without losing the CUDA-first focus
- you do not want to run every narrow prompt separately

This short route keeps the same overall coverage, but collapses the highest-overlap prompt families into a few larger questions.

### Safe Order

1. Consolidated Prompt A: CUDA-First Performance And Productization Direction
2. Then run Consolidated Prompt B and Consolidated Prompt C
   - these can run in parallel if you want to save time
3. Then optionally run Consolidated Prompt D if you still care about scope-freeze questions
4. Then use the existing final cross-prompt consolidation prompt on the outputs you actually ran

### Consolidated Prompt A

Replaces:

- Prompt 1
- Prompt 2
- Prompt 3
- Prompt 12
- Prompt 13
- Prompt 14

Use this when the real question is:

- where does the next performance dollar belong now?
- is the next blocker density/coverage, replay/prepare amortization, GPU/CPU finalist boundary, auto-promotion policy, CUDA runtime structure, or SASS-level work?

Recommended attachment set: 13 files

- `AGENTS.md`
- `REPO_MAP.md`
- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json`

```text
You are reviewing VectorGrid's current options-expression workflow as one combined CUDA-first productization problem.

Using only the attached files, answer this repo-specific question:

Given the current shipped options lane, what is the single best next implementation direction if the goal is to maximize real GPU/CUDA product value without widening scope?

You must evaluate all of these candidate directions together rather than treating them as separate prompts:

- candidate density / invalid-slot / split-policy / provider-slice coverage work
- replay/build and preparation/cache amortization work
- GPU/CPU finalist exactification and embedded-validation boundary work
- auto-backend / planner / product-promotion hardening
- CUDA runtime micro-architecture work
- SASS/PTX-level investigation
- or "do nothing major yet"

Constraints:
- stay inside the current options optimization + embedded validation workflow
- do not recommend new strategy families or workflow stages
- keep GPU-first execution as a product requirement
- keep CPU fallback/oracle behavior intact
- treat SASS-level work as guilty until proven valuable
- use the attached provider-backed evidence, not generic CUDA folklore
- explicitly distinguish these failure classes:
  - scorer/runtime issue
  - replay/prepare issue
  - GPU/CPU finalist-boundary issue
  - density / search-quality issue
  - provider-slice / overlap / eligibility issue
  - policy / promotion issue

Deliverables:
1. Summarize what the repo already completed.
2. Decompose where time and quality risk still go now.
3. Compare the candidate directions above and state which one should actually win now.
4. State what should explicitly remain unchanged.
5. Identify concrete code touch points by file and symbol.
6. Define the smallest benchmark / profiling / parity plan needed before and after implementation.
7. State exact stop conditions for each major rejected/deferred direction.

Make one coherent repo decision, not an equal-weight menu.
```

### Consolidated Prompt B

Replaces:

- Prompt 5
- Prompt 9
- Prompt 10

Use this when the real question is:

- what extra useful one-pass work should be added?
- what extra aggregate state is worth carrying?
- should the ranking functional change?

Starting assumptions for this prompt family:

- Prompt 4 is closed.
- Aggregate remains the canonical provider-download lane.
- Direct quote-aware Polygon/Massive download remains deferred.
- `DerivedMidSnapshot` remains first-class in product posture but aggregate-style in execution.
- `TopOfBookQuote` remains the only quote-side execution lane.
- Do not recommend metric/reporting ideas that depend on:
  - direct quote-aware provider download
  - derived-mid becoming quote-side
  - new realism lanes

Recommended attachment set: 9 files

- `AGENTS.md`
- `REPO_MAP.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`

```text
You are reviewing VectorGrid's current options-expression optimizer as one combined single-pass metrics and ranking-math problem.

Using only the attached files, answer this repo-specific question:

What additional useful work should this repo do inside the current options optimization pass, and should the ranking functional change, if the goal is to increase decision quality at near-constant marginal cost?

You must evaluate these questions together:

- what extra metrics are mathematically admissible from current or slightly richer aggregate state
- what extra per-tuple per-lane state budget is still acceptable
- what should happen with metrics like `median_return_on_premium`
- whether broad pre-`top_k` ranking, exactified reranking, and sealed-finalist ordering should share one functional or deliberately differ
- whether the current scalarization should stay, be minimally revised, or be replaced

Constraints:
- stay inside the current options optimization + embedded validation workflow
- no new workflow stages
- no generic Pareto expansion
- no path-replay-heavy ideas in the hot loop
- be mathematically explicit: formulas, state vectors, accumulator sets, mergeability
- distinguish:
  - exact from current state
  - exact with a few extra mergeable accumulators
  - approximate from a bounded sketch
  - not worth implementing

Deliverables:
1. Summarize current metrics, aggregate state, and ranking math.
2. Propose the best single-pass metric expansion, if any.
3. Propose the best ranking-functional direction, if any.
4. State the required accumulator/state vector and whether it widens GPU aggregate state, exact metrics, or both.
5. State what should remain unchanged.
6. Identify concrete code touch points by file and symbol.
7. Define verification, parity, and rejection criteria.

Make one coherent optimization-depth direction for this repo.
```

### Consolidated Prompt C

Replaces:

- Prompt 6
- Prompt 11

Use this when the real question is:

- how should embedded validation deepen?
- what additional frontier/confidence math is worth deriving?
- how should verdicts, adequacy, and artifact/frontier coherence improve?

Current starting truth for Consolidated Prompt C:

- Prompt 5 is now closed via:
  - [options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md)
- artifact-summary coherence is already shipped at the compact retained-leader/disagreement level
- tuple-coherent artifact list / compare / reopened summary cues already exist
- this prompt should evaluate only what embedded validation still needs **beyond that shipped baseline**

Recommended attachment set: 11 files

- `AGENTS.md`
- `REPO_MAP.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

```text
You are reviewing VectorGrid's current options-expression workflow as one combined embedded-validation frontier problem.

Using only the attached files, answer this repo-specific question:

How should the repo deepen embedded options validation and derived validation math without creating a new stage, while keeping the result tuple-coherent, decision-grade, and performance-conscious?

You must evaluate these questions together:

- adequacy / sufficiency signals from split counts, valid trades, and coverage
- shortlist / `top_k` / `sealed_finalist_count` distortion
- frontier / regret / dominance / monotone-stress-deterioration summaries
- whether new math complements or changes `inferred_validation_verdict()` / `is_stress_fragile()`
- whether artifact-level summaries should remain scalar maxima or become more tuple-coherent
- where any adopted statistic actually belongs:
  - `OptionsExpressionTupleResult`
  - verdict derivation
  - `OptionsExpressionArtifactSummary`
  - frontend-only reporting

Constraints:
- stay inside the current options optimization + embedded validation workflow
- no new validation mode
- no heavyweight rerun/bootstrap suggestions unless they clearly fit the current runtime philosophy
- do not spend your main recommendation re-solving artifact-summary coherence that Prompt 5 already shipped
- explicitly state whether each serious candidate statistic is trustworthy:
  - globally across all tuples
  - only within retained `top_k`
  - or only within sealed finalists

Deliverables:
1. Summarize current embedded validation depth and verdict logic.
2. Identify what is still genuinely missing.
3. Propose the best next embedded-validation / frontier-math additions, if any.
4. State whether the current shortlist/finalist policy should change.
5. State whether artifact/frontier summaries should become more tuple-coherent.
6. Identify concrete code touch points by file and symbol.
7. Define acceptance, interpretability, and rejection criteria.

Make one coherent embedded-validation direction for this repo.
```

### Consolidated Prompt D (Optional)

Replaces:

- Prompt 4
- Prompt 7
- Prompt 8

Use this only if you still care about current scope-freeze questions.

Recommended attachment set: 11 files

- `AGENTS.md`
- `REPO_MAP.md`
- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

```text
You are reviewing the remaining scope-freeze questions in VectorGrid's current options-expression workflow.

Using only the attached files, answer this repo-specific question:

Should the repo keep the current scope frozen, or is there one narrow additional realism/stress/structure expansion still worth allowing without hurting the GPU-first product shape?

You must evaluate these questions together:

- should quote-aware realism remain secondary or become more first-class
- should the current stress-family set stay frozen
- should the current bounded structure-family set stay frozen

Constraints:
- stay inside the current options optimization + embedded validation workflow
- no new parent strategy families
- no new workflow stages
- no generic options-platform expansion
- prefer freeze unless there is a strong current-scope reason not to

Deliverables:
1. Summarize the current frozen scope.
2. State which parts should stay frozen.
3. State whether any one narrow expansion is still justified now.
4. Identify concrete code touch points by file if anything should change.
5. Define explicit reopen criteria for any deferred/frozen lane.

Be conservative. The default answer should be to keep current scope narrow unless the repo evidence says otherwise.
```

## First-Pass Prompt Discipline

For every primary prompt in this pack, prefer answers that do all of the following:

- cite concrete repo files, symbols, runtime paths, and benchmark surfaces
- compare at least:
  - one conservative direction
  - one recommended direction
  - one more aggressive direction
- state which option should actually win now
- separate:
  - implement now
  - defer
  - reject
- explain what should remain unchanged because it is already working
- translate any externally researched reference back into repo-specific implications rather than leaving it as generic CUDA or options inspiration

What to avoid:

- generic options-backtesting advice detached from this repo
- generic CUDA best-practice lists detached from this repo
- broad architecture brainstorms with no benchmark plan
- answers that quietly reopen parent-family, structure-family, or workflow-stage expansion
- answers that treat every option as equally viable

## Second-Pass Prompt Discipline

For every second-pass prompt in this pack:

- do not merely restate the first answer
- identify contradictions, weak assumptions, and over-designed ideas
- explicitly trim advice that is low-confidence or low-ROI
- collapse the answer toward one winning direction
- use targeted current web research only when it materially sharpens a repo decision
- say what new evidence changed the recommendation, if any

## Final Consolidation Prompt Discipline

For every final consolidation prompt in this pack:

- write as if the result will be checked into `options_research/`
- present one coherent repo direction, not an equal-weight summary of alternatives
- include explicit:
  - adopted direction
  - deferred ideas
  - rejected ideas
- include file-level implementation touch points
- include a staged benchmark and verification order
- keep open questions narrow and specific

## Prompt Family Split

This file now has two different prompt families.

### A. Core GPU Productization Prompts

These are:

- Prompt 1
- Prompt 2
- Prompt 3
- Prompt 12
- Prompt 13
- Prompt 14

Use them when the question is:

- what, if anything, still needs to change in the current full-GPU options productization path?

These prompts overlap heavily.

Treat them as a decision tree, not as independent parallel work:

- run Prompt 1 first
- then run **one of** Prompt 13, Prompt 12, Prompt 2, Prompt 3, or Prompt 14 if Prompt 1 says one of those lanes is still genuinely unresolved
- do not run Prompt 2 and Prompt 3 as equal-priority first-pass prompts at the same time
- do not run Prompt 12 in parallel with Prompt 1; Prompt 12 is for the narrower GPU/CPU finalist-boundary question after Prompt 1
- do not run Prompt 13 in parallel with Prompt 1; Prompt 13 is for the narrower end-to-end cost-decomposition question after Prompt 1
- do not run Prompt 14 before Prompt 13; Prompt 14 is only for the narrower CUDA-runtime / kernel / SASS micro-architecture question after cost decomposition says that layer is worth pursuing

### B. Scope-Depth Prompts

These are:

- Prompt 4
- Prompt 5
- Prompt 6
- Prompt 7
- Prompt 8

Use them when the question is:

- how far should the current options workflow deepen without widening into a general platform?

These prompts overlap less than Prompts 1-3, but they are still not fully independent.

They should be treated as:

- related scope reviews inside the same product boundary
- not as invitations to run five simultaneous design threads and then average the answers

### C. Math-Heavy Prompts

These are:

- Prompt 9
- Prompt 10
- Prompt 11

Use them when the question is:

- what additional useful work can be justified mathematically inside one options optimization or validation pass?

These prompts are for harder mathematical reasoning about:

- sufficient statistics
- low-cost metric expansion
- ranking / scalarization math
- confidence / shrinkage / acceptance math

They should not be used for generic quant ideation. Their job is to translate mathematical reasoning into concrete repo decisions about what can be added at near-constant marginal cost.

## Current CUDA-First Review Order

If the actual goal is to maximize CUDA/GPU usage and performance inside the existing options optimization + embedded validation lane, use this order:

1. Prompt 1
2. Prompt 13
3. Prompt 12
4. Prompt 14 only if Prompt 13 says runtime micro-architecture is the real next bottleneck
5. Prompt 9
6. Prompt 10 and/or Prompt 11
7. Prompt 2 or Prompt 3 only if Prompt 1 says density or product-promotion policy is still the real blocker
8. Prompt 5 or Prompt 6 only after the CUDA-first questions above are settled
9. Prompt 4, Prompt 7, and Prompt 8 only if they materially affect density, compiled coverage, or GPU economics

Why this order:

- Prompt 1 decides whether there is any real full-GPU productization work left at all
- Prompt 13 asks where the next performance dollar actually belongs once GPU scoring is already viable: replay build, preparation/cache amortization, lane duplication, or runtime micro-architecture
- Prompt 12 targets the biggest current unresolved runtime boundary in code: GPU broad ranking versus CPU finalist exactification and embedded validation replay
- Prompt 14 is deliberately later because SASS-level or micro-architecture work is only worth it if the kernel/runtime layer is actually first-order after Prompt 13
- Prompt 9 asks what additional useful work can be added by enriching small per-tuple aggregate state rather than by replaying more paths on CPU
- Prompt 10 and Prompt 11 become more valuable after Prompt 9 clarifies what extra low-cost state is actually plausible
- Prompt 5 and Prompt 6 are still useful, but they are more product/reporting-oriented than CUDA-first

## Parallel vs Serial Use

### Safe serial default

If you want the lowest-confusion workflow, do this:

1. run Prompt 1
2. if needed, run Prompt 13 or Prompt 12 or Prompt 2 or Prompt 3
3. run Prompt 14 only if Prompt 13 says the CUDA runtime / kernel layer is the real next target
4. if needed, run Prompt 9
5. then run Prompt 10 or Prompt 11 if the remaining question is mathematical rather than architectural
6. then run one or two relevant scope-depth prompts from Prompts 4-8
7. then run the final cross-prompt consolidation prompt
8. only then make implementation changes

That is the safest pattern.

### Safe parallel use for research only

It is generally safe to run some **first-pass analysis prompts** in parallel if they are asking clearly different questions and you are still in the research stage.

Best parallel candidates:

- Prompt 4 with Prompt 7
- Prompt 4 with Prompt 8
- Prompt 7 with Prompt 8
- Prompt 9 with Prompt 11

These pairs have relatively low overlap because they separate:

- realism-lane decisions
- stress-family freeze/expansion decisions
- structure-family freeze/expansion decisions

### Serial-preferred combinations

These should usually be run in serial, not parallel:

- Prompt 1 with Prompt 2
- Prompt 1 with Prompt 3
- Prompt 2 with Prompt 3
- Prompt 5 with Prompt 6
- Prompt 6 with Prompt 7
- Prompt 9 with Prompt 10
- Prompt 10 with Prompt 11

Why:

- Prompts 1-3 overlap heavily on the same GPU productization decision
- Prompts 5-7 all touch optimization/validation depth and can easily produce partially redundant or contradictory recommendations
- Prompts 9-11 all touch the same mathematical substrate, and Prompt 10 in particular can depend on what Prompt 9 or Prompt 11 conclude is cheaply measurable

### Implementation must be serial

Even if you run research prompts in parallel, **implementation should return to a single serial branch**:

- first-pass prompts may run in parallel
- second-pass prompts should usually be serial
- final consolidation should be serial
- code changes should be serial and based on the consolidated direction, not on multiple unresolved prompt outputs

### Practical rule

If you are unsure:

- parallelize only first-pass research on clearly different questions
- serialize everything from second-pass onward

## Prompt 1: Next Full-GPU Productization Direction

### Why This Prompt Still Looks Worth Doing

This is the best first prompt because the repo already proved the basic point that matters most: the compiled-tensor GPU path can win on at least one canonical bounded workload.

That means the next mistake is no longer "is GPU real?" The next mistake is either:

- optimizing the wrong layer
- widening scope too early
- or treating several directions as equally valid when only one actually deserves implementation time

This prompt is the one that should decide whether the next implementation work belongs in:

- candidate density and search-shape
- backend planner and promotion hardening
- a narrow realism-lane adjustment that materially helps the full-GPU path
- or no major change yet

### Attach These Files

Required:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`

Optional:

- `src-tauri/src/commands_options_data.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing the next implementation direction for the options-expression optimizer in this repo.

Using only the attached files, answer this repo-specific question:

Now that the canonical SPY DTE7 aggregate workload proves the compiled-tensor GPU path can win and the live auto-GPU gates can pass, what is the single best remaining direction to implement next if the scope stays limited to full-GPU execution inside the current options lane?

Constraints:
- full GPU execution remains the target
- do not broaden into a general options platform or LEAN replacement
- do not recommend new parent-family or structure-family expansion
- do not recommend quote-provider product expansion
- keep CPU fallback behavior intact
- keep the artifact-driven workflow intact
- if older docs differ from current code or latest artifacts, treat current code and latest artifacts as the source of truth
- choose among:
  - candidate density / invalid-slot / search-shape work
  - backend planner / auto-promotion / product hardening work
  - narrow realism-lane changes that materially improve the full-GPU path
  - or "do nothing major yet" if the evidence does not justify more work

Deliverables:
1. Summarize what the repo already completed.
2. State what remains genuinely unresolved.
3. Pick one winning direction and explain why it beats at least one more conservative and one more aggressive alternative.
4. State what should explicitly remain unchanged.
5. Identify concrete code touch points by file and symbol.
6. Define the smallest benchmark and validation plan needed before implementation and after implementation.
7. State exact stop/go criteria that would cause this direction to be adopted, deferred, or rejected.

Be concrete, repo-specific, and implementation-oriented.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 1 as a critical second pass.

I want you to pressure-test the implementation direction you just recommended for this repo.

Tasks:
1. Identify where your recommendation may be overweighting the canonical SPY DTE7 evidence or underweighting broader productization risk.
2. Identify any places where you silently smuggled in feature expansion rather than true full-GPU productization work.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - GPU routing and promotion patterns for compiled research workloads
   - practical promotion-gate design for accelerator-backed search systems
   - when density/coverage issues dominate over raw scorer throughput
4. Compare your proposal against:
   - "do nothing major yet"
   - one more conservative alternative
   - one more aggressive alternative
5. Decide which direction should actually win for this repo now, if any.

Deliverables:
1. `Keep / revise / reject` verdict on your first-pass recommendation
2. The refined winning direction
3. The strongest competing explanation for why that direction may be wrong
4. The biggest implementation risk
5. The exact evidence that would change your recommendation

Stay repo-specific. Do not broaden this into general options-platform advice.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 1, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_expression_full_gpu_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current repo evidence summary
3. Final recommendation
4. Explicitly retained current strengths
5. Explicitly deferred ideas
6. Explicitly rejected ideas
7. Concrete code touch points by file and symbol
8. Benchmark and validation plan
9. Verification plan
10. Rejection criteria and stop conditions

Do not restate all brainstormed options equally. Make one decision and present one coherent direction for this repo.
```

## Prompt 2: Candidate Density, Invalid Slots, And Search-Shape Direction

### Why This Prompt Still Looks Worth Doing

This is the best deep-dive prompt if Prompt 1 concludes that the GPU scorer is already good enough and the practical blocker is still search quality and compiled coverage.

Current repo evidence already says:

- the scorer path exists
- overlap-quality and adequacy diagnostics are already shipped
- the fixed SPY/QQQ sign-off pair is now stable cold and warm
- candidate density and invalid-slot rate are still materially imperfect on bounded provider-backed slices
- `entry_past_cutoff` is reproducibly present across both sign-off roots
- `no_strike_in_tolerance` is meaningful in SPY but not stable across the sign-off pair
- bounded provider-backed slices can surface overlap/eligibility failures such as `contract_not_eligible_as_of_event`
- the overlap-quality hardening pass explicitly did **not** justify a default clamp yet

That makes density, invalid-slot causes, and search-shape design a real implementation question. Another generic GPU optimization pass is not.

It also means split-assignment policy is not just a validation afterthought. In the shipped repo, `event_chronological` versus `session_chronological` can materially change whether holdout and sealed folds receive any executable candidates at all.

### Attach These Files

Required:

- `options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md`
- `output/options_expression_overlap_quality_signoff/spy_warm/2026-04-08T21-03-50/phase05_summary.json`
- `output/options_expression_overlap_quality_signoff/qqq_warm/2026-04-08T21-04-35/phase05_summary.json`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`

Optional:

- `web/src/stores/optionsExpressionStore.ts`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing candidate density and invalid-slot causes in the options-expression optimizer in this repo.

Using only the attached files, answer this repo-specific question:

After the shipped overlap-quality hardening pass, is there still one implementable search-shape intervention worth doing now to improve useful candidate density and decision-grade coverage for the current options-expression full-GPU path, or should the repo stop here and keep the current defaults?

Constraints:
- do not broaden supported parent families or structure families
- do not weaken realism by simply ignoring invalid cases
- do not recommend raw-chain GPU simulation
- keep compiled-tensor scoring as the hot path
- use the latest invalid-reason evidence rather than generic options intuition
- treat the current overlap/adequacy reporting as already implemented baseline, not as a proposed fix
- explicitly respect that the latest sign-off pass did not justify a default clamp yet
- explicitly consider whether split-assignment policy is part of the real search-quality / validation-quality solution rather than a separate workflow concern
- explicitly consider whether provider-slice overlap / as-of eligibility is part of the real problem rather than treating every invalid as a pure tuple-grid issue
- distinguish clearly between:
  - search-space default changes
  - tuple-grid or axis redesign
  - parent-event filtering
  - split-assignment or fold-policy changes
  - dataset curation or lane constraints
  - ideas that should be rejected

Deliverables:
1. Summarize what the attached evidence already proves about candidate density, invalid-slot rate, overlap, adequacy, invalid reasons, and split-level event coverage.
2. Distinguish which invalid reasons are:
   - productively reducible
   - better handled by search-space defaults
   - signals that a profile should be constrained or filtered
   - signals that the provider-backed slice itself is not representative enough for this test
   - or not worth "fixing"
3. State explicitly whether split-assignment mode should be treated as a first-class search/coverage lever for this repo now.
4. State explicitly whether the correct answer is:
   - one narrow next intervention
   - or no further change right now beyond the shipped diagnostics
5. If you recommend an intervention, propose the single best next intervention.
6. Explain why your answer is better than at least one narrower default tweak and one broader search-space redesign.
7. Explain why your answer is better than simply stopping after the current overlap-quality pass, if you are recommending more work.
8. If you recommend stopping, explain exactly why the current evidence is insufficient to justify another search-shape change.
9. Identify concrete code touch points by file and symbol.
10. Define the smallest ablation and benchmark plan needed to validate or reject the idea.
11. State the exact measurements that would cause you to reject the intervention before or after implementation.

Be concrete, repo-specific, and hostile to fake progress. If the evidence does not justify another change, say so explicitly.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 2 as a critical second pass.

I want you to pressure-test whether your density/search-shape recommendation actually improves useful compiled coverage rather than merely gaming one metric.

Tasks:
1. Identify where your proposal may be gaming candidate density or invalid-slot rate without improving decision-grade coverage.
2. Identify where your proposal may be overfitting the current `entry_past_cutoff` signal and underweighting overlap or contract-eligibility limits.
3. Identify where your proposal may be overfitting the current SPY sign-off slice and underweighting the bounded QQQ fragility control.
4. If targeted current web research would materially improve the answer, use it selectively to study:
   - strike-window design tradeoffs for compiled option-search systems
   - practical moneyness-grid design under sparse contract availability
   - late-entry policy tradeoffs when density and realism conflict
5. Compare your proposal against:
   - keeping the current defaults unchanged
   - a conservative default-tightening alternative
   - a more aggressive tuple-grid or axis-redesign alternative
6. Decide which direction should actually win for this repo now, if any.

Deliverables:
1. `Keep / revise / reject` verdict on your first-pass recommendation
2. The refined winning intervention, or an explicit `stop here` verdict
3. The biggest false-win risk
4. The benchmark signals that would confirm the improvement is real
5. The defer conditions that should stop implementation

Stay repo-specific. Do not broaden this into generic options-search advice.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 2, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_expression_density_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current density, overlap, and invalid-slot evidence summary
3. Final recommendation
4. Explicitly retained current strengths
5. Explicitly deferred ideas
6. Explicitly rejected ideas
7. Explicit `stop here` criteria if the recommendation is no further change
8. Concrete code touch points by file and symbol
9. Ablation plan
10. Benchmark and verification plan
11. Rejection criteria and stop conditions

Do not restate all brainstormed options equally. Make one decision and present one coherent direction for this repo.
```

## Prompt 3: Auto-Backend, Planner, And Product-Promotion Hardening

### Why This Prompt Still Looks Worth Doing

This is the best deep-dive prompt if Prompt 1 concludes that the main remaining work is not candidate construction itself, but safe promotion of the GPU path as a normal product route.

Current repo evidence already says:

- live code now contains more permissive auto-GPU gates than older roadmap text
- the latest canonical workload can pass the live gates
- the benchmark story is still narrow enough that overly aggressive promotion could be misleading
- the docs-vs-code mismatch itself is now a real productization risk
- Prompt 2 is now closed:
  - the split-assignment ablation was executed
  - `session_chronological` was not promoted
  - `event_chronological` remains the default
  - split assignment remains a manual/non-default lever rather than an unresolved default-policy blocker

That makes backend policy and promotion hardening a real research question. A broad feature expansion prompt is not.

### Attach These Files

Required:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `options_research/options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`

Optional:

- `web/src/stores/optionsExpressionStore.ts`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing backend selection and GPU product-promotion policy for the options-expression optimizer in this repo.

Using only the attached files, answer this repo-specific question:

Is there one targeted change to the current auto-backend or planner policy that should be implemented now to make full-GPU execution a safer default for this options lane?

Constraints:
- do not redesign the whole feature
- do not suggest manual-user-toggle-first as the main answer unless the evidence forces it
- do not ignore the current code thresholds or latest local artifact results
- keep CPU fallback behavior intact
- do not reopen split-assignment default policy unless the attached Prompt 2 implementation guide leaves a real unresolved backend-policy dependency
- distinguish clearly between:
  - static threshold tuning
  - benchmark-derived calibration
  - workload classification or routing logic
  - user-facing explanation or diagnostics only
  - ideas that should be rejected

Deliverables:
1. Summarize what current code and attached artifacts already prove about auto-backend behavior.
2. State whether the current policy looks too conservative, too aggressive, or roughly correct.
3. Propose the single best next change, if any.
4. Explain why that change beats at least one simpler threshold tweak and one more ambitious routing redesign.
5. Identify concrete code touch points by file and symbol.
6. Define the smallest benchmark matrix needed to validate or reject the change.
7. State exact conditions that should block product promotion even if the canonical workload looks good.

Be concrete, repo-specific, and decision-oriented.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 3 as a critical second pass.

I want you to pressure-test whether your backend-policy recommendation is actually robust or just overfit to one canonical workload and a fast-moving local threshold story.

Tasks:
1. Identify any places where your recommendation depends too heavily on one canonical workload or on stale roadmap assumptions.
2. Identify any places where your recommendation confuses:
   - planner logic
   - threshold calibration
   - product messaging
   - or real GPU-route economics
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - practical accelerator-promotion policy design
   - routing heuristics for mixed CPU and GPU search systems
   - benchmark-derived gating vs fixed-threshold gating tradeoffs
4. Compare your proposal against:
   - leaving the current policy unchanged
   - one more conservative alternative
   - one more aggressive benchmark-derived alternative
5. Decide which direction should actually win for this repo now, if any.

Deliverables:
1. `Keep / revise / reject` verdict on your first-pass recommendation
2. The refined winning change
3. The highest-risk hidden assumption
4. The promotion-blocking evidence that would stop rollout
5. The exact stop conditions that should prevent implementation

Stay repo-specific. Do not broaden this into generic GPU-routing advice.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 3, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_expression_backend_policy_FINAL.md`

The document must contain:
1. Executive summary
2. Current auto-backend evidence summary
3. Final recommendation
4. Explicitly retained current strengths
5. Explicitly deferred ideas
6. Explicitly rejected ideas
7. Concrete code touch points by file and symbol
8. Benchmark matrix
9. Verification and rollout plan
10. Rejection criteria and stop conditions

Do not restate all brainstormed options equally. Make one decision and present one coherent direction for this repo.
```

## Removed Prompt Categories

These categories were intentionally removed from the broader April 8 list because they are not the best current use of a stronger model if the scope stays limited to full-GPU execution:

- broad structure-family expansion
- new parent-family support
- direct quote-aware provider download expansion
- general options-platform or LEAN-parity growth
- UI, report, or export prompts that do not materially affect full-GPU productization
- ML or meta-ranking speculation detached from the current compiled path

## Suggested Order

- Run Prompt 1 first.
- If Prompt 1 concludes that candidate density, invalid slots, or search-shape are the top unresolved issue, run Prompt 2.
- If Prompt 1 concludes that backend selection, planner logic, or product-promotion hardening are the top unresolved issue, run Prompt 3.
- If Prompt 1 concludes that no major change is justified yet, stop there and treat further prompting as likely churn.
- The second-pass and final-consolidation prompts are follow-ups inside the chosen prompt thread. They are not a serial dependency between Prompt 1, Prompt 2, and Prompt 3.

---

## Scope-Review Addendum

This addendum is for the remaining worthwhile **scope** questions inside the current options-expression lane.

Use these prompts when the question is no longer:

- “what is the next GPU productization fix?”

and is instead:

- “how far should the current options workflow deepen without widening into a general platform?”

These prompts intentionally stay inside the existing product boundary:

- current options optimization stage
- current embedded validation stage
- current unified History follow-up host
- current GPU-first philosophy

They should not be used to reopen:

- new parent strategy families
- new workflow stages
- generic LEAN parity
- broad simulator scope

### Additional Scope Guard

For every prompt in this addendum, keep these rules explicit:

- do not recommend new workflow stages beyond the current optimization + embedded validation lane
- do not recommend new strategy families
- do not recommend generic broker-style options simulation
- do not recommend CPU-only realism ideas that materially erode throughput
- prefer “more useful work in one compiled pass” over feature-count expansion
- keep GPU-first scoring as a product requirement
- keep CPU fallback/oracle behavior intact

### Addendum Review Order

1. quote-aware realism productization
2. optimization-depth expansion inside the current pass
3. embedded validation-depth expansion
4. scenario-family expansion constraints
5. bounded structure-family expansion, only if still justified
6. final cross-prompt consolidation

## Prompt 4: Quote-Aware Realism Productization

### Why This Prompt Still Looks Worth Doing

The repo already supports:

- canonical Polygon/Massive `1m` aggregate download
- quote-aware and derived-mid CSV import
- realism-lane selection and reporting

So the remaining question is no longer “should we support quote-aware realism at all?”

It is:

**how first-class should quote-aware realism become without breaking the performance-first shape of the product?**

The most important unresolved subquestion is whether direct quote-aware Polygon/Massive download is worth building, or whether quote-aware import-only is the right stopping point.

### Attach These Files

Required:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `src-tauri/src/commands_options_data.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

Optional:

- `src-tauri/src/bin/ta_options_dataset_download.rs`
- `src-tauri/src/bin/ta_options_dataset_import.rs`

### Prompt

```text
You are reviewing the remaining quote-aware realism scope decisions in VectorGrid's current options-expression workflow.

Using only the attached files, answer this repo-specific question:

How far should quote-aware realism be productized inside the existing options optimization + embedded validation lane, without eroding the GPU-first performance posture or turning the system into a quote replay simulator?

Constraints:
- Stay inside the current options workflow. Do not add new workflow stages.
- Do not suggest a generic options platform.
- Do not weaken the aggregate `1m` lane as the canonical hot path.
- Treat GPU-first scoring as a product requirement.
- Keep CPU fallback/oracle behavior unchanged.
- Distinguish clearly between:
  - aggregate lane
  - derived-mid lane
  - quote-aware lane
- Explicitly decide whether direct quote-aware Polygon/Massive download should be adopted now, deferred, or rejected.

Deliverables:
1. Summarize what the repo already ships for aggregate, derived-mid, and quote-aware realism.
2. Identify what is still genuinely unresolved.
3. Compare:
   - import-only quote-aware realism
   - direct quote-aware provider download
   - keeping quote-aware realism intentionally secondary
4. State the winning direction for this repo now.
5. Explain what should remain unchanged.
6. Identify concrete code touch points by file.
7. Give acceptance gates for adopting any additional quote-aware productization work.

Be repo-specific and decision-oriented.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 4 as a critical second pass.

I want you to pressure-test the quote-aware realism recommendation you just made for this repo.

Tasks:
1. Identify weak assumptions, vague claims, and places where your recommendation sounds plausible but is not yet well defended by the attached repo evidence.
2. Identify any ways your proposal may be over-valuing realism over actual decision quality or throughput.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - Polygon/Massive options quotes vs minute-aggregate economics
   - practical storage and workflow implications of quote-native options research
4. Compare your proposal against:
   - a more conservative aggregate-first stance
   - a more ambitious quote-aware-first stance
5. Decide which direction should actually win now.

Deliverables:
1. `Keep / revise / reject` verdict on the first-pass recommendation
2. Refined direction
3. Strongest reason to defer direct quote-aware provider download
4. Strongest reason to adopt it anyway
5. Exact conditions that would make the repo defer or reject the work
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 4, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/quote_aware_realism_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current shipped realism surface
3. Final recommendation
4. What remains canonical and unchanged
5. Deferred ideas
6. Rejected ideas
7. Concrete code touch points by file
8. Validation / verification order
9. Adoption gates
10. Stop conditions

Make one coherent decision for this repo.
```

## Prompt 5: Optimization-Depth Expansion Inside The Current Pass

### Why This Prompt Still Looks Worth Doing

This repo’s conventional optimizer already gets substantial value from computing many useful metrics at near-constant marginal cost.

The analogous open question for options is:

**what additional useful optimization metrics, disagreement signals, or low-cost summaries should be added inside the current options pass without materially changing hot-path cost?**

This is the most direct way to make the options optimizer more decision-useful without widening scope.

Current Prompt 5 starting truth:

- Prompt 4 is closed and should not be reopened here.
- Aggregate remains the canonical provider-download lane.
- Direct quote-aware Polygon/Massive download remains deferred.
- `DerivedMidSnapshot` remains first-class in product posture but aggregate-style in execution.
- `TopOfBookQuote` remains the only quote-side execution lane.

If the question is specifically about mathematically justified single-pass metric expansion from the current GPU/aggregate state, prefer Prompt 9. Prompt 5 is for the broader product/reporting layer that still stays cheap.

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`

Optional:

- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing how to deepen the current options optimization stage in VectorGrid without widening scope or materially hurting performance.

Using only the attached files, answer this repo-specific question:

What additional low-cost metrics, disagreement signals, or optimization-depth reporting should this repo add inside the current options optimization pass so the result becomes more decision-useful without becoming slower or noisier?

Constraints:
- Stay inside the existing options optimization stage.
- Do not add a new workflow stage.
- Do not add new strategy families.
- Keep the GPU scorer shape intact.
- Do not recommend additions that depend on:
  - direct quote-aware provider download
  - derived-mid becoming quote-side
  - new realism lanes
- Prefer “more useful work in one pass” over “more output for its own sake.”
- Distinguish clearly between:
  - metrics worth adding now
  - metrics worth deferring
  - metrics that should be rejected as low-ROI

Deliverables:
1. Summarize what optimization-depth surfaces the repo already ships.
2. Identify what is still genuinely missing.
3. Propose the best low-cost additions, if any.
4. Explain why those additions are likely near-constant-cost enough for this repo.
5. Identify what should remain unchanged.
6. Give concrete code touch points by file.
7. Define verification and rejection criteria.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 5 as a critical second pass.

Tasks:
1. Identify weak assumptions, vague claims, and places where your recommendation sounds more attractive than justified.
2. Identify any ways your proposal may quietly add:
   - hot-path branching
   - extra tensor width
   - extra CPU exactification
   - or reporting complexity with little decision value
3. Compare your proposal against:
   - doing nothing
   - one minimal addition
   - one more aggressive addition
4. Collapse the answer toward one winning direction.

Deliverables:
1. `Keep / revise / reject` verdict on the first-pass recommendation
2. Refined winning direction
3. Top performance risks
4. Top trust/interpretability benefits
5. Exact conditions that would make the repo defer the work
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 5, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_optimization_depth_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current shipped optimization-depth surface
3. Final recommendation
4. What remains unchanged
5. Deferred ideas
6. Rejected ideas
7. Concrete code touch points by file
8. Verification order
9. Cost-risk checklist
10. Stop conditions
```

## Prompt 6: Embedded Validation-Depth Expansion

### Why This Prompt Still Looks Worth Doing

The repo already has real options holdout, sealed, and stress validation.

So the remaining question is:

**how much further should embedded validation be surfaced and deepened before it becomes diminishing returns or a disguised new workflow stage?**

This is about stronger decision-grade validation inside the current host, not creating “options validation mode.”

If the question is specifically about mathematically justified derived validation statistics from the current lane outputs, prefer Prompt 11. Prompt 6 is for deciding how much more validation depth should be surfaced as product behavior inside the current host.

Current Prompt 6 starting truth:

- Prompt 5 is now closed via:
  - [options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md)
- compact retained leader/disagreement summary is already present on artifact list, compare, and reopened summary surfaces
- Prompt 6 should not spend its main answer re-solving that shipped summary-coherence gap
- the remaining valid question is what embedded validation still needs beyond the shipped holdout / sealed / stress lanes and the shipped Prompt 5 summary baseline

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing how to deepen embedded validation inside VectorGrid's current options-expression workflow.

Using only the attached files, answer this repo-specific question:

How much further should the repo expand embedded options validation surfaces before it becomes a new workflow stage in disguise or starts diluting the performance-first product posture?

Constraints:
- Stay inside the current options optimization + embedded validation workflow.
- Do not propose a separate validation mode.
- Do not add new parent strategy families.
- Focus on decision-grade validation depth, not dashboard sprawl.
- Do not spend your main recommendation re-solving compact artifact-summary coherence that Prompt 5 already implemented.
- Force yourself to analyze:
  - shortlist and finalist-budget economics (`top_k`, `sealed_finalist_count`)
  - validation adequacy / sufficiency cues (event count, trade count, split coverage)
  - what embedded validation still needs now that compact tuple-coherent artifact-summary surfacing already exists
- Distinguish clearly between:
  - validation improvements worth adding now
  - validation improvements worth deferring
  - validation surfaces that should be rejected

Deliverables:
1. Summarize what validation depth the repo already ships.
2. Identify what is still genuinely missing for decision-grade use.
3. Propose the best next validation-depth additions, if any.
4. Explicitly state whether the current shortlist / finalist-budget policy can hide robust winners outside the retained tuples, and if so, whether that should change.
5. Explain where the repo should stop before this becomes a separate product branch.
6. Identify whether any remaining artifact/frontier validation summary should deepen beyond the currently shipped Prompt 5 tuple-coherent baseline.
7. Identify concrete code touch points by file.
8. Define acceptance and rejection criteria.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 6 as a critical second pass.

Tasks:
1. Identify weak assumptions, vague claims, and places where your recommendation overstates what users still need.
2. Identify any ways your proposal may quietly create:
   - a new validation stage
   - duplicated artifact surfaces
   - excessive comparison complexity
   - or slow, low-value reporting work
3. Compare:
   - minimal validation-depth improvement
   - recommended validation-depth improvement
   - over-built validation expansion
4. Decide which path should actually win now.

Deliverables:
1. `Keep / revise / reject` verdict
2. Refined direction
3. The clearest “stop here” boundary
4. Implementation risks
5. Exact conditions that would make the repo defer the work
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 6, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_validation_depth_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current shipped validation surface
3. Final recommendation
4. What remains unchanged
5. Deferred ideas
6. Rejected ideas
7. Concrete code touch points by file
8. Verification order
9. Stop boundary
10. Adoption gates
```

## Prompt 7: Stress-Family Expansion Constraints

### Why This Prompt Still Looks Worth Doing

Baseline, conservative, harsher, and extreme already exist.

That means the remaining question is not “should we add stress testing?”

It is:

**is there any additional scenario-family breadth still worth adding, or is the current set already the right stopping point?**

This prompt is valuable because it can explicitly close the door on low-ROI stress-lane sprawl if the current set is already sufficient.

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`

Optional:

### Prompt

```text
You are reviewing whether VectorGrid's current options stress-family surface should expand any further.

Using only the attached files, answer this repo-specific question:

Given that the repo already ships baseline, conservative, harsher, and extreme stress lanes, is any additional stress-family expansion still justified inside the current options workflow?

Constraints:
- Stay inside the current options lane.
- Do not suggest generic scenario proliferation.
- Prefer closing scope over adding complexity unless there is clear decision value.
- Treat tensor/reporting complexity as a real cost.

Deliverables:
1. Summarize what the repo already ships.
2. State whether the current stress-family set is probably sufficient.
3. Identify the best additional stress-family idea, if any.
4. Explain why it should be adopted, deferred, or rejected.
5. Identify what should remain unchanged.
6. Give concrete code touch points by file.
7. Define stop conditions.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 7 as a critical second pass.

Tasks:
1. Identify whether your first-pass answer overvalued additional scenario breadth.
2. Identify the strongest case for simply freezing the current stress-family set.
3. Compare:
   - freeze now
   - one bounded addition
   - broader expansion
4. Decide what should actually win now.

Deliverables:
1. `Keep / revise / reject` verdict
2. Final stress-family scope recommendation
3. Strongest reason not to expand
4. Exact conditions that would justify reopening the question later
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 7, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_stress_scope_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current shipped stress-family surface
3. Final recommendation
4. Deferred ideas
5. Rejected ideas
6. Concrete code touch points by file
7. Verification order
8. Reopen criteria
```

## Prompt 8: Bounded Structure-Family Expansion Or Freeze

### Why This Prompt Still Looks Worth Doing

The repo already ships:

- `long_single`
- `debit_vertical`
- `long_pair`

The question now is not whether multi-leg support exists at all.

It is:

**should the repo add any further bounded fixed-geometry structures, or is the current set the right place to freeze breadth and focus on depth?**

This prompt should be treated as lower priority than the previous prompts.

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`

Optional:

- `web/src/stores/optionsExpressionStore.ts`

### Prompt

```text
You are reviewing whether VectorGrid should expand its current bounded options structure-family surface.

Using only the attached files, answer this repo-specific question:

Given that the repo already supports `long_single`, `debit_vertical`, and `long_pair`, should any further fixed-geometry structure families be added now, or should the current product freeze breadth and keep focusing on optimization and validation depth?

Constraints:
- Stay inside fixed-geometry, same-expiry structures.
- Do not propose rolling, assignment, or generic multi-leg scripting.
- Treat performance and tensor density as first-class constraints.
- Prefer freezing scope unless the additional family has clear decision value.

Deliverables:
1. Summarize what the repo already ships.
2. State whether breadth should freeze now.
3. If one additional family is still justified, identify it.
4. Explain why it should be adopted, deferred, or rejected.
5. Identify concrete code touch points by file.
6. Define acceptance and rejection criteria.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 8 as a critical second pass.

Tasks:
1. Identify any ways your proposal quietly drifts toward generic options strategy breadth.
2. Identify the strongest case for freezing the current structure-family set.
3. Compare:
   - freeze now
   - one bounded addition
   - broader bounded-family expansion
4. Decide what should actually win now.

Deliverables:
1. `Keep / revise / reject` verdict
2. Final structure-family scope recommendation
3. Strongest reason not to expand
4. Exact conditions that would justify reopening the question later
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 8, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_structure_scope_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current shipped structure-family surface
3. Final recommendation
4. Deferred ideas
5. Rejected ideas
6. Concrete code touch points by file
7. Verification order
8. Reopen criteria
```

## Math-Heavy Prompt Addendum

Use this addendum when you specifically want GPT-5.4 Pro to lean into higher-level math rather than only product/architecture judgment.

These prompts are meant to answer questions like:

- what additional useful work can be computed from the current pass with near-constant marginal cost?
- what extra decision-grade statistics can be recovered from low-dimensional aggregates rather than extra backtests?
- can the current ranking and validation math be improved without widening the runtime shape?

These prompts should still obey the same repo rules:

- no generic quant brainstorming
- no new workflow stages
- no new strategy families
- no generic simulator expansion
- no proposals that require replaying raw paths in the hot loop just to get a prettier statistic

## Math-Heavy Prompt Discipline

For Prompts 9-11, prefer answers that are mathematically explicit rather than verbally clever.

The best answers should usually do most of the following:

- write proposed metrics or ranking rules as explicit formulas, not only prose
- state the implied per-tuple state vector or accumulator set
- say whether the proposal is:
  - exact from current state
  - exact with a few extra mergeable accumulators
  - approximate from a bounded sketch
  - or not realistically compatible with the current pass
- state whether the proposed state is mergeable / associative enough for the current GPU-first compiled evaluation shape
- estimate whether the proposal changes tuple-output width, device-memory pressure, or CPU exactification pressure
- separate:
  - mathematically elegant ideas
  - mathematically admissible ideas
  - actually worth-implementing ideas

Relevant math lenses that are welcome when they stay repo-grounded:

- finite sufficient statistics and mergeable accumulator design
- first/second moments, downside moments, and partial expectations
- bounded quantile or histogram sketches only if the state budget stays very small
- empirical-Bayes or shrinkage-style estimators
- lower-confidence-bound or uncertainty-penalized ranking
- monotone stress-deterioration penalties
- regret, dominance, skyline, or frontier geometry from already-computed lanes

What to avoid:

- asymptotic-statistics showmanship that ignores the current state budget
- proposals that need pathwise replay but hide that fact behind notation
- mathematically pretty ranking schemes that users will not be able to reason about
- sketch-heavy ideas that silently widen the hot-path device state too much

## Prompt 9: Single-Pass Sufficient Statistics Expansion

### Why This Prompt Still Looks Worth Doing

The current options metrics surface is still relatively compact:

- `valid_trades`
- `invalid_events`
- `invalid_rate`
- `expectancy_per_trade`
- `expectancy_per_premium`
- `median_return_on_premium`
- `win_rate`
- `total_pnl_per_contract`

See [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).

That leaves a mathematically interesting question:

**what additional decision-useful statistics can be computed from a small set of per-trade sufficient statistics or cheap aggregate states, without materially changing the pass cost or forcing replay-heavy post-processing?**

This prompt matters even more because the current GPU tuple aggregate is thinner than the CPU exact metric path:

- `compute_metrics(...)` can produce the full `OptionsExpressionMetrics`
- `OptionsGpuTupleAggregate` currently retains only `valid_trades`, `wins`, `total_pnl`, and `total_premium`
- `metrics_from_aggregate(...)` currently reconstructs a reduced metric surface and necessarily zeros `median_return_on_premium`

So this prompt should pressure-test whether the next worthwhile CUDA-first improvement is a richer small aggregate state rather than more CPU exactification.

This is the closest options analogue to the earlier conventional backtest expansion from `10` to `17` and `22` metric rows at low marginal cost.

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`

Optional:

- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`

### Prompt

```text
You are reviewing the options-expression optimizer in this repo as a single-pass sufficient-statistics design problem.

Using only the attached files, answer this repo-specific question:

Given the current tuple metrics and compiled-tensor evaluation flow, what additional useful metrics could this repo compute at near-constant marginal cost if it were willing to maintain a slightly richer aggregate state per tuple, but not willing to add new replay-heavy passes?

Constraints:
- Stay inside the current options optimization + embedded validation workflow.
- Do not add new workflow stages.
- Do not propose metrics that require full path replay in the hot loop.
- Prefer algebraically decomposable or small-state statistics.
- Keep GPU-first execution as the target.
- When useful, reason in terms of explicit accumulator families such as counts, first/second moments, positive/negative-part sums, downside moments, bounded histograms, or other mergeable small-state summaries.
- For each serious candidate metric, explicitly say whether it is usable:
  - in broad pre-`top_k` GPU ranking
  - only after exactification on retained tuples
  - or only for sealed finalists
- Distinguish clearly between:
  - truly single-pass or aggregate-friendly metrics
  - metrics that need small extra state but are still plausible
  - metrics that look attractive but should be rejected because they are not cheap enough

Deliverables:
1. Summarize what the repo already computes, explicitly comparing `compute_metrics(...)`, `OptionsGpuTupleAggregate`, and `metrics_from_aggregate(...)`, and state what aggregate state that already implies.
2. Identify the best additional metrics that are mathematically compatible with the current pass shape.
3. For each candidate metric, classify it as:
   - free from current state
   - cheap with one or two extra accumulators
   - too expensive / wrong shape
4. For the metrics you keep alive, state whether they are exact, approximate, or sketch-based, and give the minimal accumulator/state vector they require.
5. Quantify the extra per-tuple per-lane state budget and say whether the change widens `OptionsGpuTupleAggregate`, `OptionsExpressionMetrics`, or both.
6. State explicitly what this repo should do about `median_return_on_premium`: keep it exactified-only, approximate it with a bounded sketch, or reduce its importance.
7. Propose the single best expansion set for this repo now.
8. Explain why it is likely near-constant-cost enough to be worth it.
9. Identify concrete code touch points by file and symbol.
10. Define rejection criteria if the metric set turns out to widen tuple state or GPU output too much.

Think of this as a sufficient-statistics and low-dimensional-aggregate design problem, not a generic feature ideation exercise.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 9 as a critical mathematical second pass.

Tasks:
1. Identify where your proposed metric set may secretly depend on:
   - order-sensitive path information
   - full replay
   - nonlinear statistics that are not actually decomposable
   - or extra state that is too large for the current compiled-tensor shape
2. Identify which of your proposed metrics are truly derived from small sufficient statistics and which are not.
3. Compare:
   - the most conservative metric expansion
   - your recommended metric expansion
   - one more ambitious but riskier expansion
4. Collapse the answer toward one winning metric set.

Deliverables:
1. `Keep / revise / reject` verdict
2. Refined winning metric set
3. The strongest false-cheap metric in your first answer
4. The most important extra accumulator(s), if any
5. Exact conditions that would make the repo stop before implementation
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 9, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_single_pass_metrics_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current metrics and implied aggregate state
3. Final recommended metric expansion
4. Required accumulator/state vector for the adopted metrics
5. Metrics that should remain unchanged
6. Deferred metric ideas
7. Rejected metric ideas
8. Concrete code touch points by file and symbol
9. Verification order
10. Cost-risk checklist
11. Stop conditions
```

## Prompt 10: Robust Ranking Functional And Scalarization Math

### Why This Prompt Still Looks Worth Doing

The current repo already has a concrete scalar robust-ranking function in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs):

- coverage term from `1 - invalid_rate`
- weighted combination of:
  - search expectancy-per-premium
  - holdout expectancy-per-premium
  - stress expectancy-per-premium
  - harsher stress expectancy-per-premium
  - extreme stress expectancy-per-premium

That creates a strong math-heavy question:

**is the current scalarization mathematically the right one, or is there a better monotone, shrinkage-aware, or dominance-aware ranking functional that still fits the current pass and artifact shape?**

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`

Optional:

- `web/src/stores/optionsExpressionStore.ts`

### Prompt

```text
You are reviewing the options-expression optimizer in this repo as a ranking-functional design problem.

Using only the attached files, answer this repo-specific question:

Is the current scalar robust-ranking function mathematically the best fit for this repo, or should it be replaced by a better low-cost scalarization that still respects the current GPU-first compiled evaluation shape?

Constraints:
- Stay inside scalar or very low-dimensional ranking math that fits the current artifact and runtime model.
- Do not suggest full Pareto workflow expansion.
- Do not add new workflow stages.
- Keep sealed metrics out of first-pass tuple ranking unless the attached files justify changing that.
- Prefer formulations that can be computed from already-available metrics or very small additional state.
- When useful, explicitly consider shrinkage, lower-confidence-bound, coverage-aware, and monotone stress-penalized scalarizations.
- Treat these as separate decision points unless the attached files prove otherwise:
  - broad pre-`top_k` ranking
  - exactified reranking on retained tuples
  - sealed-finalist ordering
- Explicitly analyze the current missing-lane fallback rules and the fact that `robust_score` uses only search-lane coverage in its current coverage term.
- Distinguish clearly between:
  - ranking function improvements that are plausible now
  - mathematically elegant but product-wrong ideas
  - ideas that require too much new state or workflow complexity

Deliverables:
1. Summarize the current ranking math and its likely strengths/weaknesses.
2. Compare the current scalarization against at least:
   - one conservative alternative
   - one shrinkage-aware or stability-aware alternative
   - one more ambitious dominance-aware alternative
3. Write the leading candidate functionals explicitly enough that an engineer could implement them without inventing the math, including any normalization, penalties, or shrinkage terms.
4. State the single best ranking functional for this repo now.
5. Explain why it is better than the current one without widening the product.
6. State whether the same functional should govern all three ranking/ordering points or whether the repo should deliberately keep them different.
7. Identify exactly which existing metrics it consumes and whether any extra state is required.
8. State whether any shrinkage, confidence, or dispersion term can be built from current `valid_trades`, `invalid_events`, and `invalid_rate`, or whether Prompt 9-style extra state would be required first.
9. Identify concrete code touch points by file and symbol.
10. Define verification and regression criteria.
11. State explicit reasons to reject a replacement even if it looks mathematically cleaner.

Treat this as a scalarization and robustness-math problem grounded in this repo, not a generic portfolio-theory exercise.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 10 as a critical mathematical second pass.

Tasks:
1. Identify where your ranking-functional recommendation may be overfitting elegance rather than product usefulness.
2. Identify any hidden assumptions about:
   - sample size
   - stability
   - calibration across lanes
   - monotonicity
   - and comparability of stressed vs unstressed metrics
3. Compare:
   - keep current robust score
   - minimally revise current robust score
   - replace it with your recommended new functional
4. Collapse the answer toward one winning direction.

Deliverables:
1. `Keep / revise / reject` verdict
2. Refined winning functional
3. The strongest mathematical objection to it
4. The strongest product argument in its favor
5. Exact evidence that would stop the repo from changing the ranking math
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 10, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_ranking_functional_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current ranking math
3. Final recommended functional
4. Required inputs / state for that functional
5. What remains unchanged
6. Deferred alternatives
7. Rejected alternatives
8. Concrete code touch points by file and symbol
9. Verification order
10. Regression risks
11. Stop conditions
```

## Prompt 11: Validation Frontier And Confidence Math Without Extra Stages

### Why This Prompt Still Looks Worth Doing

The repo already computes:

- selection metrics
- holdout metrics
- sealed metrics
- multiple stress lanes
- validation verdicts

That creates another useful math-heavy question:

**what additional confidence, stability, or validation-frontier math can be derived from the current split/stress outputs without turning validation into another heavyweight execution stage?**

This prompt is about extracting more decision quality from the validation math already present.

It should also explicitly test whether the next useful derived statistics are cross-lane frontier / regret geometry signals rather than more dashboard-style metrics:

- search winner vs validated winner gaps
- search-to-holdout or holdout-to-sealed degradation
- monotone stress deterioration flags
- dominance / non-dominance summaries across search, holdout, sealed, and stressed lanes

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing the options-expression workflow in this repo as a validation-frontier and confidence-math problem.

Using only the attached files, answer this repo-specific question:

What additional confidence, stability, or validation-frontier statistics could this repo derive from the metrics it already computes across search, holdout, sealed, and stress lanes, without creating another heavyweight execution stage?

Constraints:
- Stay inside the current options optimization + embedded validation workflow.
- Do not add a new validation stage.
- Prefer statistics that can be derived from existing lane metrics or very small additional aggregate state.
- Do not suggest heavyweight bootstrap or resampling unless you can justify it inside the current runtime philosophy.
- Force yourself to analyze:
  - adequacy / sufficiency signals from split event counts, valid-trade counts, and coverage
  - whether shortlist and finalist-budget policy distort the frontier the user actually sees
  - whether derived statistics should stay tuple-coherent rather than mixing per-lane leaders from different tuples
- When useful, explicitly consider regret vectors, dominance tags, shrinkage between lanes, lower-confidence-bound style acceptance math, and monotone stress-deterioration summaries.
- Ground the answer against the current `inferred_validation_verdict()` / `is_stress_fragile()` semantics and say whether your proposal complements them or changes them.
- For each serious candidate statistic, say whether it is trustworthy:
  - globally across all tuples
  - only within retained `top_k`
  - or only within sealed finalists
- Distinguish clearly between:
  - useful derived validation statistics
  - statistics that need small extra state and may still be worth it
  - statistics that should be rejected as too costly or misleading

Deliverables:
1. Summarize what validation metrics and lanes the repo already has.
2. Identify the best mathematically defensible derived validation statistics for this repo now.
3. Compare:
   - one conservative confidence/stability addition
   - one recommended frontier-style or regret-style addition
   - one more ambitious but likely-too-costly addition
4. For the finalists you recommend, say whether they are zero-extra-state, require a few extra accumulators, or are only possible with a heavier runtime change.
5. State the winning direction.
6. Explain why it improves decision quality without widening the workflow.
7. State whether each adopted statistic belongs on `OptionsExpressionTupleResult`, in verdict derivation, in `OptionsExpressionArtifactSummary`, or only in frontend-derived reporting.
8. Identify concrete code touch points by file and symbol.
9. Define verification and rejection criteria.

Treat this as a confidence and frontier-math problem grounded in the repo's current validation surfaces, not as generic statistical wishlist building.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 11 as a critical mathematical second pass.

Tasks:
1. Identify where your proposed validation statistics may sound rigorous but may actually be unstable, low-sample, or poorly calibrated for this repo.
2. Identify any hidden dependence on:
   - additional reruns
   - pathwise data that the repo does not currently retain cheaply
   - or acceptance rules that become too opaque for users
3. Compare:
   - minimal derived-stat addition
   - your recommended validation-math addition
   - a more ambitious but likely-overbuilt alternative
4. Collapse the answer toward one winning direction.

Deliverables:
1. `Keep / revise / reject` verdict
2. Refined winning validation-math addition
3. The strongest risk of false rigor
4. The clearest product upside
5. Exact conditions that would make the repo stop before implementation
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 11, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_validation_math_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current validation frontier surface
3. Final recommended derived validation statistics
4. Zero-extra-state vs extra-state classification
5. What remains unchanged
6. Deferred ideas
7. Rejected ideas
8. Concrete code touch points by file and symbol
9. Verification order
10. Interpretability risks
11. Stop conditions
```

## Prompt 12: GPU-Resident Finalist Exactification And Embedded Validation Boundary

### Why This Prompt Still Looks Worth Doing

The current options GPU route is not "GPU everywhere."

Today the repo already does all of the following on GPU for the broad tuple set:

- search-lane ranking aggregates
- holdout-lane aggregates
- conservative stress aggregates
- harsher stress aggregates

But after that it still truncates to `top_k` and calls CPU-side finalist replay:

- `finalize_ranked_results(...)`
- `exactify_ranked_results(...)`
- `apply_sealed_finalist_stage(...)`
- exact `extreme` stress evaluation

That makes the current unresolved CUDA-first question very concrete:

**what is the single best next way to reduce the remaining CPU finalist-validation boundary without widening scope or damaging correctness?**

This prompt is intentionally narrower than Prompt 1. Use it when the repo-specific question is the hybrid boundary itself.

### Attach These Files

Required:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json`

Optional:

- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing the current GPU/CPU execution boundary inside VectorGrid's options-expression optimizer and embedded validation flow.

Using only the attached files, answer this repo-specific question:

Given the current hybrid route, what is the single best next CUDA-first intervention to reduce CPU finalist exactification or embedded validation replay without widening scope or weakening correctness?

The attached code already implies this current boundary:

- broad tuple ranking across search / holdout / stress / harsher lanes is GPU-accelerated
- the repo then truncates to top_k
- CPU exactification and sealed finalist work still happen through finalist replay
- extreme stress is still exact CPU-side evaluation

Constraints:
- stay inside the current options optimization + embedded validation workflow
- do not propose a new workflow stage
- do not broaden parent families or structure families
- do not suggest raw full-path replay on GPU unless the attached files make that obviously defensible
- keep CPU fallback and correctness-oracle behavior intact
- preserve the current artifact-driven workflow
- explicitly analyze whether the current `top_k` and `sealed_finalist_count` are already the right containment boundary for CPU replay work
- distinguish clearly between:
  - a GPU-boundary improvement worth implementing now
  - a change worth deferring
  - an aggressive full-device idea that should be rejected

Deliverables:
1. Summarize the current GPU vs CPU responsibility split by file and symbol.
2. Identify the largest remaining CPU finalist-validation work after GPU ranking and explain why it still exists.
3. Compare at least:
   - keep the current hybrid boundary
   - one targeted GPU-boundary reduction
   - one more aggressive near-full-device finalist-validation design
4. State the single best direction for this repo now.
5. Explain why it improves CUDA usage and performance without creating semantic drift or bloating device state.
6. Identify concrete code touch points by file and symbol.
7. Define the smallest benchmark, parity, and regression plan needed before and after implementation.
8. State exact stop conditions that would cause the repo to keep the current hybrid boundary.

Be repo-specific. Treat this as a current-runtime-boundary problem, not as generic CUDA ideation.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 12 as a critical second pass.

I want you to pressure-test whether your proposed CUDA-first boundary change actually helps the repo rather than merely moving work around.

Tasks:
1. Identify where your recommendation may overstate the cost of the current CPU finalist stage or understate the complexity of GPU-side parity.
2. Identify any hidden dependence on:
   - richer per-tuple device output state
   - extra kernel launches
   - memory-pressure growth
   - approximate metrics that stop matching CPU exactification semantics
   - or sealed/finalist logic that becomes harder to reason about
3. Compare:
   - keep current hybrid boundary
   - your recommended targeted GPU-boundary reduction
   - a more aggressive full-device finalist-validation alternative
4. Decide which direction should actually win for this repo now.

Deliverables:
1. `Keep / revise / reject` verdict on your first-pass recommendation
2. Refined winning direction
3. The strongest semantic-parity objection
4. The strongest CUDA/product argument in favor
5. Exact evidence that would make the repo stop before implementation

Stay repo-specific. Do not turn this into a generic GPU-everything argument.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 12, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_gpu_finalist_boundary_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current GPU vs CPU boundary
3. Final recommendation
4. What remains unchanged
5. Deferred ideas
6. Rejected ideas
7. Concrete code touch points by file and symbol
8. Benchmark and parity plan
9. Regression risks
10. Stop conditions

Make one coherent decision for this repo.
```

## Prompt 13: End-To-End Cost Decomposition And Replay/Prepare Amortization

### Why This Prompt Still Looks Worth Doing

Once the GPU scorer is already viable, raw scorer throughput is no longer the only thing that matters.

The shipped repo now exposes timing for:

- replay/build input work
- tuple / candidate / summary preparation
- GPU tensor scoring
- GPU exactify and sealed-stage post-pass
- total preview wall time

That creates a more practical performance question:

**where does the next performance dollar actually belong now: replay build, preparation/cache amortization, lane duplication, GPU runtime micro-architecture, or nowhere meaningful yet?**

This prompt is valuable because it can explicitly reject "more CUDA work" if the measured bottleneck is still elsewhere, or identify the exact non-scorer runtime surface that deserves optimization next.

### Attach These Files

Required:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json`

Optional:

- `src-tauri/src/commands_options_data.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing the next performance direction for VectorGrid's current options-expression workflow as an end-to-end cost-decomposition problem.

Using only the attached files, answer this repo-specific question:

Now that the compiled-tensor GPU scorer is already viable, where does the next performance dollar actually belong if the repo wants faster real previews/runs rather than just faster scorer microbenchmarks?

Force yourself to decompose cost across at least these buckets:

- replay/build input work
- candidate and summary preparation / cache amortization
- baseline/conservative/harsher lane duplication
- GPU runtime internals (chunking, uploads, output copies, kernel count)
- finalist exactification / sealed post-pass

Constraints:
- stay inside the current options optimization + embedded validation workflow
- do not suggest broad feature expansion
- do not assume more CUDA automatically means better end-to-end performance
- keep CPU fallback correctness intact
- use the attached timing artifacts rather than generic accelerator intuition
- distinguish clearly between:
  - one implement-now performance direction
  - one plausible but defer-worthy direction
  - one direction that should be rejected as fake progress

Deliverables:
1. Summarize what the attached evidence already proves about cold vs warm cost, replay/build cost, preparation cost, and GPU scorer cost.
2. State where the next performance dollar should actually go now.
3. Compare at least:
   - replay/prepare amortization or caching work
   - GPU runtime micro-architecture work
   - "do nothing major yet"
4. Explain why the winning direction beats the others for this repo now.
5. Identify concrete code touch points by file and symbol.
6. Define the smallest benchmark and verification plan needed to validate or reject the recommendation.
7. State exact measurements that would cause the repo to stop before implementation.

Be repo-specific and hostile to fake performance wins.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 13 as a critical second pass.

Tasks:
1. Identify where your recommendation may be overfitting cold-preview evidence or underweighting warm-cache behavior.
2. Identify any places where you are treating currently separate costs as if they were independently removable when the code suggests they are coupled.
3. Compare:
   - replay/prepare amortization first
   - GPU runtime micro-architecture first
   - keep current implementation and stop
4. Collapse the answer toward one winning direction.

Deliverables:
1. `Keep / revise / reject` verdict on the first-pass recommendation
2. Refined winning direction
3. The strongest alternative explanation for where the time really goes
4. The biggest implementation-risk or fake-win risk
5. Exact evidence that would change the recommendation

Stay repo-specific. Do not turn this into a generic caching lecture.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 13, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_end_to_end_cost_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current end-to-end cost decomposition
3. Final recommendation
4. What remains unchanged
5. Deferred ideas
6. Rejected ideas
7. Concrete code touch points by file and symbol
8. Benchmark order
9. Verification order
10. Stop conditions

Make one coherent decision for this repo.
```

## Prompt 14: CUDA Runtime And SASS-Level Micro-Architecture Audit

### Why This Prompt Still Looks Worth Doing

This prompt exists for one narrow reason:

sometimes the remaining performance question really is inside the CUDA runtime or kernel layer, but sometimes SASS-level curiosity is just a distraction from larger replay/build or CPU-bound costs.

For the current options workflow, that means this prompt should only be used after Prompt 13 if the evidence says the next meaningful bottleneck is actually in:

- kernel launch shape
- tuple chunking
- upload / output-copy strategy
- instruction mix
- memory access pattern
- or PTX/SASS-level code generation quality

If the current end-to-end bottleneck is still replay/build, preparation, or CPU finalist work, this prompt should be able to say “do not spend time on SASS now.”

### Attach These Files

Required:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/options_types.rs`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json`
- `scripts/run_gpu_perf_gate.ps1`

Optional:

- `ARCHITECTURE_NOTES.md`
- `RELEASE.md`

### Prompt

```text
You are reviewing VectorGrid's current options-expression CUDA runtime as a narrow kernel/runtime economics problem.

Using only the attached files, answer this repo-specific question:

Is SASS-level or CUDA-runtime micro-architecture work actually worth doing now for the current options-expression workflow, or should the repo explicitly defer it in favor of larger end-to-end bottlenecks?

You must treat this as a gated decision, not as an excuse to always recommend lower-level GPU tuning.

Force yourself to analyze at least these layers separately:

- end-to-end preview / run wall time
- replay/build and preparation work
- GPU scorer runtime share
- finalist exactify / sealed post-pass share
- CUDA runtime shape:
  - tuple chunking
  - launch count
  - outcome tensor uploads
  - host/device copy behavior
  - output-buffer reuse
- kernel micro-architecture:
  - thread-to-tuple mapping
  - loop structure
  - memory access pattern
  - likely occupancy / latency-hiding limits
  - whether SASS/PTX inspection is likely to change a repo decision

Constraints:
- stay inside the current options optimization + embedded validation workflow
- do not recommend generic CUDA heroics
- do not recommend SASS work unless the attached evidence makes it genuinely high-ROI
- keep CPU fallback and correctness intact
- keep the current artifact-driven workflow intact
- if current evidence is insufficient for a SASS-level decision, say so explicitly
- distinguish clearly between:
  - implement-now runtime work
  - measure-first profiling work
  - reject/defer SASS-level work

Deliverables:
1. Summarize what the attached evidence already proves about end-to-end cost share versus pure CUDA scorer cost.
2. State whether SASS-level or lower-level CUDA-runtime work is justified now.
3. Compare at least:
   - do nothing at the SASS layer
   - runtime-structure work without SASS (chunking, uploads, copies, launch shape)
   - true SASS/PTX-level investigation
4. State the single best direction for this repo now.
5. Explain why it beats the other two options.
6. Identify concrete code touch points by file and symbol.
7. Define the exact profiling evidence required before any SASS-level implementation work should begin.
8. State explicit stop conditions that would make the repo reject SASS-level work for the current options lane.

Be repo-specific and skeptical. Treat “assembly-level tuning” as guilty until proven valuable.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 14 as a critical second pass.

Tasks:
1. Identify where your recommendation may be overreacting to microbenchmark-style CUDA numbers while underweighting end-to-end product timings.
2. Identify any ways your answer may be confusing:
   - runtime-structure tuning
   - profiler-guided measurement
   - and true SASS-level hand optimization
3. Compare:
   - defer SASS entirely
   - measure-first with Nsight / profiler evidence but no implementation yet
   - proceed directly into CUDA-runtime or SASS work
4. Collapse the answer toward one winning direction.

Deliverables:
1. `Keep / revise / reject` verdict on the first-pass recommendation
2. Refined winning direction
3. The strongest anti-SASS argument for this repo right now
4. The strongest pro-SASS argument that could still become valid later
5. Exact evidence that would change the recommendation

Stay repo-specific. Do not answer with generic CUDA folklore.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 14, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_cuda_runtime_sass_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current CUDA/runtime evidence
3. Final recommendation
4. What remains unchanged
5. Deferred ideas
6. Rejected ideas
7. Concrete code touch points by file and symbol
8. Profiling / measurement order
9. Verification order
10. Stop conditions

Make one coherent decision for this repo.
```

## Final Cross-Prompt Consolidation Prompt

Use this only after running the most relevant prompts above.

### Attach These Files

Required:

- the outputs from the prompt threads you actually ran
- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`

### Prompt

```text
You are consolidating multiple GPT-5.4 Pro review threads about the remaining scope decisions in VectorGrid's current options-expression workflow.

Using the attached prompt-thread outputs and current repo docs, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`options_research/options_scope_decisions_FINAL.md`

Constraints:
- Make one coherent repo direction.
- Do not treat all alternatives as equally viable.
- Keep the current product narrow in breadth and deeper in decision quality.
- Keep GPU-first performance as a first-class product constraint.
- Do not recommend new strategy families or new workflow stages.

The document must contain:
1. Executive summary
2. Current shipped state
3. Adopt now
4. Defer
5. Reject
6. What remains unchanged
7. Concrete code touch points by file
8. Staged implementation order
9. Verification order
10. Narrow open questions, if any

The result should read like a real repo decision document, not a brainstorm recap.
```
