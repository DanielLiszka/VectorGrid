# Next-Wave Repo Improvement Implementation Plan

Date: `2026-04-17`

## Current constraint

This plan is being written while a CPU-intensive and GPU-intensive task is already running in the background on the local machine.

That matters for execution strategy:

- avoid new heavyweight benchmark runs while the background load is active
- avoid GPU profiling or large workspace rebuild loops until the machine is idle
- prefer low-impact planning, targeted code reading, narrow edits, and verification steps that can be deferred cleanly

This document is therefore an implementation plan, not an execution log.

---

## Why these items are the right next steps

The most useful next work falls into five practical buckets:

1. reduce remaining JSON and allocation churn in the meta / holdout path
2. keep pushing validation toward reusable prepared state instead of repeated recomputation
3. harden workspace build and test behavior on Windows so success does not depend on ad hoc manual serialization
4. finish artifact canonicalization unification so save and repair semantics cannot drift
5. expand performance infrastructure so future optimization passes are allocation-aware and not just runtime-aware

There is also one larger follow-up architecture lane:

6. move more validation-stage work onto true GPU-resident execution for supported strategy families

These are ordered by expected usefulness, implementation safety, and repo-wide payoff.

---

## Files and surfaces to treat as primary entry points

### Performance / orchestration

- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

### Build and workflow hardening

- `src-tauri/Cargo.toml`
- `Cargo.toml`
- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `README.md`
- `AGENTS.md`
- `src-tauri/src/bin/*.rs`

### Artifact normalization and save / repair semantics

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_campaign.rs`

---

## Non-goals

This plan intentionally does not propose:

- removing any product functionality
- reducing CPU-only support
- introducing CUDA-only assumptions into shared paths
- broad frontend redesign
- speculative rewrites without parity tests
- large generic abstraction layers that make strategy-local code harder to follow

---

## Recommended implementation order

1. Build-memory and workspace hardening
2. Meta / holdout typed fingerprint and allocation reductions
3. Validation prepared-state reuse expansion
4. Artifact canonicalization unification
5. Perf infrastructure expansion
6. Composite-family validation GPU residency follow-up

Reasoning:

- item 1 improves day-to-day safety and unblocks broader iteration on Windows
- items 2 and 3 are the best near-term performance return
- item 4 removes save vs repair drift and lowers maintenance cost
- items 5 and 6 are important, but benefit from the earlier cleanup first

---

## Workstream 1: Workspace Build-Memory Hardening

### Problem

The repo already acknowledges Windows workspace-test instability and pagefile / linker pressure:

- `scripts/run_repo_quality_gate.ps1` already runs `cargo test --workspace -j 1` for the optional workspace leg
- `AGENTS.md` still presents raw `cargo test --workspace` as the default unit-test command
- `src-tauri` auto-discovers a large set of binaries under `src-tauri/src/bin/`

The current `src-tauri` bin surface includes:

- `ta_app_path_faceoff.rs`
- `ta_campaign_perf.rs`
- `ta_double_ma_validation_cost.rs`
- `ta_headless_validate.rs`
- `ta_options_dataset_download.rs`
- `ta_options_dataset_import.rs`
- `ta_options_expression_compare.rs`
- `ta_options_expression_feasibility.rs`
- `ta_options_expression_smoke.rs`
- `ta_options_parent_artifact.rs`
- `ta_profile_route_matrix.rs`
- `ta_refactor_snapshots.rs`
- `ta_workflow_regression.rs`

These bins are useful, but compiling all of them in the workspace test surface increases memory pressure and can trip Windows nightly toolchain instability.

### Objective

Make the repo’s full test/build workflow deterministic on this class of machine without removing any CLI utility or feature.

### Proposed implementation

#### Phase 1: Formalize the serialized workspace test path

- Add a dedicated wrapper script, for example:
  - `scripts/run_workspace_tests_stable.ps1`
- Behavior:
  - set `CARGO_BUILD_JOBS=1`
  - optionally clear wrapper env vars the same way `run_repo_quality_gate.ps1` already does
  - run `cargo test --workspace -j 1`
  - print a short rationale about Windows pagefile / linker / nightly stability

#### Phase 2: Update docs and operator guidance

- Update `README.md`
- Update `AGENTS.md`
- Change the recommended workspace command from a raw `cargo test --workspace` instruction to:
  - the stable wrapper script, or
  - explicit `cargo test --workspace -j 1` on Windows

#### Phase 3: Audit auto-discovered bins

- Convert the most expensive auto-discovered bins to explicit `[[bin]]` entries in `src-tauri/Cargo.toml`
- For bins that do not need to be compiled as test harnesses, set:
  - `test = false`
- For bins that are optional tooling only, consider:
  - `required-features = [...]`
  - or moving them into a separate crate dedicated to perf / tooling / research CLIs

This does not remove those binaries. It only reduces accidental workspace-test surface area.

#### Phase 4: Shard heavy integration surfaces

- Identify whether `src-tauri/tests/*.rs` and large bin harnesses should be split into:
  - stable functional workspace tests
  - tooling / regression / report-generation lanes
- Make those lanes separately invokable from scripts

### Acceptance criteria

- full workspace tests pass reliably on Windows with the recommended documented command
- no product CLI is removed
- CI/local instructions stop implying unsafe default parallel workspace compiles on heavy machines

### Risks

- setting `test = false` on the wrong bin can hide bin-local unit tests
- moving bins into a separate crate can complicate pathing if done too broadly

### Checklist

- [ ] Add `scripts/run_workspace_tests_stable.ps1`
- [ ] Update `README.md` workspace test guidance
- [ ] Update `AGENTS.md` workspace test guidance
- [ ] Audit each `src-tauri/src/bin/*.rs` file for true test-harness need
- [ ] Convert qualifying bins to explicit `[[bin]]` entries with `test = false`
- [ ] Keep user-facing CLI behavior unchanged
- [ ] Add a short rationale note to the repo quality gate docs

---

## Workstream 2: Meta / Holdout Typed Fingerprints and Allocation Reduction

### Problem

The prior performance pass removed several expensive sort-heavy and canonicalization-heavy operations, but the meta / holdout surface is still orchestration-heavy and still spends real effort on:

- repeated `serde_json::Value` handling
- canonical JSON key generation for grouping and fanout
- repeated short-lived vectors and maps
- repeated decode / re-encode patterns in holdout batch assembly

Primary hot surfaces:

- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/validation_shared.rs`

### Objective

Reduce orchestration overhead without changing ranking semantics, candidate identity semantics, or saved artifact behavior.

### Proposed implementation

#### Phase 1: Introduce typed key wrappers for hot grouping paths

Add explicit lightweight key types for:

- representative candidate identity
- holdout config identity
- meta-group identity where the grouping contract is already stable

Candidate shape:

```rust
struct RepresentativeKey(Arc<str>);
struct HoldoutConfigKey(Arc<str>);
struct MetaGroupKey(Arc<str>);
```

The main point is not the wrapper itself. The point is to:

- compute the canonical string once
- carry it explicitly
- stop regenerating it across inner loops

#### Phase 2: Split “transport JSON” from “execution typed state”

In `src-tauri/src/commands_meta.rs`, keep `Value` only at ingress / egress boundaries and convert earlier into typed execution payloads.

Preferred shape:

- request normalization phase
- typed prepared-holdout-batch phase
- execution phase
- response serialization phase

This keeps the JSON boundary near the edges instead of across the inner orchestration loops.

#### Phase 3: Reuse scratch buffers inside holdout summarization

Introduce explicit reusable scratch structs for repeated metrics aggregation.

Example direction:

```rust
struct HoldoutSummaryScratch {
    scores: Vec<f64>,
    sharpes: Vec<f64>,
    max_dds: Vec<f64>,
    pnls: Vec<f64>,
}
```

Use `clear()` plus reserved capacity instead of rebuilding small vectors repeatedly per batch or per candidate family.

#### Phase 4: Audit `HashMap` capacity and fanout structures

Where group sizes are known or bounded from prepared batches:

- use `with_capacity`
- avoid repeated growth / rehash where practical

This is especially relevant for:

- candidate fanout maps
- representative lookup maps
- per-group accumulators

### Acceptance criteria

- same meta-selection outputs for representative deterministic test fixtures
- same holdout summary outputs for existing tests
- fewer repeated canonical-key builds inside inner orchestration loops
- no change to saved artifact schema

### Risks

- a typed-key refactor can accidentally change equivalence semantics if canonicalization order changes
- premature pooling can make code harder to read if scratch ownership is not clear

### Checklist

- [ ] Add explicit typed key wrappers for hot meta / holdout grouping surfaces
- [ ] Move `Value` handling toward command boundaries
- [ ] Introduce reusable holdout summary scratch buffers
- [ ] Pre-size hot `HashMap` / `Vec` allocation sites where cardinality is known
- [ ] Add parity tests for representative grouping and holdout fanout
- [ ] Confirm no saved artifact field names or candidate identity semantics change

---

## Workstream 3: Validation Prepared-State Reuse Expansion

### Problem

`double_ma_validate` and `validation_pipeline` still contain repeated work that is logically derived from the same candidate series or the same per-candidate metric collections.

The previous pass already introduced cached step-return bundles in:

- `crates/app_core/src/double_ma_validate.rs`

That idea is useful and should be extended rather than abandoned.

Relevant surfaces:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/composite_validate.rs`

### Objective

Reduce repeated derived-series computation and repeated summary-vector materialization without changing validation semantics.

### Proposed implementation

#### Phase 1: Formalize a reusable prepared-candidate context

Introduce a candidate-local prepared bundle for reusable derived state.

Possible contents:

- aligned MA outputs
- entry / active flags
- step returns
- spread ratio series
- optional regime-state views
- any repeatedly consumed trade or drawdown helper arrays

Preferred property:

- one owner
- clearly documented lifetime
- no ambiguity about whether a downstream helper may mutate or borrow

#### Phase 2: Thread prepared context through validation modules

Expand bundle-aware helper usage across:

- CSCV preparation
- perturb statistics
- trade bootstrap
- SPA validation inputs where the same base information is already available
- walk-forward helpers where repeated candidate-local prep exists

#### Phase 3: Extend the same pattern into composite validation

`crates/app_core/src/composite_validate.rs` calls into `validation_pipeline` heavily and is large enough that repeated summary prep likely exists there too.

Do not try to genericize both paths immediately.

Instead:

- identify the repeated composite candidate prep
- add a strategy-local prepared bundle
- wire it through the repeated modules

That stays aligned with the repo’s strategy-local architecture guidance in `AGENTS.md`.

#### Phase 4: Remove more sort-heavy summary paths

Continue replacing “sort a full vector only to read one or two quantiles” with:

- `select_nth_unstable`-style nearest-rank helpers
- or streaming summaries where exact medians are not required

Priority surfaces:

- bootstrap quantiles
- walk-forward summary lanes
- candidate-level validation summary aggregators

### Acceptance criteria

- identical candidate verdicts and summary metrics on deterministic fixtures
- same aux-stat outputs for finalized candidates
- fewer repeated per-candidate derived-series builds in validation

### Risks

- prepared-state reuse can create subtle stale-data bugs if reused across incompatible candidate contexts
- mutability discipline must stay explicit

### Checklist

- [ ] Define a reusable prepared-candidate context for Double-MA validation
- [ ] Expand bundle-aware helpers beyond the current finalist aux-stat subset
- [ ] Audit `validation_pipeline.rs` for remaining full-sort quantile paths
- [ ] Add parity tests comparing legacy and prepared-state outputs
- [ ] Repeat the same strategy-local pattern for `composite_validate.rs`
- [ ] Keep CPU-only behavior and exact warmup semantics unchanged

---

## Workstream 4: Artifact Canonicalization Unification

### Problem

There was already a real example of save-vs-repair drift in the run-artifact path. The fix aligned `save_run_artifact_blocking` with repair normalization, but the architecture should go further so this cannot silently diverge again.

Primary surface:

- `src-tauri/src/run_artifacts.rs`

Secondary callers:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_campaign.rs`

### Objective

Guarantee that canonical save behavior and repair behavior share the same normalization contract by construction.

### Proposed implementation

#### Phase 1: Introduce a single canonical normalization entrypoint

Preferred internal shape:

- alias promotion
- dataset meta normalization
- route / lineage / fingerprint backfill
- dataset identity backfill
- final canonical serialization

The save path and repair path should both pass through the same normalization function family, differing only in:

- whether the source input is already persisted
- whether the result is written in place or to another path
- whether a dry-run response is requested

#### Phase 2: Reduce duplicate backfill logic

Audit whether any of the following are currently duplicated across save / repair / summary paths:

- route provenance extraction
- config fingerprint version filling
- dataset identity backfill
- tag normalization
- lineage extraction

Where duplication exists, centralize it.

#### Phase 3: Add stronger contract tests

Add tests that explicitly assert:

- save then repair is a no-op
- repair then save is a no-op
- report save / artifact reopen paths preserve canonicalized artifacts

### Acceptance criteria

- no save-vs-repair drift remains for supported artifact kinds
- canonical repair on a fresh canonical save is a no-op
- artifact-related report flows stay loadable

### Risks

- centralizing too much can make error messages less specific if care is not taken
- some legacy alias migrations may need to remain caller-sensitive

### Checklist

- [ ] Introduce one canonical normalization entrypoint for save / repair
- [ ] Remove duplicated backfill logic where it is behaviorally identical
- [ ] Add save-then-repair and repair-then-save no-op tests
- [ ] Verify report and campaign artifact callers remain compatible
- [ ] Keep all supported legacy alias migrations intact

---

## Workstream 5: Perf Infrastructure Expansion

### Problem

The repo has strong runtime performance gates, but future work would benefit from more direct coverage of:

- meta-selection throughput
- holdout orchestration throughput
- artifact normalization throughput
- allocation and memory behavior

Current helpful surfaces already exist:

- `scripts/run_cpu_perf_gate.ps1`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `crates/app_core/benches/*`

### Objective

Make future performance work evidence-backed at the exact surfaces being optimized.

### Proposed implementation

#### Phase 1: Add targeted benches

New bench candidates:

- meta selection grouping and ranking
- holdout prepared-batch build
- holdout summary aggregation
- run-artifact save / repair canonicalization
- report generation path that exercises validation workbook build

#### Phase 2: Add allocation-aware reporting

If practical without destabilizing the bench environment:

- record peak RSS or rough process memory around select benchmark commands
- or add an optional allocation-count mode for local diagnosis

This should be opt-in and should not turn the standard perf gate into a noisy harness.

#### Phase 3: Add machine-idle guidance

Document which gates are safe while a background workload is active and which should wait.

Recommended split:

- safe while machine is busy:
  - narrow unit tests
  - doc updates
  - lightweight file-scoped checks
- wait until machine is idle:
  - `run_cpu_perf_gate.ps1`
  - `run_gpu_perf_gate.ps1`
  - full Criterion sweeps
  - Nsight / CUDA profiling

### Acceptance criteria

- each major optimized surface has a direct benchmark or gate
- perf regressions are easier to attribute to a specific subsystem
- contributors have clearer instructions about when to run heavy gates

### Risks

- too many benches can increase maintenance overhead
- memory metrics can be noisy if collected without care

### Checklist

- [ ] Add targeted benches for meta / holdout / artifact normalization
- [ ] Keep heavy perf gates separate from fast local verification
- [ ] Document “busy machine” vs “idle machine” verification guidance
- [ ] Keep the existing workflow regression gate as the top-level integration signal

---

## Workstream 6: Composite Validation GPU Residency Follow-Up

### Problem

The repo’s architecture notes and `AGENTS.md` target GPU-executed end-to-end stages where practical. The next large performance win after the current CPU/orchestration cleanup is likely deeper validation-stage GPU residency for supported strategy families.

Relevant surfaces:

- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### Objective

Increase validation-stage GPU residency for supported strategies without regressing CPU fallback or product route clarity.

### Proposed implementation

#### Phase 1: Explicit capability inventory

For each supported profile family, document:

- optimization GPU route
- validation GPU route
- fallback route
- blockers to true validation-stage residency

#### Phase 2: Strategy-local validation GPU plans

Do not build a giant generic framework first.

Instead, plan strategy-local execution lanes for:

- `CompositeTrend`
- `BreakoutMomentum`
- `RegimeHybrid`

aligned with the route policy already documented in `AGENTS.md`.

#### Phase 3: Keep device-resident intermediates through validation submodules

Focus on:

- avoiding needless host round-trips
- keeping device price buffers and derived series alive for the full validation stage where parity permits
- preserving CPU fallback and hybrid behavior

### Acceptance criteria

- route policy remains explicit and truthful
- CPU-only builds continue to work unchanged
- GPU validation-stage wins are backed by targeted perf evidence

### Risks

- this is the highest-complexity workstream in the document
- mixing optimization-stage and validation-stage residency assumptions can create subtle correctness issues

### Checklist

- [ ] Build a per-strategy validation GPU capability map
- [ ] Identify one strategy family for the first GPU validation residency slice
- [ ] Keep the work strategy-local instead of genericizing too early
- [ ] Require targeted perf validation and parity tests before rollout

---

## Verification plan by machine state

### Safe while the background CPU/GPU task is active

- read-only repo inspection
- implementation-plan writing
- small documentation edits
- targeted file-scoped unit tests only if strictly needed
- narrow script edits that do not force workspace rebuilds

### Wait until the machine is idle

- `cargo test --workspace`
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- large Criterion sweeps
- Nsight / CUDA profiling
- full release-profile regression runs

---

## Recommended first slice

If only one next implementation slice is taken, it should be:

1. Workstream 1, Phase 1 through Phase 3
2. Workstream 2, Phase 1 through Phase 3

That combination gives the best immediate payoff:

- safer everyday workspace verification on Windows
- meaningful further reduction in meta / holdout overhead
- no product-surface removal
- limited architecture risk

---

## Final execution checklist

- [ ] Land a stable workspace-test wrapper and document it
- [ ] Audit and reduce unnecessary bin test compilation in `src-tauri`
- [ ] Add typed key wrappers and reusable scratch buffers to meta / holdout orchestration
- [ ] Expand prepared-state reuse in validation and composite validation
- [ ] Unify save / repair artifact canonicalization behind one canonical path
- [ ] Add direct benchmarks for the newly optimized surfaces
- [ ] Defer heavy perf validation until the current background load is finished
- [ ] After the machine is idle, run workspace tests, CPU perf gate, and GPU perf gate as applicable

