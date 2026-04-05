# `double_crossover` Research Question Pack For ChatGPT Research Mode / GPT-5.4 Pro

Date: 2026-04-05

Purpose: a repo-owned list of the highest-value long-form research questions still worth asking after the current `double_crossover` / `double_ma` optimization campaign.

This file is not a product optimization plan. It is a prompt pack for determining whether the completed work can still support a defensible systems / CS research direction, and if so which one.

## How To Use This Pack

- Use one question per chat.
- Attach the shared baseline files first.
- Then attach the question-specific files.
- Ask for literature comparisons, novelty testing, and explicit rejection criteria.
- Do not ask the model for generic CUDA advice.
- Do not ask it to rescue weak ideas the repo already closed.

Recommended instruction for every thread:

- distinguish what the repo already proved from what still requires new evidence
- compare the attached repo state against prior work, not against generic engineering folklore
- be explicit about whether the result is:
  - publishable systems / CS research
  - a practitioner engineering case study
  - or not strong enough

## Shared Baseline Files

Attach these to every prompt:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`
- `target/cupti_sass_probe/large_lockstep_region_summary_2026-04-05.md`

Useful optional attachments for some threads:

- `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_dataflow_architecture_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_paper_outline_2026-04-03.md`
- `CUDA kernel review/Prompt1/xlarge_cache_reuse_probe_findings_2026-04-04.md`
- `CUDA kernel review/Prompt2/tm_native_vwma_pilot_findings_2026-04-04.md`

## Shared Answer Shape

Ask for this structure in every thread:

1. What the repo already proved.
2. What remains unresolved.
3. Closest prior work and how close it really is.
4. The strongest remaining claim, if any.
5. The minimum additional experiment set required.
6. What would falsify or downgrade the claim.
7. Whether the result is better framed as:
   - systems / CS research
   - compilers / tooling research
   - computer engineering case study
   - practitioner writeup only

## Highest-Value Questions

These are ranked. Question 1 is the best first use of a long research-mode run.

## Question 1: Is The Static + Dynamic Attribution Story Actually Novel Enough?

Why this still matters:

- The repo’s strongest surviving research framing is still static SASS plus runtime attribution for choosing the correct optimization layer.
- The current question is no longer “did it work?” but “is this sufficiently different from existing systems / GPU optimization literature?”

Attach in addition:

- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`
- `target/cupti_sass_probe/large_lockstep_region_summary_2026-04-05.md`

Prompt:

```text
I want you to evaluate the strongest remaining research framing for this repo after a completed CUDA optimization campaign.

Using the attached repo documents and broad literature search, answer this question:

Is the repo’s static-plus-dynamic attribution story for irregular exact CUDA kernels actually novel enough to support a systems / CS research paper, or is it only a strong engineering case study?

Constraints:
- Do not answer with generic statements about CUDA optimization.
- Compare against prior work on GPU profiling, optimization guidance, SASS-guided tuning, compiler-aware performance debugging, and GPU autotuning.
- Treat the attached repo documents as the ground truth for what was already implemented and measured.

Deliverables:
1. Summarize the repo’s actual contribution candidate in one paragraph.
2. Identify the closest prior work and how similar it really is.
3. State whether the contribution is:
   - clearly novel enough
   - borderline
   - or too close to known engineering practice
4. If borderline, identify the one or two changes in framing or evidence that would most improve it.
5. State the strongest honest paper title and claim this repo could currently support.
6. State the most likely reviewer objection.
7. State whether the result is better positioned as systems CS, compilers/tooling, CE, or practitioner engineering.
```

## Question 2: Can The CUPTI + SASS + Source Mapping Become A Real Tooling Contribution?

Why this still matters:

- The CUPTI branch did not produce a new product optimization lead, but it did produce region-level instruction attribution that refined the intra-symbol story.
- That may be more valuable as tooling research than as optimization guidance.

Attach in addition:

- `crates/app_core/src/bin/cupti_sass_probe.rs`
- `scripts/run_cupti_sass_probe.ps1`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_implementation_plan_2026-04-05.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`

Prompt:

```text
I want you to evaluate whether the repo’s CUPTI follow-up can be turned into a tooling or methodology paper rather than a product optimization lead.

Using the attached repo files and broad literature search, answer this question:

Can a workflow that combines static SASS region mapping, Nsight profiling, and CUPTI SASS metrics into source-level optimization-layer decisions be a meaningful tooling or methodology contribution?

Constraints:
- Do not assume the contribution is product speedup.
- Compare against prior GPU profiling, performance-debugging, and compiler/tooling work.
- Be explicit about what the attached repo helper does and does not currently prove.

Deliverables:
1. Identify the closest tooling and methodology papers.
2. Explain whether the attached workflow is genuinely different, only incrementally different, or too derivative.
3. Define the minimum feature set needed for a credible tooling artifact.
4. State what additional evaluation would be required across more than one kernel or workload.
5. State whether the contribution should be framed as:
   - a tool
   - a workflow
   - an attribution methodology
   - or not a viable paper artifact
6. Give a clear keep / revise / abandon verdict.
```

## Question 3: Is Replay-Authoritative Payload Separation The Strongest New Project?

Why this still matters:

- The external review consistently ranked replay-authoritative payload separation / compact exact search-stage work as the strongest downstream architecture study.
- That is still open, but it is clearly a new project rather than a continuation of bounded tuning.

Attach in addition:

- `SASS research/double_crossover_phase2_replay_dataflow_architecture_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`

Prompt:

```text
I want you to evaluate one possible next-stage research project for this repo.

Using the attached repo documents and broad literature search, answer this question:

Is replay-authoritative payload separation for irregular exact GPU search a strong enough new systems research project, and if so what would the actual contribution be?

Constraints:
- Treat this as a new architecture project, not a bounded product optimization pass.
- Compare against prior work on GPU search pipelines, exact reranking, compact candidate representations, and replay/recompute tradeoffs.
- Use the attached repo evidence to explain why this idea would begin from a real bottleneck history rather than from speculation.

Deliverables:
1. Explain what replay-authoritative payload separation would mean in general terms.
2. Explain whether the idea looks novel, partially explored elsewhere, or too close to known GPU search practice.
3. Define the minimal prototype that would be needed to test the idea.
4. Define the baseline comparisons and evaluation surfaces.
5. State the strongest possible paper claim and the most likely failure mode.
6. Conclude whether this is the best new project to pursue from the current repo state.
```

## Question 4: Is One Final `replay_other` Split Still Research-Justified?

Why this still matters:

- The current research docs explicitly allow one more measurement-only pass that splits `replay_other`.
- That is the cleanest remaining unresolved empirical question from the original campaign.

Attach in addition:

- `SASS research/double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`

Prompt:

```text
I want you to judge whether one final measurement-only pass is still worth doing for research quality.

Using the attached repo evidence and broad literature search, answer this question:

Is a final split of the retained `replay_other` bucket into launch-submit vs replay-kernel vs decode/writeback still worth doing from a research perspective, or is it unlikely to materially improve the current artifact?

Constraints:
- Treat this as a measurement-only question, not permission for another optimization prototype.
- Focus on whether the extra causal resolution would materially strengthen a publishable claim.

Deliverables:
1. Explain what is still ambiguous in the current replay attribution story.
2. State whether this ambiguity materially harms the strongest remaining paper framing.
3. Define the minimum measurement set required to resolve it.
4. State what possible outcomes would actually change the narrative.
5. Give a do now / optional / not worth doing verdict.
```

## Question 5: Could Distribution-Aware Specialization Be A Better Research Story Than It Looks?

Why this still matters:

- The external review kept distribution-aware specialization alive as an unresolved research direction.
- The repo never executed a serious study there, but it also never promoted it.

Attach in addition:

- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `crates/app_core/src/kernels/double_crossover.cu`
- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`

Prompt:

```text
I want you to evaluate whether workload-sensitive specialization is a stronger research direction than the repo currently assumes.

Using the attached repo files and broad literature search, answer this question:

Could a distribution-aware specialization study around irregular pair grouping, warp dedup effectiveness, or workload-sensitive GPU execution become a better research direction than the current attribution/tooling story?

Constraints:
- Do not answer with generic “specialization can help” statements.
- Tie the answer to the repo’s actual kernel structure and current evidence.
- Compare against prior work on workload-sensitive GPU specialization, adaptive execution, and irregular SIMT optimization.

Deliverables:
1. Explain what the specialization question would actually be in this repo.
2. State whether there is enough existing evidence to justify the study.
3. Define the minimum experiment set required.
4. Compare its likely novelty against the static+dynamic attribution story.
5. State whether this should be promoted, deferred, or rejected.
```

## Question 6: What Would Actually Justify Post-Compilation Work Here?

Why this still matters:

- Post-compilation feasibility was scoped but never executed.
- The main risk is drifting into architecture-specific microtuning with weak scientific value.

Attach in addition:

- `SASS research/double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`

Prompt:

```text
I want you to evaluate the post-compilation branch honestly before any implementation begins.

Using the attached repo evidence and broad literature search, answer this question:

What exact evidence would be required to justify post-compilation optimization research for this repo, and does the current state come anywhere close to meeting that bar?

Constraints:
- Do not assume that post-compilation equals novelty.
- Distinguish source-level limits, runtime/dataflow limits, and truly residual compiled-code behavior.
- Compare against existing post-compilation GPU work such as native schedule search, binary rewriting, and assembly-level autotuning.

Deliverables:
1. Define the actual scientific question that a post-compilation branch would need to answer.
2. Identify the minimum evidence required before such a branch is justified.
3. Explain whether the current repo evidence supports, weakly supports, or does not support that branch.
4. State the biggest risk of pursuing it anyway.
5. Give a pursue / defer / reject verdict.
```

## Question 7: Is There A Benchmark / Artifact Paper Hidden In This Repo?

Why this still matters:

- The repo has a large artifact bundle, rejected branches, retained wins, and mixed positive/negative results.
- That may support a benchmark, evaluation, or artifact paper even if the “algorithm” paper is weak.

Attach in addition:

- `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_artifact_index_2026-04-03.md`
- `SASS research/double_crossover_phase2_paper_outline_2026-04-03.md`

Prompt:

```text
I want you to evaluate whether this repo is better positioned for an artifact, benchmark, or evaluation paper than for a novel optimization-method paper.

Using the attached repo evidence and broad literature search, answer this question:

Is there a credible benchmark, artifact-evaluation, or negative-results paper hiding in this repo, and if so what would its exact angle be?

Constraints:
- Compare against existing GPU benchmark suites, artifact papers, case studies, and negative-result venues or tracks.
- Do not assume that “large artifact bundle” alone is enough.

Deliverables:
1. Explain the strongest benchmark/artifact-style framing available.
2. State what class of reader would actually benefit from it.
3. Define the minimal packaging and evaluation needed.
4. Compare this path against the stronger methodology/tooling path.
5. State whether this is a fallback only or a genuinely better direction.
```

## Question 8: What Venues And Review Standards Actually Fit This Work?

Why this still matters:

- A lot of the confusion in this campaign came from not knowing whether the output was aiming at systems CS, CE, compilers, or practitioner engineering.
- A venue-aware answer helps decide whether any direction is worth reopening at all.

Attach in addition:

- `SASS research/double_crossover_phase2_paper_outline_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`

Prompt:

```text
I want a venue-aware evaluation of the repo’s remaining research options.

Using the attached repo evidence and broad literature search, answer this question:

Which publication venues, tracks, or communities would be the most natural fit for the strongest remaining research direction from this repo, and what review standards would matter most there?

Constraints:
- Distinguish systems CS, compilers/tooling, CE, and practitioner-oriented venues.
- Be explicit about what contribution styles each venue would likely accept or reject.

Deliverables:
1. Rank the most plausible venue families.
2. For each, state the expected contribution style and likely reviewer standards.
3. State which of the repo’s remaining research directions best fits each venue family.
4. State whether any direction looks strong enough to justify reopening active technical work.
```

## Questions I Would Not Spend Research Mode On

These look low-value now because the repo already closed them or they are too generic:

- “How can I optimize this CUDA kernel more?”
- “Should I do more SASS tuning?”
- “Would generic cache improvements help?”
- “Is CUPTI useful for finding the next product optimization?”
- “Should I broadly expand TM-native support?”
- “Should I revisit score-first or reducer-first work?”

Those questions are either already closed by repo evidence or too underspecified to justify a 40-minute research run.

## Recommended Order

If you only run three threads, use this order:

1. `Question 1: Is The Static + Dynamic Attribution Story Actually Novel Enough?`
2. `Question 2: Can The CUPTI + SASS + Source Mapping Become A Real Tooling Contribution?`
3. `Question 3: Is Replay-Authoritative Payload Separation The Strongest New Project?`

That sequence should tell you whether:

- the current campaign can still be framed as research,
- CUPTI strengthens that framing enough to matter,
- or the only remaining serious option is to start a new architecture project.
