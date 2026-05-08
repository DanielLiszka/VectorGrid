# Workflow Benchmark Coverage Matrix

Date: 2026-05-06
Status: audit plan plus initial support/campaign gates

## Goal

Define the full set of workflows that deserve performance coverage. This matrix should drive benchmark case creation and prevent optimizing only the already-visible Double-MA path.

## Coverage Matrix

| Workflow | Existing route | Current likely authority | Coverage status | Audit priority |
|---|---|---|---|---|
| CSV/sample data load | data commands | support perf gate | medium | high |
| Polygon cache load/top-off/repair | data commands | support perf gate for status/cache-adjacent paths; provider repair/top-off remains manual/credentialed | mixed | medium |
| Double-MA optimization, CPU | app-core/Tauri | CPU perf gate | strong-ish | high |
| Double-MA optimization, GPU | app-core/CUDA | GPU perf gate, faceoff scripts | strong | highest |
| Composite profile optimization | Double-MA product route plus composite paths | CPU/GPU gates partly | mixed | high |
| CompositeMo optimization | CPU | CPU perf gate likely partial | weak/mixed | medium |
| Double-MA validation | CPU/GPU fixed-candidate | GPU validation gate, workflow gate | medium/strong | highest |
| Composite validation | CPU/GPU route matrix | GPU validation gate partially | medium | high |
| Cross-ticker validation | Tauri orchestration | workflow/campaign-adjacent | weak | high |
| Candidate drilldown | analysis commands | GPU follow-up gate exists | medium | high |
| Local stability | analysis commands | GPU follow-up gate exists | medium | high |
| Options-expression optimization | app-core/Tauri | options GPU promotion gate | medium | high |
| Options validation/replay | options commands | targeted tests plus fixture-dependent options authority | weak/fixture-dependent | high |
| Options event-path inspection | options commands | fixture-dependent options authority, no reusable evidence yet | weak/fixture-dependent | medium |
| Research campaign | headless campaign runner | campaign perf gate, workflow gate | medium | high |
| Meta selection / Stage 4/5 | meta commands | ML/meta gates and targeted tests | weak | medium-high |
| ML Signal export/cache/validation/sweep | ML Signal commands | ML Signal quality gate | weak/mixed | medium-high |
| Portfolio feasibility | portfolio command | support perf gate | medium | medium |
| Local/paper/live execution scaffolding | execution commands | support perf gate for no-credential status/local-paper paths, execution safety gate for behavior | medium | medium |
| Artifact save/load/hydration | Tauri artifacts | support perf gate | medium | high |
| XLSX report generation | Tauri report commands | support perf gate plus output parity | medium | medium |
| Dynamic strategy IR preview | dynamic commands | support perf gate | medium | low-medium |
| System/settings/runtime commands | system/settings | frontend/release smoke | weak | low |
| History/library render | frontend | frontend E2E mocked | weak for latency | medium |
| Top results table render | frontend virtual table | component tests | weak for perf | medium |
| Validation table render | frontend virtual table | component/browser tests | weak for perf | medium |
| Options panel render | frontend | component tests | weak for perf | medium |
| App startup/shell mount | frontend/Tauri | release smoke | weak for latency | medium |

## Required Case Families

### Data Load

- small sample CSV cold load;
- large local CSV cold load;
- cached Polygon dataset load;
- resample from loaded dataset;
- dataset repair/top-off dry run.

### Optimization

- Double-MA CPU 58,300 pairs / 200k bars;
- Double-MA GPU 58,300 pairs / 200k bars;
- Double-MA GPU 250k pairs / 200k bars;
- composite profile route with ADX off;
- composite profile route with ADX on;
- CompositeMo CPU representative case.

### Validation

- Double-MA fixed-candidate CPU;
- Double-MA fixed-candidate GPU;
- GPU main plus intentional CPU-tail case;
- walk-forward enabled;
- CSCV/PBO enabled;
- SPA enabled;
- cross-ticker preserved-finalists mode.

### Follow-Up

- candidate drilldown after optimization;
- local stability after optimization;
- drilldown then local stability warm/cached;
- selected finalist revalidation.

### Options

- options-expression run with small imported fixture;
- options-expression run with realistic medium fixture;
- validation artifact replay;
- event-path inspection;
- quote-quality summary path;
- stress lane display normalization.

### Campaign

- optimize-only campaign;
- optimize-and-validate campaign;
- skip-existing campaign;
- failed dataset campaign;
- host preload and VRAM prefetch where available.

### ML / Meta

- meta selection over small saved-run set;
- meta selection over large saved-run set;
- Stage 4 experimental run;
- Stage 5 ML confidence run;
- ML confidence train/apply/evaluate;
- meta artifact replay/list/load.

### ML Signal

- dataset export;
- score-cache generation;
- validation;
- policy sweep;
- model bundle list/status;
- native unavailable fallback.

### Portfolio / Execution

- portfolio feasibility with 10/100 candidates;
- local paper snapshot;
- execution policy preview;
- local paper one-shot dry-run path;
- runner status without live credentials;
- live snapshot no-credential/mock path.

### Artifact/Report

- save run artifact;
- load run artifact detail;
- hydrate latest run;
- list artifact library;
- generate validation XLSX;
- generate campaign summary/report.

### Frontend

- app shell mount;
- top table render with large result rows;
- validation table render with large rows;
- options panel render with large tuple set;
- history library render with large saved-run list;
- drill chart render with many markers.

### Runtime / Tooling

- app startup to shell mount;
- workflow restoration;
- command palette open/search;
- task center with progress event burst;
- dynamic strategy IR preview;
- GPU status command;
- repo quality gate phase timing.

## Case Definition Template

```md
### Case: <case_id>

- Workflow:
- Timing boundary:
- Primary command/script:
- Input fixture:
- Dataset rows:
- Candidate count:
- Backend route:
- Feature flags:
- Expected output:
- Baseline path:
- Candidate output path:
- Companion diagnostics:
- Pass/fail policy:
- Current status:
```

## Prioritized Missing Coverage

1. Full validation module stack timing by route and CPU-tail class.
2. Candidate drilldown/local-stability warm follow-up timing.
3. Campaign throughput and skip-existing economics.
4. Artifact save/load/report generation throughput.
5. Options validation/replay/event-path timing.
6. Frontend large-table and chart-marker latency.
7. Data load/cache/resample cold versus warm timing.
8. ML/meta, ML Signal, and portfolio timing cases.
9. Execution scaffolding latency without live credentials.
10. Runtime shell and tooling phase timing.

## Definition Of Done

- Every row in the coverage matrix has either an authority script, a planned case, or a documented reason for deferral.
- Every high-priority weak row has a proposed benchmark case.
- Cold and warm boundaries are explicit.
- No workflow is optimized without a corresponding case id.
