# `double_crossover` Phase 2 Research Artifact Plan

Date: 2026-04-03

## Purpose

Shift the current branch from open-ended performance tuning to a research-artifact workstream.

From this point forward, further effort is justified only if it materially improves the final research artifact. Product-facing code changes are no longer the default objective.

## Primary Artifact Target

The strongest single artifact to target now is:

1. a repo-owned research memo or paper draft on **static-plus-dynamic attribution for irregular exact CUDA kernels**, using `double_crossover` as the main case study

Working claim:

1. static SASS plus runtime attribution can correctly identify when the next useful optimization should be:
   - a bounded source/runtime refactor
   - a replay/dataflow intervention
   - or explicit deferral of more code changes
2. in this repo, that methodology already produced:
   - one retained lockstep-body improvement
   - one retained replay-batching improvement
   - one measured rejection of descriptor reuse as the next branch

## Why This Artifact Wins

This is stronger than “one more optimization pass” because the repo already has:

1. pre-change static SASS evidence
2. retained code changes with verified wins
3. post-change runtime attribution
4. an explicit negative result that rules out a plausible next step

That is a real methodology artifact, not just a benchmark diary.

## Minimum Evidence Bundle

The artifact should treat the following as required:

1. baseline static SASS findings:
   - `SASS research/double_crossover_performance_findings_and_code_suggestions.md`
   - `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`
2. retained runtime/profiling narrative:
   - `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
   - `SASS research/double_crossover_phase2_windowed_replay_kernel_study_2026-04-03.md`
   - `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`
3. replay follow-on measurement and decision artifacts:
   - `SASS research/double_crossover_phase2_replay_artifact_manifest_2026-04-03.md`
   - `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
   - `SASS research/double_crossover_phase2_replay_branch_decision_2026-04-03.md`
   - `SASS research/double_crossover_phase2_replay_methodology_note_2026-04-03.md`
4. authoritative verification surfaces:
   - `target/gpu_perf_summary.json`
   - `target/gpu_alma_contract_rows_perf_summary.json`
5. representative code ownership surfaces:
   - `crates/app_core/src/kernels/double_crossover.cu`
   - `crates/app_core/src/gpu_backtest_kernel.rs`
   - `crates/app_core/src/double_ma.rs`
   - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

## Stop Rule For Further Code Changes

Do not start another performance-oriented code prototype unless one of the following becomes true:

1. the next measurement-only pass splits `replay_other` and reveals one dominant subcomponent that clearly clears the branch’s numeric promotion rules
2. the research artifact explicitly needs one more bounded A/B intervention to close a causal gap that cannot be closed by existing evidence

Otherwise:

1. freeze code changes on this branch
2. continue only with artifact assembly, analysis synthesis, and research framing

## Allowed Remaining Technical Work

The following are still justified:

1. one more measurement-only pass that splits `replay_other` into:
   - replay launch submit
   - replay kernel execution
   - replay decode or writeback
2. targeted artifact cleanup:
   - normalize metric tables
   - consolidate filenames and manifests
   - extract before/after summaries
3. save-ready research writing:
   - memo
   - outline
   - experiment table
   - claims and non-claims

The following are not justified by default:

1. another replay-side code prototype
2. broader `compact_exact_search_stage` work
3. new SASS rewriting work
4. generic profiling not tied to a research claim

## Research Deliverables

### D1. Canonical Findings Memo

Create:

1. `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`

It should include:

1. problem framing
2. repo constraints
3. static evidence summary
4. retained interventions
5. post-change attribution
6. rejected branch rationale
7. current frontier
8. research claims and non-claims

### D2. Experiment Table

Create:

1. `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`

It should summarize:

1. hypothesis
2. intervention
3. measured surfaces
4. before
5. after
6. verification
7. retained or rejected

### D3. Artifact Manifest

Extend the existing manifest chain so there is one clear index for an external reviewer.

Create:

1. `SASS research/double_crossover_phase2_research_artifact_index_2026-04-03.md`

### D4. Paper-Or-Memo Outline

Create:

1. `SASS research/double_crossover_phase2_paper_outline_2026-04-03.md`

This should frame the strongest publishable direction as:

1. SASS-guided source/runtime optimization decisions for irregular exact kernels

## Checklist

### A. Freeze And Scope

- [x] Freeze new optimization code changes unless they satisfy the stop rule.
- [x] Treat the current retained code state as the default evidence base.
- [x] Record the primary artifact target and non-goals in the main Phase 2 plan.

### B. Evidence Consolidation

- [x] Consolidate the retained intervention sequence into one canonical findings memo.
- [x] Create one experiment table covering the lockstep-body pass, replay-batching pass, and descriptor-reuse rejection.
- [x] Create one research artifact index that points to all supporting docs and key profiler outputs.

### C. Optional Final Measurement

- [ ] Decide whether splitting `replay_other` is required for the final artifact or merely nice to have.
- [ ] If required, run exactly one more measurement-only pass and no new prototype.
- [ ] If not required, document that the artifact stops with the current evidence.

### D. Writing

- [x] Write the canonical research findings memo.
- [x] Write the paper-or-memo outline.
- [x] State the main claim, the negative result, and the stop rule clearly.
- [x] Separate product conclusions from research conclusions.

### E. Final Branch Decision

- [x] Decide whether the branch ends as:
  - a finished internal research memo
  - a paper-ready outline plus artifact bundle
  - or a research pause pending one final measurement-only pass

## Success Criteria

This workstream is successful when all of the following are true:

1. the repo has one canonical research memo that explains the retained wins and the rejected next branch
2. the evidence bundle is organized enough that an external model or reviewer can follow the full causal story
3. further optimization work is either explicitly justified by a research gap or explicitly stopped

## Current Recommendation

The current recommendation is:

1. do not start another optimization prototype
2. produce the canonical findings memo, experiment table, artifact index, and paper outline
3. only then decide whether one final measurement-only pass on `replay_other` is necessary

## Current Outcome

The branch now qualifies as:

1. a finished internal research memo
2. a paper-ready outline plus artifact bundle

Optional follow-on:

1. one final measurement-only pass on `replay_other` is still allowed, but it is no longer required to justify the current artifact bundle
