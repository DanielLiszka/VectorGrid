# GPT-5.4 Pro Prompt Pack For Options-Expression Research

Last revised: 2026-04-08

Purpose: a trimmed, implementation-oriented prompt pack for handing the remaining worthwhile `options_expression` full-GPU research questions to a stronger model such as GPT-5.4 Pro.

This file intentionally replaces the broader April 8 prompt list. That earlier format still encouraged menu-style brainstorming. The only prompts kept here are the ones most likely to produce a defensible recommendation about what to implement or change next in this repo.

Current repo evidence already says:

- the artifact-driven options-expression workflow is already shipped
- supported parent families already include `DoubleMa`, `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- supported structure families already include `long_single`, `debit_vertical`, and `long_pair`
- the compiled-tensor GPU scorer path is real, benchmarked, and already usable on bounded canonical workloads
- the canonical SPY DTE7 aggregate workload now clears the live auto-GPU gates in code
- the remaining uncertainty is mostly about next-step ROI inside the current full-GPU lane, not about whether the feature exists at all

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
- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/options_parent_adapters.rs`
- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`

Note on file lists:

- `Required` means the minimum file set that should be attached for a grounded answer.
- `Optional` means useful expansion context, not mandatory context.
- Do **not** interpret `Required` as “every conceivably relevant file in the repo.”
- When in doubt, prefer the smaller required set first; only add optional files if the prompt needs more implementation detail.

Recommended shared framing for every thread:

- the current shipped slice already includes saved artifacts, tuple drilldown, event replay, holdout/sealed/stress reporting, and a compiled-tensor GPU scorer
- direct provider download is still bounded to `1m` aggregate datasets; quote-aware and derived-mid realism lanes are currently CSV-import paths
- live code now uses more permissive auto-GPU gates than some older docs; if docs and code disagree, prefer current code plus latest local artifacts
- the latest canonical artifacts show useful GPU viability, but still only middling compiled density and materially non-zero invalid-slot rates
- the dominant invalid reasons in the latest canonical artifact are `entry_past_cutoff` and `no_strike_in_tolerance`

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

Use them when the question is:

- what, if anything, still needs to change in the current full-GPU options productization path?

These prompts overlap heavily.

Treat them as a decision tree, not as independent parallel work:

- run Prompt 1 first
- then run **either** Prompt 2 **or** Prompt 3 if Prompt 1 says one of those lanes is still genuinely unresolved
- do not run Prompt 2 and Prompt 3 as equal-priority first-pass prompts at the same time

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

## Parallel vs Serial Use

### Safe serial default

If you want the lowest-confusion workflow, do this:

1. run Prompt 1
2. if needed, run Prompt 2 or Prompt 3
3. then run one or two relevant scope-depth prompts from Prompts 4-8
4. then run the final cross-prompt consolidation prompt
5. only then make implementation changes

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
- the latest canonical artifact still shows only middling candidate density
- the invalid-slot rate is still materially non-zero
- `entry_past_cutoff` dominates the current invalid breakdown
- `no_strike_in_tolerance` is the secondary recurring invalid reason

That makes density, invalid-slot causes, and search-shape design a real implementation question. Another generic GPU optimization pass is not.

### Attach These Files

Required:

- `output/options_expression_phase05_gpu_gate/2026-04-07T20-41-53/2026-04-07T20-41-53/phase05_summary.json`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `src-tauri/src/commands_options_data.rs`

Optional:

- `web/src/stores/optionsExpressionStore.ts`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`

### Prompt

```text
You are reviewing candidate density and invalid-slot causes in the options-expression optimizer in this repo.

Using only the attached files, answer this repo-specific question:

What is the single best implementable way to improve useful candidate density and search coverage for the current options-expression full-GPU path without weakening realism or broadening product scope?

Constraints:
- do not broaden supported parent families or structure families
- do not weaken realism by simply ignoring invalid cases
- do not recommend raw-chain GPU simulation
- keep compiled-tensor scoring as the hot path
- use the latest invalid-reason evidence rather than generic options intuition
- distinguish clearly between:
  - search-space default changes
  - tuple-grid or axis redesign
  - parent-event filtering
  - dataset curation or lane constraints
  - ideas that should be rejected

Deliverables:
1. Summarize what the attached evidence already proves about candidate density, invalid-slot rate, invalid reasons, and event coverage.
2. Distinguish which invalid reasons are:
   - productively reducible
   - better handled by search-space defaults
   - signals that a profile should be constrained or filtered
   - or not worth "fixing"
3. Propose the single best next intervention.
4. Explain why it is better than at least one narrower default tweak and one broader search-space redesign.
5. Identify concrete code touch points by file and symbol.
6. Define the smallest ablation and benchmark plan needed to validate or reject the idea.
7. State the exact measurements that would cause you to reject the intervention before or after implementation.

Be concrete, repo-specific, and hostile to fake progress.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 2 as a critical second pass.

I want you to pressure-test whether your density/search-shape recommendation actually improves useful compiled coverage rather than merely gaming one metric.

Tasks:
1. Identify where your proposal may be gaming candidate density or invalid-slot rate without improving decision-grade coverage.
2. Identify where your proposal may be overfitting the current `entry_past_cutoff` signal and underweighting other structural limits.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - strike-window design tradeoffs for compiled option-search systems
   - practical moneyness-grid design under sparse contract availability
   - late-entry policy tradeoffs when density and realism conflict
4. Compare your proposal against:
   - a conservative default-tightening alternative
   - a more aggressive tuple-grid or axis-redesign alternative
5. Decide which direction should actually win for this repo now, if any.

Deliverables:
1. `Keep / revise / reject` verdict on your first-pass recommendation
2. The refined winning intervention
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
2. Current density and invalid-slot evidence summary
3. Final recommendation
4. Explicitly retained current strengths
5. Explicitly deferred ideas
6. Explicitly rejected ideas
7. Concrete code touch points by file and symbol
8. Ablation plan
9. Benchmark and verification plan
10. Rejection criteria and stop conditions

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

That makes backend policy and promotion hardening a real research question. A broad feature expansion prompt is not.

### Attach These Files

Required:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
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

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
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

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`

Optional:

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
- Distinguish clearly between:
  - validation improvements worth adding now
  - validation improvements worth deferring
  - validation surfaces that should be rejected

Deliverables:
1. Summarize what validation depth the repo already ships.
2. Identify what is still genuinely missing for decision-grade use.
3. Propose the best next validation-depth additions, if any.
4. Explain where the repo should stop before this becomes a separate product branch.
5. Identify concrete code touch points by file.
6. Define acceptance and rejection criteria.
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
- Distinguish clearly between:
  - truly single-pass or aggregate-friendly metrics
  - metrics that need small extra state but are still plausible
  - metrics that look attractive but should be rejected because they are not cheap enough

Deliverables:
1. Summarize what the repo already computes and what aggregate state that already implies.
2. Identify the best additional metrics that are mathematically compatible with the current pass shape.
3. For each candidate metric, classify it as:
   - free from current state
   - cheap with one or two extra accumulators
   - too expensive / wrong shape
4. Propose the single best expansion set for this repo now.
5. Explain why it is likely near-constant-cost enough to be worth it.
6. Identify concrete code touch points by file and symbol.
7. Define rejection criteria if the metric set turns out to widen tuple state or GPU output too much.

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
4. Metrics that should remain unchanged
5. Deferred metric ideas
6. Rejected metric ideas
7. Concrete code touch points by file and symbol
8. Verification order
9. Cost-risk checklist
10. Stop conditions
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
3. State the single best ranking functional for this repo now.
4. Explain why it is better than the current one without widening the product.
5. Identify concrete code touch points by file and symbol.
6. Define verification and regression criteria.
7. State explicit reasons to reject a replacement even if it looks mathematically cleaner.

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
4. What remains unchanged
5. Deferred alternatives
6. Rejected alternatives
7. Concrete code touch points by file and symbol
8. Verification order
9. Regression risks
10. Stop conditions
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

### Attach These Files

Required:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`

Optional:

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
- Distinguish clearly between:
  - useful derived validation statistics
  - statistics that need small extra state and may still be worth it
  - statistics that should be rejected as too costly or misleading

Deliverables:
1. Summarize what validation metrics and lanes the repo already has.
2. Identify the best mathematically defensible derived validation statistics for this repo now.
3. Compare:
   - one conservative confidence/stability addition
   - one recommended frontier-style addition
   - one more ambitious but likely-too-costly addition
4. State the winning direction.
5. Explain why it improves decision quality without widening the workflow.
6. Identify concrete code touch points by file and symbol.
7. Define verification and rejection criteria.

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
4. What remains unchanged
5. Deferred ideas
6. Rejected ideas
7. Concrete code touch points by file and symbol
8. Verification order
9. Interpretability risks
10. Stop conditions
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
