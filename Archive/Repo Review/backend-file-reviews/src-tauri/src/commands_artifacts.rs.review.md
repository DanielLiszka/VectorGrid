# Review Packet: `src-tauri/src/commands_artifacts.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_artifacts.rs`
- Packet status: `Reviewed`
- Recommended action bucket: `Accept as-is`
- Sensitivity tags: `Public-surface-sensitive`, `Runtime-critical`, `Semantics-sensitive`

## What This File Owns

`commands_artifacts.rs` is the Tauri command-boundary module for saved-run and artifact-library persistence workflows.

It owns `21` desktop commands covering:

- save:
  - `save_run_artifact`
  - `save_batch_summary`
- list:
  - `list_run_artifacts`
  - `list_live_session_artifacts`
  - `list_artifact_library`
  - `list_meta_artifacts`
  - `list_stage4_artifacts`
  - `list_stage5_artifacts`
  - `list_ml_policy_eval_artifacts`
- load and hydrate:
  - `load_latest_run_artifact_summary`
  - `load_latest_run_artifact_hydration`
  - `load_run_artifact`
  - `load_run_artifact_hydration`
  - `load_run_artifact_detail`
  - `load_live_session_artifact_detail`
  - `load_meta_artifact_replay`
  - `load_stage5_artifact_replay`
  - `load_ml_policy_eval_artifact_replay`
- mutation and OS integration:
  - `update_artifact_tags`
  - `delete_run_artifact`
  - `open_runs_dir`

This file does not own:

- artifact schema enrichment
- filesystem rooting and path-safety policy
- artifact scanning, filtering, replay shaping, or hydration logic
- retention/pruning logic
- app-core strategy or runtime logic

Those concerns live in `src-tauri/src/run_artifacts.rs`.

## Classification

- Public surface: yes
  - all functions are `#[tauri::command]` exports in `src-tauri/src/commands_artifacts.rs:11-230`
- Hot path: no
  - the file is persistence and retrieval orchestration, not compute
- CUDA/runtime-critical: runtime-relevant, not CUDA-critical
  - the important behavior here is keeping filesystem-heavy work off the async/UI-sensitive command path
- Strategy-local: no
- Support code: yes, but persistence-sensitive and public-surface-sensitive

## Direct Interactions

### Upstream registration and contract exposure

- Re-exported from `src-tauri/src/commands.rs:44-51`
- Registered into the Tauri invoke handler in `src-tauri/src/lib.rs:150-170`
- Declared in the frontend known-command registry in `web/src/tauri/commands.ts:70-90`

### Runtime and state surface

- Uses `tauri::AppHandle` rather than `State<AppState>`
- Uses `tauri::async_runtime::spawn_blocking(...)` uniformly across the module
- Does not mutate in-memory application state directly

### Backend delegation target

All real behavior is delegated to `src-tauri/src/run_artifacts.rs`, including:

- save and enrichment
  - `save_run_artifact_blocking`
  - `save_batch_summary_blocking`
- listing
  - `list_run_artifacts_blocking`
  - `list_live_session_artifacts_blocking`
  - `list_artifact_library_blocking`
  - `list_meta_artifacts_blocking`
  - `list_stage4_artifacts_blocking`
  - `list_stage5_artifacts_blocking`
  - `list_ml_policy_eval_artifacts_blocking`
- load, replay, and hydrate
  - `load_latest_run_artifact_summary_blocking`
  - `load_latest_run_artifact_hydration_blocking`
  - `load_run_artifact_blocking`
  - `load_run_artifact_hydration_blocking`
  - `load_run_artifact_detail_blocking`
  - `load_live_session_artifact_detail_blocking`
  - `load_meta_artifact_replay_blocking`
  - `load_stage5_artifact_replay_blocking`
  - `load_ml_policy_eval_artifact_replay_blocking`
- mutation and reveal
  - `update_artifact_tags_blocking`
  - `delete_run_artifact_blocking`
  - `open_runs_dir_blocking`

Relevant `run_artifacts.rs` anchors from the current code:

- `save_run_artifact_blocking`: `src-tauri/src/run_artifacts.rs:1001`
- `save_batch_summary_blocking`: `src-tauri/src/run_artifacts.rs:1148`
- `list_artifact_library_blocking`: `src-tauri/src/run_artifacts.rs:2338`
- `load_run_artifact_hydration_blocking`: `src-tauri/src/run_artifacts.rs:2422`
- `load_latest_run_artifact_summary_blocking`: `src-tauri/src/run_artifacts.rs:2438`
- `load_latest_run_artifact_hydration_blocking`: `src-tauri/src/run_artifacts.rs:2451`
- `load_run_artifact_detail_blocking`: `src-tauri/src/run_artifacts.rs:2559`
- `load_live_session_artifact_detail_blocking`: `src-tauri/src/run_artifacts.rs:2575`
- `load_meta_artifact_replay_blocking`: `src-tauri/src/run_artifacts.rs:2583`
- `load_stage5_artifact_replay_blocking`: `src-tauri/src/run_artifacts.rs:2606`
- `load_ml_policy_eval_artifact_replay_blocking`: `src-tauri/src/run_artifacts.rs:2630`
- `open_runs_dir_blocking`: `src-tauri/src/run_artifacts.rs:2664`
- `list_run_artifacts_blocking`: `src-tauri/src/run_artifacts.rs:2774`
- `list_live_session_artifacts_blocking`: `src-tauri/src/run_artifacts.rs:2787`
- `list_meta_artifacts_blocking`: `src-tauri/src/run_artifacts.rs:2805`
- `list_stage4_artifacts_blocking`: `src-tauri/src/run_artifacts.rs:2811`
- `list_stage5_artifacts_blocking`: `src-tauri/src/run_artifacts.rs:2817`
- `list_ml_policy_eval_artifacts_blocking`: `src-tauri/src/run_artifacts.rs:2823`
- `load_run_artifact_blocking`: `src-tauri/src/run_artifacts.rs:2829`
- `delete_run_artifact_blocking`: `src-tauri/src/run_artifacts.rs:2836`
- `update_artifact_tags_blocking`: `src-tauri/src/run_artifacts.rs:2846`

### Filesystem and path-policy dependencies

Filesystem safety is not enforced in this wrapper. It is delegated to `run_artifacts.rs`, including:

- runs-root resolution under app-local data dir
- canonicalization and containment checks
- artifact save/update/delete path validation
- retention and pruning rules

That means this wrapper must stay aligned with `run_artifacts` contracts and must not bypass them.

### Representative direct consumers

Frontend consumers:

- artifact persistence from optimization:
  - `web/src/services/optimizeService.ts:2738`
- artifact persistence from validation:
  - `web/src/services/validationService.ts:1940`
- artifact persistence from ML signal:
  - `web/src/services/mlSignalService.ts:582`
- workflow bootstrap and latest-artifact hydration:
  - `web/src/services/workflowArtifactsSync.ts:70-78`
  - `web/src/services/workflowArtifactsSync.ts:120-128`
- saved-run history flows:
  - `web/src/services/historyService.ts:971-989`
  - `web/src/services/historyService.ts:1112-1117`
  - `web/src/services/historyService.ts:1120-1125`
  - `web/src/services/historyService.ts:1173-1174`
  - `web/src/services/historyService.ts:1342-1345`
- artifact-library replay and tagging flows:
  - `web/src/services/metaService.ts`
  - `web/src/services/optionsExpressionService.ts`
- live-session artifact review:
  - `web/src/services/executionService.ts:2195`
- runs-dir reveal from settings:
  - `web/src/services/settingsService.ts:12`

Backend-adjacent callers that use `run_artifacts` more directly:

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_reports.rs`

## Standards and Research Log

This packet was written only after a targeted research pass against official sources relevant to a Tauri persistence-boundary module.

### Sources consulted

1. Tauri command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why: this file is entirely a Tauri command surface
   - Relevant guidance:
     - async commands are preferred for heavy work
     - heavy work should not run on the main-thread command path
     - `Result` return types are the standard command error shape

2. Rust Book error handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why: this file is almost entirely error-forwarding boundary code
   - Relevant guidance:
     - failure propagation should preserve clarity
     - avoid obscuring error context without a strong reason

3. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why: this file has no direct Rust-side tests and depends on layered coverage

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why: this is a public command surface where predictability, dependability, and debuggability matter

5. Rust API Guidelines naming guidance
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why: command names form a cross-layer contract with the frontend

6. Clippy documentation
   - Source: <https://doc.rust-lang.org/clippy/>
   - Why: this module should remain simple enough that correctness, suspiciousness, complexity, and perf lints are sufficient guardrails

## Applicable Current Standards

- Tauri command-boundary standard:
  - keep blocking filesystem work off the async/UI-sensitive path
  - preserve the registered command names and argument expectations unless a coordinated migration exists
- Persistence-boundary standard:
  - do not bypass path-validation and runs-root policy owned by `run_artifacts`
- Repo-specific backend standard:
  - preserve artifact compatibility and saved-run workflows
  - avoid local refactors that destabilize persistence behavior without improving the underlying `run_artifacts` owner

## Findings

### Correctness / Safety

No objective local correctness or safety bug was identified in `commands_artifacts.rs`.

Why:

- the wrapper consistently delegates persistence work into `run_artifacts`
- it does not implement ad hoc filesystem logic of its own
- it keeps blocking I/O off the command async boundary through `spawn_blocking`

Important boundary constraint:

- the file is only safe because it does not bypass the containment and path-safety rules enforced in `run_artifacts`

### API and Naming

No naming issue found.

The command names are explicit and already baked into a wide frontend contract. Renaming any of them would be public-surface churn with immediate regression risk across history, workflow hydration, meta replay, live-session review, settings, optimize, validation, and ML flows.

### Error Handling

The wrapper uses `Result<_, String>` everywhere and flattens join failures with `.map_err(|e| e.to_string())?`.

Assessment:

- acceptable in the current repo command model
- not ideal if the long-term goal is richer structured error objects over Tauri
- not a file-local improvement target because a local conversion would create inconsistency across the command surface

Additional note:

- `load_run_artifact` uses raw `serde_json::Value` on the boundary, which is flexible but also weaker than typed DTO transport
- that is likely intentional because artifact schemas are versioned and heterogeneous
- any move away from `Value` would need a broader artifact-schema migration, not a local wrapper edit

### Module Boundaries / Technical Debt

The boundary split is good.

Positive points:

- the wrapper keeps artifact persistence policy inside `run_artifacts`
- it keeps the Tauri layer thin
- it does not intermingle app state, strategy logic, and filesystem concerns

Minor local debt:

- there is substantial repetition across the 21 wrappers

Assessment of the repetition:

- the repetition is acceptable because each wrapper is trivial and explicit
- introducing generic abstractions would likely make this file harder to scan without reducing meaningful risk
- local deduplication is not justified unless the command family expands further or a shared typed error strategy is introduced

### Docs / Examples

There is no module-level documentation comment.

Assessment:

- low severity
- acceptable today for an internal Tauri module
- if this file grows further, a short module comment documenting the invariant would help:
  - all filesystem behavior is delegated to `run_artifacts`
  - wrappers must preserve `spawn_blocking`
  - wrappers must not bypass path-safety checks owned elsewhere

### Tests / Missing Verification

This is the main local gap.

Objective issue:

- `commands_artifacts.rs` has no direct Rust-side tests

Context:

- backend behavior is heavily tested in `run_artifacts.rs`
- frontend command-contract coverage is broad across history, workflow hydration, meta replay, live-session review, settings, optimize, validation, and ML flows
- but the wrapper itself has no local regression net for:
  - wrong command wiring
  - wrong argument forwarding
  - wrong return-shape forwarding
  - accidental future divergence in the `spawn_blocking` boundary pattern

Assessment:

- real test debt, but moderate rather than severe
- because the file is thin and the owner module plus frontend consumers already cover most behavior
- if this wrapper changes, direct command-boundary regression coverage becomes justified

### Perf / Allocation / CUDA Concerns

No local performance problem found.

Important runtime observation:

- this is not a benchmark-sensitive file
- the only material runtime concern is preserving the off-thread I/O boundary with `spawn_blocking`

Do not optimize casually:

- do not replace the uniform `spawn_blocking` pattern with direct synchronous file work on the Tauri command path
- do not add caching or path preprocessing here unless the owning `run_artifacts` layer requires it

## Recommended Action

`Accept as-is`

Reasoning:

- no local correctness, naming, or performance defect was found
- the current module boundary is appropriate
- the wrapper is intentionally thin and explicit
- the most meaningful improvement work belongs in `run_artifacts.rs`, not here

Secondary deferred note:

- if the repo later standardizes structured Tauri error transport or typed artifact DTO migration, revisit this file as part of that broader command-surface effort

## Safe-Now vs Sensitive Work

### Safe now

- add a short module-level invariant comment if the command family continues to grow
- add direct wrapper-level integration tests if this file is modified anyway

### Semantics-sensitive

- changing any command name
- changing argument names or return shapes
- changing `serde_json::Value` usage for raw artifact save/load paths
- changing `Result<_, String>` behavior on the public Tauri boundary

### Runtime-sensitive

- removing or bypassing `spawn_blocking`
- inserting direct filesystem behavior into the wrapper
- bypassing `run_artifacts` path-safety and runs-root policy

### Benchmark-sensitive

- none locally

This file is persistence-sensitive and runtime-sensitive, not benchmark-sensitive.

## Cross-File Coordination

If this file changes, the main coordinated surfaces are:

- `src-tauri/src/run_artifacts.rs` as the actual owner of artifact semantics
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/metaService.ts`
- `web/src/services/executionService.ts`
- `web/src/services/settingsService.ts`
- backend callers in `src-tauri/src/commands_campaign.rs` and `src-tauri/src/commands_reports.rs`

Practical implication:

- wrapper-only hardening can stay local
- anything that changes artifact names, return shapes, or hydration/replay semantics is immediately cross-surface work

## Code-Quality and Debt Register

High-value quality improvements:

- add direct command-boundary coverage for forwarding and registration
- keep the wrapper intentionally thin and explicit
- prefer comments or packet references over adding generic abstraction across `21` wrappers

Debt that is safe to reduce now:

- missing wrapper-level tests

Debt that should be deferred:

- structured Tauri error migration
- typed DTO migration for raw artifact payloads
- command-family consolidation or pruning without a product-level artifact surface decision

## Performance Opportunity Register

There is no meaningful local optimization target in this wrapper.

Useful planning note:

- if artifact-library performance becomes a real issue, pagination, indexing, summary extraction, or lighter hydration belong in `run_artifacts.rs`, not here
- preserve `spawn_blocking` for filesystem-heavy work
- do not introduce wrapper-local caching that duplicates owner logic

## Implementation Staging

`Safe now`

- add wrapper forwarding tests
- add a short module invariant comment if the command family continues to grow

`Coordination required`

- command-name or response-shape changes
- replay or hydration contract changes
- any structured-error or typed-DTO migration

`Defer`

- pagination/indexing strategy
- hydration optimization work
- broader artifact-surface simplification

## Change Blast Radius

- `Local only`
  - comments
  - direct wrapper tests
- `Same subsystem`
  - changes that stay entirely aligned with `run_artifacts.rs`
- `Cross backend/frontend contract`
  - history screens
  - latest-run hydration
  - replay flows
  - artifact tagging/deletion
  - live-session review
  - runs-dir reveal

## Observability Gaps

Boundary-level observability is thin.

Notable gaps:

- no wrapper-level latency telemetry for artifact scans or hydration
- no command-level diagnostics around delete/tag/open failures beyond returned error strings
- no explicit “heavy operation” visibility at the command layer for large artifact libraries

These gaps do not justify local instrumentation churn by default, but they should be remembered if artifact-library scale becomes an issue.

## Open Questions and Assumptions

- Assumption:
  - `run_artifacts.rs` remains the only owner of path-safety and runs-root policy
- Open question:
  - is stage4/stage5 naming compatibility still intentionally transitional, or should the command surface eventually converge on one naming scheme?
- Open question:
  - do replay and hydration payloads need stronger version tagging at the command boundary?
- Open question:
  - should malformed latest-artifact summaries expose richer diagnostics to the frontend than they do today?

## Exact Local Verification Required If Changed

If this file changes, run at minimum:

1. Backend confidence:
   - `cargo test --workspace`

2. Command registry and cross-layer command presence:
   - `npm --prefix web run test -- src/tauri/commands.test.ts`

3. Saved-run history and artifact mutation flows:
   - `npm --prefix web run test -- src/services/historyService.test.ts src/services/workflowArtifactsSync.test.ts src/services/settingsService.test.ts`

4. Artifact-library and replay flows:
   - `npm --prefix web run test -- src/services/metaService.test.ts src/services/executionService.test.ts`

5. Save-artifact producer flows if save semantics or response shape change:
   - `npm --prefix web run test -- src/services/optimizeService.test.ts src/services/validationService.test.ts src/services/mlSignalService.test.ts`

Recommended additional verification if the command surface changes materially:

6. Broad repo confidence:
   - `powershell -File scripts/run_repo_quality_gate.ps1`

## Regression and Feature-Preservation Constraints

Any future change to this file must preserve:

- command names and argument expectations consumed by the frontend
- latest-run hydration behavior
- saved-run history actions
- meta/stage5/policy-eval replay flows
- live-session artifact review flows
- runs-dir reveal behavior
- delegation into `run_artifacts` for actual path policy and filesystem semantics

This file should only receive additive hardening or coordinated contract migrations. Do not remove or collapse artifact-family commands as “cleanup” unless the broader artifact product surface changes intentionally.
