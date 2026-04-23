# Review Packet: `crates/app_core/src/composite_validate.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_validate.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-value validation surface with broad functional coverage, but not public-review ready as-is because several public request/result fields are narrower than they appear, GPU reopt code still contains production panic paths, and some GPU-family routes carry benchmark-sensitive inefficiencies and route-label ambiguity`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot-path`, `Validation-critical`, `CUDA/runtime-critical`, `Benchmark-sensitive`, `Cross-layer-contract-sensitive`

## What This File Owns

`composite_validate.rs` is the composite-family validation execution hub. It is broader than a thin adapter. In one module it defines the composite-family parameter DTO, the public validation request/result DTOs, family detection and route-diagnostic helpers, CPU candidate-series builders, GPU fixed-candidate preparation/dispatch, auxiliary robustness/stress work, and the public blocking validation entrypoints.

This file owns:

- public composite candidate DTO:
  - `CompositeParams`: `crates/app_core/src/composite_validate.rs:227-261`
- public validation request/result DTOs:
  - `CompositeValidationRequest`: `9584-9627`
  - `CompositeValidationCandidateResult`: `9630-9664`
  - `CompositeValidationResult`: `9745-9765`
- family routing and route-diagnostic helpers:
  - `clear_validation_strategy_route_diagnostics`: `327-339`
  - `take_validation_strategy_route_diagnostics`: `342-355`
  - `composite_validation_family`: `372-395`
- GPU fixed-candidate preparation and dispatch:
  - route eligibility and blockers: `490-648`
  - family dispatcher: `9309-9360`
- CPU family candidate-series builders:
  - `build_candidate_series`: `9894-10056`
  - family-specific builders beginning at `10058`
- public blocking entrypoints:
  - `run_composite_validation_blocking_with_candles`: `12262-12275`
  - `run_composite_validation_blocking_with_candles_with_aux_mask`: `12277-12931`
- auxiliary validation work that stays local here:
  - regime-slice robustness: `13199-13331`
  - friction-stress scenarios and summaries: `13333-13525`
- in-file integration test suite:
  - starts at `13527`

This file does not own:

- shared candidate orchestration and verdict sorting
  - `crates/app_core/src/validation_core.rs`
  - `crates/app_core/src/validation_pipeline.rs`
- Tauri cross-ticker orchestration and validation-context enforcement
  - `src-tauri/src/commands_validation.rs`

## Classification

- Public surface: yes
  - request/result DTOs and entrypoints are re-exported from `crates/app_core/src/lib.rs:138-141`
- Hot path: yes
  - every candidate validation run flows through this file
- CUDA/runtime-critical: high
  - fixed-candidate GPU validation, route selection, fallback reasons, and auxiliary GPU paths live here
- Strategy-local: mixed
  - it is shared orchestration, but it also embeds family-specific validation logic
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- public re-export surface:
  - `crates/app_core/src/lib.rs:138-141`
- shared validation orchestration:
  - `crates/app_core/src/validation_core.rs:7`
  - `crates/app_core/src/validation_pipeline.rs:244`
  - `crates/app_core/src/validation_pipeline.rs:437`
  - `crates/app_core/src/validation_pipeline.rs:668`
  - `crates/app_core/src/validation_pipeline.rs:826`
  - `crates/app_core/src/validation_pipeline.rs:1202`
  - `crates/app_core/src/validation_pipeline.rs:1343`
  - `crates/app_core/src/validation_pipeline.rs:1519`
- shared validation DTO/config surface:
  - `crates/app_core/src/validation_shared.rs:1-360`
- strategy-family compute modules used from here:
  - `asr.rs`, `vcc.rs`, `vemr.rs`, `rocb.rs`, `vcet.rs`, `mkcs.rs`, `kqsr.rs`, `carm.rs`, `kvdf.rs`, `cdmc.rs`
- GPU metric kernels used from here:
  - `asr_gpu_kernel.rs`, `vcc_gpu_kernel.rs`, `vemr_gpu_kernel.rs`, `rocb_gpu_kernel.rs`, `vcet_gpu_kernel.rs`, `mkcs_gpu_kernel.rs`, `kqsr_gpu_kernel.rs`, `carm_gpu_kernel.rs`, `kvdf_gpu_kernel.rs`, `cdmc_gpu_kernel.rs`
- composite optimization/output parity anchors:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
- Tauri orchestration that wraps this core API:
  - `src-tauri/src/commands_validation.rs:342-425`
  - `430-530`
  - `533-575`
- benchmark harnesses that call this core API directly:
  - `crates/app_core/benches/composite_validation_workflow.rs:523-552`
  - `804-837`

### Downstream verification impact if changed

- changing request/result DTO semantics affects:
  - Tauri command callers
  - bench/bin direct callers
  - any frontend/service layer that relies on the serialized shape
- changing GPU route or fallback behavior affects:
  - route diagnostics and user-facing backend summaries
  - GPU/CPU parity tests
  - workflow/benchmark conclusions
- changing cross-ticker or validation-context ownership affects:
  - `src-tauri/src/commands_validation.rs`
  - any direct `ta_app_core` caller not going through Tauri

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API contracts, documentation expectations, recoverable error handling, test-backed behavior, and CUDA optimization discipline for benchmark-sensitive GPU route changes.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes request/result DTOs and public blocking entrypoints
   - Standard informed:
     - public surfaces should be predictable, documented, and dependable

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public DTOs and entrypoints currently have no rustdoc despite being re-exported from the library surface
   - Standard informed:
     - public APIs should explain contract scope, caller expectations, and important caveats

3. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file is primarily a `Result<_, String>` API, but several GPU reopt paths still use `panic!`
   - Standard informed:
     - recoverable operational mismatches should surface as ordinary errors, not process-aborting panics

4. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - this file relies heavily on an in-file integration suite to lock route, parity, and auxiliary-validation behavior
   - Standard informed:
     - important route and contract behavior should be explicitly encoded in focused tests

5. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file contains many GPU validation paths and route-policy decisions that are tempting optimization targets
   - Standard informed:
     - benchmark-sensitive GPU changes should follow an assess/measure/optimize loop rather than ad hoc cleanup

## Findings

## Correctness And Safety

### 1. The file has broad functional coverage, but route diagnostics only clear/collect for the first candidate family

- Evidence:
  - clear helper branches on `candidates.first()` only:
    - `crates/app_core/src/composite_validate.rs:327-339`
  - take helper does the same:
    - `342-355`
  - main validation still runs per candidate through shared orchestration:
    - `12365-12647`
  - final response publishes only that one collected diagnostic bucket:
    - `12901-12903`
- Why it matters:
  - CPU validation permits mixed-family batches because candidate building is per candidate
  - if mixed-family requests are used, later families can lose route diagnostics or inherit stale state
- Smallest justified fix:
  - either reject mixed-family `CompositeValidationRequest`s up front
  - or clear/collect diagnostics for every family present in `req.candidates`
- Review judgment:
  - safe-now correctness/observability hardening

## API And Naming

### 1. `cross_ticker` is part of the public core contract, but this module treats it as a no-op

- Evidence:
  - request surface exposes `cross_ticker`:
    - `crates/app_core/src/composite_validate.rs:9604`
  - candidate/top-level results expose cross-ticker fields:
    - `9655-9657`
    - `9761`
  - candidate assembly hardcodes cross-ticker fields to `None`:
    - `12477-12478`
    - `12606-12607`
  - final result also hardcodes `cross_ticker: None`:
    - `12927`
  - the Tauri command peels off and applies cross-ticker work outside this core function:
    - `src-tauri/src/commands_validation.rs:392-421`
    - request rebuilding strips `cross_ticker` back to `None`: `533-575`, especially `563`
- Why it matters:
  - a direct `ta_app_core` caller can set `cross_ticker` and receive a silent no-op instead of cross-ticker results or an explicit error
- Smallest justified fix:
  - either reject `cross_ticker` in the core entrypoint with a clear error that points to the Tauri orchestration layer
  - or document that cross-ticker is an outer-orchestration concern and should not be passed to the core blocking function directly
- Review judgment:
  - safe-now contract clarification

### 2. `validation_context` is public on the core request type, but only the Tauri layer enforces it

- Evidence:
  - request surface exposes `validation_context`:
    - `crates/app_core/src/composite_validate.rs:9626`
  - public blocking entrypoints are re-exported from:
    - `crates/app_core/src/lib.rs:138-141`
  - the core run function never reads `validation_context`
  - validation is enforced only in the Tauri command layer:
    - `src-tauri/src/commands_validation.rs:131-145`
    - `342-367`
  - there are direct non-Tauri callers:
    - `crates/app_core/benches/composite_validation_workflow.rs:523-552`
    - `804-837`
    - `src-tauri/src/bin/ta_workflow_regression.rs:302`
- Why it matters:
  - this request field currently looks like a core-library contract even though its semantics are owned elsewhere
- Smallest justified fix:
  - either remove/move the field out of the core DTO in a compatibility-reviewed cleanup
  - or validate it in the core entrypoint as well
  - at minimum, document that it is only meaningful when the Tauri command layer wraps this API
- Review judgment:
  - safe-now API ownership clarification

### 3. `max_candidates` silently self-heals `0` and silently truncates oversized candidate lists

- Evidence:
  - public request field:
    - `crates/app_core/src/composite_validate.rs:9609-9610`
  - `0` becomes default `32`:
    - `271-273`
    - `12290-12292`
  - oversized requests are truncated in place:
    - `12293-12295`
- Why it matters:
  - callers do not get an error, warning, or result-side note that the request was rewritten
  - downstream verdict/reporting then reflects the truncated set only
- Smallest justified fix:
  - document the coercion/truncation behavior explicitly
  - or return a result-side note/diagnostic when truncation occurs
  - rejecting `0` is also defensible, but that is a broader compatibility choice
- Review judgment:
  - documentation-first fix is safest

## Error Handling

### 1. GPU reopt paths still use `panic!` for missing cached metrics even though the API is otherwise recoverable

- Evidence:
  - representative panic sites:
    - base composite: `crates/app_core/src/composite_validate.rs:1460-1470`
    - ASR: `2029-2035`
    - MKCS: `2984-2994`
    - ROCB: `3492-3500`
    - VEMR: `4737-4745`
    - VCC: `5802-5807`
    - KQSR: `6973-6983`
    - CARM: `7760-7768`
    - KVDF: `8256-8265`
    - CDMC: `9016-9025`
- Why it matters:
  - these are internal mismatches inside a public blocking validation API that otherwise returns `Result<_, String>`
  - a missing cached range metric should fail the validation run cleanly, not abort the process
- Smallest justified fix:
  - convert each `panic!` site into a descriptive `Err(...)`
  - keep the detailed range/family context in the error string
- Review judgment:
  - highest-priority safe-now fix in this file

## Module Boundaries And Technical Debt

### 1. The file is not actually a thin adapter, and the local architecture note is stale

- Evidence:
  - architecture note says strategy validators act as thin adapters:
    - `ARCHITECTURE_NOTES.md:82`
  - this module defines DTOs, route helpers, GPU prep, CPU builders, auxiliary stats, public entrypoints, and a very large in-file test suite:
    - `crates/app_core/src/composite_validate.rs:227-261`
    - `9584-9765`
    - `9894-10056`
    - `12262-12931`
    - `13199-13525`
    - `13527+`
- Why it matters:
  - contributors relying on the architecture note will underestimate the amount of behavior concentrated here
  - that mismatch makes safe refactoring harder than it needs to be
- Smallest justified fix:
  - update the architecture note to describe this module as the current validation execution hub
  - defer any real decomposition work until there is a bounded extraction plan
- Review judgment:
  - documentation/architecture debt, not an immediate behavior bug

### 2. Cross-layer ownership is split in ways the file does not document

- Evidence:
  - cross-ticker work lives outside the core run function:
    - `src-tauri/src/commands_validation.rs:392-421`
    - `430-530`
  - validation-context enforcement also lives outside the core run function:
    - `131-145`
    - `350-367`
- Why it matters:
  - the public core DTOs currently suggest that these concerns belong to the core engine
  - they do not, at least not fully
- Smallest justified fix:
  - add module-level rustdoc describing what this file owns versus what the Tauri wrapper owns
- Review judgment:
  - safe-now clarity improvement

## Docs And Examples

### 1. Public DTOs and entrypoints have no rustdoc despite being part of the library surface

- Evidence:
  - public items with no rustdoc:
    - `CompositeParams`: `crates/app_core/src/composite_validate.rs:227`
    - `CompositeValidationRequest`: `9584`
    - `CompositeValidationCandidateResult`: `9630`
    - `CompositeValidationResult`: `9745`
    - public entrypoints: `12262`, `12277`
- Why it matters:
  - the biggest risks in this file are contract ownership and optional-lane semantics
  - callers should not need to reverse-engineer those from the command layer or tests
- Smallest justified fix:
  - add rustdoc that explains:
    - which request fields are core-engine inputs
    - which are wrapper/orchestration concerns
    - what route strings and fallback reasons actually mean
- Review judgment:
  - safe-now documentation improvement

## Tests And Missing Verification

### 1. The in-file suite is strong, but it misses direct core-API tests for the contract gaps above

- Evidence:
  - strong existing anchors include:
    - core smoke: `crates/app_core/src/composite_validate.rs:13799`
    - aux-work behavior: `13879`, `13934`, `13987`
    - route reporting: `14051`, `14116`, `14191`, `14271`
    - invalid candidate rejection: `16090`
    - iterative engine path: `16619`
    - MO parity anchor: `16851`
    - rerun equivalence: `17016`
    - probability gates: `17103`
    - many GPU route/parity tests in the `21695+` range
  - missing direct core-level assertions:
    - passing `cross_ticker` to the core blocking function should either error or be explicitly documented/tested as ignored
    - passing/omitting `validation_context` to the core blocking function is not covered because enforcement happens only in Tauri
    - mixed-family route-diagnostic collection is not directly tested
- Why it matters:
  - the biggest current risk is not raw feature absence, but callers misunderstanding which layer owns which behavior
- Smallest justified fix:
  - add targeted tests that lock whichever contract the team chooses for:
    - `cross_ticker`
    - `validation_context`
    - mixed-family route diagnostics
- Review judgment:
  - high-value missing verification

## Performance, Allocation, And CUDA Concerns

### 1. `VCC`, `CARM`, and `CDMC` pay for GPU metric passes that are then discarded while the run still reports a GPU route

- Evidence:
  - `VCC` builds exact CPU metrics/series first:
    - `crates/app_core/src/composite_validate.rs:5001-5002`
  - then runs GPU metric evaluation:
    - `5141-5147`
  - checks only the output length:
    - `5148-5150`
  - then overwrites with `exact_metrics`:
    - `5151`
  - while still reporting a GPU route:
    - `5177-5180`
  - `CARM` follows the same pattern:
    - exact CPU metrics: `7188-7189`
    - GPU metric call: `7372-7378`
    - overwrite: `7382`
    - GPU route label: `7420-7424`
  - `CDMC` follows the same pattern:
    - exact CPU metrics collected during prep: `9102-9103`
    - GPU metric call: `9241-9247`
    - overwrite: `9251`
    - GPU route label: `9289-9293`
- Why it matters:
  - this is benchmark-sensitive waste
  - it also makes route reporting mildly misleading because the in-sample metric surface is still coming from CPU exact recomputation
- Smallest justified fix:
  - if CPU exact metrics are the intended source of truth for these families, skip the discarded GPU metric call entirely
  - otherwise keep the GPU metric result and move parity-only exact recomputation behind test/debug-only verification
- Review judgment:
  - benchmark-sensitive; do not change casually without route/parity benchmarks

### 2. GPU walk-forward/reopt helpers rebuild device prep instead of reusing the already prepared fixed-candidate evaluation state

- Evidence:
  - dispatcher receives `gpu_eval` but several families immediately branch into per-candidate/per-family rebuild helpers:
    - `crates/app_core/src/composite_validate.rs:9400-9447`
    - `9463-9551`
  - ASR walk-forward rebuilds device prep and sweeps:
    - `1485-1555`
  - ASR reopt does the same:
    - `1718-1848`
  - MKCS walk-forward also rebuilds device prep:
    - `2580-2675`
  - only base composite has a meaningful cached prep object:
    - `399-455`
    - `9478-9491`
- Why it matters:
  - this inflates auxiliary GPU validation cost and muddies GPU/CPU benchmark conclusions
  - the largest validation workloads are exactly where duplicated uploads and sweeps hurt most
- Smallest justified fix:
  - extend `GpuCompositeFixedCandidateValidationEval` with reusable family-specific prep
  - have walk-forward/reopt helpers consume cached prep instead of reconstructing it
- Review judgment:
  - real performance opportunity, but benchmark-sensitive

### 3. Auto-route CPU-preference reasons are policy strings backed by specific measured workloads and should not be “cleaned up” casually

- Evidence:
  - `Auto` CPU-preference reasons cite measured workloads directly:
    - `crates/app_core/src/composite_validate.rs:458-485`
  - there are tests locking current policy, for example:
    - `22531-22579`
    - `22584+`
- Why it matters:
  - these are not generic style choices
  - changing them is a benchmark policy decision
- Review judgment:
  - defer unless refreshed benchmark evidence exists

## Code-Quality Improvement Register

- safe-now:
  - reject or explicitly document `cross_ticker` at the core boundary
  - document or enforce `validation_context` ownership
  - replace GPU reopt `panic!` sites with ordinary `Err(...)`
  - document request coercion/truncation for `max_candidates`
  - document mixed-family route-diagnostic behavior or reject mixed-family requests
- defer-later:
  - split family-specific GPU prep helpers into dedicated modules only with a bounded extraction plan

## Technical-Debt Reduction Register

- safe-now:
  - update `ARCHITECTURE_NOTES.md` so it no longer misdescribes this module as a thin adapter
  - add rustdoc to public DTOs and entrypoints
- defer:
  - break the file into smaller family-specific validation units after contract gaps are stabilized
- do not casually “clean up”:
  - auto-route CPU/GPU policy strings
  - route labels or fallback wording that downstream tests already lock

## Performance Opportunity Register

- safe structural improvements:
  - avoid eager/discarded GPU metric work for `VCC`, `CARM`, and `CDMC`
  - reuse family-specific GPU prep across walk-forward/reopt auxiliary lanes
- benchmark-sensitive candidates:
  - any change to GPU/CPU source-of-truth metrics for `VCC`, `CARM`, `CDMC`
  - any change to auto route policy for ASR/VEMR/ROCB/MKCS/KQSR/KVDF/CARM
- measurement-first note:
  - apply the CUDA APOD discipline here: measure the actual hotspot before changing route behavior or duplicated GPU prep

## Recommended Action

- Overall bucket: `Small surgical refactor`
- Accept as-is:
  - main shared orchestration flow through `validation_core`
  - broad file-local test coverage
- Document only:
  - core vs Tauri ownership for `cross_ticker` and `validation_context`
  - `max_candidates` coercion/truncation behavior
  - auto-route benchmark policy caveats
- Small surgical refactor:
  - replace GPU reopt `panic!` sites with `Err(...)`
  - reject or document unsupported core-only request knobs
  - fix mixed-family route-diagnostic handling
- Larger refactor but defer:
  - split this file by family or by GPU-vs-CPU execution layers
- Benchmark-sensitive; do not touch yet:
  - discarded GPU metric passes and reusable-prep refactors
  - auto route-policy thresholds and messaging

## Exact Local Verification Required If Changed

If this file changes, run at minimum:

1. `cargo test -p ta_app_core composite_validation_smoke_runs_and_returns_stats -- --nocapture`
2. `cargo test -p ta_app_core composite_validation_iterative_engine_emits_delta_fields -- --nocapture`
3. `cargo test -p ta_app_core composite_mo_candidates_validate_consistently -- --nocapture`
4. `cargo test -p ta_app_core composite_validation_rejects_invalid_candidate_params -- --nocapture`
5. If touching route/fallback behavior and building with `cuda,cuda-backtest-kernel`, run representative GPU route/parity anchors:
   - `base_composite_validation_auto_keeps_gpu_route_when_uniform`
   - `vcc_validation_auto_uses_gpu_mixed_route_on_current_policy`
   - `carm_validation_gpu_kernel_advanced_route_outcome_parity_on_current_fixture`
   - `cdmc_validation_gpu_kernel_advanced_route_outcome_parity_on_current_fixture`
   - `kqsr_validation_gpu_kernel_reports_route_diagnostics`
6. If touching `cross_ticker` ownership or result fields, also run Tauri wrapper tests:
   - `cargo test -p ta_desktop_demo_app cross_ticker_composite_smoke_applies_stats -- --nocapture`
   - `cargo test -p ta_desktop_demo_app composite_cross_ticker_request_strips_full_validation_stack -- --nocapture`

## Verification Performed For This Packet

Executed on the current default feature set:

- `cargo test -p ta_app_core composite_validation_smoke_runs_and_returns_stats -- --nocapture`
- `cargo test -p ta_app_core composite_validation_iterative_engine_emits_delta_fields -- --nocapture`
- `cargo test -p ta_app_core composite_mo_candidates_validate_consistently -- --nocapture`

Note:

- GPU-specific route/parity tests in this file are largely feature-gated and were not exercised in the current default non-CUDA test run.

## Regression Risk And Feature-Preservation Constraints

This file should preserve:

- current validation result shape
- current route/fallback reporting vocabulary unless intentionally changed across callers/tests
- current feature coverage for walk-forward, reopt, SPA, perturb, trade-bootstrap, CSCV/PBO, and Tauri-managed cross-ticker orchestration

The biggest regression risks are:

- turning documented wrapper-owned fields into silent behavior changes
- changing GPU route labels without preserving parity tests and benchmark evidence
- breaking direct `ta_app_core` callers by changing request semantics without documenting the new ownership model

The correct improvement posture is to make the contract more explicit and the failure mode more recoverable first, then tackle performance-sensitive GPU cleanup with measurement.
