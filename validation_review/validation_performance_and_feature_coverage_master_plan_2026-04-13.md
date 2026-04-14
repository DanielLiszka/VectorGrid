# Validation Performance And Feature Coverage Master Plan

Last updated: 2026-04-13

## Purpose

Define the next repo-wide validation program after the first-wave `DoubleMa` throughput bundle. The goal is to improve existing validation performance and to close or document existing validation-feature coverage gaps across the current strategy surface without adding new product lanes.

This program also treats current `DoubleMa` validation support as the default feature-parity baseline for the rest of the repo. For every strategy family, the implementation goal is:

- match existing `DoubleMa` validation feature support where technically and product-wise feasible
- explicitly document and benchmark any remaining exception when parity is not feasible, not exact, or not worth forcing

This plan is intentionally broader than the closed `validation_throughput_cuda_plan*.md` bundle. That earlier bundle remains the historical first wave for `DoubleMa` CUDA validation throughput. This document is the new master plan for:

- residual `DoubleMa` validation performance work
- shared validation-core throughput
- cross-strategy validation feature coverage
- route and fallback truthfulness
- per-strategy validation posture hardening

## Relationship To Existing Validation Review Docs

- `validation_throughput_cuda_plan.md`
  - historical implementation plan for the first `DoubleMa` validation-throughput wave
- `validation_throughput_cuda_plan_addendum.md`
  - corrections and follow-on ideas for the same wave
- `validation_throughput_plan_disposition_matrix.md`
  - live closure/status document for that narrower first-wave bundle

This document does not reopen completed items from that bundle unless a later measurement shows a real regression or a still-open downstream dependency.

## Scope Boundaries

In scope:

- existing validation execution paths for all current strategy families
- existing validation modules:
  - main fixed-candidate validation
  - walk-forward
  - reopt walk-forward
  - CSCV/PBO
  - SPA
  - perturb
  - trade bootstrap
  - cross-ticker reruns
  - finalist aux analytics such as regime-slice robustness and friction stress where the family supports them
- shared validation-core throughput, scratch reuse, and deterministic measurement behavior
- command-layer orchestration and request-shaping overhead that materially affects validation latency
- route/fallback summaries and validation capability truthfulness
- benchmark and measurement surfaces used to justify validation policy
- feature-parity work that brings non-`DoubleMa` strategies closer to the existing `DoubleMa` validation module set where feasible

Out of scope:

- new strategy families
- new validation methods that do not already exist in the repo
- UI/UX redesign work
- options-expression roadmap expansion
- broad architecture rewrites that are not required by measured validation bottlenecks

## Non-Negotiable Constraints

- Preserve correctness, parity, and deterministic seeded behavior.
- Keep CPU-only builds working without a CUDA toolchain.
- Treat benchmark and profiling evidence as authoritative over intuition.
- Run one benchmark, workflow perf gate, or profiling session at a time.
- Prefer additive hardening over broad refactors.
- Keep route and fallback reporting more truthful, not less.
- Do not weaken existing validation semantics merely to improve timing.

## Current Repo Posture

- `DoubleMa` validation already completed a meaningful first throughput wave:
  - exact-combo GPU validation
  - ranged batching
  - aux-work masking
  - scratch reuse
  - cross-ticker alignment/dedupe cleanup
  - validation execution-profile telemetry
- `DoubleMa` remains the validation truth anchor because it has the strongest existing timing and parity instrumentation.
- Shared validation cost still concentrates in:
  - `crates/app_core/src/validation_core.rs`
  - `crates/app_core/src/validation_pipeline.rs`
  - `crates/app_core/src/iterative_sim.rs`
- Command-layer orchestration remains a separate end-to-end latency surface:
  - `src-tauri/src/commands_validation.rs`
  - `src-tauri/src/validation_common.rs`
  - `web/src/services/validationRequestBuilder.ts`
  - `web/src/services/validationService.ts`
- Current strategy families do not share one uniform validation posture:
  - some families inherit the `DoubleMa` validator
  - some use `composite_validate.rs`
  - some keep mixed or CPU-preferred validation policies
  - some keep GPU main routes with CPU-tail modules

## Implemented Progress

Current landed slices in this program:

- shared validation-core cleanup in `validation_core.rs`
  - remove temporary `start_ts` materialization and track `common_start_t` directly during candidate collection
- shared validation-core cleanup in `validation_pipeline.rs`
  - precompute robust-Pareto comparison inputs before the dominance loop
  - reserve walk-forward and reopt walk-forward buffers from an estimated window count
  - pre-parse finalist key JSON once for reopt drift tracking instead of reparsing on every winner transition
  - replace full-sort median and quantile reductions in walk-forward, reopt walk-forward, and trade bootstrap with in-place order-stat selection
  - stream perturb summary accumulation instead of materializing temporary sharpe/max-dd/pnl vectors
- cross-ticker orchestration cleanup
  - pre-size `CrossTickerAcc` metric buffers to dataset count
  - precompute cross-ticker Pareto comparison inputs before the dominance loop
- routed composite GPU validation truth
  - `Vcc` now uses the dedicated GPU walk-forward and reopt walk-forward helpers on the `GpuKernel` path instead of always falling back to the generic candidate-series route
- routed `DoubleMa` TrendQuality parity-safety mitigation
  - `GpuKernel` and `Auto` fixed-candidate validation now fall back to CPU for TrendQuality requests without ADX confirmation because the exact-combo GPU validation path is not parity-safe on that route
  - targeted regression coverage now locks that fallback policy, and a small TrendQuality uploaded-regime/device-parity test tightens the boundary between host regime construction and the remaining kernel-side bug
- routed and composite-family coverage truth
  - routed `DoubleMa` profiles already inherit the full `DoubleMa` validation stack on the current repo surface; remaining differences are route-policy exceptions rather than missing module support
  - Cluster B has broader module coverage than the first draft implied, but still keeps real parity gaps in composite `cross_ticker` scope and composite-vs-`DoubleMa` aux analytics breadth
- Cluster B helper-parity prep
  - the dedicated `Cdmc` GPU walk-forward helper now preserves downside/ulcer/martin metrics instead of zero-filling them, and a helper-direct regression test locks that behavior before any future dispatch change

Important non-landed findings from the same audit:

- the remaining TrendQuality ADX-off `GpuKernel` parity issue sits below the validation reducers in the exact-combo GPU regime path; the shipped CPU fallback is the safe mitigation, but the underlying kernel-side repair still needs a narrower follow-up slice
- `Carm` and `Cdmc` already have dedicated GPU walk-forward/reopt helpers, but direct dispatch to those helpers does not currently satisfy the existing CPU-match tolerances in the current regression tests. Those routes remain deferred parity work rather than safe drop-in replacements.
- composite `cross_ticker` remains partial relative to the `DoubleMa` baseline even though it is broader than the first draft implied; preserved-finalists reruns keep CSCV/PBO and hard gates, but they do not yet rerun the full walk-forward/reopt/SPA/perturb/bootstrap stack.
- composite-family aux analytics remain narrower than the `DoubleMa` baseline. Current composite candidates expose `regime_slice_robustness` and `friction_stress`, but not `entry_margin_fragility`, `underwater_duration`, `checkpoint_robustness`, or `benchmark_slice_summary`.

## Double-MA Feature Parity Baseline

For this program, the existing `DoubleMa` validation surface is the baseline feature set that other strategies should be compared against.

Current `DoubleMa` baseline modules:

- main fixed-candidate validation
- walk-forward
- reopt walk-forward
- CSCV/PBO
- SPA
- perturb
- trade bootstrap
- cross-ticker reruns
- finalist aux analytics where applicable in the current family surface

The parity question for every non-`DoubleMa` bucket is:

- can this bucket support the same module?
- can it support the same module only via CPU tail or mixed route?
- is the module blocked by genuine strategy semantics or runtime architecture?
- is the module missing only because implementation never caught up?

The program should assume parity is the target unless one of those questions yields a real blocker backed by code and measurement.

## Canonical Code Surface

Open these first for this program:

- `REPO_MAP.md`
- `gpt54_feature_review_map.md`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`

Primary measurement/evidence surfaces:

- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`

## Strategy Buckets

### 1. Double-MA family

Current owner files:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/double_ma.rs`
- `src-tauri/src/commands_validation.rs`

Current members:

- `DoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

Current posture:

- strongest validation-throughput baseline in the repo
- routed-family members inherit the `DoubleMa` validation engine
- `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` still need explicit route-policy and module-coverage truth audits, especially around ADX-driven route differences

### 2. CompositeCpu and CompositeMo

Current owner files:

- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`

Current posture:

- `CompositeCpu` has its own validation family and GPU route constraints
- `CompositeMo` remains CPU-preferred for validation, with current GPU fixed-candidate support intentionally limited to plain `CompositeCpu` hard-gate candidates

### 3. Standalone profile Cluster A

Current members:

- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`
- `Vcet`

Current posture from `REPO_MAP.md`:

- `Asr`, `Vemr`, and `Rocb` are still CPU-preferred on measured validation workloads
- `Vcc` is mixed because exact CPU-reference prep still matters, but `GpuKernel` now has dedicated GPU walk-forward and reopt walk-forward coverage
- `Vcet` keeps the main GPU fixed-candidate route and current runtime tests cover walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap on `GpuKernel`

### 4. Standalone profile Cluster B

Current members:

- `Mkcs`
- `Kqsr`
- `Carm`
- `Kvdf`
- `Cdmc`

Current posture from `REPO_MAP.md`:

- `Mkcs`, `Kqsr`, `Carm`, and `Kvdf` remain CPU-preferred or CPU-fallback-sensitive on measured validation workloads
- `Cdmc` is more GPU-favorable than `Carm`, but still keeps mixed-route characteristics

### 5. Special flows

Current members:

- `MlSignal`

Current posture:

- separate workflow and timing surface
- validation cost includes score-cache, replay, and policy-sweep concerns rather than only fixed-candidate backtest throughput

## Program Goals

1. Reduce end-to-end validation latency for existing workflows without changing verdict semantics.
2. Make per-strategy validation coverage explicit:
   - full support
   - partial support
   - CPU-tail support
   - deliberate CPU-preferred policy
   - unsupported by design
3. Move every non-`DoubleMa` strategy as close as practical to the current `DoubleMa` validation module set.
4. Require explicit documented exceptions when a strategy cannot or should not match the `DoubleMa` module set.
5. Keep backend route summaries, fallback reasons, and frontend capability summaries aligned with measured reality.
6. Move repeated work and scratch allocation out of hot validation loops where possible.
7. Make performance evidence strategy-specific instead of assuming `DoubleMa` conclusions generalize to every family.

## Required Measurement Posture

- Use `ta_double_ma_validation_cost.rs` as the primary `DoubleMa` cost boundary.
- Extend or add equivalent cost authority for standalone/composite strategy validation when the current composite bench is not enough.
- Treat Criterion benches as supplementary evidence, not the only timing source.
- Record cold/warm distinction whenever cache or residency changes matter.
- Capture requested backend, backend used, backend route, fallback reason, bars, candidate counts, enabled modules, and execution-profile fields whenever those are available.

## Execution Model

Every implementation slice in this program should follow the same sequence:

1. Audit the current bucket with file-backed notes.
2. Run the current timing surface on the correct boundary.
3. Classify the issue:
   - shared-core cost
   - strategy-local cost
   - command-layer overhead
   - route-policy drift
   - missing module coverage
   - deliberate unsupported posture
4. Implement only the smallest additive change that improves:
   - throughput
   - feature coverage
   - route/fallback truth
   - test coverage
5. Re-run targeted tests and the relevant timing surface.
6. Update the tracker and any route-truth docs that changed.

## What Counts As A Coverage Gap

This program is not only about raw speed. A validation-feature coverage gap exists when one of these is true:

- a strategy family lacks an existing validation module that should already apply to it
- a strategy family claims broader validation support than the backend actually executes
- a module exists, but only through a silent CPU fallback that is not surfaced clearly
- a module is implemented but not wired through the command/request path for that family
- a strategy remains CPU-preferred for good reasons, but that posture is not documented or benchmark-backed

This program should distinguish among:

- true missing implementation
- truthful CPU-preferred policy
- mixed-route support
- CPU-tail completion of a nominal GPU route
- unsupported by design

The default decision rule is:

- if a non-`DoubleMa` strategy can reasonably support a `DoubleMa` validation module, the plan should treat that as implementation work
- if it cannot, the plan should capture the blocker explicitly instead of leaving the gap as an implied future task

## Common Performance Questions To Ask In Every Bucket

- Is the validation path recomputing indicator or signal state that could be reused across candidates or ranges?
- Is host-side reshaping, cloning, hashing, or canonicalization dominating more than the actual validation logic?
- Are optional modules such as walk-forward, reopt walk-forward, SPA, perturb, bootstrap, cross-ticker, or aux analytics forcing repeated full recomputation?
- Is GPU validation paying so much staging or CPU-reference prep that the route should remain CPU-preferred?
- Is a mixed route actually the correct product posture, but the user-facing summary overstates GPU coverage?
- Is the timing evidence on the right boundary, or are we measuring a microbench while the real product cost lives elsewhere?

## Concrete Deliverables

This implementation program should leave behind these concrete artifacts:

- a maintained strategy-by-strategy validation tracker
- one or more bucket-specific timing authorities for validation cost
- route-truth updates in `REPO_MAP.md` when posture changes
- targeted regression tests for route, fallback, and module coverage
- execution-profile telemetry where timing attribution is currently opaque
- explicit notes for strategies that remain CPU-preferred by design
- a per-bucket parity assessment against the `DoubleMa` validation feature baseline
- explicit documented exception reasons whenever full parity is not feasible

## Master Checklist

### Program setup

- [x] Create a repo-wide validation performance and coverage master plan.
- [x] Create a companion execution tracker.
- [ ] Fill the tracker with file-backed module coverage notes for every strategy bucket.
- [ ] Record the canonical timing surface for each strategy bucket.
- [ ] Identify any bucket that lacks a trustworthy timing surface and needs one added.
- [ ] Add a parity-to-`DoubleMa` assessment for every non-`DoubleMa` strategy bucket.

### Double-MA anchor work

- [ ] Re-audit residual `DoubleMa` validation cost after the first throughput bundle.
- [ ] Confirm routed `DoubleMa` profile coverage for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`.
- [ ] Confirm inherited-module truth for `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, and `Vfct`.
- [ ] Tighten `DoubleMa` command-layer and cross-ticker overhead if current app-path timing still justifies it.

### Shared validation-core work

- [ ] Audit `validation_core.rs` for repeated work, unnecessary materialization, and avoidable allocation.
- [ ] Audit `validation_pipeline.rs` for reusable scratch, repeated reductions, and deterministic cost inflation.
- [ ] Audit `iterative_sim.rs` only where it materially affects validation throughput or CPU-tail cost.
- [ ] Add or tighten telemetry when shared-path module cost is currently opaque.

### Command and contract work

- [ ] Audit `commands_validation.rs` for end-to-end orchestration overhead.
- [ ] Audit `validation_common.rs` for repeated canonicalization, matching, and cross-ticker cost.
- [ ] Audit `validationRequestBuilder.ts` for candidate inflation, route-summary drift, or unsupported module exposure.
- [ ] Audit `validationService.ts` for latency inflation caused by orchestration or artifact follow-up.

### Strategy-family work

- [ ] Audit `CompositeCpu` validation coverage and fallback rules.
- [ ] Audit `CompositeMo` validation posture and document intentional CPU-preferred limits.
- [ ] Audit Cluster A one strategy at a time: `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`.
- [ ] Audit Cluster B one strategy at a time: `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`.
- [ ] Audit `MlSignal` on its own timing boundary and coverage checklist.
- [ ] For every non-`DoubleMa` strategy, decide explicitly whether each missing module is:
  - parity work to implement
  - parity work to defer
  - or a documented exception

### Verification and closeout

- [ ] Add or tighten targeted tests for every changed route, fallback, or module contract.
- [ ] Re-run the correct timing surface after each implementation slice.
- [ ] Update the tracker after each slice.
- [ ] Update `REPO_MAP.md` if route or coverage truth changes.
- [ ] Keep prior first-wave `DoubleMa` throughput docs untouched unless a new measurement truly reopens them.

## Work Phases

### Phase 0. Baseline And Coverage Inventory

Objectives:

- define the full strategy-by-strategy validation tracker
- audit current module coverage for every strategy bucket
- identify which current gaps are:
  - true missing support
  - CPU-tail support
  - route-policy restrictions
  - frontend/backend summary drift
- define the canonical timing surface for each bucket

Expected outputs:

- tracker status filled in for every strategy bucket
- explicit benchmark/cost harness owner for each bucket
- first list of coverage gaps that are real implementation work rather than documentation drift

Implementation detail:

- do not assume strategy support from naming alone
- confirm support by reading the relevant request builder, command entrypoint, validator, and tests
- capture whether support is:
  - inherited from `DoubleMa`
  - native in `composite_validate.rs`
  - special-cased elsewhere
- capture whether each module is:
  - implemented
  - CPU-tail
  - mixed
  - limited
  - intentionally unsupported

Phase 0 checklist:

- [x] Audit the `DoubleMa` family against `double_ma_validate.rs` and current tests.
- [ ] Audit `CompositeCpu` and `CompositeMo` against `composite_validate.rs` and route notes in `REPO_MAP.md`.
- [ ] Audit Cluster A strategies against `composite_validate.rs` and strategy-local tests.
- [x] Audit Cluster B strategies against `composite_validate.rs` and strategy-local tests.
- [ ] Audit `MlSignal` timing and validation ownership separately from the fixed-candidate families.
- [ ] Mark every bucket in the tracker with a file-backed coverage posture.
- [ ] Mark every bucket with its canonical timing authority.
- [ ] Mark every non-`DoubleMa` bucket with a parity target relative to the `DoubleMa` baseline.
- [ ] Record explicit exception reasons for any module that is unlikely to reach parity.

### Phase 1. Residual Double-MA Validation Pass

Objectives:

- confirm whether meaningful residual `DoubleMa` validation bottlenecks remain after the first throughput bundle
- audit routed `DoubleMa` family members for coverage and route truth
- tighten command-layer and cross-ticker overhead that still affects `DoubleMa` validation latency

Target files:

- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`

Implementation detail:

- do not re-open first-wave completed work without evidence
- focus on residual bottlenecks that still show up in the current code or current measurements
- prioritize:
  - command-layer overhead
  - routed-profile truth
  - CPU-tail follow-up work
  - cross-ticker rerun cost

Phase 1 checklist:

- [ ] Re-run `ta_double_ma_validation_cost.rs` on a current representative case.
- [ ] Compare app-path cost against core-only bench cost.
- [ ] Identify whether the next open issue is main-route cost, CPU-tail cost, or orchestration cost.
- [ ] Audit routed-family coverage and route truth for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`.
- [ ] Add targeted tests if routed-profile behavior or summaries change.
- [ ] Use the audited `DoubleMa` baseline as the parity reference for later buckets.

### Phase 2. Shared Validation Core Throughput

Objectives:

- reduce repeated work in shared validation loops and stats folding
- review scratch reuse and allocation policy inside:
  - `validation_core.rs`
  - `validation_pipeline.rs`
  - `iterative_sim.rs`
- make shared-path telemetry strong enough to attribute module cost rather than only total run cost

Target files:

- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- supporting stats/common helpers when directly implicated

Implementation detail:

- this phase should prefer improvements that benefit multiple strategy families at once
- do not move strategy-specific logic into the shared core only to make the code more generic
- keep changes local to measured pain points such as:
  - repeated candidate post-processing
  - repeated slice prep
  - repeated scratch allocation
  - repeated bootstrap or SPA staging

Phase 2 checklist:

- [x] Audit `parallel_collect_validation_candidates(...)` and the downstream collection path for avoidable materialization.
- [x] Audit CSCV/PBO, SPA, perturb, and bootstrap helpers for scratch reuse opportunities.
- [x] Audit reopt walk-forward loops for repeated work that can be collapsed safely.
- [ ] Add telemetry if the dominant shared-cost bucket is still not visible.
- [ ] Add or tighten focused tests for any changed deterministic or seeded path.

### Phase 3. Command-Layer Orchestration And Contract Overhead

Objectives:

- reduce validation launch and rerun overhead outside the core engines
- tighten preserved-finalists and cross-ticker rerun shaping
- reduce request-builder or service-layer drift that causes oversized candidate sets or misleading route expectations

Target files:

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`

Implementation detail:

- this phase should focus on end-to-end latency and contract truth, not cosmetic cleanup
- the strongest targets are:
  - duplicate canonicalization
  - unnecessary dataset slicing or cloning
  - candidate inflation
  - validation command selection drift
  - route-summary claims that do not match backend reality

Phase 3 checklist:

- [ ] Audit cross-ticker preserved-finalists rerun shaping for hidden extra work.
- [x] Audit candidate-key and dataset canonicalization paths for redundant work.
- [ ] Audit request-builder candidate selection and module exposure for oversized validation requests.
- [ ] Audit service-layer artifact follow-up if it materially inflates validation launch or replay latency.
- [ ] Add or tighten tests for any request or summary contract change.

### Phase 4. Routed Double-MA Family Coverage Hardening

Objectives:

- explicitly validate module coverage and route truth for:
  - `CompositeTrend`
  - `RegimeHybrid`
  - `BreakoutMomentum`
  - the remaining double-like profiles
- ensure inherited `DoubleMa` validation behavior stays honest for routed profiles with additional indicator or ADX constraints

Implementation detail:

- this phase is partly a coverage audit and partly a truthfulness pass
- do not assume every inherited module is equally supported once profile-specific indicator constraints are involved
- if a routed profile should remain CPU-fallback-sensitive for a module, document it and surface it truthfully

Phase 4 checklist:

- [x] Confirm module support and route truth for `CompositeTrend`.
- [x] Confirm module support and route truth for `RegimeHybrid`.
- [x] Confirm module support and route truth for `BreakoutMomentum`.
- [ ] Confirm inherited-module support for `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, and `Vfct`.
- [x] Update the tracker posture for every routed or double-like profile audited.
- [x] For every routed or double-like profile, record which remaining module gaps are true parity work versus documented exceptions.

### Phase 5. CompositeCpu And CompositeMo

Objectives:

- audit `CompositeCpu` validation module coverage and fallback rules
- isolate HalfTrend/uniform-parameter constraints that turn GPU validation into CPU fallback
- document and measure `CompositeMo` validation posture so CPU-preferred policy remains evidence-backed instead of implicit

Implementation detail:

- `CompositeCpu` and `CompositeMo` should not be treated as one validator merely because they are related product surfaces
- distinguish:
  - plain `CompositeCpu` hard-gate validation
  - `CompositeMo` validation posture
  - GPU fixed-candidate limits that only apply to plain `CompositeCpu`

Phase 5 checklist:

- [ ] Confirm current `CompositeCpu` module coverage.
- [ ] Confirm current `CompositeCpu` GPU fallback blockers.
- [ ] Confirm current `CompositeMo` validation modules and route policy.
- [ ] Decide whether any current `CompositeMo` limits are implementation gaps or deliberate posture.
- [ ] Add timing evidence if the current composite bench is too coarse to justify policy decisions.
- [ ] Record the `DoubleMa` parity gap list for both `CompositeCpu` and `CompositeMo`.

### Phase 6. Standalone Cluster A

Objectives:

- handle one strategy at a time:
  - `Asr`
  - `Vcc`
  - `Vemr`
  - `Rocb`
  - `Vcet`
- decide whether each strategy should remain CPU-preferred, stay mixed, or receive more aggressive validation-path work
- close existing module-coverage gaps where the feature already exists elsewhere in the repo and should be supported for that strategy

Implementation detail:

- do not batch all Cluster A strategies into one implementation patch
- each strategy needs:
  - current route truth
  - module coverage truth
  - timing evidence
  - a decision on whether to preserve or change posture

Phase 6 checklist:

- [ ] `Asr`: confirm why `Auto` remains CPU-preferred and whether that policy still holds.
- [x] `Vcc`: isolate CPU-reference prep cost and mixed-route justification.
- [ ] `Vemr`: confirm whether measured validation still justifies CPU preference.
- [ ] `Rocb`: confirm whether measured validation still justifies CPU preference.
- [ ] `Vcet`: keep route-truth surfaces aligned and isolate any remaining non-parity areas such as cross-ticker mode/default behavior.
- [ ] For each Cluster A strategy, record the module-by-module parity gap relative to `DoubleMa`.

### Phase 7. Standalone Cluster B

Objectives:

- handle one strategy at a time:
  - `Mkcs`
  - `Kqsr`
  - `Carm`
  - `Kvdf`
  - `Cdmc`
- prefer measured route-policy truth over speculative GPU expansion
- only upgrade GPU validation posture where per-strategy staging and exactness constraints justify it

Implementation detail:

- Cluster B should likely start with `Cdmc` because current repo notes already say it is more GPU-favorable than `Carm`
- keep the default assumption conservative:
  - if a strategy remains CPU-preferred on evidence, say so explicitly
  - do not force GPU parity work simply to make the support matrix look more uniform

Phase 7 checklist:

- [x] `Mkcs`: audit current route posture and module coverage.
- [x] `Kqsr`: audit OHLC-driven staging cost and module coverage.
- [ ] `Carm`: compare mixed-route cost against `Cdmc`.
- [x] `Kvdf`: audit HLCV staging cost, route blockers, and module support.
- [x] `Cdmc`: evaluate as the first likely Cluster B implementation candidate.
- [x] For each Cluster B strategy, record the module-by-module parity gap relative to `DoubleMa`.

### Phase 8. Special Validation Flows

Objectives:

- audit `MlSignal` validation throughput separately from the normal fixed-candidate families
- tighten validation-replay and policy-sweep latency only where existing workflow cost justifies it

Implementation detail:

- `MlSignal` should not inherit assumptions from fixed-candidate backtest validation
- timing evidence should distinguish:
  - dataset export cost
  - score-cache cost
  - policy-sweep cost
  - replay cost

Phase 8 checklist:

- [ ] Define the canonical `MlSignal` validation timing boundary.
- [ ] Audit existing module or flow coverage for `MlSignal`.
- [ ] Identify whether current pain is validation engine cost, replay cost, or policy-sweep overhead.
- [ ] Add timing or telemetry only where the current evidence is too coarse.
- [ ] Decide explicitly which `DoubleMa` baseline modules meaningfully map to `MlSignal` and which do not.

### Phase 9. Closeout And Governance Sync

Objectives:

- keep the tracker current
- update `REPO_MAP.md` when route or coverage truth materially changes
- update adjacent improvement docs when current posture changes enough to invalidate prior notes

Phase 9 checklist:

- [ ] Update the tracker after every completed implementation slice.
- [ ] Update `REPO_MAP.md` whenever strategy validation posture changes materially.
- [ ] Update validation-review docs if a prior assumption becomes stale.
- [ ] Record unresolved follow-on work explicitly instead of burying it in commit history.

## Definition Of Done For Any Strategy Bucket

- current validation modules are explicitly classified as:
  - supported
  - supported via CPU tail
  - partially supported
  - unsupported by design
- the bucket has an explicit parity assessment against the current `DoubleMa` validation baseline
- every remaining parity gap is classified as:
  - implementation work
  - deferred work
  - or documented exception
- route and fallback summaries are truthful in backend and frontend surfaces
- timing evidence exists on the correct boundary for the bucket
- targeted tests cover the changed route, fallback, or module contract
- no benchmark or profiling claims rely on overlapping local runs

## Acceptance Checklist For Any Implementation Slice

- [ ] The changed bucket has a before and after timing note on the correct evidence boundary.
- [ ] Route and fallback behavior are explicitly described and still truthful.
- [ ] Any feature-coverage change is reflected in the tracker.
- [ ] Any parity-to-`DoubleMa` change or exception is reflected in the tracker.
- [ ] Targeted tests were added or updated if behavior changed.
- [ ] Repo-wide docs were updated if current posture changed materially.
- [ ] No completed first-wave `DoubleMa` item was reopened without evidence.

## Verification Expectations

Minimum per-slice verification:

- targeted Rust tests near the touched validation surface
- targeted web tests when request or capability summaries change
- one relevant benchmark or cost-harness run for the touched bucket

Broader verification when risk is higher:

- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- bucket-specific perf or workflow evidence commands added by the implementation slice

## Review Discipline

- Prefer findings with file references and timing-boundary notes over broad qualitative claims.
- Keep implementation slices small enough that the tracker can move one row at a time.
- If a strategy remains CPU-preferred after review, record that as a valid outcome rather than as an unfinished failure.
- If a module is unsupported by design for a strategy, document it explicitly in the tracker and route summaries rather than implying silent future support.
- If a strategy cannot match the `DoubleMa` validation baseline, record the exact reason instead of leaving a vague gap.

## Immediate Next Step

Finish Phase 0 inventory and keep the companion tracker aligned to the code that already shipped. The next implementation slice should be chosen from the highest-value open runtime truths that remain on current evidence:

- the exact-combo `DoubleMa` TrendQuality ADX-off GPU parity repair behind the new CPU fallback
- `Carm`/`Cdmc` helper-route parity before any direct helper dispatch change
- or another shared validation-core hotspot if fresh timing shows the bottleneck has moved back out of the strategy-local paths
