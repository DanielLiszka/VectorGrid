# ML Meta Portfolio And Execution Performance Audit

Date: 2026-05-06
Status: audit plan

Update 2026-05-07: `scripts/run_support_perf_gate.ps1 -Case portfolio` now covers a deterministic portfolio feasibility scaling fixture, and `-Case execution` covers no-credential idle runner status, execution support matrix, Alpaca credential status shape, and disposable local-paper reset/snapshot timing. Live broker/API calls remain manual exclusions.

## Goal

Cover existing non-optimizer workflows that still affect research throughput and app responsiveness: meta selection, ML confidence, ML Signal, portfolio feasibility, and paper/live execution scaffolding.

These are not the project's primary performance identity surfaces, but they are in the current command surface and must be included in a repo-wide audit.

## Scope

- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/commands_portfolio.rs`
- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/portfolio_feasibility.rs`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/services/portfolioService.ts`
- `web/src/services/executionService.ts`

## Audit Boundaries

| Surface | Timing boundary | Notes |
|---|---|---|
| Meta selection | engine call and Tauri command path | candidate grouping, artifact parsing, selection logic |
| Stage 4 experimental | engine call | currently research/experimental |
| Stage 5 ML confidence | train/evaluate/apply command path | native/model availability must be explicit |
| ML confidence policy eval | command path | policy arm metrics and artifact save |
| ML Signal dataset export | command path plus file write | can be IO-heavy |
| ML Signal score cache generation | command path | cache size and model bundle status matter |
| ML Signal validation | engine call / command path | separate model inference from validation loop |
| ML Signal policy sweep | engine call / command path | sweep size and cache hit ratio matter |
| Portfolio feasibility | engine call / command path | candidate count and correlation/drawdown inputs |
| Local paper snapshots | command path | no live credentials required |
| Live snapshot/status | command path | must avoid real broker dependency for perf audit |
| Runner lifecycle | mocked/local command path | safety first; perf is secondary |

## Measurement Cases

### Meta / ML Confidence

- run meta selection on small saved-run set;
- run meta selection on large saved-run set;
- train ML confidence model with fixture data;
- apply ML confidence model to fixture candidates;
- evaluate ML confidence policy with fixed policy arms;
- load/replay meta artifact;
- list meta/stage/policy artifacts.

### ML Signal

- export ML signal dataset;
- generate score cache with native model available;
- generate score cache with native unavailable fallback;
- run ML signal validation;
- run ML signal policy sweep;
- list model bundles;
- query model status.

### Portfolio

- analyze portfolio feasibility with 10 candidates;
- analyze portfolio feasibility with 100 candidates;
- analyze portfolio feasibility with mixed asset classes;
- export/copy summary if present.

### Execution Scaffolding

- get local paper snapshot;
- reset local paper broker;
- preview execution policy decision;
- execute one local paper signal in dry/local mode;
- cancel local paper open orders;
- flatten local paper position;
- get live runner status without starting broker actions;
- get live snapshot with mocked/no credentials path;
- test Alpaca connection is excluded from automated perf baselines unless explicitly mocked.

## Performance Risks

| Surface | Risks |
|---|---|
| Meta selection | parsing many artifacts, grouping/sorting, repeated metric extraction |
| ML confidence | native library load, model serialization, feature schema checks |
| ML Signal score cache | large dataset export, model inference loop, cache write |
| ML Signal policy sweep | repeated threshold/policy evaluation |
| Portfolio feasibility | quadratic pair/correlation work, repeated metrics extraction |
| Execution workspace | polling frequency, snapshot serialization, event fanout |
| Artifact list/replay | many model/policy artifacts parsed repeatedly |

## Safety Boundaries

- No live broker actions as part of performance automation.
- No credential-dependent benchmark cases.
- No order placement timing claims.
- Local paper/dry-run paths only unless a manual explicit test is approved.
- Execution safety gate remains correctness/safety authority, not performance authority.

## Instrumentation Fields

- artifact count scanned;
- candidate count;
- feature row count;
- score-cache row count;
- model bundle id/version;
- native model availability;
- policy arm count;
- portfolio candidate count;
- execution snapshot order/position count;
- command wall milliseconds;
- serialization milliseconds where available.

## Optimization Ideas To Validate

- Cache parsed meta artifact summaries.
- Avoid re-reading full artifacts during meta grouping.
- Separate ML model load time from inference time.
- Add score-cache hit/miss diagnostics.
- Precompute ML Signal feature schema compatibility.
- Bound portfolio pairwise/correlation work with early diagnostics.
- Throttle live/paper snapshot polling and event fanout.

## Verification

- `powershell -File scripts/run_ml_signal_quality_gate.ps1`
- `powershell -File scripts/run_execution_safety_gate.ps1`
- targeted Rust tests for meta/ML/portfolio logic;
- `npm --prefix web run test -- metaService mlSignalService portfolioService executionService`;
- `npm --prefix web run typecheck`.

## Regression Watchpoints

- Performance changes must not weaken execution safety gates.
- Missing native ML dependencies must remain explicit.
- Policy-evaluation order and artifact output must remain stable.
- Portfolio feasibility must not emit order instructions.
- Execution polling changes must not hide emergency state.

## Definition Of Done

- ML/meta, ML Signal, portfolio, and execution scaffolding each have at least one timing case.
- Perf audit can distinguish compute, artifact IO, native model load, and UI render time.
- No live credential or broker side effect is required for automated performance evidence.
