# External Research Notes

Date: 2026-04-29
Status: primary-source research notes for the long-running feature task plans

## Verdict

The task plans in this folder are not all fully fleshed out yet. They are good scoped planning documents, but the high-risk tasks need more detailed contracts before implementation.

The strongest ready-to-start plans are:

- Portfolio Feasibility Workspace, limited to read-only artifact analysis.
- Stage Completion Reporting Expansion, limited to optional/backfilled metadata.
- Frontend QA Automation Expansion, limited to selectors, fixtures, and browser tests.

The plans needing more pre-code design are:

- Local Paper Brackets And Broker Watcher.
- GPU Validation Route Productization.
- Any future portfolio allocation output beyond feasibility.
- Options-specific Monte Carlo stress work.

## Source Findings

### Portfolio Construction Boundary

QuantConnect's LEAN docs separate portfolio construction from risk management and execution. Their portfolio construction model converts alpha insights into `PortfolioTarget` objects, and a `PortfolioTarget` provides the number of units of an asset to hold.

Source: https://www.quantconnect.com/docs/v2/writing-algorithms/algorithm-framework/portfolio-construction/key-concepts

Implication for this repo:

- The Portfolio Feasibility Workspace should remain read-only in the first slice.
- Any future allocation/target output must be a separate plan with risk model, rebalance model, margin model, and execution isolation.
- The current command risk policy is correct: feasibility must not emit weights, trades, broker orders, or durable portfolio artifacts.

### Alpaca Bracket Order Semantics

Alpaca documents bracket orders as three linked orders: entry, take-profit, and stop-loss. The exit legs do not become active until the entry order is completely filled. Cancellation of one order cancels remaining open orders in the group. Buy brackets require take-profit limit above stop-loss stop price; sell brackets reverse that relationship. Both take-profit limit and stop-loss stop prices are required. Extended hours are not supported for bracket orders, and `time_in_force` must be `day` or `gtc`. Partial take-profit fills adjust the stop-loss quantity. Nested order reads expose child legs.

Source: https://docs.alpaca.markets/docs/orders-at-alpaca

Implication for this repo:

- The local paper bracket simulator needs an explicit parent/child state machine.
- The simulator needs deterministic rules for partial fills and sibling cancellation.
- The UI must expose unsupported or approximated semantics instead of implying broker parity.
- Broker watcher work should normalize independent order records plus nested legs.

### Tauri Command/Event Boundary

Tauri documents commands for frontend-to-Rust calls and events/channels for Rust-to-frontend communication. Tauri's event system is not designed for low-latency or high-throughput use; channels are the streaming-oriented option.

Source: https://v2.tauri.app/es/develop/calling-frontend/

Implication for this repo:

- Long-running watchers and campaign-like workflows should keep command invocation separate from progress streaming.
- High-frequency broker or GPU progress should avoid event floods.
- Payloads that need type safety should remain command/result shaped where practical.

### NVIDIA CUDA Validation And Profiling

NVIDIA Compute Sanitizer provides memcheck, racecheck, and initcheck for CUDA memory, race, and uninitialized memory problems.

Source: https://docs.nvidia.com/compute-sanitizer/ComputeSanitizer/index.html

NVIDIA Nsight Systems captures CUDA API calls, GPU workload traces, memory transfers, kernel executions, and process/thread timeline data. Some collection modes can add significant overhead.

Source: https://docs.nvidia.com/nsight-systems/UserGuide/

NVIDIA Nsight Compute is the kernel profiler. Its profiling guide emphasizes metric collection overhead and recommends starting from programming model, hardware implementation, and performance guidelines.

Source: https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html

Implication for this repo:

- GPU validation changes need correctness checks before performance tuning.
- Nsight Systems should be used first for workflow timeline and transfer/launch issues.
- Nsight Compute should be reserved for kernel-level bottlenecks after a kernel is known to be the issue.
- Compute Sanitizer should be part of correctness-sensitive CUDA validation changes.

### Playwright Selector Strategy

Playwright recommends user-facing locators where possible, but supports test ids when role/text locators are not practical. The test id attribute is configurable, so the repo's `data-qa-action` convention can be made first-class in Playwright config.

Source: https://playwright.dev/docs/locators

Implication for this repo:

- The QA plan should keep user-facing assertions for visible behavior.
- Stable `data-qa-action` ids are appropriate for dense desktop workflow controls.
- The action inventory should be backed by Playwright config rather than ad hoc selector strings where possible.

### GitHub Actions Matrix And Artifacts

GitHub Actions supports matrix jobs for job variations and workflow artifacts for saving build/test outputs after a workflow run.

Sources:

- https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations
- https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts

Implication for this repo:

- Browser QA, desktop smoke, CPU perf, and GPU perf evidence should be artifact-producing jobs when moved into CI.
- Matrix dimensions should be explicit: surface, fixture family, backend mode, and optional desktop/GPU availability.

## Plan Maturity Ratings

| Plan | Current maturity | Missing before implementation |
| --- | --- | --- |
| Portfolio Feasibility Workspace | High for read-only UI | Detailed UI wire model and artifact selection state |
| Advanced Strategy Drilldown And Stability | Medium | Per-family parameter/replay inventory |
| Local Paper Brackets And Broker Watcher | Medium-low | Order state machine, fill model, watcher idempotency spec |
| GPU Validation Route Productization | Medium-low | Feature matrix, parity protocol, profiling protocol |
| Stage Completion Reporting Expansion | High | Exact new packet fields per stage |
| Options Finalist Acceptance Dashboard | Medium | View-model schema and legacy artifact cases |
| Monte Carlo Reporting And Export Hardening | Medium-high | Export schema and legacy field matrix |
| Frontend QA Automation Expansion | High | Selector naming registry and fixture ownership map |

