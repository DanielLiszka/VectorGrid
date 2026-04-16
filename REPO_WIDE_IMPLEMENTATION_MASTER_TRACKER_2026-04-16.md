# Repo-Wide Implementation Master Tracker

Last updated: 2026-04-16

## Purpose

This document is the authoritative execution tracker for the next repo-wide implementation program.

It consolidates four closely related improvement plans into one governed program so the repo can be improved without fragmenting effort across overlapping ledgers:

1. `Workflow Contract And Replay Hardening`
2. `Validation Truth Alignment`
3. `Artifact System Cleanup`
4. `Options Expression Shipped-Slice Hardening`

The primary objective is not feature expansion.

The primary objective is to make current workflows more internally consistent, easier to maintain, safer to replay, and less likely to drift between:

- frontend request builders
- Tauri command boundaries
- persisted artifacts
- reopened/replayed workflows
- report/export surfaces
- operator-visible route and fallback summaries

## Governing Inputs

Use these as the default routing and policy inputs before touching code:

- [REPO_MAP.md](./REPO_MAP.md)
- [AGENTS.md](./AGENTS.md)
- [IMPROVEMENT_MASTER_PLAN_2026-04-08.md](./IMPROVEMENT_MASTER_PLAN_2026-04-08.md)
- [IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md](./IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md)
- [options_expression_optimizer_viable_plan_2026-04-02.md](./options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](./options_research/options_expression_future_scope_roadmap_2026-04-07.md)

When these docs disagree with current code, trust the code-facing entry points listed in `REPO_MAP.md`.

## Program Charter

This program should:

- improve current workflows without creating net-new product lanes
- preserve existing CPU fallback and currently supported safe routes
- reduce frontend/backend/artifact drift
- make replay and restore behavior deterministic and inspectable
- keep report and validation summaries honest relative to actual runtime behavior
- reduce warning noise only when the cleanup is low-risk and does not hide behavior

This program should not:

- expand the options workflow beyond the currently shipped slice
- add new benchmark-driven work before semantic consistency work is stable
- broaden scope into vendor code, generated artifacts, or inactive research notes
- remove capabilities as a disguised cleanup path
- perform large architectural rewrites unless a bounded migration path is written first

## Global Execution Rules

- Semantics first, performance second.
- Request, command, artifact, and replay correctness should be fixed before benchmark-sensitive refactors.
- Use targeted verification after every bounded slice.
- Use broad repo gates only after a logically complete cluster, not after every tiny edit.
- Run benchmark, faceoff, profiling, or workflow timing scripts only after semantic hardening is green.
- Serialize any benchmark or profiling run; never overlap them.
- Prefer updating this file after each completed slice instead of creating overlapping ad hoc trackers unless a sub-area becomes large enough to justify its own narrow implementation plan.

## Program Composition

### Primary epic

- `Epic A`: Workflow Contract And Replay Hardening

### Secondary but in-scope epics

- `Epic B`: Validation Truth Alignment
- `Epic C`: Artifact System Cleanup
- `Epic D`: Options Expression Shipped-Slice Hardening

### Cross-cutting support tracks

- `Track X1`: Warning-noise and dormant-path cleanup
- `Track X2`: Verification and gate discipline
- `Track X3`: Documentation and operator-facing wording consistency

## Recommended Execution Order

1. Epic A: Workflow Contract And Replay Hardening
2. Epic C: Artifact System Cleanup
3. Epic B: Validation Truth Alignment
4. Epic D: Options Expression Shipped-Slice Hardening
5. Track X1: Warning-noise and dormant-path cleanup
6. Track X2: Broad quality gates and, later, performance-sensitive follow-up

Rationale:

- Epic A reduces the highest-risk drift first.
- Epic C depends on replay and handoff contracts being explicit.
- Epic B is safer once request and artifact truth are tighter.
- Epic D should inherit the now-hardened artifact/replay/report patterns instead of inventing its own parallel logic.

## Completion Standard For Any Slice

A slice is only complete when all of the following are true:

- the affected workflow behaves consistently across live use, saved-run load, and rerun
- route and fallback summaries match real backend behavior
- error messages are explicit enough to diagnose the failure class
- artifact payloads carry the metadata needed for reopen and replay
- targeted tests are added or updated
- no obvious compatibility regression is introduced in the same surface

## Epic A: Workflow Contract And Replay Hardening

### Goal

Make workflow requests, saved-run replay, current-run hydration, and rerun behavior agree across the main repo workflows.

### Priority surfaces

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`

### Key risks to eliminate

- replay infers a different mode than the original live run
- artifact restore loses required metadata for reopen/rerun
- route summaries and actual fallback behavior diverge
- reopened runs land in the wrong workflow step or shell mode
- optional-path failures are swallowed and become silent operator confusion

### Checklist

- [ ] Build a repo-grounded contract matrix for optimize, validate, ML Signal, artifact reopen, and rerun flows.
- [ ] Identify where request-builder output shape differs from saved artifact request shape.
- [ ] Identify where rerun logic reconstructs less information than live execution had.
- [ ] Normalize frontend defaults so empty or omitted fields do not change replay mode silently.
- [ ] Normalize Tauri-side command validation for malformed paths, bundle ids, manifest references, and report targets.
- [ ] Ensure artifact payloads carry the minimum metadata needed for deterministic reopen.
- [ ] Ensure workflow step restoration is explicit rather than inferred from partial state when the artifact already carries stronger truth.
- [ ] Add targeted tests for apply-saved-run, rerun-saved-run, and artifact restore across at least conventional, validation-heavy, and ML Signal paths.

### Exit criteria

- replayed workflow mode equals original workflow intent
- restored workflow step equals saved artifact stage when stage truth exists
- rerun requests do not silently narrow or widen execution mode
- all touched flows have targeted regression coverage

## Epic B: Validation Truth Alignment

### Goal

Make validation summaries, warnings, route notes, and exported report semantics match actual backend validation behavior.

### Priority surfaces

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

### Key risks to eliminate

- UI route notes claiming support that the backend actually falls back from
- XLSX or CSV report wording hiding CPU fallback or mixed-route behavior
- summary text implying a stronger verdict than the backend actually computed
- validation warnings reflecting stale route policy rather than current code

### Checklist

- [ ] Audit frontend validation route text against current backend route policy in code.
- [ ] Audit report builders for fallback/mixed-route wording drift.
- [ ] Normalize the distinction between supported, CPU-fallback, mixed-route, and unsupported.
- [ ] Ensure shortlist, verdict, and warning summaries reuse the same truth model where practical.
- [ ] Add or extend tests where report wording and UI wording are expected to stay aligned.
- [ ] Review strategy-family route notes called out in `REPO_MAP.md` and correct any stale product-facing claims.

### Exit criteria

- frontend route summary, backend behavior, and report wording no longer disagree in the touched lanes
- validation warning text is explicit about fallback or unsupported scope
- workbook tests cover the corrected summary semantics

## Epic C: Artifact System Cleanup

### Goal

Make artifact save, list, reopen, hydrate, compare, and rerun behavior consistent across history-heavy flows.

### Priority surfaces

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/historyService.ts`
- `web/src/components` artifact-heavy panels routed from history and library flows
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports_api.rs`

### Key risks to eliminate

- artifact metadata means different things in different UI surfaces
- open/reveal/report actions accept malformed paths inconsistently
- tagged retention and family-specific retention logic drift from operator expectations
- meta/stage/ML artifact families restore partial state differently for no strong reason

### Checklist

- [ ] Inventory artifact family metadata actually persisted today.
- [ ] Define the minimum canonical metadata set needed for list, reopen, rerun, compare, and reveal.
- [ ] Normalize path sanitation and open/reveal behavior across artifact and report commands.
- [ ] Normalize artifact-family naming and UI summaries where semantics are duplicated but rendered differently.
- [ ] Add targeted tests for artifact CRUD, hydration, and reopen/reveal failure paths.
- [ ] Review prune and retention logic for asymmetric family behavior that is accidental rather than intentional.

### Exit criteria

- artifact metadata presentation is materially more consistent across history/library/replay surfaces
- open/reveal/report path failure behavior is explicit and uniform in the touched commands
- save/load/replay tests cover the corrected contracts

## Epic D: Options Expression Shipped-Slice Hardening

### Goal

Strengthen the current options workflow that already exists without turning this program into an options feature expansion effort.

### Priority surfaces

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`

### Key risks to eliminate

- shipped-slice request legality not matching actual backend support
- feasibility and tuple inspection artifacts carrying insufficient reopen truth
- event-path or report surfaces using wording that implies a larger product scope than exists
- saved options artifacts replaying differently than live requests

### Checklist

- [ ] Reconfirm the current shipped slice and explicitly mark out-of-scope future ideas.
- [ ] Audit request shaping against current backend support and failure modes.
- [ ] Audit options artifact and report payloads for replay and inspectability gaps.
- [ ] Tighten feasibility and realism diagnostics already described in current options docs where they are clearly implementable.
- [ ] Add tests for current request-builder legality, artifact replay, and report/open/reveal behavior.
- [ ] Keep all work scoped to the existing slice rather than broadening the options product.

### Exit criteria

- options request and artifact behavior are more deterministic inside the already-shipped scope
- no scope creep into new options product lanes occurred
- touched flows have targeted tests

## Track X1: Warning-Noise And Dormant-Path Cleanup

### Goal

Reduce compile-time warning noise that obscures real regressions, but only in bounded, low-risk slices.

### Candidate surfaces

- dormant helper binaries under `crates/app_core/src/bin/`
- narrowly dead `cfg(test)` helpers
- obviously feature-gated or no-op stubs that can be isolated more cleanly

### Rules

- do not hide real warnings with blanket `allow` attributes when a cleaner gate/stub split is available
- do not refactor large runtime modules just to reduce warnings
- prefer narrow feature gating, test gating, or explicit stub entry points

### Checklist

- [ ] Reduce warning clusters in helper bins that are inactive outside feature-enabled paths.
- [ ] Gate test-only helpers to test builds where practical.
- [ ] Add a short note when a warning is intentionally left because the safer cleanup would be too invasive.

## Track X2: Verification And Gate Discipline

### Goal

Keep implementation work evidence-backed without dragging every semantic slice into unnecessary benchmark work.

### Default verification posture

- targeted `cargo test` for touched crates/modules
- targeted `vitest` for touched web services/stores/components
- command-boundary tests in `src-tauri` when command behavior changes

### Use broader gates when a cluster is complete

- `powershell -File scripts/run_repo_quality_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- `cargo test --workspace`

### Defer performance-sensitive verification until semantics are stable

- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- CPU perf gates
- GPU perf gates
- workflow timing or benchmark faceoffs

### Checklist

- [ ] Keep a running list of which targeted tests validate each completed slice.
- [ ] Run broad non-benchmark quality gates after a logically complete cluster.
- [ ] Only re-enter benchmark or perf-gate work once semantic contract hardening is green.

## Track X3: Documentation And Operator-Facing Wording Consistency

### Goal

Keep UI copy, report wording, and operational docs aligned to actual behavior after implementation changes.

### Candidate surfaces

- `README.md`
- `RELEASE.md`
- `web/README.md`
- inline help text in workflow panels
- report summary wording where tests anchor expected phrasing

### Checklist

- [ ] Update operator-facing wording after route/fallback truth changes.
- [ ] Keep `REPO_MAP.md` references honest if an entrypoint or ownership boundary materially changes.
- [ ] Avoid silently leaving stale product wording after implementation hardening is finished.

## Dependency Map

- Epic A unblocks Epic C.
- Epic A reduces false assumptions inside Epic B.
- Epic B and Epic C should both inform Epic D so the options slice inherits the hardened artifact and validation conventions.
- Track X1 should happen opportunistically during the epics, not as a large standalone rewrite.
- Track X2 should run throughout.
- Track X3 should close behind each finished cluster.

## Immediate First Tranche

The highest-value opening tranche is:

1. conventional workflow request/replay drift
2. ML Signal request/replay/artifact drift
3. artifact open/reveal/reopen/report command consistency
4. validation route summary honesty in the currently touched strategy families

That tranche should produce:

- a stronger contract matrix
- better replay determinism
- fewer silent failure paths
- better confidence that later report and options hardening work will not duplicate the same mistakes

## Explicit Non-Goals For This Tracker

- adding new strategy families
- expanding options beyond the current shipped slice
- redesigning the shell or result workspaces as a visual project
- reopening large historical research topics unless current code demands it
- benchmark hunting before contract correctness work is complete

## Update Protocol

After each completed slice:

- mark the relevant checklist items in this file
- record the exact targeted verification commands run
- note any intentionally deferred work and why it was deferred
- only create a narrower implementation plan if one epic branch becomes too large to track cleanly here

Until that threshold is crossed, this file should remain the source-of-truth tracker for this repo-wide implementation program.
