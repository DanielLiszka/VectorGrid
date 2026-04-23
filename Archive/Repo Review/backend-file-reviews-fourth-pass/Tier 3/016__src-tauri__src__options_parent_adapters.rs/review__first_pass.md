# Review Packet: `src-tauri/src/options_parent_adapters.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/options_parent_adapters.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Focused adapter layer for options parent replay, but not yet fully hardened because several replay invariants are implicit, strategy-kind resolution is duplicated, and direct compatibility coverage is thinner than the semantics risk`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Replay-semantics-sensitive`, `Artifact-compat-sensitive`, `Options-workflow-sensitive`

## What This File Owns

`options_parent_adapters.rs` owns the adapter layer that decides which saved parent-strategy artifacts can seed the options-expression workflow and how those artifacts are replayed into an options signal manifest.

This file owns:

- the allowlist for currently supported parent strategy families:
  - `OptionsSupportedParentFamily`: `src-tauri/src/options_parent_adapters.rs:11-27`
- extraction of strategy kind from saved artifact JSON:
  - `resolve_strategy_kind`: `44-50`
- artifact-to-family mapping and support-list messaging:
  - `supported_options_parent_strategy_list`: `53-55`
  - `supported_options_parent_family_from_artifact`: `57-60`
  - `supported_options_parent_family_from_kind`: `63-73`
  - `is_supported_options_parent_strategy_kind`: `75-76`
- conversion from a saved parent artifact into replay inputs for `compute_double_ma_drilldown_blocking_with_candles`:
  - `build_double_like_parent_replay`: `79-169`
  - `build_supported_options_parent_replay`: `171-196`
- dispatch to app-core signal-manifest builders for the supported parent families:
  - `build_supported_options_parent_signal_manifest`: `198-278`

This file does not own:

- the Tauri command boundary that invokes these adapters
  - that lives in `src-tauri/src/commands_options_expression.rs:1448-1575`
- saved-artifact provenance validation after replay inputs are built
  - that lives in `src-tauri/src/commands_options_expression.rs:2038-2125`, `3311-3393`
- the actual event-manifest projection semantics
  - that lives in `crates/app_core/src/options_signal_manifest.rs:32-261`
- the actual drilldown execution cost
  - that lives behind `compute_double_ma_drilldown_blocking_with_candles` in `commands_options_expression.rs:1547-1551`

## Classification

- Public surface: internal/backend-only
  - functions are `pub`, but this is a crate-internal adapter surface rather than frontend IPC
- Hot path: no
  - this is orchestration and JSON adaptation, not a tight compute loop
- Runtime-critical: moderate
  - wrong replay shaping changes options parent semantics and saved-artifact reusability
- CUDA/runtime-critical: no
  - it feeds replay inputs but does not own GPU/runtime routing
- Strategy-local: partially
  - it is a strategy-family adapter over four related parent families
- Low-risk support code: no
  - light execution cost, but semantics-sensitive

## Direct Interactions

### Backend interaction map

- options-expression command flow validates supported parent type here:
  - `src-tauri/src/commands_options_expression.rs:1455-1469`
- the same command flow then builds replay inputs here and uses them to rerun drilldown:
  - `src-tauri/src/commands_options_expression.rs:1546-1551`
- signal-manifest creation depends on the family and `trade_on_next_bar` emitted here:
  - `src-tauri/src/commands_options_expression.rs:1558-1572`
- provenance validation then checks the recomputed replay against saved artifact expectations:
  - `src-tauri/src/commands_options_expression.rs:2046-2048`
  - `src-tauri/src/commands_options_expression.rs:2114-2116`

### App-core contract anchors

- app-core manifest construction uses `trade_on_next_bar` to shift the decision bar:
  - `crates/app_core/src/options_signal_manifest.rs:43-61`
- all four family-specific manifest entrypoints currently forward to the same double-like projection helper:
  - `140-261`
- the key semantic test covering `trade_on_next_bar` is:
  - `crates/app_core/src/options_signal_manifest.rs:280-327`

## Standards And Research Log

This packet was written after targeted external review focused on adapter API clarity, JSON artifact extraction, command-path semantics, and the behavior of optional serialized fields.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file is an adapter/conversion surface with string-based family mapping and error-returning conversion paths
   - Standard informed:
     - conversion APIs should make invariants and failure modes explicit and avoid unnecessary drift-prone duplication

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - this file deserializes partially populated saved artifacts and optional validation candidate paths
   - Standard informed:
     - partially populated JSON inputs should be handled deliberately, especially when fallbacks affect semantics

3. `serde_json::Value` JSON Pointer documentation
   - Source: <https://docs.rs/serde_json/latest/serde_json/enum.Value.html>
   - Why it was consulted:
     - this file uses JSON Pointer heavily for replay extraction and fallback resolution
   - Standard informed:
     - missing JSON paths produce `None`, so fallback chains are part of the real contract and need direct verification

4. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this adapter sits directly on a Tauri command path even though it is not itself a command
   - Standard informed:
     - command-adjacent helpers need predictable error transport and should keep semantics stable because failures surface to the desktop client

## Findings

## Correctness And Safety

### 1. No current file-local correctness bug stands out, but the adapter is intentionally relying on a shared double-like artifact/request shape across four families

- Evidence:
  - all four supported families flow through the same replay builder:
    - `171-196`
  - replay deserializes:
    - `DoubleMaRequest`: `84-93`
    - `DoubleMaValidationParams`: `112-130`
    - `DoubleMaParamsResolved`: `133-150`
  - the resulting request is always a `DoubleMaDrilldownRequest`: `155-166`
- Why it matters:
  - this is valid only while `DoubleMa`, `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` remain artifact-compatible with the double-MA replay shape
  - if one family's saved artifact shape diverges, this file becomes the first place where replay silently stops being valid
- Review judgment:
  - this is a semantics-sensitive compatibility dependency, not proof of a present bug
- Smallest justified fix:
  - document the compatibility assumption locally and add family-specific minimal-fixture tests so the assumption fails loudly if drift starts

## API And Naming

### 1. Strategy-kind extraction is duplicated across this adapter and `commands_options_expression.rs`

- Evidence:
  - adapter helper:
    - `resolve_strategy_kind`: `44-50`
  - unsupported-parent error path reimplements the same fallback chain inline:
    - `src-tauri/src/commands_options_expression.rs:1457-1467`
- Why it matters:
  - drift in the JSON-pointer fallback order or trimming behavior can change what users see in errors versus what the adapter actually accepts
- Smallest justified fix:
  - reuse the adapter's strategy-kind extraction helper in the command error path instead of maintaining two copies

### 2. The supported-family inventory is duplicated as both code and prose

- Evidence:
  - enum variants and mapping table:
    - `11-27`, `63-73`
  - human-readable support list string:
    - `53-55`
- Why it matters:
  - adding or removing a supported family requires touching both logic and prose
  - mismatches would surface as misleading user-facing errors
- Smallest justified fix:
  - derive the support-list string from the same canonical family table, or add a test that the message stays aligned with the mapping

### 3. `is_supported_options_parent_strategy_kind` appears to be test-only at the moment

- Evidence:
  - compile output during this review pass emitted:
    - `warning: function 'is_supported_options_parent_strategy_kind' is never used`
    - location `src-tauri/src/options_parent_adapters.rs:75`
  - repo references found in this review pass were only test references:
    - `src-tauri/src/commands_options_expression.rs:2666-2697`
- Why it matters:
  - an unused public helper is a small contract-smell and an indicator that production code is already using duplicated logic elsewhere
- Smallest justified fix:
  - either use this helper from production code or make its intended test-only role explicit

## Error Handling

### 1. Local error messages are generally useful, but replay-source fallbacks are under-documented rather than under-validated

- Evidence:
  - missing `run.req` is explicit: `84-87`
  - missing requested candidate index is explicit: `95-105`
  - fallback to `run.res.best_params` occurs only when no validation candidate is present: `132-150`
- Why it matters:
  - callers get actionable error text today
  - the real risk is that maintainers may not realize which replay source won:
    - indexed validation candidate
    - candidate `0`
    - `run.res.best_params`
- Smallest justified fix:
  - add a short comment or adapter-local test naming the fallback order explicitly

## Module Boundaries And Technical Debt

### 1. The replay request contains two fixed invariants that are currently undocumented

- Evidence:
  - `data_id: "artifact_replay"`: `156`
  - `bins: Some(256)`: `165`
- Why it matters:
  - both are semantics-affecting choices in a replay-sensitive path
  - future maintainers cannot tell whether these are:
    - arbitrary defaults
    - compatibility requirements
    - performance/shape requirements for drilldown downstream
- Smallest justified fix:
  - add short comments explaining why those values are fixed, especially `bins: Some(256)`

### 2. Family dispatch is repetitive, but this repo's strategy-local architecture means it should not be over-collapsed casually

- Evidence:
  - family replay dispatch repeats the same call four times: `182-195`
  - family manifest dispatch repeats the same argument list four times: `214-277`
  - app-core currently forwards all four builders into the same helper:
    - `crates/app_core/src/options_signal_manifest.rs:140-261`
- Why it matters:
  - there is maintenance duplication today
  - however, repo policy explicitly prefers strategy-local structure when behavior may diverge later
- Review judgment:
  - avoid genericizing this into a "single enum plus generic callback" cleanup unless product strategy is changing
  - the right safe-now improvement is documentation and test coverage, not flattening away the strategy-local seams

## Docs And Examples

### 1. The adapter needs a short module-level explanation of the "double-like parent" compatibility contract

- Missing documentation topics:
  - why these four families are currently supported together
  - what artifact shape must remain compatible for replay to work
  - what should change if one family gets a distinct saved-artifact layout

### 2. The replay fallback order should be documented near `build_double_like_parent_replay`

- Missing documentation topics:
  - explicit order:
    - requested validation candidate
    - candidate `0`
    - `run.res.best_params`
  - why `trade_on_next_bar` is taken from the selected strategy payload rather than always from `run.req.strategy`

## Tests And Missing Verification

### 1. Current coverage proves only a small slice of the adapter contract directly

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app options_parent_strategy_kind_support_matches_current_double_like_scope -- --nocapture`
    - passed `1` test
  - `cargo test -p ta_desktop_demo_app saved_artifact_replay_provenance -- --nocapture`
    - passed `2` tests
  - `cargo test -p ta_app_core manifest_projection_uses_previous_bar_for_decision_when_trade_is_next_bar -- --nocapture`
    - passed `1` test
- Existing strengths:
  - mapping allowlist test:
    - `src-tauri/src/options_parent_adapters.rs:284-303`
  - provenance drift tests after replay:
    - `src-tauri/src/commands_options_expression.rs:3311-3393`
  - downstream `trade_on_next_bar` semantic test:
    - `crates/app_core/src/options_signal_manifest.rs:280-327`

### 2. The direct adapter coverage is thinner than the semantics risk

- Missing tests found in this review pass:
  - no minimal-fixture replay build test for each supported family
  - no direct test for `parent_candidate_index` selecting a nonzero candidate
  - no direct test for the fallback from validation candidate params to `run.res.best_params`
  - no direct test that `trade_on_next_bar` is sourced from the chosen replay strategy payload
  - no test that the prose support list stays aligned with the actual family mapping

### 3. If this file changes, verification should span both adapter and downstream semantics

- Exact local verification required if changed:
  - `cargo test -p ta_desktop_demo_app options_parent_strategy_kind_support_matches_current_double_like_scope -- --nocapture`
  - `cargo test -p ta_desktop_demo_app saved_artifact_replay_provenance -- --nocapture`
  - `cargo test -p ta_app_core manifest_projection_uses_previous_bar_for_decision_when_trade_is_next_bar -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. There is no meaningful performance opportunity inside this file itself

- Evidence:
  - work here is light JSON extraction and small request assembly
  - the expensive replay step begins later at:
    - `src-tauri/src/commands_options_expression.rs:1548-1551`
- Review judgment:
  - do not spend effort micro-optimizing clones or match statements here

### 2. The only performance-sensitive choice worth documenting is replay shape, not local CPU cost

- Evidence:
  - `bins: Some(256)` is a fixed replay-shape choice: `165`
- Measure-first guidance:
  - if replay cost or manifest size becomes an issue, measure the downstream effect of replay-bin changes before altering this adapter
  - do not change replay defaults as a readability cleanup

## Code-Quality Improvement Register

- Safe now:
  - deduplicate strategy-kind extraction
  - document replay-source fallback order
  - document `artifact_replay` and `bins: Some(256)` invariants
  - add adapter-local fixture tests for each supported family
- Defer unless strategy architecture changes:
  - collapsing family-specific dispatch into a generic abstraction
  - removing strategy-local manifest entrypoints in app-core

## Technical-Debt Reduction Register

- Safe to reduce now:
  - support-list duplication
  - unused helper drift around `is_supported_options_parent_strategy_kind`
  - under-tested replay-source fallback behavior
- Defer:
  - any attempt to replace the strategy-local family surface with a generic shared adapter model
- Useful debt that should not be "cleaned up" casually:
  - keeping family-specific manifest entrypoints visible, even if they currently forward to the same helper

## Performance Opportunity Register

- Safe structural improvement:
  - none urgent inside this file beyond documenting replay invariants
- Benchmark-sensitive or semantics-sensitive:
  - changing replay bin counts
  - changing how strategy-specific artifacts are reinterpreted into the shared drilldown shape

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - do not narrow the supported parent-family set without an explicit product decision
  - do not remove strategy-local seams merely because the current implementation is similar across families
- Regression risk if changed carelessly:
  - options replay can target the wrong candidate params
  - decision-bar timing can drift if `trade_on_next_bar` handling changes
  - saved parent artifacts can become unreplayable
- Safe change posture:
  - prefer additive tests, clearer invariants, and de-duplication of helper logic over structural rewrites
