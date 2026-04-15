# Review Packet: `src-tauri/src/commands_meta.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_meta.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and fairly disciplined, but not yet clean enough to accept as-is`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Artifact-contract-sensitive`, `Long-running-workflow`, `ML-policy-sensitive`

## What This File Owns

`commands_meta.rs` is the Tauri-side backend boundary for the Meta workspace, ML-confidence model lifecycle, Stage 4/Stage 5 reranking, offline policy evaluation, and saved meta/report artifact export.

This file owns:

- ML-confidence model status and CRUD-adjacent command surface:
  - `ml_confidence_model_status`: `src-tauri/src/commands_meta.rs:941-949`
  - `train_ml_confidence_model`: `src-tauri/src/commands_meta.rs:951-989`
  - `apply_ml_confidence_model`: `src-tauri/src/commands_meta.rs:991-1031`
  - `delete_ml_confidence_model`: `src-tauri/src/commands_meta.rs:1033-1048`
- Stage 4 / Stage 5 request normalization and orchestration:
  - request/result DTOs at `src-tauri/src/commands_meta.rs:86-257`
  - source resolution, auto-train, rerank, and artifact export at `src-tauri/src/commands_meta.rs:1099-1872`
  - public wrappers at `src-tauri/src/commands_meta.rs:1875-1897`
- offline ML-policy evaluation over saved Meta history:
  - request/result DTOs at `src-tauri/src/commands_meta.rs:86-145`
  - evaluation logic at `src-tauri/src/commands_meta.rs:1188-1508`
  - public wrapper at `src-tauri/src/commands_meta.rs:1899-1911`
- Meta selection over saved run artifacts, optional holdout evaluation, and XLSX/JSON output:
  - `run_meta_selection`: `src-tauri/src/commands_meta.rs:1932-1946`
  - blocking implementation: `src-tauri/src/commands_meta.rs:1978-2318`
  - holdout evaluation helpers: `src-tauri/src/commands_meta.rs:2646-3106`

This file does not own:

- the core meta-selection math or ML-confidence model implementation
- report/artifact replay normalization
- frontend request-building and panel-state behavior
- validation engines themselves

Those concerns are delegated to:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_campaign.rs`
- `web/src/services/metaService.ts`

## Classification

- Public surface: yes
  - command re-export lives in `src-tauri/src/commands.rs:93-96`
  - command registration lives in `src-tauri/src/lib.rs:136-149`
- Hot path: no
  - this is not a per-tick or live-trading loop
  - it is a heavy operator-triggered workflow surface
- Runtime-critical: yes
  - it owns long-running desktop workflows, shared run-slot occupancy, cancellation expectations, and artifact/report persistence
- Benchmark-sensitive: moderately
  - no benchmark gate is required today
  - Meta selection and holdout evaluation can still be expensive enough that cancellation and scan behavior matter
- CUDA/runtime-critical: not CUDA-critical
- Strategy-local: no
  - it spans multiple strategy families and delegates the strategy-specific validation work to other modules
- Low-risk support code only: no

## Direct Interactions

### Backend owners this file depends on

- shared progress sink and report builders:
  - `src-tauri/src/commands_optimize.rs:28-55`
  - `src-tauri/src/commands_reports_meta_xlsx.rs`
- artifact listing, path suggestion, pruning, and replay:
  - `src-tauri/src/run_artifacts.rs`
- run-slot and cancel-token ownership:
  - `src-tauri/src/state.rs:559-575`
- campaign orchestration:
  - `src-tauri/src/commands_campaign.rs:3597-3601`
- validation engines used during holdout:
  - `src-tauri/src/commands_validation.rs`
  - `crates/app_core/src/meta_select.rs`
  - `crates/app_core/src/ml_confidence.rs`

### Frontend contract anchors

- command allow-list:
  - `web/src/tauri/commands.ts:56-69`
- tracked/cancelable task policy:
  - `web/src/stores/taskStore.ts:248-282`
- primary frontend consumer:
  - `web/src/services/metaService.ts:2657-2885`
- Stage 5 / Stage 4 compatibility presentation:
  - `web/src/services/metaService.ts:239-242`
  - `web/src/services/metaService.ts:705-721`
  - `web/src/components/MetaWorkspacePanel.test.ts:561-572`

### REPO_MAP anchors

- module ownership note:
  - `REPO_MAP.md:570-571`
- current-state Stage 4/Stage 5 notes:
  - `REPO_MAP.md:1450-1455`
  - `REPO_MAP.md:1477-1481`

## Standards and Research Log

This packet was written after a targeted official-source review focused on Tauri command/runtime semantics, Tokio cancellation behavior, and stable Rust API/error-handling expectations for long-running desktop workflows.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file exposes a broad command surface with multiple long-running async wrappers
   - Standard informed:
     - heavy work should be isolated from the UI-sensitive command path
     - command DTOs are public IPC contracts

2. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - this file mixes `AppState::try_begin_run`, shared cancel state, and independent filesystem commands
   - Standard informed:
     - shared mutable state and command-level concurrency guarantees should be explicit, not assumed

3. Tokio task documentation
   - Source: <https://docs.rs/tokio/latest/tokio/task/>
   - Why it was consulted:
     - these commands rely on `spawn_blocking` and user-visible cancellation semantics
   - Standard informed:
     - `spawn_blocking` work is not forcibly abortable once started
     - cancellation expectations must match the actual cooperative checks in the code

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file publishes DTOs, artifact kinds, and compatibility aliases that frontend and replay tooling depend on
   - Standard informed:
     - contract stability matters more than local convenience

5. Rust Book error handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file frequently degrades or swallows failures while continuing a workflow
   - Standard informed:
     - data-loss-like failures should not disappear silently

6. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - the file has extensive helper coverage, but the review needed to assess whether real command/runtime behavior is verified
   - Standard informed:
     - tests should cover public behavior boundaries and failure modes, not only pure helpers

## Applicable Current Standards

- Long-running-command standard:
  - commands exposed as tracked or cancelable by the frontend should actually honor the shared cancel model, or the UX should stop promising that they do
- Artifact-contract standard:
  - artifact kind names, replay fields, and save paths are public backend contracts because replay and UI tooling depend on them
- Input-corpus integrity standard:
  - skipped or corrupted input artifacts should be surfaced explicitly when they can change meta-selection results
- Filesystem mutation standard:
  - model writes and deletes should not rely on overlapping public commands behaving politely
- Compatibility standard:
  - Stage 4 / Stage 5 alias behavior can remain for compatibility, but it should be treated as an intentional contract, not an accidental naming artifact

## Findings

### Correctness / Safety

#### 1. Unreadable or malformed run artifacts are silently dropped from meta selection

Objective issue:

- `read_json_value` converts any open or parse failure into `None`:
  - `src-tauri/src/commands_meta.rs:327-330`
- `run_meta_selection_blocking` simply advances progress and continues when that happens:
  - `src-tauri/src/commands_meta.rs:2017-2029`

Impact:

- a corrupted or unreadable run artifact changes the selection corpus without warning
- the resulting Meta output can look authoritative even though the input set was incomplete

Smallest justified fix:

- count and report skipped unreadable artifacts in `result.warnings`
- optionally include the first few offending filenames for operator diagnosis

Sensitivity:

- safe-now

#### 2. Holdout evaluation can succeed with zero evaluated datasets after all holdout inputs fail

Objective issue:

- holdout dataset load failures only increment `skipped`:
  - `src-tauri/src/commands_meta.rs:2772-2777`
- validation failures for both validation routes only increment `skipped` by collapsing work into `ok = false` and later `skipped += 1`:
  - `src-tauri/src/commands_meta.rs:2873-2880`
  - `src-tauri/src/commands_meta.rs:2972-2979`
- the function then returns `Ok(())` when `datasets == 0`:
  - `src-tauri/src/commands_meta.rs:3089-3092`

Impact:

- a caller can explicitly request holdout evaluation and still receive a successful Meta result with no strong signal that zero holdout datasets were actually evaluated
- this weakens the trustworthiness of holdout-adjusted ranking

Smallest justified fix:

- emit an explicit warning when `total > 0` but `datasets == 0`
- if the product wants stricter semantics, consider returning an error only when holdout was explicitly required and every candidate dataset failed

Sensitivity:

- safe-now

#### 3. ML-confidence model mutations are unlocked and write directly to the final model path

Objective issue:

- manual training writes directly to the final model path:
  - `src-tauri/src/commands_meta.rs:964-966`
- Stage 5 auto-train writes to the same final model path:
  - `src-tauri/src/commands_meta.rs:1524-1550`
- model apply reads the same file directly:
  - `src-tauri/src/commands_meta.rs:999-1004`
- delete removes the same file directly:
  - `src-tauri/src/commands_meta.rs:1039-1043`
- these model CRUD commands do not use the shared run guard:
  - `src-tauri/src/commands_meta.rs:941-1048`

Impact:

- overlapping manual train/apply/delete and Stage 5 auto-train can race on the same scope-local model file
- a reader can observe transient parse failures or missing-file behavior that is not tied to model quality, only to write timing

Smallest justified fix:

- use temp-file-and-rename model writes
- add a per-scope model mutation lock or reuse the shared run mutex for train/delete operations

Sensitivity:

- safe-now
- filesystem-contract-sensitive

### API and Naming

#### 4. Stage 4 is a compatibility alias, not a distinct artifact family

Objective issue:

- `suggest_stage4_artifact_path` delegates to the Stage 5 path helper:
  - `src-tauri/src/commands_meta.rs:590-592`
- the saved artifact kind is Stage-5-specific:
  - `src-tauri/src/commands_meta.rs:1852-1855`
- the response returns identical `stage4_json_path` and `stage5_json_path`:
  - `src-tauri/src/commands_meta.rs:1865-1870`
- the frontend already treats this as compatibility behavior:
  - `web/src/services/metaService.ts:239-242`
  - `web/src/services/metaService.ts:705-721`
  - `REPO_MAP.md:1450-1455`
  - `REPO_MAP.md:1477-1478`

Impact:

- artifact lineage, replay naming, and save-path semantics no longer distinguish Stage 4 from Stage 5
- changing this now is a cross-layer contract migration, not a local cleanup

Recommended action:

- document this as an intentional compatibility alias
- defer any rename or path split until replay tooling, frontend copy, and saved-artifact readers are updated together

Sensitivity:

- semantics-sensitive
- larger refactor but defer

### Error Handling

#### 5. Cancellation semantics are inconsistent with the frontend task model

Objective issue:

- the frontend marks these commands as cancelable via `cancel_current_run`:
  - `run_meta_selection`
  - `run_meta_stage4_experimental`
  - `run_meta_stage5_ml_confidence`
  - `train_ml_confidence_model`
  - `apply_ml_confidence_model`
  - `evaluate_ml_confidence_policy`
  - anchors: `web/src/stores/taskStore.ts:262-282`
- only `run_meta_selection` actually threads the cancel token into blocking work:
  - `src-tauri/src/commands_meta.rs:1938-1943`
  - checks at `src-tauri/src/commands_meta.rs:2018-2019`, `src-tauri/src/commands_meta.rs:2121-2123`, `src-tauri/src/commands_meta.rs:2768-2769`
- `run_meta_stage4_experimental` and `evaluate_ml_confidence_policy` acquire the run guard but do not consume the cancel token:
  - `src-tauri/src/commands_meta.rs:1875-1911`
- `train_ml_confidence_model` and `apply_ml_confidence_model` do not even participate in the shared run guard:
  - `src-tauri/src/commands_meta.rs:951-1031`

Impact:

- the user-facing cancel affordance promises more than the backend actually implements
- expensive Stage 5 or policy-eval work can occupy the shared run slot without an effective cancel path
- model commands are advertised like run tasks even though they do not share the same concurrency model

Smallest justified fix:

- either wire cancel-token checks through Stage 5 and policy evaluation
- or stop advertising those commands as cancelable by `cancel_current_run`
- for model CRUD/apply, either remove them from the shared cancel map or move them under a deliberate locking/cancellation policy

Sensitivity:

- safe-now

### Module Boundaries / Tech Debt

#### 6. Artifact-family filtering is duplicated locally instead of being owned in one place

Objective issue:

- local filename denylist lives here:
  - `src-tauri/src/commands_meta.rs:333-344`
- artifact-kind and replay-family rules already exist in `run_artifacts.rs`

Impact:

- artifact-surface drift now requires synchronized updates in multiple files
- the current Stage 4/Stage 5 alias complexity is exactly the kind of drift this duplication invites

Recommended action:

- centralize artifact-family filtering rules in `run_artifacts.rs` or a shared helper consumed from there

### Docs / Examples

#### 7. The Stage 4 / Stage 5 alias contract is documented indirectly, not locally

Objective issue:

- the current compatibility behavior is described in `REPO_MAP.md` and frontend presentation
- the command file itself does not explain why Stage 4 writes Stage-5-style artifacts or why both response paths are identical

Impact:

- the next maintainer can misread the alias as accidental debt and break replay compatibility while “cleaning up”

Recommended action:

- add a short local comment or module note near `suggest_stage4_artifact_path` and the shared Stage 4/Stage 5 response path

### Tests / Missing Verification

#### 8. Helper coverage is strong, but public command/runtime verification is still thin

Objective issue:

- in-file unit coverage is substantial:
  - `src-tauri/src/commands_meta.rs:3108-3930+`
- missing coverage includes:
  - no app-level Tauri test for any public `commands_meta.rs` command
  - no IPC-backed verification of public response DTO serialization
  - no backend integration test for save-path/report-path side effects
  - no direct frontend test coverage for `run_meta_stage4_experimental` or `delete_ml_confidence_model`
  - no true cross-layer test proving the frontend assumptions about Stage 4/Stage 5 aliasing still match backend output

Impact:

- command/runtime regressions can slip through even though helper logic is well tested

### Perf / Allocation / CUDA Concerns

#### 9. Performance pressure is mostly about cancellation granularity and scan transparency, not micro-allocations

Objective issue:

- Meta selection scans the saved run directory and parses artifact JSON serially:
  - `src-tauri/src/commands_meta.rs:1983-2029`
- cancellation checks are present only at coarse boundaries in the Meta selection path
- Stage 5 and policy-evaluation blocking flows do not currently expose meaningful cooperative cancellation

Recommended action:

- improve cancellation/visibility before attempting any CPU micro-optimization
- do not spend time on local allocation cleanup here unless profiling identifies a real bottleneck

## Recommended Action

- Overall packet disposition: `Small surgical refactor`

Safe-now changes:

- warn when unreadable artifacts are skipped
- surface a warning or stricter outcome when holdout evaluation processes zero datasets
- make cancellation semantics match the task model
- harden model writes and deletes with a clearer concurrency policy
- document the Stage 4 / Stage 5 alias locally

Larger refactor but defer:

- if the product wants Stage 4 to become a real distinct artifact family again, do that as a coordinated contract migration, not as a local rename

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
2. Add and run targeted tests for:
   - unreadable artifact warning propagation
   - holdout-requested path with zero successfully evaluated datasets
   - Stage 5 / policy-eval cancellation behavior
   - model read/write/delete overlap on the same scope
3. `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
4. If any response shape, aliasing field, or command contract changes, update and rerun the relevant mocked frontend tests:
   - `web/src/services/metaService.test.ts`
   - `web/src/components/MetaWorkspacePanel.test.ts`
   - `web/src/tauri/invoke.contract.test.ts`
5. If save-path or replay-path semantics change, add or update a Tauri-side runtime/wiring test under `src-tauri/tests/`

No CPU or GPU perf gate is required unless the change also alters the underlying app-core optimization or validation engines.

## Cross-File Coordination

- `src-tauri/src/run_artifacts.rs`
  - artifact kinds, replay families, path suggestion, and pruning rules live there
- `src-tauri/src/commands_campaign.rs`
  - campaign auto-meta flow consumes `run_meta_selection_blocking` output directly
- `web/src/services/metaService.ts`
  - Stage 5 replay path, ML-model lifecycle flows, and saved-artifact UX depend on this file's response shapes
- `web/src/stores/taskStore.ts`
  - cancelability and task semantics are coordinated there
- `REPO_MAP.md`
  - current alias and storage-location notes should stay aligned with real backend behavior

## Code-Quality and Debt Register

- High-value debt:
  - cancellation semantics do not match the frontend task model
  - model file mutation has no explicit locking/atomic-write policy
  - silent corpus truncation can occur during artifact scan
- Medium-value debt:
  - Stage 4 compatibility alias is real but easy to misread
  - holdout-zero-dataset outcome is under-signaled
- Low-value debt:
  - artifact-family filtering logic is duplicated locally

## Performance Opportunity Register

- Keep:
  - heavy work in `spawn_blocking`
  - saved-artifact reuse rather than recomputing everything from frontend state
- Improve:
  - cancellation responsiveness and operator feedback
  - visibility into scan failures before attempting speed work
- Avoid:
  - micro-optimizing helper allocations while scan integrity and cancellation gaps remain unresolved

## Implementation Staging

1. Safe-now hardening
   - add skipped-artifact warnings
   - add zero-holdout warning behavior
   - align task cancel semantics with actual backend capabilities
   - make model writes safer
2. Verification hardening
   - add command/runtime tests for the missing public behaviors
   - add frontend contract tests for any changed aliasing or warning fields
3. Contract cleanup
   - if desired, plan a real Stage 4 / Stage 5 contract split as a coordinated migration

## Change Blast Radius

- High blast radius:
  - Stage 4 / Stage 5 artifact naming or response-field changes
  - task cancel semantics that the frontend currently depends on
- Medium blast radius:
  - model write/delete concurrency policy
  - holdout warning/error behavior
- Low blast radius:
  - local documentation
  - artifact-skip diagnostics

## Observability Gaps

- no explicit warning count for unreadable or malformed run artifacts skipped during Meta selection
- no strong signal when holdout evaluation requested work but completed with zero usable datasets
- no explicit trace of which commands truly honor `cancel_current_run`
- no model-mutation telemetry around train/apply/delete overlap

## Open Questions and Assumptions

- Assumption:
  - the current Stage 4 surface is intentionally compatibility-only, not an actively distinct backend stage
- Open question:
  - should ML model train/apply/delete join the shared run mutex, or should they use a separate per-scope model lock?
- Open question:
  - should holdout-zero-dataset be a warning, or a hard error when holdout was explicitly requested?
