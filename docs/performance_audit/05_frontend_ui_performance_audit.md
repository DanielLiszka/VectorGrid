# Frontend UI Performance Audit

Date: 2026-05-06
Status: audit plan

## Goal

Measure and improve user-visible responsiveness in the Vue/Tauri workbench without confusing frontend latency with backend compute performance.

## Scope

- `web/src/App.vue`
- `web/src/components/workbench/WorkbenchShell.vue`
- large panels:
  - `TopWorkspacePanel.vue`
  - `ValidationWorkspacePanel.vue`
  - `OptionsExpressionPanel.vue`
  - `HistoryWorkspacePanel.vue`
  - `MetaWorkspacePanel.vue`
  - `CandidateDetailWorkspacePanel.vue`
- stores and services:
  - Pinia stores
  - request/normalization services
  - chart/table models

## Frontend Timing Boundaries

| Boundary | Meaning |
|---|---|
| shell mount | first usable workbench shell |
| panel mount | async component load plus first render |
| mocked workflow | UI behavior with mocked Tauri responses |
| real command render | invoke command through Tauri/backend then render result |
| table render | data normalization plus visible virtual rows |
| chart render | chart model creation plus first nonblank render |
| store update fanout | Pinia state mutation through dependent computed/views |

## Audit Targets

| Surface | Bottleneck risks |
|---|---|
| Top results | large row normalization, virtual rows, heatmap markers |
| Validation table | evidence summary normalization, virtual row churn |
| Options panel | large component size, many computed sections, large tuple rows |
| History library | artifact list hydration, detail loading, comparisons |
| Meta/campaign | large aggregate table, campaign progress update frequency |
| Drill/candidate detail | chart markers, trade table sync, curve model creation |
| Workbench shell | unnecessary panel mounts, store watcher fanout |

## Instrumentation Plan

Add development-only timing helpers first:

```ts
performance.mark("vg:validation:normalize:start")
performance.mark("vg:validation:normalize:end")
performance.measure(
  "vg:validation:normalize",
  "vg:validation:normalize:start",
  "vg:validation:normalize:end",
)
```

Requirements:

- dev-only or feature-flagged;
- no noisy production logs;
- optional export of measurements into QA artifacts;
- labels include mocked versus real backend.

## Frontend Optimization Checklist

- [ ] Verify async panels are not mounted when inactive.
- [ ] Check large computed values for unnecessary recompute.
- [ ] Check service normalization for repeated full-array work.
- [ ] Check virtual table row keys and stable row identity.
- [ ] Check chart marker limits and selected-marker rerender behavior.
- [ ] Check Pinia store mutations for large object replacement.
- [ ] Check options panel component size and split candidates.
- [ ] Check history/artifact hydration for incremental loading.
- [ ] Check bundle/chunk size before adding libraries.

## Candidate Metrics

- shell first usable time;
- panel first render time;
- normalization milliseconds;
- row count;
- visible virtual row count;
- marker count;
- chart model build milliseconds;
- render long tasks over 50ms;
- memory before/after large artifact load;
- bundle chunk size.

## Verification

- `npm --prefix web run test -- <targeted test>`
- `npm --prefix web run typecheck`
- `npm --prefix web run build`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`

If timing assertions are added:

- keep thresholds loose and environment-aware;
- store diagnostic output rather than brittle pass/fail until stable.

## Regression Watchpoints

- Virtualization must remain accessible.
- Canvas/chart evidence needs text summaries.
- Do not add heavy analytics libraries to default bundle without bundle evidence.
- Mocked frontend performance is not real backend latency.
- Avoid watcher changes that break persisted workspace state.

## Definition Of Done

- Major panels have basic timing diagnostics.
- Large tables and charts have row/marker count telemetry.
- The slowest frontend surfaces are ranked.
- Any UI optimization has before/after timing in the same boundary.
