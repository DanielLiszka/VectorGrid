# Review Packet: `crates/app_core/src/options_expression.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_expression.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally strong and better-tested than average, but not fully public-review ready because public blocking APIs still hide a large amount of semantics behind undocumented split/caching/backend behavior, cache lock poisoning can still panic instead of returning a structured failure, and several fallback paths are intentionally silent in ways that make profiler/runtime diagnosis harder than it should be`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot-path`, `CUDA-runtime-critical`, `Options-workflow`, `Semantics-sensitive`, `Benchmark-sensitive`

## What This File Owns

`options_expression.rs` is the core backend engine for the options-expression workflow. It owns the full execution pipeline from split assignment and candidate compilation through ranking, exactification, sealed-finalist promotion, coverage diagnostics, feasibility analysis, and the blocking inspection APIs.

It owns:

- process-wide cache wiring and GPU probe helpers:
  - test-only GPU overrides: `crates/app_core/src/options_expression.rs:61-83`
  - GPU planner probe bridge: `102-135`
  - process-wide structural/summary caches: `290-319`
- dataset indexing and split assignment:
  - `IndexedOptionsDataset`: `430-460`
  - fill-model validation: `468-479`
  - `assign_splits`: `482-590`
- candidate selection and per-trade evaluation:
  - candidate selection: `931-1144`
  - detailed trade replay: `1168-1441`
  - summary replay: `1444-1650`
  - metric helpers: `1653-1744`
- candidate/exit axes and cache compilation:
  - `CandidateAxis`: `1747-1790`
  - `StructuralCandidateCache`: `1792-2005`
  - `ExitAxis`: `2007-2074`
  - `ExitSummaryCache`: `2076-2095`
  - tuple-grid compilation: `2097-2138`
- prepared-run ownership and backend auto-decision:
  - `PreparedOptionsExpressionRun`: `2237-2256`
  - auto-backend heuristics: `2290-2374`
  - prepare pipeline: `2502-2809`
- ranking and post-ranking stages:
  - ranking order helpers: `2812-2925`
  - exactify and sealed-finalist stages: `2927-3204`
  - CPU ranking: `3701-3779`
  - runtime backend dispatch: `3783-3839`
- cache build/reuse helpers:
  - structural candidate compilation: `3841-4207`
  - cache get-or-build wrappers: `4209-4289`
- public entry points:
  - `run_options_expression_blocking`: `5222-5258`
  - `analyze_options_expression_feasibility_blocking`: `5261-5614`
  - `inspect_options_expression_tuple_blocking`: `5630-5685`
  - `inspect_options_expression_event_path_blocking`: `5687-5755`
- extensive file-local tests:
  - `5768-8967`

This file does not own:

- the request/result type definitions and wire-format DTOs:
  - `crates/app_core/src/options_types.rs`
- dataset import/build contracts:
  - `crates/app_core/src/options_data_store.rs`
- signal-manifest materialization and provenance:
  - `crates/app_core/src/options_signal_manifest.rs`
  - `src-tauri/src/commands_options_expression.rs`
- the GPU runtime implementation and kernel launch details:
  - `crates/app_core/src/options_expression_gpu.rs`
  - `crates/app_core/src/kernels/options_expression.cu`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:50`
  - public entry points are re-exported at `crates/app_core/src/lib.rs:220-223`
- Hot path: yes
  - this file owns tuple preparation, ranking, and exactification
- CUDA/runtime-critical: yes
  - it decides CPU versus GPU execution and drives GPU planner/scorer usage
- Strategy-local: yes
  - it is specific to the options-expression backend workflow
- Low-risk support code: no
  - this is core execution logic with persisted artifact and command-boundary impact

## Direct Interactions

### Backend interaction map

- public DTO and contract dependency:
  - `crates/app_core/src/options_types.rs:312-390`
  - `994-1117`
- dataset import/build contract dependency:
  - `crates/app_core/src/options_data_store.rs:291-433`
- GPU device and runtime bridge:
  - `crates/app_core/src/cuda_device.rs:81-97`
  - `crates/app_core/src/options_expression_gpu.rs:1304-1353`
- crate export surface:
  - `crates/app_core/src/lib.rs:220-223`
- Tauri command boundary:
  - `src-tauri/src/commands_options_expression.rs:1719-1815`
  - `1934-2087`
- artifact replay and dataset provenance enforcement:
  - `src-tauri/src/commands_options_expression.rs:1395-1706`
- profiling workflow coupling:
  - NVTX scopes in this file:
    - `2507`
    - `2529`
    - `3097`
    - `3139`
    - `3464`
    - `5227`
    - `5266`
  - Nsight script:
    - `scripts/profile_options_expression_nsys.ps1`
- bench surfaces:
  - `crates/app_core/benches/options_expression_candidate_compile.rs`
  - `crates/app_core/benches/options_expression_summary_compile.rs`
  - `crates/app_core/benches/options_expression_cpu_score.rs`
  - `crates/app_core/benches/options_expression_gpu_score.rs`

### Downstream verification impact if changed

- changing split semantics affects:
  - coverage adequacy verdicts
  - search/holdout/sealed tuple ranking
  - feasibility reporting shown at the Tauri boundary
- changing backend-selection or fallback semantics affects:
  - `commands_options_expression.rs`
  - persisted artifacts that record `backend_used` / fallback reason
  - Nsight- and bench-driven workflow expectations
- changing cache identity or inspection behavior affects:
  - warm feasibility-preview reuse
  - drill-down latency in tuple and event-path inspection flows

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API expectations, error and synchronization semantics, ordered-map behavior, median-selection cost, time-series split guidance, enum wire-format discipline, and GPU host/device transfer guidance.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exports a public blocking API with large behavioral surface area
   - Standard informed:
     - public APIs should make semantics, naming, and failure behavior intentional

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public entry points currently have no rustdoc examples or failure-contract sections
   - Standard informed:
     - public items should document examples plus `Errors` / `Panics` considerations where relevant

3. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - the file collapses a wide range of failures into `Result<_, String>`
   - Standard informed:
     - recoverable error paths should stay explicit, not silently blur distinct failure classes

4. `std::sync::OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - this file uses `OnceLock` for process-wide cache stores and test-only GPU overrides
   - Standard informed:
     - `OnceLock` is appropriate for write-once shared state and, unlike `Mutex`, is not poisoned by panic

5. `std::sync::Mutex`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why it was consulted:
     - cache access is guarded by `Mutex` and currently uses `.expect(...)` on poisoned locks
   - Standard informed:
     - lock poisoning is a real failure mode, so public APIs should decide whether to recover, reinitialize, or deliberately panic

6. `BTreeMap` in `std::collections`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeMap.html>
   - Why it was consulted:
     - cache eviction removes `keys().next()`, which depends on `BTreeMap` key ordering
   - Standard informed:
     - `BTreeMap` iterates in key order, so current eviction is lexicographic-first, not recency-aware

7. `slice::select_nth_unstable_by`
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable_by>
   - Why it was consulted:
     - `median(...)` currently sorts the entire vector
   - Standard informed:
     - Rust provides partial-selection primitives that can matter if median computation ever becomes a measured hotspot

8. scikit-learn `TimeSeriesSplit`
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html>
   - Why it was consulted:
     - this file owns chronological split assignment for search/holdout/sealed evaluation
   - Standard informed:
     - time-ordered model evaluation should preserve chronology and make split behavior explicit

9. Serde enum representations
   - Source: <https://serde.rs/enum-representations.html>
   - Why it was consulted:
     - public run/feasibility DTOs currently expose backend identity as free-form strings
   - Standard informed:
     - when result-surface values come from a closed backend set, enum-based wire formats are worth preferring over ad hoc string literals

10. NVIDIA CUDA C++ Best Practices Guide
    - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
    - Why it was consulted:
      - this file decides whether to route a run into GPU scoring and aggregates GPU planner/runtime evidence
    - Standard informed:
      - host/device transfer overhead matters, and keeping reusable inputs/device-side buffers across kernel work is a legitimate performance consideration

## Findings

## Correctness And Safety

### 1. Cache poisoning still turns public blocking entry points into panics instead of recoverable errors

- Evidence:
  - process-wide caches are guarded by `Mutex<BTreeMap<...>>`:
    - `crates/app_core/src/options_expression.rs:290-300`
  - public prepare flow reaches `get_or_build_structural_candidate_cache` and `get_or_build_exit_summary_cache`:
    - `2502-2809`
    - `4209-4289`
  - those helpers call `.lock().expect(...)` on poisoned locks:
    - `4230`
    - `4244`
    - `4268`
    - `4281`
- Why it matters:
  - the exported API surface returns `Result<_, String>`, so callers reasonably expect recoverable backend failures
  - a poisoned cache lock bypasses that contract and panics the process instead
- Smallest justified fix:
  - convert poisoned-cache handling into an explicit recovery path:
    - either clear/reinitialize the cache on poison
    - or convert poison into a structured `Err(...)`
  - do not leave this as an implicit panic path in the public run/feasibility/inspection surface
- Review judgment:
  - medium-high correctness / runtime-hardening issue

### 2. Split-assignment fallback semantics are real product behavior, but they are currently implicit rather than explicit

- Evidence:
  - when total events `< 3`, every event is forced into `Search`:
    - `crates/app_core/src/options_expression.rs:501-510`
  - when `SessionChronological` has fewer than 3 sessions, the code falls back to event-level boundaries:
    - `561-572`
  - tests pin the session-preserving case only when enough sessions exist:
    - `6697-6737`
    - `7132-7181`
- Why it matters:
  - these are meaningful semantics changes for holdout/sealed evaluation, not just implementation detail
  - the current behavior may be acceptable, but it is not discoverable from the public API
- Smallest justified fix:
  - keep the fallback behavior if it is product-correct
  - but document it and surface a diagnostic note in run/feasibility output when fallback semantics were applied
- Review judgment:
  - medium correctness / semantics-clarity issue

## API And Naming

### 1. Public result surfaces still encode backend identity as string literals

- Evidence:
  - public DTO fields:
    - `OptionsExpressionFeasibilityReport.best_available_backend`: `crates/app_core/src/options_types.rs:994-1004`
    - `OptionsExpressionRunResult.backend_used`: `1081-1097`
  - this file hardcodes backend names:
    - `recommended_backend`: `2362-2366`
    - runtime backend used/fallback values: `3810-3836`
    - feasibility fallback rewrites back to `"cpu_reference"`: `5533-5537`
- Why it matters:
  - the backend set is effectively closed in current code
  - free-form strings make result-surface drift easier than it should be across engine, Tauri, artifacts, and tests
- Smallest justified fix:
  - introduce a serializable backend enum and convert to strings only at outer compatibility seams if needed
  - if that is too broad for now, at least centralize the current string constants
- Review judgment:
  - medium public-surface / contract-shape issue

## Error Handling

### 1. The file collapses distinct runtime failures into plain `String` errors across all public entry points

- Evidence:
  - public entry points all return `Result<_, String>`:
    - `run_options_expression_blocking`: `5222-5258`
    - `analyze_options_expression_feasibility_blocking`: `5261-5614`
    - `inspect_options_expression_tuple_blocking`: `5630-5685`
    - `inspect_options_expression_event_path_blocking`: `5687-5755`
- Why it matters:
  - dataset mismatch, split invalidity, inspection index errors, GPU planner errors, and cache/runtime issues are materially different classes
  - flattening them into `String` makes downstream handling and diagnostics weaker than necessary
- Smallest justified fix:
  - keep `String` at the outermost Tauri boundary if desired
  - but use an internal error enum in `app_core` so failure categories remain explicit and testable
- Review judgment:
  - medium error-handling / maintainability issue

### 2. Combined GPU-scoring failure is intentionally silent when legacy GPU scoring succeeds

- Evidence:
  - combined scoring path is attempted first:
    - `crates/app_core/src/options_expression.rs:3490-3513`
  - on failure, the code immediately falls back to `score_gpu_lanes_legacy(...)`:
    - `3510-3513`
  - no diagnostic note is attached to a successful legacy fallback in the run path
- Why it matters:
  - this preserves functionality, which is good
  - but it removes evidence that the preferred GPU path failed, making profiler/runtime diagnosis harder
- Smallest justified fix:
  - keep the legacy fallback
  - add a debug note or surfaced diagnostic field when combined scoring failed but legacy fallback succeeded
- Review judgment:
  - medium observability / error-handling issue

## Module Boundaries And Technical Debt

### 1. One file currently owns too many distinct responsibilities

- Evidence:
  - this single module owns:
    - split assignment
    - candidate selection
    - summary replay
    - cache identity/reuse
    - CPU ranking
    - GPU dispatch
    - coverage adequacy
    - public inspection APIs
  - file size is very large and spans `5768-8967` lines of tests after the main runtime body
- Why it matters:
  - local reasoning cost is high
  - adjacent changes are more likely to create accidental coupling across runtime, reporting, and inspection concerns
- Smallest justified fix:
  - do not attempt a broad rewrite now
  - instead carve out bounded internal submodules in a later pass:
    - split logic
    - cache compilation
    - ranking
    - inspection
- Review judgment:
  - medium technical-debt issue, but larger refactor should be deferred

### 2. Cache eviction is bounded but not recency-aware

- Evidence:
  - caches are bounded by `OPTIONS_EXPRESSION_PREPARE_CACHE_MAX_ENTRIES = 32`:
    - `crates/app_core/src/options_expression.rs:41`
  - eviction removes `cache.keys().next()`:
    - `316-319`
  - the caches are `BTreeMap`s:
    - `290-300`
- Why it matters:
  - current eviction removes the lexicographically smallest key, not the least recently used or least recently built key
  - this is a real performance and reuse-policy choice, not a neutral implementation detail
- Smallest justified fix:
  - document that the current cache is a simple bounded reuse cache, not an LRU
  - if reuse behavior becomes important, move to an explicitly recency-aware structure in a later measured pass
- Review judgment:
  - medium technical-debt / perf-policy issue

## Docs And Examples

### 1. The public API has effectively no rustdoc for behavior that callers actually need to know

- Evidence:
  - public entry points:
    - `crates/app_core/src/options_expression.rs:5222`
    - `5261`
    - `5630`
    - `5687`
  - there is no rustdoc describing:
    - split fallback behavior
    - backend auto-selection meaning
    - when GPU fallback is silent versus surfaced
    - cache reuse assumptions
    - the fact that inspection reruns prepare plus ranking
- Why it matters:
  - this is a public re-exported backend surface
  - current callers can only learn semantics by reading a very large implementation file
- Smallest justified fix:
  - add rustdoc to the four public functions
  - include:
    - what each entry point does
    - what it may recompute
    - what the major `Errors` cases are
    - key semantic notes for split behavior and backend fallback

## Tests And Missing Verification

### 1. Current coverage is strong, but a few contract edges are still not directly pinned

- Evidence:
  - file-local verification:
    - `cargo test -p ta_app_core options_expression::tests:: -- --nocapture`
      - passed `37` tests
  - adjacent Tauri-boundary verification:
    - `cargo test -p ta_desktop_demo_app commands_options_expression::tests:: -- --nocapture`
      - passed `15` tests
  - bench compile verification:
    - `cargo bench -p ta_app_core --bench options_expression_candidate_compile --no-run`
    - `cargo bench -p ta_app_core --bench options_expression_summary_compile --no-run`
    - `cargo bench -p ta_app_core --bench options_expression_cpu_score --no-run`
    - `cargo bench -p ta_app_core --bench options_expression_gpu_score --no-run`
      - all completed successfully
- Missing tests I would want before changing this file:
  - split fallback when total events `< 3`
  - `SessionChronological` fallback when session count `< 3`
  - poisoned-cache recovery if the panic path is changed
  - explicit coverage that a combined-GPU failure records a diagnostic note if such a note is added

## Performance, Allocation, And CUDA Concerns

### 1. The inspection entry points are objectively expensive because they rerun the engine instead of consuming a prepared/ranked snapshot

- Evidence:
  - `inspect_options_expression_tuple_blocking` reruns:
    - `prepare_options_expression_run(...)`: `5636`
    - `rank_compiled_tuples(...)`: `5637`
  - `inspect_options_expression_event_path_blocking` does the same:
    - `5693-5701`
- Why it matters:
  - this is acceptable for correctness-first tooling
  - but it is not a cheap drill-down path and should be treated as such in docs and future UX work
- Smallest justified fix:
  - document the cost model now
  - only build a lighter-weight inspection path if it is actually needed and benchmarked
- Review judgment:
  - medium performance / UX-observability issue

### 2. Major prepare/ranking refactors are benchmark-sensitive and should not be done casually

- Evidence:
  - prepare eagerly builds baseline, conservative, and harsher candidate/summary caches:
    - `2535-2701`
  - GPU path intentionally reuses prepared flat inputs and output/cache planning:
    - `2684-2701`
    - `3325-3665`
  - runtime auto behavior is intentionally aligned with current product gating, not just naive measured speedup:
    - policy comment: `42-45`
    - tests pinning current posture: `7949-8008`
- Why it matters:
  - this file already sits on a measured CPU/GPU boundary
  - “cleanups” that change preparation order, ranking flow, or backend selection policy are likely to move benchmark results or workflow semantics
- Review judgment:
  - benchmark-sensitive; do not touch without measured justification

### 3. `median(...)` sorts the full vector, which is acceptable now but is only worth changing if profiling proves it matters

- Evidence:
  - `median` fully sorts:
    - `crates/app_core/src/options_expression.rs:1653-1663`
  - current call sites are in post-ranking exactification / enrichment:
    - `3007-3060`
    - `4327-4343`
- Review judgment:
  - safe structural opportunity only if measurement later shows median computation is non-trivial
  - otherwise this should stay low priority

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc to the four public entry points
  - convert cache-poison panic paths into explicit `Err(...)` or recovery behavior
  - add explicit diagnostics when split-assignment fallback semantics are applied
  - centralize backend-name literals even if a public enum is deferred
  - add a debug/diagnostic note when combined GPU scoring falls back to the legacy scorer
- Coordination required:
  - moving to typed backend enums across `options_types`, artifacts, and Tauri consumers
  - building a lighter-weight inspection path
  - recency-aware cache replacement
- Defer until later pass:
  - full file decomposition into multiple internal modules
  - changes to prepare/rank ordering or backend auto-policy

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing docs
  - panic-based cache poison handling
  - hidden fallback semantics
  - backend literal centralization
- Coordination required:
  - typed backend result surfaces
  - cache policy redesign
  - inspection-path architecture
- Useful debt that should not be “cleaned up” casually:
  - current auto-backend heuristics preserving the existing promotion envelope
  - the three-lane baseline/conservative/harsher preparation structure
  - GPU fallback behavior that preserves capability instead of failing runs outright

## Performance Opportunity Register

- Safe structural improvements:
  - documentation clarifying inspection cost and fallback behavior
  - targeted note propagation for combined-to-legacy GPU fallback
- Benchmark-sensitive candidates:
  - cache-policy redesign
  - prepare-pipeline decomposition
  - backend auto-threshold changes
  - exactify/median micro-optimizations
- Not the right place to chase performance without evidence:
  - “simplifying” the baseline/conservative/harsher prepare sequence
  - changing CPU/GPU route policy purely for aesthetics

## Observability And Diagnostics Gaps

- split fallback semantics are not surfaced to callers
- combined GPU scoring failure is not preserved when legacy GPU scoring succeeds
- cache reuse policy is implicit
- public docs do not explain that inspection is a full recompute path

## Implementation Staging Guidance

- Safe now:
  - rustdoc
  - additive tests for fallback semantics
  - cache-poison hardening
  - backend-string centralization
  - additional diagnostic notes
- Coordination required:
  - typed backend enums
  - recency-aware caching
  - inspection-path architecture
- Defer until later pass:
  - broader module split
  - backend auto-policy changes
  - prepare/rank pipeline restructuring

## Change Blast Radius

- Local only:
  - rustdoc
  - additive fallback tests
  - cache-poison error-path cleanup
  - combined-fallback diagnostic note
- Same subsystem:
  - backend-name typing and cache-policy changes affect options-expression runtime and artifact surfaces
- Cross-backend / frontend-backend contract:
  - DTO changes affect `commands_options_expression.rs`, persisted artifacts, and UI consumers

## Open Questions Or Assumptions

- assumption:
  - the `< 3` event and `< 3` session fallbacks are intentional product behavior rather than accidental shortcuts
- open question:
  - whether inspection cost is acceptable long-term once artifact counts and tuple counts grow
- open question:
  - whether backend identity is intended to remain stringly typed for external-compatibility reasons

## Recommended Action

`Small surgical refactor`

The right next pass is:

- document the public semantics
- harden cache poisoning into a recoverable path
- make silent fallback behavior more observable
- defer structural decomposition and benchmark-sensitive ranking/pipeline changes

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core options_expression::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_options_expression::tests:: -- --nocapture`
- `cargo check -p ta_app_core --tests`
- `cargo bench -p ta_app_core --bench options_expression_candidate_compile --no-run`
- `cargo bench -p ta_app_core --bench options_expression_summary_compile --no-run`
- `cargo bench -p ta_app_core --bench options_expression_cpu_score --no-run`
- `cargo bench -p ta_app_core --bench options_expression_gpu_score --no-run`
- if GPU ranking/runtime behavior changes:
  - `powershell -File scripts/profile_options_expression_nsys.ps1 -CaptureMode nvtx`

## Regression Risk And Feature-Preservation Constraints

- do not change runtime auto-backend policy casually; current behavior is intentionally preserving the shipped promotion envelope
- do not change `< 3` event or session fallback semantics silently; either preserve them or make the contract change explicit
- do not refactor prepare/rank staging without benchmark evidence and options-workflow regression verification
- do not remove CPU fallback or degrade drill-down capability in the name of cleanup
