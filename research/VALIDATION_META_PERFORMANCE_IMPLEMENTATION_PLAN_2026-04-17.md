# Validation / Meta / Holdout Performance Implementation Plan

Date: `2026-04-17`

## Surface under review

This pass is limited to the current performance-critical backend path that dominates the deterministic workflow:

- meta selection ingestion and grouping
- holdout orchestration
- Double-MA validation execution
- shared validation helpers
- cross-ticker validation orchestration
- Stage 5 policy-evaluation throughput only where it materially overlaps the same bottlenecks

It intentionally does **not** expand into:

- broad frontend or UX changes
- product-level metric definition changes
- speculative rewrites of the validation architecture without profiling evidence
- CUDA-only work that breaks CPU-only parity or CPU-only builds
- generic "make everything async / parallel" refactors

---

## Files reviewed

### Primary repo surfaces

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/ml_confidence.rs`

### Existing benchmark / gate surfaces

- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/baselines/workflow_regression_baseline.json`

### Existing style references

- `Batch review archive/bundle12/bundle12_meta_stage5_throughput_plan_v2.md`

---

## Research inputs used before implementation planning

This plan is intentionally based on repo inspection plus external performance guidance, not guesswork. The most relevant research takeaways are:

1. Benchmark and profiling need to lead the work, not follow it.
   - Rust Performance Book, Benchmarking: https://nnethercote.github.io/perf-book/benchmarking.html
   - Rust Performance Book, Profiling: https://nnethercote.github.io/perf-book/profiling.html

2. Heap allocation and clone reduction often give measurable wins in real Rust codebases.
   - Rust Performance Book, Heap Allocations: https://nnethercote.github.io/perf-book/heap-allocations.html

3. If only quantiles or medians are needed, full sorting is usually the wrong primitive.
   - Rust `slice::select_nth_unstable`: https://doc.rust-lang.org/nightly/std/primitive.slice.html#method.select_nth_unstable

4. Parallelism needs explicit granularity control; nested parallelism can regress throughput.
   - Rayon `IndexedParallelIterator`: https://docs.rs/rayon/latest/rayon/iter/trait.IndexedParallelIterator.html

5. `serde_json::Value::Object` is BTreeMap-backed by default unless `preserve_order` is enabled.
   - `serde_json::Value`: https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html

6. GPU performance work should minimize host/device transfers and keep intermediates resident on device when possible.
   - NVIDIA CUDA C++ Best Practices Guide: https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/

7. Criterion measurement overhead matters for very small benches; use the existing benches carefully rather than overfitting to noisy micro-benches.
   - Criterion timing loops: https://bheisler.github.io/criterion.rs/book/user_guide/timing_loops.html

8. If hashing becomes hot, faster hashers can matter, but only after profiling confirms that hashing is a real bottleneck.
   - Rust Performance Book, Hashing: https://nnethercote.github.io/perf-book/hashing.html

These sources support the general posture of this plan:

- measure first
- reduce repeated allocation / serialization work
- replace full sorting with partial selection where semantics allow
- avoid accidental nested parallelism
- keep GPU data resident when GPU execution is already valid

---

## Repo-local baseline and bottleneck picture

The checked-in workflow baseline at `scripts/baselines/workflow_regression_baseline.json` currently reports:

- total runtime: `229 ms`
- optimize stage: `22 ms`
- validate stage: `119 ms`
- meta-select stage: `14 ms`
- holdout stage: `55 ms`

That means the current deterministic workflow is dominated by:

1. validation
2. holdout
3. only then meta-select

This is important because it rules out a common failure mode: over-investing in small meta-select micro-optimizations while the larger wall-time sits in validation and holdout orchestration.

The correct priority order from current evidence is:

1. holdout execution path
2. validation execution path
3. meta-select grouping / canonicalization
4. cross-ticker and Stage 5 tail work

---

## Scope guard and implementation posture

Keep this work:

- benchmark-first
- parity-preserving
- backend-aware
- local and surgical
- organized around existing verification surfaces

Do **not** turn this pass into:

- a generic validation-engine rewrite
- a large "clean architecture" refactor
- speculative GPU-first behavior changes without fallback rules
- changes to persisted canonical key formats without migration planning
- a broad concurrency experiment across all loops at once

---

## Executive summary

The highest-value opportunities are concentrated in a few specific areas.

### Priority 1: Stop forcing CPU holdout reruns when a valid fixed-candidate GPU route already exists

Current state:

- `src-tauri/src/commands_meta.rs:2933`
- `src-tauri/src/commands_meta.rs:3032`

Both sites hard-force `Backend::CpuOnly` for holdout reruns.

Why this matters:

- The workflow baseline says holdout is already a large stage.
- The Double-MA validation layer already has a prepared fixed-candidate GPU route:
  - `crates/app_core/src/double_ma_validate.rs:3514`
  - `crates/app_core/src/double_ma_validate.rs:3554`
- NVIDIA guidance strongly favors keeping intermediate work on the device once GPU execution is already justified.

Expected impact:

- highest single end-to-end payoff on supported Double-MA holdout shapes
- likely multi-x gains on GPU-capable systems for non-iterative fixed-candidate batches

Main constraint:

- iterative validation is not currently parity-safe on the GPU holdout route because iterative fields are left unset around `crates/app_core/src/double_ma_validate.rs:3606`
- this must remain conditional, not global

### Priority 2: Reuse prepared CPU validation state instead of rebuilding the same MA / series work repeatedly

Current state:

- `build_candidate_series` work is repeated around:
  - `crates/app_core/src/double_ma_validate.rs:1899`
  - `crates/app_core/src/double_ma_validate.rs:1908`
  - main CPU eval at `crates/app_core/src/double_ma_validate.rs:3632`
  - CSCV at `crates/app_core/src/double_ma_validate.rs:3750`
  - finalist caches at `crates/app_core/src/double_ma_validate.rs:3810`
  - reopt walk-forward at `crates/app_core/src/double_ma_validate.rs:3951`
  - SPA at `crates/app_core/src/double_ma_validate.rs:4010`
  - perturb at `crates/app_core/src/double_ma_validate.rs:3389`
  - trade bootstrap at `crates/app_core/src/double_ma_validate.rs:3418`

Why this matters:

- this is exactly the kind of repeated allocation / repeated computation shape that the Rust Performance Book warns is often worth attacking after profiling
- holdout already groups candidates by shared MA config in `src-tauri/src/commands_meta.rs:2826`, but the CPU execution path is not exploiting that fact deeply enough

Expected impact:

- major CPU win for validation-heavy workflows
- likely high payoff even on non-GPU machines

Main constraint:

- preserve exact MA semantics, warmup behavior, NaN layout, ALMA parameters, and common-start alignment

### Priority 3: Remove orchestration-only JSON churn in meta-select and holdout

Current state:

- canonical meta grouping is hot in:
  - `crates/app_core/src/meta_select.rs:578`
  - `crates/app_core/src/meta_select.rs:1130`
  - `crates/app_core/src/meta_select.rs:1501`
- shared canonical JSON utilities also churn allocations in:
  - `crates/app_core/src/validation_shared.rs:449`
  - `crates/app_core/src/validation_shared.rs:472`
- holdout orchestration repeats `Value -> typed -> Value -> canonical string` work in:
  - `src-tauri/src/commands_meta.rs:2830`
  - `src-tauri/src/commands_meta.rs:2882`
  - `src-tauri/src/commands_meta.rs:2968`
  - `src-tauri/src/commands_meta.rs:3067`

Why this matters:

- this is pure overhead that scales with candidate count and holdout dataset fanout
- some of the current canonicalization work may be redundant because `serde_json::Value::Object` is already BTreeMap-backed by default in this repo configuration

Expected impact:

- strong medium-priority win
- especially valuable because it reduces CPU and allocation pressure before any deeper engine work starts

Main constraint:

- `MetaCandidateAggregate.key` is effectively a persisted stable identity
- internal caches are safe
- changing the external key format is not safe without migration or a strict compatibility story

### Priority 4: Replace full sorts with partial selection where only medians / quantiles are needed

Current state:

- `meta_select.rs` sorts several vectors only to read a few quantiles:
  - `crates/app_core/src/meta_select.rs:692`
  - `crates/app_core/src/meta_select.rs:716`
  - `crates/app_core/src/meta_select.rs:720`
  - `crates/app_core/src/meta_select.rs:1428`
- holdout also performs full-vector sort work in:
  - `src-tauri/src/commands_meta.rs:3124`
  - `src-tauri/src/commands_meta.rs:3137`

Why this matters:

- `select_nth_unstable` exists exactly for this use case
- these vectors are built frequently enough that avoiding `O(n log n)` full sorts is a defensible optimization

Expected impact:

- moderate but broad payoff
- low code size relative to potential throughput win

Main constraint:

- deterministic tie behavior must stay controlled
- do not silently change observable group ranking or persisted output ordering unless explicitly intended

### Priority 5: Clean up cross-ticker data copies and candidate remapping

Current state:

- `src-tauri/src/commands_validation.rs:24` copies six vectors in `slice_candles`
- `src-tauri/src/commands_validation.rs:62` reuses that copying path in horizon alignment
- candidate remapping serializes canonical JSON twice:
  - `src-tauri/src/commands_validation.rs:257`
  - `src-tauri/src/commands_validation.rs:315`

Why this matters:

- cross-ticker is not the top baseline bottleneck, but it is structurally expensive and easy to worsen later if left unaddressed

Expected impact:

- medium payoff on cross-ticker-heavy workloads
- lower priority than validation / holdout / meta

Main constraint:

- preserve the timestamp-overlap-first and tail-fallback alignment contract in `src-tauri/src/commands_validation.rs:72`
- positional candidate matching is unsafe because results are resorted before return in `crates/app_core/src/validation_pipeline.rs:1446`

---

## Detailed findings and implementation guidance

## Finding 1: Holdout reruns are hard-forced to CPU

- Severity: high
- Type: performance
- Timing boundary affected: end-to-end workflow and holdout stage
- Primary locations:
  - `src-tauri/src/commands_meta.rs:2933`
  - `src-tauri/src/commands_meta.rs:3032`
  - `crates/app_core/src/double_ma_validate.rs:3514`
  - `crates/app_core/src/double_ma_validate.rs:3554`

### Why it matters

The holdout stage is already `55 ms` in the checked-in workflow baseline. The current orchestration throws away an available acceleration path by always recompiling reruns for `Backend::CpuOnly`, even when the request shape already matches the fixed-candidate Double-MA GPU validation route.

### Recommended change

Implement backend-aware holdout request compilation:

1. Introduce a narrow helper in `commands_meta.rs` that decides the holdout backend per group.
2. Allow the GPU fixed-candidate route only when:
   - strategy family is supported
   - request is non-iterative
   - current fixed-candidate validation hard gates are satisfied
3. Fall back to CPU otherwise.
4. Keep this behind a narrow rollout flag or explicit capability gate first, then make it default only after parity proof.

### Constraints

- preserve current CPU fallback behavior
- preserve holdout scoring semantics in:
  - `src-tauri/src/commands_meta.rs:2983`
  - `src-tauri/src/commands_meta.rs:3082`
- do not route iterative mode through the current GPU path until iterative fields are fully populated and parity-tested

### Verification

- add focused tests that prove backend selection is unchanged for unsupported shapes
- add GPU-vs-CPU parity tests for supported fixed-candidate holdout batches
- run:
  - `cargo test -p ta_app_core`
  - `cargo test --workspace`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- on a GPU-capable machine, run:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Finding 2: Holdout request compilation repeats dataset-invariant JSON work

- Severity: high
- Type: performance
- Timing boundary affected: holdout stage
- Primary locations:
  - `src-tauri/src/commands_meta.rs:2826`
  - `src-tauri/src/commands_meta.rs:2830`
  - `src-tauri/src/commands_meta.rs:2871`
  - `src-tauri/src/commands_meta.rs:2882`
  - `src-tauri/src/commands_meta.rs:2968`
  - `src-tauri/src/commands_meta.rs:3013`
  - `src-tauri/src/commands_meta.rs:3023`
  - `src-tauri/src/commands_meta.rs:3067`

### Why it matters

The current holdout loop repeatedly:

- builds JSON string config keys
- deserializes representative params from `Value`
- reruns validation
- serializes validated params back to JSON
- recomputes canonical group keys

That is pure orchestration cost. It scales with `datasets x groups x representatives`.

### Recommended change

Add an internal typed holdout compilation phase:

1. Compile representative params once per group before entering the dataset loop.
2. Preserve the existing permissive decode behavior, including legacy alias handling.
3. Carry an internal `compiled_group_key` or stable canonical byte key through reruns instead of re-materializing JSON every time.
4. Keep the existing external persisted key shape intact.

### Constraints

- preserve legacy compatibility for representative decoding
- do not change persisted meta key strings
- avoid introducing duplicate internal key implementations unless the shared canonicalizer is unified at the same time

### Verification

- add tests around legacy representative decode fallback
- add tests that identical groups still collapse identically
- add bench coverage or stage diagnostics in:
  - `crates/app_core/benches/e2e_workflow_ml.rs`
  - `src-tauri/src/bin/ta_workflow_regression.rs`

## Finding 3: CPU Double-MA validation rebuilds MA matrices and candidate series too often

- Severity: high
- Type: performance
- Timing boundary affected: validation stage and CPU holdout
- Primary locations:
  - `crates/app_core/src/double_ma_validate.rs:1899`
  - `crates/app_core/src/double_ma_validate.rs:1908`
  - `crates/app_core/src/double_ma_validate.rs:3632`
  - `crates/app_core/src/double_ma_validate.rs:3750`
  - `crates/app_core/src/double_ma_validate.rs:3810`
  - `crates/app_core/src/double_ma_validate.rs:3951`
  - `crates/app_core/src/double_ma_validate.rs:4010`
  - `crates/app_core/src/double_ma_validate.rs:3389`
  - `crates/app_core/src/double_ma_validate.rs:3418`

### Why it matters

The same derived state is rebuilt across the main evaluation path and downstream validation phases. This is likely the biggest CPU-side waste in the codebase right now.

### Recommended change

Introduce a staged prepared CPU validation state for Double-MA:

1. Separate "dataset-derived / MA-derived state" from "phase-specific scoring logic".
2. Build candidate-series state once per request batch.
3. Reuse it for:
   - base validation
   - CSCV
   - finalist-only auxiliary metrics
   - reopt walk-forward
   - SPA
   - perturb
   - trade bootstrap
4. Keep memory bounded by reusing only the data needed for downstream enabled phases.

### Constraints

- preserve common-start and warmup alignment from:
  - `crates/app_core/src/validation_core.rs:42`
  - `crates/app_core/src/validation_pipeline.rs:1390`
- keep CPU-only builds working
- avoid a blanket cache that holds large vectors longer than necessary

### Verification

- add or extend Double-MA workflow benches:
  - `crates/app_core/benches/double_ma_validation_workflow.rs`
- run:
  - `cargo test -p ta_app_core`
  - `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`

## Finding 4: Finalist auxiliary analytics recompute the same step-return bundle

- Severity: medium
- Type: performance
- Timing boundary affected: validation stage tail latency
- Primary locations:
  - `crates/app_core/src/double_ma_validate.rs:2153`
  - `crates/app_core/src/double_ma_validate.rs:2321`
  - `crates/app_core/src/double_ma_validate.rs:2333`
  - `crates/app_core/src/double_ma_validate.rs:2501`
  - `crates/app_core/src/double_ma_validate.rs:2600`
  - `crates/app_core/src/double_ma_validate.rs:2799`
  - `crates/app_core/src/double_ma_validate.rs:3826`

### Why it matters

When multiple auxiliary summaries are enabled together, the same expensive step-return preparation is rebuilt several times inside the finalist loop.

### Recommended change

Cache an immutable per-finalist step-return bundle and reuse it across:

- regime-slice robustness
- entry-margin fragility
- underwater-duration analysis
- checkpoint robustness
- benchmark slice summary

### Constraints

- cache key must include the exact series and `start_t` assumptions
- avoid retaining bulky per-finalist data when those auxiliary paths are disabled

### Verification

- add targeted unit tests around shared-step-return helper equivalence
- extend Double-MA validation bench coverage with auxiliary metrics enabled

## Finding 5: SPA bootstrap recomputes mean / std inside each sample

- Severity: medium
- Type: performance
- Timing boundary affected: SPA-heavy validation runs
- Primary locations:
  - `crates/app_core/src/validation_pipeline.rs:1520`
  - `crates/app_core/src/validation_pipeline.rs:1564`
  - `crates/app_core/src/validation_pipeline.rs:1589`
  - `crates/app_core/src/validation_pipeline.rs:1603`

### Why it matters

The SPA implementation already computes invariant candidate diffs once, but it still recomputes mean and variance-like quantities inside each bootstrap sample.

### Recommended change

Refactor SPA bootstrap to:

1. precompute reusable per-candidate base statistics
2. separate resample accumulation from invariant candidate metadata
3. preserve arithmetic order where needed for parity

### Constraints

- if floating-point order changes, prove tolerance or preserve order exactly
- do not compromise cancellation or progress reporting

### Verification

- add focused SPA regression tests
- benchmark `spa` and `wf_reopt_cscv_spa` in `double_ma_validation_workflow.rs`

## Finding 6: Meta-select canonical group-key generation is allocation-heavy

- Severity: high
- Type: performance
- Timing boundary affected: meta-select stage and holdout preparation
- Primary locations:
  - `crates/app_core/src/meta_select.rs:578`
  - `crates/app_core/src/meta_select.rs:653`
  - `crates/app_core/src/meta_select.rs:1130`
  - `crates/app_core/src/meta_select.rs:1501`
  - `crates/app_core/src/validation_shared.rs:449`
  - `crates/app_core/src/validation_shared.rs:472`

### Why it matters

Every shortlisted candidate rebuilds JSON wrappers, recursively canonicalizes values, and emits a fresh canonical string. The clustered path also eagerly computes representative signatures. This is one of the hottest non-validation CPU paths in the repo.

### Recommended change

Implement the following in order:

1. precompute artifact-level context signatures once
2. add a `Value`-native canonical-key helper so the code can avoid repeated `to_value -> deep clone -> to_string`
3. make representative signature materialization lazy in clustered mode
4. reserve map / vector capacity where candidate counts are known up front

### Constraints

- preserve exact external key bytes for persisted identities
- do not assume object order is insertion-order based; in this repo configuration it is BTreeMap-backed by default
- if a writer-based canonical serializer is introduced, prove output compatibility with the old path

### Verification

- add golden tests for canonical key equivalence
- extend:
  - `crates/app_core/benches/meta_select_workflow.rs`
- run:
  - `cargo test -p ta_app_core`
  - `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`

## Finding 7: Full sorting is used where partial selection is enough

- Severity: medium
- Type: performance
- Timing boundary affected: meta-select, holdout post-processing, policy-evaluation ranking
- Primary locations:
  - `crates/app_core/src/meta_select.rs:692`
  - `crates/app_core/src/meta_select.rs:716`
  - `crates/app_core/src/meta_select.rs:720`
  - `crates/app_core/src/meta_select.rs:1428`
  - `src-tauri/src/commands_meta.rs:3124`
  - `src-tauri/src/commands_meta.rs:3137`

### Why it matters

The code frequently sorts full vectors to read median, p10, or p90 values. The standard library already exposes `select_nth_unstable`, which is the correct primitive when full ordering is not needed.

### Recommended change

1. Add a small internal quantile helper that uses `select_nth_unstable_by`.
2. Keep full sort only where stable full ordering is semantically required.
3. For policy-evaluation ranking, consider a bounded top-k heap if the consumer only needs a prefix.

### Constraints

- preserve deterministic tie behavior for externally visible outputs
- document any remaining places where full sort is intentionally kept

### Verification

- add tests comparing old and new quantile results
- run meta-select and policy-eval benchmark paths before and after

## Finding 8: Cross-ticker clones full candle vectors and remaps with canonical JSON strings

- Severity: medium
- Type: performance
- Timing boundary affected: cross-ticker validation
- Primary locations:
  - `src-tauri/src/commands_validation.rs:24`
  - `src-tauri/src/commands_validation.rs:62`
  - `src-tauri/src/commands_validation.rs:220`
  - `src-tauri/src/commands_validation.rs:257`
  - `src-tauri/src/commands_validation.rs:266`
  - `src-tauri/src/commands_validation.rs:295`
  - `src-tauri/src/commands_validation.rs:315`
  - `src-tauri/src/validation_common.rs:338`
  - `src-tauri/src/validation_common.rs:379`

### Why it matters

This path currently pays for:

- full candle deserialization
- optional full subcopy for aligned windows
- repeated candidate cloning
- canonical JSON serialization for candidate remapping
- full sorting plus an `O(C^2)` Pareto pass

### Recommended change

Implement in stages:

1. add a view/range-based candle slicing model instead of immediate owned copies
2. add a stable internal candidate id carried across base and rerun results
3. reduce request cloning by sharing immutable candidate params
4. replace median full sorts with partial selection
5. evaluate a cheaper frontier routine only if survivor counts justify it

### Constraints

- preserve exact alignment contract in `commands_validation.rs:72`
- do not switch to positional candidate matching
- keep dataset-level parallelism disabled by default until oversubscription risk is measured

### Verification

- add tests for overlap-first and tail-fallback alignment
- add cross-ticker-specific benchmark coverage if this path becomes a priority

## Finding 9: Stage 5 policy evaluation still has avoidable sort and retraining cost

- Severity: medium
- Type: performance
- Timing boundary affected: offline policy evaluation
- Primary locations:
  - `src-tauri/src/commands_meta.rs:1279`
  - `src-tauri/src/commands_meta.rs:1389`

### Why it matters

This path is not the top workflow bottleneck today, but it will remain expensive on long meta histories because it retrains for every prefix and performs multiple ranking passes.

### Recommended change

Short term:

- replace full ranking sorts with partial top-k selection where applicable
- remove avoidable group cloning

Medium term:

- investigate checkpoint or incremental prefix training only if identical output can be proven

### Constraints

- do not change the meaning of the historical policy-evaluation metric
- treat approximate or statistically similar output as insufficient unless explicitly accepted

### Verification

- extend `crates/app_core/benches/e2e_workflow_ml.rs`
- compare before/after output artifacts for bit-for-bit or tolerance-defined parity

---

## Recommended implementation order

## Phase 0: Measurement scaffolding and guardrails

Goal:

- make every later optimization measurable

Tasks:

- confirm `ta_workflow_regression` still reflects the target workflow
- add per-stage diagnostics where blind spots remain
- decide which workstreams need Criterion coverage vs workflow-gate coverage vs both

Deliverables:

- clear before/after numbers for holdout, validation, and meta-select
- at least one targeted bench for each high-priority workstream

## Phase 1: Low-risk orchestration wins

Goal:

- remove pure JSON and clone overhead before touching execution semantics

Tasks:

- typed holdout representative compilation
- internal cached group keys
- meta-select context-signature caching
- selection-based quantile helpers

Why first:

- these are high-signal, low-risk improvements
- they shrink CPU noise before more structural work

## Phase 2: CPU validation-state reuse

Goal:

- stop rebuilding candidate-series state across downstream phases

Tasks:

- introduce prepared batch state
- reuse it across base validation and downstream enabled phases
- cache finalist step-return bundles

Why second:

- this is likely the largest CPU-side payoff
- best done after orchestration noise is reduced

## Phase 3: Conditional GPU holdout route

Goal:

- unlock the highest end-to-end payoff without compromising correctness

Tasks:

- implement backend-aware holdout request selection
- gate GPU use to parity-safe request shapes only
- add CPU fallback and parity tests

Why third:

- highest upside
- medium correctness risk
- easiest to validate once CPU-side orchestration and caching are cleaner

## Phase 4: Validation algorithm cleanups

Goal:

- improve SPA / CSCV / tail analytics after the main reuse work lands

Tasks:

- SPA invariant-stat reuse
- CSCV preparation cleanup
- additional partial-selection replacements

## Phase 5: Cross-ticker cleanup

Goal:

- address a structurally expensive path without destabilizing core validation

Tasks:

- view-based slicing
- candidate-id remapping
- request-sharing cleanup

## Phase 6: Stage 5 tail work

Goal:

- optimize policy evaluation only after the larger workflow bottlenecks have been addressed

Tasks:

- bounded ranking
- cloning cleanup
- only then revisit identical-output prefix-training reuse

---

## Verification matrix

Use the existing repo tooling instead of ad hoc timing.

### Functional verification

- `cargo test -p ta_app_core`
- `cargo test --workspace`

### Workflow-level performance verification

- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`

### GPU-specific verification when relevant

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

### Targeted bench surfaces

- meta-select: `crates/app_core/benches/meta_select_workflow.rs`
- end-to-end meta / holdout / ML: `crates/app_core/benches/e2e_workflow_ml.rs`
- Double-MA validation: `crates/app_core/benches/double_ma_validation_workflow.rs`

### Profiling guidance

If measured regressions or unclear wins remain:

- use a sampling profiler first
- use allocation profiling when JSON / clone churn appears dominant
- only add more concurrency after measuring current worker saturation

---

## Risks and non-negotiable invariants

1. Do not change holdout scoring semantics by accident.
2. Do not change persisted canonical key strings without a compatibility plan.
3. Do not parallelize outer loops on top of inner Rayon loops blindly.
4. Do not break CPU-only builds or CPU-only runtime behavior.
5. Do not change warmup / common-start alignment.
6. Do not assume positional candidate mapping is safe after validation sorting.
7. Do not enable GPU holdout globally before iterative-mode parity exists.

---

## Detailed checklist

- [ ] Confirm current workflow baseline numbers from `scripts/baselines/workflow_regression_baseline.json` are still the active comparison point.
- [ ] Add or confirm targeted benchmark coverage for meta-select, holdout, and Double-MA validation paths before structural changes.
- [ ] Add explicit before/after stage diagnostics for any path not already visible in `ta_workflow_regression`.
- [ ] Implement an internal typed holdout representative compilation layer in `commands_meta.rs`.
- [ ] Preserve legacy representative decode compatibility, including permissive field handling.
- [ ] Replace repeated holdout JSON key reconstruction with cached internal group-key material.
- [ ] Add golden tests proving external meta-group keys remain unchanged.
- [ ] Add internal `Value`-native canonical-key helper to reduce `to_value -> clone -> to_string` churn.
- [ ] Precompute artifact-level context signatures in `meta_select.rs`.
- [ ] Make clustered representative signature generation lazy where possible.
- [ ] Replace full-sort quantile extraction with `select_nth_unstable` helpers where full ordering is not required.
- [ ] Add deterministic tie-handling tests for any quantile / ranking helper changes.
- [ ] Introduce prepared CPU Double-MA validation state for reused MA / candidate-series work.
- [ ] Reuse prepared state across base validation, CSCV, finalist processing, reopt walk-forward, SPA, perturb, and trade bootstrap where valid.
- [ ] Add bounded caches for finalist step-return bundles.
- [ ] Verify common-start / warmup alignment is unchanged after validation-state reuse.
- [ ] Refactor SPA bootstrap to reuse invariant candidate statistics.
- [ ] Add focused SPA parity tests.
- [ ] Implement backend-aware holdout backend selection in `commands_meta.rs`.
- [ ] Keep GPU holdout disabled for iterative or unsupported request shapes.
- [ ] Add GPU-vs-CPU parity coverage for supported fixed-candidate Double-MA holdout batches.
- [ ] Preserve current CPU fallback behavior for unsupported or parity-risky shapes.
- [ ] Add a view/range-based candle slicing path for cross-ticker validation.
- [ ] Keep timestamp-overlap-first and tail-fallback behavior unchanged.
- [ ] Introduce a stable internal candidate id for cross-ticker remapping.
- [ ] Avoid positional matching for cross-ticker candidate reconciliation.
- [ ] Reduce cross-ticker request cloning by sharing immutable candidate-param storage where practical.
- [ ] Revisit cross-ticker post-processing sorts only after the larger data-path costs are addressed.
- [ ] Replace obvious Stage 5 full-sort ranking work with bounded selection where semantics allow.
- [ ] Avoid speculative incremental Stage 5 training until identical-output proof exists.
- [ ] Run `cargo test -p ta_app_core`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`.
- [ ] Run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`.
- [ ] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` for any GPU-route changes on a GPU-capable machine.
- [ ] Record before/after stage numbers and attach them to the implementation PR or follow-up report.

---

## Recommended first implementation batch

If this work is split into the fastest safe first pass, the first batch should be:

1. typed holdout representative compilation and cached internal group keys
2. meta-select context-signature caching and shared canonicalizer cleanup
3. partial-selection replacement for meta-select / holdout quantiles
4. prepared CPU Double-MA validation-state reuse

That batch keeps the work concentrated on:

- the current dominant CPU path
- low-risk orchestration waste
- performance work that can be validated with existing benches and gates

The GPU holdout route should be the next batch, not the first one, because its upside is high but the parity surface is narrower and more fragile.

---

## Closing assessment

There is clear room for further performance improvement in this repo. The strongest opportunities are not generic "optimize Rust" work; they are specific to the current architecture:

- holdout reruns are using the wrong backend for some supported shapes
- validation is rebuilding too much derived state
- meta-select and holdout are paying unnecessary JSON / canonicalization overhead
- some quantile and cross-ticker paths are using heavier primitives than necessary

The repo already has enough performance infrastructure to make these changes responsibly. The correct next step is to implement the first batch with benchmark-backed checkpoints after each workstream, not to attempt one large combined optimization pass.
