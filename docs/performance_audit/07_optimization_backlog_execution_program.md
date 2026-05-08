# Optimization Backlog And Execution Program

Date: 2026-05-06
Status: execution plan

## Goal

Turn audit results into a disciplined optimization backlog. The backlog should rank work by measured user/workflow impact, not intuition.

## Sequencing Principles

1. Measure before optimizing.
2. Preserve correctness and numeric policy.
3. Preserve CPU-only builds.
4. Remove avoidable CPU tails before claiming GPU gains.
5. Improve warm app-path workflows before adding new feature surface.
6. Keep artifact/report correctness ahead of serialization shortcuts.
7. Use companion profiling to explain, not approve, performance claims.

## Backlog Template

```md
## PERF-<id>: <title>

- Workflow:
- Claim class:
- Timing boundary:
- Current baseline:
- Target:
- Evidence source:
- Bottleneck class:
- Proposed change:
- Files likely touched:
- Correctness tests:
- Perf gate:
- Rollback:
- Risk:
- Status:
```

## Initial Hypothesis Backlog

These are hypotheses until measured.

| ID | Hypothesis | Likely impact | Evidence needed |
|---|---|---|---|
| PERF-001 | Validation CPU tails dominate some GPU validation paths | high | validation phase timing, GPU validation gate |
| PERF-002 | Candidate drilldown/local-stability can reuse warm data better | high | GPU follow-up gate, follow-up bench |
| PERF-003 | Campaign throughput loses time in dataset prep/artifact matching | high | campaign perf timing |
| PERF-004 | Options event/tuple loops have avoidable repeated scans | high | options profiling and CPU perf |
| PERF-005 | Artifact library list parses too much JSON | medium-high | artifact list timing |
| PERF-006 | XLSX report generation blocks after compute | medium | report phase timing |
| PERF-007 | Large validation/options tables spend time in normalization/render | medium | frontend timing diagnostics |
| PERF-008 | GPU H2D/D2H materialization hides behind kernel-speed claims | high | Nsight Systems plus route diagnostics |
| PERF-009 | Repeated kernel launch overhead matters for small follow-ups | medium | Nsight Systems; CUDA Graph spike only if proven |
| PERF-010 | Large object replacement in Pinia causes avoidable recompute | medium | frontend store timing |
| PERF-011 | Meta/ML artifact parsing dominates aggregate workflows | medium | meta workflow timing and artifact scan counts |
| PERF-012 | ML Signal score-cache generation mixes model load and inference time | medium | ML Signal phase timing |
| PERF-013 | Portfolio feasibility pairwise work grows faster than expected | medium | candidate-count scaling case |
| PERF-014 | Execution/task progress events cause UI update fanout | medium | event burst timing and task-store diagnostics |
| PERF-015 | App startup/preflight blocks shell readiness | medium | startup phase timing |

## Batch 1: Measurement Foundation

- [x] Create workflow case registry.
- [x] Run plan-only/current gates where possible.
- [x] Add lightweight phase timing where absent for support/report/artifact/campaign surfaces.
- [x] Capture first `output/performance_audit/<date>/summary.md`.
- [x] Rank top bottlenecks in the machine-readable case registry and audit docs.

## Batch 2: Low-Risk Backend Wins

Only after Batch 1:

- [x] Remove measured unnecessary clones.
- [ ] Add lazy detail/hydration where artifact timing proves it.
- [ ] Cache deterministic summaries with invalidation.
- [x] Tighten validation module phase reuse where correctness is unchanged.

## Batch 3: GPU Route Wins

Only after Batch 1:

- [x] Remove avoidable host materialization in the validation regime-filter upload path.
- [ ] Attribute or eliminate CPU tails.
- [ ] Improve selected Double-MA follow-up route.
- [ ] Consider pinned memory/memory pools only with measured transfer/allocation bottleneck.

## Batch 4: Frontend Responsiveness

Only after frontend timing:

- [ ] Split overly large components only where mount/update timing justifies it.
- [ ] Memoize expensive row/view models.
- [ ] Move large artifact flattening to workers if measured.
- [ ] Keep accessibility behavior intact.

## Batch 5: Support Workflow Coverage

Only after identity-path baselines exist:

- [ ] Add ML/meta phase timing.
- [ ] Add ML Signal score-cache/model-load timing.
- [x] Add portfolio feasibility scaling case to the support authority harness.
- [x] Add execution scaffolding no-credential timing.
- [x] Add dynamic/system tooling timing to the support authority harness.

## Batch 6: Baseline Promotion

Only after candidate runs stabilize:

- [ ] Decide which candidate results should become shared baselines.
- [ ] Refresh baselines through explicit baseline mode.
- [ ] Review baseline changes like source code.
- [ ] Update README benchmark claims only if timing boundary and scenario match.

## Reporting Format

Every optimization PR should include:

- before/after summary;
- exact gate/script;
- case id;
- timing boundary;
- hardware;
- dataset hash;
- route;
- correctness tests;
- residual caveats.

## Definition Of Done

- Performance backlog is ranked by evidence.
- Top three optimizations have clear implementation tasks.
- No performance claim lacks a timing boundary.
- No optimization compromises correctness, reproducibility, or CPU-only support.
