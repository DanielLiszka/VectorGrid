# Custom Strategy Feature Master Plan

Last updated: 2026-03-04
Status: Draft v1 (initial scope and implementation checklist)
Owner: App team

## Objective

Enable users to define and run custom strategies from the frontend UI, with backend execution using the existing dynamic indicator dispatch/routing foundation.

Primary goals:
- preserve correctness and existing strategy behavior,
- preserve performance (CPU and GPU),
- avoid feature loss,
- keep implementation maintainable and low-debt.

## Current Baseline

- Dynamic indicator catalog/dispatch is integrated in backend (`vector-ta 0.1.5` + app adapter layer).
- Route policy is in place:
  - GPU-preferred indicators from `gpu_speedups.csv` route to GPU paths when supported,
  - non-preferred/unsupported indicators route to CPU.
- Existing strategy runners (Double MA / Composite variants / Composite MO) remain primary production paths.
- Dynamic strategy path exists but is still tied to existing strategy profile flow and experimental rollout controls.
- Existing fixed strategies remain discrete user-selectable options and are not replaced in v1.

## Product Direction (No-Code UX)

Custom strategy UI should be no-code and guided:

- Show two primary sections:
  - `Entry conditions`
  - `Exit conditions`
- Each section has a `+` action to add a condition row.
- Condition row builder flow:
  1. pick left operand from searchable indicator list (or price source),
  2. auto-populate indicator params with defaults,
  3. choose operator from constrained list,
  4. pick right operand via `+` (price / indicator output / constant / optional lag),
  5. allow additional rows and group confluence using `ALL` or `ANY`.
- UI should surface GPU hints:
  - indicators that are GPU-preferred by policy can show a small accelerator badge/icon,
  - route explanation should still remain textual for clarity.

Example target expressions:
- `alma < close`
- `alma_fast < alma_slow`
- `bbands.lower > low_range_constant`
- confluence like `(alma_fast > alma_slow) AND (close > ott_line)`.

## Feasibility Notes (Compiled Backend)

Compiled Rust backend is compatible with dynamic strategies because strategy definitions are treated as data:

- frontend emits JSON strategy spec,
- backend validates and compiles spec into IR/plan structures,
- runtime executes precompiled operator/dispatch primitives against that plan.

No runtime code generation or scripting engine is required for v1.

Key feasibility constraints:
- A compiled backend can interpret a strategy plan cheaply, but it cannot "magically" reuse a highly specialized GPU kernel without an explicit generic-kernel implementation.
- Full cartesian optimization across many indicator param sweeps can explode combinatorially; v1 must include guardrails.

## Practical End-to-End Strategy (Parity First)

Phase the rollout around reproducibility of existing strategies:

- first milestone: builder can recreate current fixed strategies via UI selections,
- run those equivalent definitions through custom pipeline,
- verify correctness and performance parity against fixed-strategy execution,
- only then expand expression/operator richness.

This keeps scope controlled and ensures production behavior is preserved while adding flexibility.

## Scope

In scope (initial custom strategy feature):
- frontend strategy builder for indicator + condition + signal mapping,
- backend command + validation for custom strategy spec,
- backend compilation to dynamic IR + route planning + execution,
- result payload compatible with existing optimization/analysis UI.
- template presets that mirror existing fixed strategies (for parity testing and user onboarding).

Out of scope (initial version):
- full validation/meta-analysis pipeline redesign for arbitrary custom strategies,
- distributed custom strategy execution,
- custom portfolio-level multi-strategy orchestration.

Low-priority / deferred:
- `pattern_recognition` integration (multi-output 2D boolean matrix for many patterns).
  - Keep explicitly deferred unless required for v1 launch.

## Functional Requirements (v1)

- User can define:
  - indicator nodes (`indicator_id`, params, output slot),
  - boolean conditions (lhs/rhs refs, operator),
  - condition groups (`all` / `any`),
  - signal bindings (long/short entry/exit groups),
  - basic execution policy fields (costs, risk controls, mode/backend selection).
- User can do this entirely through UI controls (no custom code input).
- User can run optimization on custom strategy with existing backend modes where compatible.
- Backend returns:
  - best params/metrics,
  - top rows,
  - route metadata (CPU/GPU per indicator),
  - fallback notes when applicable.

## Non-Functional Requirements

- CPU-only builds remain fully functional.
- CUDA builds remain optional and feature-gated.
- No regression against existing strategy flows.
- Deterministic behavior for same input/config.
- Clear validation errors for malformed custom specs.

## High-Level Architecture

1. Frontend strategy builder -> JSON strategy spec.
2. Tauri command receives spec.
3. Backend normalizes + validates spec.
4. Backend compiles spec to `StrategyIr` (dynamic indicator nodes).
5. Route planner assigns indicator routes (CPU/GPU).
6. Execution engine runs optimization/eval using dynamic adapters.
7. Existing UI result surfaces consume output.

## Practical v1 Feature Matrix

This matrix keeps v1 shippable while preserving performance:

- `CPU-only` backend:
  - indicator compute: CPU dispatch
  - rule evaluation: CPU
  - backtest + metrics: CPU
- `GPU unified` backend (v1 default behavior):
  - indicator compute: mixed CPU/GPU per policy
  - rule evaluation: CPU (series already materialized on host)
  - backtest + metrics: CPU
- `GPU kernel` backend:
  - v1: only when strategy maps to existing kernel-supported forms
  - future: generic predicate evaluation kernel + VRAM-resident pipeline for arbitrary rules

## Optimization/Search Space (Major Oversight to Lock Down)

Custom strategies are only useful if users can sweep/optimize parameters safely and predictably.

Problems to avoid:
- full cartesian explosion (e.g., 5 indicators * 3 swept params each),
- allocating or transferring huge indicator matrices unnecessarily,
- confusing UX where "Run" locks the app for minutes with no estimate.

v1 recommendation:
- Support a bounded, explicit search space model:
  - parameters are either `Pinned` (single value) or `Swept` (range/step or categorical choices),
  - UI must show estimated combo count and warn/refuse above a hard limit.
- Prefer sampling-based optimization (MO/random/TPE) when combos exceed a threshold.
- Start with parity templates where sweep shape matches existing optimizers:
  - Double-like strategies: optimize fast/slow MA period ranges (+ optional small set of pinned filters).

Implementation note:
- Treat "search space" as a first-class part of custom spec, not implicit UI state.

## Indicator Output Types and Rule Semantics (Must Restrict v1)

Not all indicators yield simple numeric `f64` series. The dynamic catalog includes:
- numeric outputs (`f64`/`f32`),
- integral outputs (`i32`),
- boolean outputs (`bool`),
- matrix-like outputs expressed as `(rows, cols)` + optional `pattern_ids`.

v1 recommendation:
- Allow in conditions:
  - numeric series outputs (`f64`/`f32`) directly,
  - optionally `i32` via explicit cast-to-f64 rule (if needed).
- Defer:
  - boolean series as direct operands unless we add explicit ops like `is_true` / `is_false`,
  - matrix outputs unless user selects a single row/output (see `pattern_recognition` plan).
- Define NaN/non-finite semantics:
  - comparisons against non-finite yield `false`,
  - cross ops treat non-finite as "no cross".

## Warmup and Alignment (Correctness Risk)

Indicators often have warmup periods; for confluence rules we must align validity.

v1 recommendation:
- Compute `effective_start_t` as the maximum of:
  - each required series first-finite index,
  - each indicator warmup,
  - each `lag` used by any condition operand,
  - execution policy offsets (trade-on-next-bar, etc).
- Do not allow entries/exits prior to `effective_start_t`.
- Surface a UI hint like: `warmup bars = X (strategy starts at bar X)`.

## Execution Model Freeze (Avoid Scope Creep)

The biggest feasibility risk is accidentally turning "custom strategy" into a full general-purpose
portfolio simulator. v1 should explicitly freeze execution semantics to what the app already does well.

v1 recommendation (lock these explicitly in schema + docs):
- Use the existing single-strategy execution semantics already implemented by fixed strategies.
- Keep position model simple:
  - single position at a time per symbol (no pyramiding),
  - allow long-only first; enable short only if already supported by current engines.
- Keep timing semantics explicit:
  - whether signals execute on current bar close vs next bar open (default should match existing app behavior).
- Keep costs/slippage/risk controls as pass-through of existing fields:
  - do not invent new risk-control DSL in v1.

This keeps the custom strategy feature focused on *signal generation + parameter optimization*,
not re-implementing the entire backtest simulator.

## Strategy Spec Contract (initial)

Required top-level fields:
- `schema_version`
- `profile = "custom"`
- `ma_source`
- `indicators[]`
- `conditions[]`
- `groups[]`
- `signals`
- `execution` (strategy/risk/control settings)

Indicator node (v1):
- `id: string`
- `indicator_id: string`
- `params: [{ key, value }]`
- `output_id: string`

Condition node (v1):
- `id: string`
- `op: gt|ge|lt|le|cross_up|cross_down`
- `lhs` / `rhs` as value refs:
  - indicator slot id,
  - ma_source,
  - close,
  - constant.
  - optional `lag: u32` for "trend" style comparisons (example: `alma(lag=1)`).

## Pattern Recognition Deferred Plan

`pattern_recognition` is deferred due to output shape mismatch:
- returns pattern matrix semantics (effectively 2D boolean outputs),
- current v1 condition/value-ref model is series-oriented.

Deferred options:
- Option A: expose selected pattern as single derived series output (preferred future path).
- Option B: add matrix-indexed value refs in IR (higher complexity).

## Performance Policy

- Keep existing speedup-policy routing:
  - GPU if indicator is GPU-preferred by `gpu_speedups.csv` and capability permits,
  - otherwise CPU.
- Reuse staged CUDA inputs across GPU-routed indicators per run.
- Keep route-aware indicator result caching by `(dataset, indicator, params, route)`.
- Maintain CPU/GPU perf gates for regression detection.
- Keep fixed strategies untouched as performance reference baselines.

## GPU Backend Integration Notes (v1)

Custom strategy path can use the existing GPU backend foundation for indicator computation/routing:

- indicators in `gpu_speedups.csv` and GPU-capable route to GPU,
- non-preferred/unsupported indicators route to CPU.

For v1 practicality:
- prioritize CPU-side condition evaluation with GPU/CPU indicator routing,
- preserve existing highly-optimized fixed GPU strategy paths as-is,
- treat full arbitrary-condition GPU predicate execution as a later optimization phase if needed.

## UX Details (Builder MVP)

Builder should be optimized for user trust and speed of iteration:
- Searchable indicator picker (category + GPU hint + output picker for multi-output).
- Operator picker restricted to allowed ops for the selected operand types.
- Right-hand operand picker supports:
  - price sources (`close`, `ma_source`),
  - other indicator outputs,
  - constants (with inline numeric input),
  - optional `lag` selector for "trend" comparisons (defer if backend IR does not support it yet).
- Entry/exit group mode toggle (`ALL` / `ANY`).
- Preview area:
  - estimated combos,
  - expected backend route summary,
  - warmup start bar hint.

## Phased Checklist

## Phase 0: Spec Freeze and Guardrails

- [ ] Freeze v1 custom strategy JSON schema.
- [ ] Freeze v1 operator set and operand types.
- [ ] Freeze v1 optimization/search space schema and limits.
- [ ] Freeze v1 execution model semantics (trade timing, long/short, position model).
- [ ] Define max limits (indicator count, condition count, group size) for safety.
- [ ] Define backend availability for custom strategies (disable GPU-kernel unless template-mapped).
- [ ] Lock deferred list (including `pattern_recognition`).
- [ ] Add schema compatibility/versioning notes.
- [ ] Lock parity-first target list: fixed strategies that must be recreatable from builder.

## Phase 1: Backend Contract and Validation

- [ ] Add `Custom` strategy profile in app request/normalization layer.
- [ ] Add parser/validator for custom strategy payload.
- [ ] Map custom payload to dynamic `StrategyIr`.
- [ ] Enforce v1 output-type restrictions (numeric-only, matrix handling).
- [ ] Define warmup/alignment semantics and validate constraints.
- [ ] Add strict error messages (unknown indicator, bad output_id, bad params, bad refs).
- [ ] Add unit tests for invalid/valid payloads.

## Phase 2: Backend Execution Integration

- [ ] Add backend entrypoint for custom strategy optimization run.
- [ ] Integrate compile -> plan -> execute path for custom profile.
- [ ] Ensure route metadata is returned to caller.
- [ ] Ensure cancellation/progress hooks work for custom runs.
- [ ] Add CPU/GPU fallback behavior tests.
- [ ] Add correctness tests for warmup/alignment and non-finite rule semantics.

## Phase 3: Frontend Builder MVP

- [ ] Add strategy builder UI state model (indicators/conditions/groups/signals).
- [ ] Add indicator selector using dynamic indicator catalog API with search.
- [ ] Add accelerator badge/icon hint for GPU-preferred indicators.
- [ ] Add param editors with schema-aware controls.
- [ ] Add condition/group/signal editing UI with validation hints.
- [ ] Implement entry/exit section layout with add-row (`+`) interactions.
- [ ] Show estimated combo count and enforce hard limits.
- [ ] Serialize to custom strategy payload and invoke backend command.
- [ ] Add template import buttons for current fixed strategies.

## Phase 4: UX and Safety Hardening

- [ ] Add client-side validation before submit.
- [ ] Add route summary panel (which indicators go CPU/GPU and why).
- [ ] Add warnings for unsupported/deferred features.
- [ ] Add import/export JSON for custom strategy configs.

## Phase 5: Testing and Parity

- [ ] Add integration tests for end-to-end custom strategy runs (CPU).
- [ ] Add integration tests for end-to-end custom strategy runs (GPU where available).
- [ ] Add parity tests against equivalent static strategy definitions.
- [ ] Add UI tests for builder serialization and command invocation.
- [ ] Add parity snapshot suite: recreated fixed strategies (builder) vs fixed-strategy outputs.

## Phase 6: Performance Gates

- [ ] Add benchmark cases for custom strategy dynamic path (CPU).
- [ ] Add benchmark cases for custom strategy dynamic path (GPU).
- [ ] Run perf gates and document baseline vs candidate.
- [ ] Confirm no material regressions in existing strategy benchmark suites.

## Phase 7: Rollout

- [ ] Release behind feature flag.
- [ ] Add telemetry-free local artifacts for debugging custom run failures.
- [ ] Prepare user docs for custom strategy builder usage.
- [ ] Decide promote-to-default criteria.

## Risks and Mitigations

- Risk: scope creep from advanced strategy semantics.
  - Mitigation: strict v1 schema freeze and deferred list.
- Risk: correctness drift vs static strategies.
  - Mitigation: parity tests + shadow-mode style checks.
- Risk: GPU route confusion for users.
  - Mitigation: explicit route summaries/warnings in UI.
- Risk: technical debt from ad-hoc per-indicator logic.
  - Mitigation: keep descriptor/dispatch-driven architecture only.

## Definition of Done (v1)

- User can create a custom strategy in UI and run it successfully.
- Backend validates and executes custom strategy through dynamic IR/plan path.
- CPU/GPU routing behavior follows policy and is surfaced clearly.
- Tests and perf gates pass.
- Existing strategy flows remain intact and non-regressed.
- Existing fixed strategies remain available as separate options.
- Builder can recreate at least one existing fixed strategy with parity.

## Progress Log

- 2026-03-04: Created initial master plan and checklist for custom strategy feature implementation.
- 2026-03-04: Added no-code UX direction (entry/exit builder flow), parity-first rollout strategy, and explicit GPU integration notes while preserving fixed strategies as discrete options.
