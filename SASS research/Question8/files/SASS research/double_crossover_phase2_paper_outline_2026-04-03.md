# `double_crossover` Phase 2 Paper Outline

Date: 2026-04-03

## Working Title

Static-Plus-Dynamic Attribution For Irregular Exact CUDA Kernels: A `double_crossover` Case Study

## Thesis

Static SASS alone is not enough to choose the next optimization layer for irregular exact CUDA kernels, but static SASS combined with bounded runtime attribution can correctly distinguish:

1. when a source/runtime refactor is justified
2. when a replay/dataflow intervention is justified
3. when further code changes should be deferred

## Abstract Skeleton

1. Introduce the problem: optimized CUDA kernels still leave ambiguous next steps in irregular, control-heavy exact-search pipelines.
2. Present the repo as a case study with aligned source, runtime, and disassembly artifacts.
3. Show that static SASS identified early wrapper/contract targets.
4. Show that runtime attribution isolated replay/dataflow as a later hotspot.
5. Show that one replay/dataflow intervention produced a retained win.
6. Show that a plausible next step, descriptor reuse, was rejected by direct measurement.
7. Argue that the contribution is a methodology for branch selection, not just a sequence of local speedups.

## Sections

### 1. Introduction

1. motivation
2. why irregular exact kernels are hard to optimize
3. why generic profiler advice is insufficient

### 2. Case Study System

1. `double_crossover` exact `double_ma` route
2. `full17` and `trade22` metric contracts
3. shortlist slice semantics
4. exact row authority and CPU fallback constraints

### 3. Method

1. static SASS inspection
2. runtime profiling stack:
   - built-in timing
   - direct-runner JSONL
   - Nsight Systems
   - Nsight Compute
3. bounded branch-promotion rules

### 4. Early Static-Guided Intervention

1. wrapper and contract issues
2. retained lockstep-body pass
3. validation and perf gating

### 5. Replay Attribution And Intervention

1. shortlist-slice attribution
2. tiny-launch replay diagnosis
3. retained replay-batching intervention
4. resulting shortlist-heavy gains

### 6. Negative Result: Descriptor Reuse Rejected

1. replay micro-attribution setup
2. descriptor prep and upload measured at `0 ms`
3. branch-decision rationale
4. why rejection is still useful evidence

### 7. Discussion

1. what this methodology gets right
2. what remains unresolved in `replay_other`
3. limits of the current artifact

### 8. Conclusion

1. static-plus-dynamic attribution is the real contribution
2. the repo demonstrates retained wins and measured non-promotion of a plausible branch
3. future work: only if a further split of `replay_other` reveals a dominant subcomponent

## Figures And Tables

1. optimization timeline:
   - static SASS findings
   - lockstep-body pass
   - shortlist attribution
   - replay-batching pass
   - descriptor-reuse rejection
2. experiment table:
   - hypotheses, interventions, outcomes
3. shortlist-heavy before/after performance table
4. replay micro-attribution table

## Claims

The paper can safely claim:

1. the methodology identified two retained bounded wins
2. the methodology rejected one unjustified next branch
3. the artifact is useful for irregular exact CUDA pipelines

The paper should not claim:

1. global optimality
2. that the repo has exhausted all possible speedups
3. that descriptor reuse is impossible in general

## Current Recommendation

Use this outline only after the canonical findings memo and experiment table are treated as stable.
