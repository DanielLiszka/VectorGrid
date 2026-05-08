# Runtime Shell Tooling And Support Surface Performance Audit

Date: 2026-05-06
Status: audit plan

Update 2026-05-07: `scripts/run_support_perf_gate.ps1 -Case dynamic` covers dynamic IR/GPU-status tooling timing, and `-Case live-status` covers no-credential Polygon live idle status/stop command timing. Actual provider stream start remains manual/credentialed.

## Goal

Cover the remaining runtime and support surfaces that affect perceived performance, developer throughput, or diagnostic responsiveness: app startup, shell mount, settings/workflow restoration, dynamic strategy preview, system commands, command palette, task center, live stream event handling, and repo tooling.

## Scope

- `web/src/App.vue`
- `web/src/preflight.ts`
- `web/src/components/workbench/WorkbenchShell.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/stores/workflowStore.ts`
- `web/src/services/workflowBootstrap.ts`
- `src-tauri/src/commands_dynamic.rs`
- `src-tauri/src/commands_system.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/state.rs`
- `scripts/*.ps1` gates and profiling helpers
- `src-tauri/src/bin/*` diagnostic binaries

## Support Surface Matrix

| Surface | Why it matters | Timing boundary | Priority |
|---|---|---|---|
| App startup/preflight | first impression and dev/test loop | shell first usable | medium |
| Workbench shell mount | every user session | frontend mount | medium |
| Workflow restoration | app resumes state on launch | local storage/settings read + store sync | medium |
| Settings persistence | avoids UI stalls during preference changes | command path / frontend write | low-medium |
| Command palette | navigation productivity | open/search/action latency | low-medium |
| Task center | long-run visibility | progress event processing/render | medium |
| Dynamic strategy preview | existing command surface | command path | low-medium |
| GPU status | diagnostics readiness | command path | low |
| Window effects | desktop polish, should be cheap | command path | low |
| Live candle stream events | event throughput/backpressure | event ingest/render | medium |
| Repo quality gates | developer cycle time | script wall time | medium |
| Benchmark gates | evidence generation throughput | serialized gate wall time | medium |

## Measurement Cases

### Runtime / Shell

- app boot to shell mount;
- preflight duration;
- workflow state restoration;
- switch workspace via rail;
- switch workflow step;
- open command palette and search;
- open task center with many events;
- restore settings perspective.

### Dynamic/System Commands

- preview dynamic Double-MA IR;
- format dynamic strategy IR JSON;
- encode dynamic strategy constant bits;
- get GPU status;
- set window effect;
- save simple summary Markdown.

### Event Throughput

- optimization progress event burst;
- campaign progress event burst;
- live candle stream event burst;
- live execution runner event burst;
- task-store throttling behavior.

### Tooling

- repo quality gate wall time;
- workspace stable test wrapper wall time;
- frontend E2E gate wall time;
- CPU perf gate wall time;
- GPU perf gate wall time;
- release candidate gate wall time.

## Instrumentation Fields

- boot stage label;
- stage start/end timestamp;
- component mount milliseconds;
- command palette item count;
- task event count;
- progress event throttle count;
- shell workspace switch milliseconds;
- script wall time;
- script included lanes;
- skipped lane count;
- failure phase.

## Optimization Ideas To Validate

- Avoid mounting inactive panels.
- Reduce preflight synchronous work.
- Keep workflow restoration cheap and deterministic.
- Throttle progress/event updates without losing final state.
- Lazy-load command palette heavy data.
- Make gate scripts report phase timings.
- Split slow quality-gate phases only if semantics remain clear.

## Verification

- `npm --prefix web run test -- workflowStore CommandPaletteOverlay TaskCenterOverlay`
- `npm --prefix web run typecheck`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1` for tooling behavior if scripts change

## Regression Watchpoints

- Startup timing fields must not become runtime identity.
- Event throttling must not hide cancellation/failure/final status.
- Command palette must remain keyboard accessible.
- Gate script speedups must not skip required checks silently.
- Dynamic strategy preview performance must not weaken validation of generated IR.

## Definition Of Done

- Runtime/support command surfaces have explicit low/medium/high performance priority.
- App startup, shell mount, task progress, and command palette have timing cases.
- Developer tooling wall-time is visible and phase-attributed.
