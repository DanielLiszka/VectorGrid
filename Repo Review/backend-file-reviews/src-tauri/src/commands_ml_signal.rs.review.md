# Review Packet: `src-tauri/src/commands_ml_signal.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_ml_signal.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small and coherent, but not yet clean enough to accept as-is`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Filesystem-contract-sensitive`, `Workflow-specific`, `Validation-contract-sensitive`

## What This File Owns

`commands_ml_signal.rs` is the Tauri-side backend boundary for the dedicated ML Signal workflow.

This file owns:

- public Tauri commands for:
  - dataset export: `src-tauri/src/commands_ml_signal.rs:95-109`
  - score-cache generation: `src-tauri/src/commands_ml_signal.rs:111-125`
  - ML-signal validation: `src-tauri/src/commands_ml_signal.rs:127-143`
  - ML-signal policy sweep: `src-tauri/src/commands_ml_signal.rs:145-161`
  - model-bundle listing: `src-tauri/src/commands_ml_signal.rs:163-173`
  - model-bundle status: `src-tauri/src/commands_ml_signal.rs:175-203`
- the ML-signal-specific validation context guard:
  - `validate_ml_signal_context`: `src-tauri/src/commands_ml_signal.rs:31-73`
- ML model-bundle directory and manifest-path resolution:
  - `ml_signal_models_dir`: `src-tauri/src/commands_ml_signal.rs:75-81`
  - `bundle_manifest_path`: `src-tauri/src/commands_ml_signal.rs:83-85`
- the only local response DTO:
  - `MlSignalModelStatusResponse`: `src-tauri/src/commands_ml_signal.rs:14-18`

This file does not own:

- the underlying ML-signal dataset export, score-cache build, validation, or policy-sweep algorithms
- run-artifact storage/hydration logic
- frontend ML-signal workflow presentation

Those concerns are delegated to:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Classification

- Public surface: yes
  - commands are re-exported from `src-tauri/src/commands.rs:103-106`
  - commands are registered in `src-tauri/src/lib.rs:139-144`
- Hot path: no
  - this file is operator-triggered workflow orchestration, not a live runtime path
- Runtime-critical: moderately
  - it participates in the shared run mutex and cancel flow for long-running ML-signal jobs
  - it also exposes user-visible model-bundle filesystem status
- Benchmark-sensitive: low
  - the expensive work is delegated into app-core and already offloaded through `spawn_blocking`
- CUDA/runtime-critical: no
- Strategy-local: yes
  - this is a dedicated ML Signal workflow surface
- Low-risk support code only: no
  - the file is small, but it still owns public command and filesystem behavior

## Direct Interactions

### Backend owners this file depends on

- shared run-slot and loaded-data ownership:
  - `src-tauri/src/state.rs:408-422`
  - `src-tauri/src/state.rs:559-575`
- app-core ML-signal operations:
  - `crates/app_core/src/ml_signal.rs:1010`
  - `crates/app_core/src/ml_signal.rs:1585`
  - `crates/app_core/src/ml_signal.rs:2182`
  - `crates/app_core/src/ml_signal.rs:2216`
  - `crates/app_core/src/ml_signal.rs:2335`
  - `crates/app_core/src/ml_signal.rs:2378`
- run-artifact hydration/summary consumers:
  - `src-tauri/src/run_artifacts.rs:1774-1857`
- campaign routing explicitly excludes ML Signal:
  - `src-tauri/src/commands_campaign.rs:3814`

### Frontend contract anchors

- command allow-list:
  - `web/src/tauri/commands.ts:59-64`
- primary frontend consumer:
  - `web/src/services/mlSignalService.ts:829-1178`
- tracked/cancelable task policy:
  - `web/src/stores/taskStore.ts:215-218`
  - `web/src/stores/taskStore.ts:252-255`
  - `web/src/stores/taskStore.ts:272-275`
- workflow artifact contract:
  - `web/src/stores/workflowArtifactsStore.ts:58-174`
- history/replay consumers:
  - `web/src/services/historyService.test.ts:423-974`

### REPO_MAP anchors

- module ownership note:
  - `REPO_MAP.md:572`
- ML Signal file map:
  - `REPO_MAP.md:1488`

## Standards and Research Log

This packet was written after a targeted official-source review focused on Tauri command/state semantics, Tokio blocking-task cancellation, and Rust filesystem/path APIs relevant to model-bundle lookup.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file is a thin public Tauri command boundary and uses async wrappers over blocking work
   - Standard informed:
     - heavy work belongs off the command/UI-sensitive path
     - command DTOs are public IPC contracts

2. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - four commands participate in the shared run mutex and cancel token
   - Standard informed:
     - command concurrency guarantees should be consistent with frontend task semantics

3. Tokio task documentation
   - Source: <https://docs.rs/tokio/latest/tokio/task/>
   - Why it was consulted:
     - the long-running commands all use `spawn_blocking`
   - Standard informed:
     - cancellation is cooperative and only meaningful if the delegated blocking work actually checks the token

4. Rust `std::fs::create_dir_all`
   - Source: <https://doc.rust-lang.org/std/fs/fn.create_dir_all.html>
   - Why it was consulted:
     - read-only status/list commands currently call a helper that always creates the models directory
   - Standard informed:
     - `create_dir_all` is a real mutation and should not be hidden behind what appears to be a read-only query

5. Rust `std::fs::canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why it was consulted:
     - model status builds a path from user-provided `bundle_id` with no containment check
   - Standard informed:
     - when a command resolves a user-derived path into a sensitive app-local directory, containment should be explicit rather than assumed

6. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - the file is a small but public workflow surface and forwards multiple app-core DTOs directly over IPC
   - Standard informed:
     - public behavior should be predictable, testable, and not surprising

## Applicable Current Standards

- Tauri boundary standard:
  - heavy ML-signal work should remain off the command thread
  - command DTOs and status payloads are public frontend contracts
- Shared-run standard:
  - commands presented as cancelable by the frontend should actually participate in the run slot and cancel flow, or the frontend should not promise that behavior
- Filesystem-boundary standard:
  - model-bundle lookup must remain constrained to the app-local ML-signal models directory
  - read-only inspection commands should not mutate disk unless that behavior is explicit and justified
- Validation-contract standard:
  - ML-signal validation context checks should stay aligned with the wider validation artifact contract, or local divergence should be clearly intentional

## Findings

### Correctness / Safety

#### 1. `ml_signal_model_status` allows user-controlled path escape from the ML models directory

Objective issue:

- bundle manifest paths are built from raw trimmed `bundle_id`:
  - `src-tauri/src/commands_ml_signal.rs:83-85`
- `ml_signal_model_status` uses that path directly:
  - `src-tauri/src/commands_ml_signal.rs:176-201`
- there is no containment check, canonicalization check, or segment sanitization

Impact:

- a `bundle_id` containing `..` or path separators can escape `models/ml_signal` and probe an arbitrary `manifest.json`
- this turns a status command into a filesystem traversal surface

Smallest justified fix:

- restrict `bundle_id` to a safe filename segment policy, or canonicalize the resolved manifest path and reject anything outside `models/ml_signal`

Sensitivity:

- safe-now
- high confidence

#### 2. Read-only model-inspection commands mutate disk and require write permission

Objective issue:

- `ml_signal_models_dir` always calls `create_dir_all`:
  - `src-tauri/src/commands_ml_signal.rs:75-81`
- both read-only commands call it:
  - `list_ml_signal_model_bundles`: `src-tauri/src/commands_ml_signal.rs:163-173`
  - `ml_signal_model_status`: `src-tauri/src/commands_ml_signal.rs:175-203`
- the underlying app-core listing logic already tolerates a missing root directory:
  - `crates/app_core/src/ml_signal.rs:2378-2385`

Impact:

- simple list/status reads now mutate disk
- those commands can fail in read-only or permission-constrained environments where a pure status query should still succeed

Smallest justified fix:

- split the helper into:
  - a read-only `ml_signal_models_dir_if_present`
  - and a create-if-needed path used only by actual writers

Sensitivity:

- safe-now

### Error Handling

#### 3. One malformed bundle manifest can fail the entire bundle list

Objective issue:

- `list_ml_signal_model_bundles` directly returns `list_ml_signal_model_bundle_summaries(&dir)`:
  - `src-tauri/src/commands_ml_signal.rs:163-173`
- the app-core iterator propagates manifest classification errors per entry:
  - `crates/app_core/src/ml_signal.rs:2385-2406`

Impact:

- one unreadable or malformed bundle manifest can brick the whole bundle-picker/status list
- that is brittle for an inspection surface where partial results are more useful than all-or-nothing failure

Smallest justified fix:

- degrade to partial results plus issues per bad bundle, or surface a best-effort list with an aggregated warning

Sensitivity:

- safe-now
- may require coordinated adjustment in app-core or wrapper behavior

### API and Naming

#### 4. `validate_ml_signal_context` is a local fork of broader validation-contract logic

Objective issue:

- the ML-signal wrapper has its own local compatibility gate:
  - `src-tauri/src/commands_ml_signal.rs:31-73`
- the general validation path uses shared helpers elsewhere:
  - `src-tauri/src/commands_validation.rs`
  - `src-tauri/src/validation_common.rs`
- this local version hard-requires `strategy_kind == "MlSignal"` and independently compares CSV path and timeframe

Impact:

- ML-signal validation acceptance can drift from the rest of the validation surface
- old or manually built artifacts can be rejected or accepted differently than comparable non-ML workflows

Recommended action:

- document the divergence if intentional
- otherwise consider converging on shared validation-common logic

Sensitivity:

- semantics-sensitive
- defer if shared-validation migration would change existing acceptance rules

### Tests / Missing Verification

#### 5. The file has no Rust tests of its own and no command-level wiring coverage

Objective issue:

- there is no `#[cfg(test)]` block in this file
- the module-scoped test run succeeded with `0` matching tests:
  - local verification: `cargo test -p ta_desktop_demo_app commands_ml_signal -- --nocapture`
- the only current runtime wiring test file covers cancel commands only:
  - `src-tauri/tests/command_runtime_wiring.rs:10-31`

Impact:

- local helpers like `validate_ml_signal_context`, `bundle_manifest_path`, and status fallback behavior are completely unverified on the Rust side
- there is no IPC-backed test proving these commands still wire correctly after change

Recommended action:

- add direct Rust tests for:
  - context mismatch handling
  - bundle ID validation / path containment
  - missing-manifest fallback payload
  - read-only list/status behavior

### Perf / Allocation / CUDA Concerns

#### 6. There is no meaningful local performance problem here

Observation:

- the expensive work is already offloaded using `spawn_blocking`:
  - `src-tauri/src/commands_ml_signal.rs:104-158`
- the wrapper itself does not present a hot-loop performance concern

Recommended action:

- do not spend time micro-optimizing this file
- fix correctness, filesystem behavior, and verification first

## Recommended Action

- Overall packet disposition: `Small surgical refactor`

Safe-now changes:

- constrain `bundle_id` to the ML models directory
- stop mutating disk from read-only list/status commands
- make bundle listing more resilient to malformed bundles
- add Rust tests for local helper and command behavior

Document-only if code is not being changed yet:

- explicitly note whether ML-signal validation intentionally diverges from the shared validation-common policy

Larger refactor but defer:

- if the team wants ML-signal validation contract unification, treat that as a coordinated cross-file cleanup with `commands_validation.rs` and `validation_common.rs`

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_ml_signal -- --nocapture`
2. Add and run targeted Rust tests for:
   - `bundle_id` traversal and containment rejection
   - read-only status/list behavior when the models directory is absent or read-only
   - missing-manifest fallback response
   - `validate_ml_signal_context` mismatches for strategy kind, CSV path, and timeframe
3. `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
4. If command payload shapes or response fields change, update and rerun:
   - `web/src/services/mlSignalService.test.ts`
   - `web/src/stores/workflowArtifactsStore.test.ts`
   - `web/src/services/historyService.test.ts`
5. If workflow artifact shape changes, verify the saved-run replay path still hydrates `ml_signal` details correctly

## Cross-File Coordination

- `crates/app_core/src/ml_signal.rs`
  - owns the real export/build/validation/sweep logic and bundle-manifest classification
- `src-tauri/src/run_artifacts.rs`
  - consumes and summarizes the persisted `ml_signal` artifact subtree
- `web/src/services/mlSignalService.ts`
  - owns the real frontend contract and saved-artifact mutation path
- `web/src/stores/workflowArtifactsStore.ts`
  - normalizes the saved `ml_signal` payload for replay/history
- `web/src/services/historyService.ts`
  - depends on the saved-run artifact shape remaining compatible

## Code-Quality and Debt Register

- High-value debt:
  - bundle-status lookup is missing path containment hardening
  - the file has zero Rust tests
- Medium-value debt:
  - read-only commands mutate disk
  - bundle-list failure semantics are too brittle
- Low-value debt:
  - ML-signal validation contract logic is locally duplicated rather than obviously shared

## Performance Opportunity Register

- Keep:
  - `spawn_blocking` offload for heavy app-core work
- Improve only if needed:
  - none locally beyond avoiding unnecessary directory creation on read paths
- Avoid:
  - micro-optimizing the wrapper before fixing filesystem and verification gaps

## Implementation Staging

1. Safe-now hardening
   - add `bundle_id` containment validation
   - split read-only vs create-if-needed models-dir helpers
   - make list behavior robust to bad bundles
2. Verification hardening
   - add Rust tests for local helpers and command behaviors
   - update frontend contract tests if any response shape changes
3. Contract cleanup
   - decide whether ML-signal validation should share more logic with the general validation path

## Change Blast Radius

- High blast radius:
  - anything that changes the saved `ml_signal` artifact payload shape
- Medium blast radius:
  - model-bundle status/list response changes
  - validation-context acceptance behavior
- Low blast radius:
  - path containment hardening
  - read-only helper split

## Observability Gaps

- no explicit warning path for malformed bundles during listing
- no test-backed signal for when read-only model inspection unexpectedly creates directories
- no Rust-side coverage proving context mismatch messages stay stable

## Open Questions and Assumptions

- Assumption:
  - ML-signal validation is intentionally kept on a dedicated workflow lane and not folded into campaign orchestration
- Open question:
  - should malformed bundles appear as degraded entries in the list, or should the list API grow a `warnings` side channel?
- Open question:
  - should ML-signal validation converge on `validation_common.rs`, or is local strictness part of the product contract?
