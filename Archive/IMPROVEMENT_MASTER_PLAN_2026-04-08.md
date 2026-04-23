# Improvement Master Plan

Last updated: 2026-04-08

## Goal

Continuously improve the existing VectorGrid application without adding net-new product areas.

This plan assumes:

- the feature inventory in [IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md](/C:/Rust Projects/ta_desktop_demo/IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md) is the source list of current surfaces
- [REPO_MAP.md](/C:/Rust Projects/ta_desktop_demo/REPO_MAP.md) remains the routing entrypoint
- work should be done in bounded slices with targeted verification

## Operating Rules

- do not add a new feature lane unless it is required to stabilize an existing one
- favor clarity, correctness, performance, and operator trust over breadth
- prefer finishing weak surfaces over starting adjacent ones
- every improvement slice should end with verification and an updated checklist state

## Recommended Execution Order

1. Core stability and operator trust
2. Optimization and validation correctness
3. Results clarity and artifact usability
4. Specialized workflow hardening
5. Testing, performance, and release discipline
6. Documentation and maintenance cleanup

## Completion Standard For Any Area

- behavior is clear and internally consistent
- existing tests are updated or new targeted tests are added
- route/fallback behavior is explicit
- error states are understandable
- docs or inline guidance are sufficient for future maintenance

## Area Plans

### 1. Desktop shell and overlays

- [ ] Audit top-level shell layering for duplicated or weak status surfaces
- [ ] Simplify overlay priority rules between runtime banner, task center, and command palette
- [ ] Normalize shell empty/loading/error states to one visual language
- [ ] Add or tighten shell-level tests where behavior is fragile

### 2. Startup and boot behavior

- [x] Disable startup animation overlay at the top-level mount site
- [ ] Verify boot-phase visibility is still honest through runtime banner/task surfaces
- [ ] Remove or archive startup overlay code only after deciding whether the component will return
- [ ] Add one explicit product note describing current startup behavior and rationale

### 3. Settings and workstation ergonomics

- [ ] Audit every settings control for real effect, stale options, and confusing labels
- [ ] Tighten runtime diagnostics wording in the settings workspace
- [ ] Ensure desktop notification/window-state settings degrade clearly on unsupported paths
- [ ] Add targeted tests for any settings-to-behavior coupling that is currently implicit

### 4. Data loading and dataset prep

- [ ] Review CSV load/resample/clear flows for bad-state recovery and user messaging
- [ ] Improve dataset readiness cues so optimization/validation state is never ambiguous
- [ ] Audit Polygon repair/top-off/validation flows for result clarity and cancellation behavior
- [ ] Add narrow regression coverage for data reset and rehydrate transitions

### 5. Optimization request shaping

- [ ] Audit optimize request builder legality checks against current backend route policy
- [ ] Reduce control drift between frontend capability summaries and backend truth
- [ ] Simplify or clarify advanced options that are too easy to misuse
- [ ] Expand tests for route-specific control legality where gaps remain

### 6. Optimization execution correctness and clarity

- [ ] Audit optimization result provenance so users can see exact vs approximate vs fallback execution clearly
- [ ] Tighten backend route/fallback diagnostics for operator-visible runs
- [ ] Review cancellation behavior and partial-result behavior for consistency
- [ ] Validate that current top-k and shortlist behavior stays aligned across routes

### 7. Overview/top/pareto results

- [ ] Review ranking/result tables for ambiguous metrics, labels, and sorting rules
- [ ] Tighten virtualization and table rendering around large result sets
- [ ] Improve result provenance display without adding new analysis products
- [ ] Add focused UI tests for the highest-risk result table interactions

### 8. Validation/drilldown/heatmap results

- [ ] Audit validation presentation for agreement with backend verdict semantics
- [ ] Improve drilldown/local-stability readability where current output is dense
- [ ] Tighten heatmap hover/selection/export behavior and edge-case handling
- [ ] Add tests for result-state transitions that currently rely on manual confidence

### 9. History/artifacts/meta/debug surfaces

- [ ] Normalize artifact metadata presentation across history, library, and replay views
- [ ] Improve search/filter/tagging clarity in artifact-heavy flows
- [ ] Reduce duplication between debug/meta/history surfaces where the same information appears differently
- [ ] Add regression coverage for artifact hydration and replay loading

### 10. Validation robustness pipeline

- [ ] Audit walk-forward/reopt/SPA/CSCV defaults and operator-facing descriptions
- [ ] Improve validation verdict explainability from backend to frontend
- [ ] Tighten fallback reporting when GPU validation paths are unavailable
- [ ] Add targeted parity tests for validation output contracts where practical

### 11. Cross-ticker/meta/ML confidence

- [ ] Review meta-selection and ML-confidence surfaces for unclear lifecycle/state messaging
- [ ] Tighten artifact naming/provenance in stage 4 / stage 5 outputs
- [ ] Verify model status and policy-evaluation flows expose failure and stale-model states clearly
- [ ] Add targeted checks around model-application and replay consistency

### 12. Fidelity audit and evidence tooling

- [x] Implement isolated fidelity-audit harness
- [ ] Run one bounded real-data audit and inspect the first artifact
- [ ] Define pass/fail interpretation thresholds for rank/top-k/verdict agreement
- [ ] Document when the harness should be rerun as part of workflow changes

### 13. Double-MA core engine

- [ ] Keep exact-grid path stable and benchmarked as the anchor execution surface
- [ ] Audit `GpuVdps` telemetry usefulness and user-facing interpretation
- [ ] Tighten regression coverage for route/fallback/metric-contract interactions
- [ ] Run perf gates before shipping any materially invasive engine change

### 14. Composite and specialized strategy families

- [ ] Inventory current parity state by family and confirm product-facing route claims are honest
- [ ] Tighten strategy-local organization where current flows are hard to follow
- [ ] Improve fallback and unsupported-feature messaging per strategy family
- [ ] Add targeted tests at the request-shaping and route-policy boundary

### 15. CompositeMo

- [ ] Clarify current CPU-bound status in product-facing surfaces and docs
- [ ] Review result usability and request validation without expanding scope
- [ ] Add or tighten tests around current multi-objective request handling
- [ ] Decide whether any existing controls should be hidden or simplified until stronger support exists

### 16. ML Signal

- [ ] Audit shell separation between ML Signal and the conventional workflow for user confusion
- [ ] Tighten dataset/model/status messaging in ML Signal panels
- [ ] Review policy sweep and validation output readability
- [ ] Add regression tests for mode switching and ML-specific state hydration

### 17. Options expression workflow

- [ ] Harden the current shipped options slice rather than expanding its ambition
- [ ] Improve artifact/reporting clarity for feasibility, tuple inspection, and event-path inspection
- [ ] Tighten overlap-quality and realism diagnostics already planned in repo docs
- [ ] Add targeted tests around current request-building and artifact replay behavior

### 18. Live execution runner

- [x] Audit risk-sensitive actions for confirmation, clarity, and status reporting
- [x] Tighten live snapshot and runner-state refresh behavior
- [x] Improve degraded/error-state wording for broker/runtime integration failures
- [x] Add tests for runner lifecycle transitions that can affect operator trust

### 19. Task lifecycle and runtime instrumentation

- [ ] Review task status taxonomy for any states that are redundant or unclear
- [ ] Tighten event throttling and progress presentation for long-running tasks
- [ ] Improve normalized runtime error messages where they are still too raw
- [ ] Add tests around cancellation, no-op completion, and late-event handling

### 20. Desktop/system integration

- [ ] Audit window-state restore/show behavior and edge-case logging
- [ ] Tighten notification permission/status handling in the UI
- [ ] Review desktop log surfacing for noise vs usefulness
- [ ] Add small verification notes for platform-specific behavior that is easy to forget

### 21. Reports/file operations

- [ ] Review report/save/open/reveal flows for consistency and failure messaging
- [ ] Tighten filename/path sanitization expectations in docs and tests
- [ ] Confirm report/export surfaces reflect current artifact schemas
- [ ] Add regression coverage for the most common export/save paths

### 22. Frontend test coverage

- [ ] Map high-risk UI flows with weak or no automated coverage
- [ ] Prefer focused component/store/service tests before broad snapshot-style additions
- [ ] Keep browser UX coverage limited to flows that catch real integration issues
- [ ] Track flaky or slow tests and simplify them instead of letting them accumulate

### 23. Backend/perf/profiling gates

- [ ] Keep CPU/GPU/workflow gates runnable and documented
- [ ] Audit benchmark scripts for stale assumptions or misleading defaults
- [ ] Tie major optimization-path changes to perf-gate evidence before handoff
- [ ] Keep profiling notes close to scripts so perf work remains reproducible

### 24. Command/output parity

- [ ] Keep generated command registry in sync with Tauri handler surface
- [ ] Run parity checks after command-surface changes
- [ ] Review output parity baselines when artifact schemas change
- [ ] Document any intentional compatibility breaks explicitly

### 25. Documentation and operational guidance

- [ ] Keep `REPO_MAP.md` current when subsystem ownership or routing changes
- [ ] Keep improvement docs synchronized with real code progress
- [ ] Reduce stale tracking docs where they duplicate newer canonical ledgers
- [ ] Write short maintenance notes after tricky fixes so future work starts faster

## Suggested Near-Term Sequence

This is the recommended next bounded sequence if the goal is continuous improvement without feature expansion:

1. Startup/boot cleanup
2. Optimization request-shaping audit
3. Validation verdict clarity audit
4. History/artifact surface cleanup
5. Fidelity audit first live run
6. Options-expression hardening slice
7. Live execution operator-safety cleanup
8. Perf-gate and benchmark maintenance pass

## Checklist Use Policy

- only mark an item complete when code and verification are both done
- if a line turns out to require a new feature, split it out and keep this plan improvement-only
- prefer updating this document after each completed slice instead of creating new ad hoc trackers unless the area is unusually complex
