# Long-Running Feature Task Plans

Date: 2026-04-29
Status: scoped implementation plans for additive long-running feature work

## Purpose

This folder captures long-running feature opportunities that can be implemented independently with explicit scope, checklists, validation expectations, and regression controls.

The plans are intentionally separate from the existing roadmap files. Existing docs remain the source of historical context; this folder is the execution-oriented task set for future implementation.

## Plans

1. [Portfolio Feasibility Workspace](01_portfolio_feasibility_workspace.md)
2. [Advanced Strategy Drilldown And Local Stability Parity](02_advanced_strategy_drilldown_stability.md)
3. [Local Paper Brackets And Broker Watcher](03_local_paper_brackets_and_broker_watcher.md)
4. [GPU Validation Route Productization](04_gpu_validation_route_productization.md)
5. [Stage Completion Reporting Expansion](05_stage_completion_reporting_expansion.md)
6. [Options Finalist Acceptance Dashboard](06_options_finalist_acceptance_dashboard.md)
7. [Monte Carlo Reporting And Export Hardening](07_monte_carlo_reporting_export_hardening.md)
8. [Frontend QA Automation Expansion](08_frontend_qa_automation_expansion.md)

## Research Pass Assessment

Date: 2026-04-29

The current plans are scoped and actionable, but they are not all fully implementation-ready. They are strong enough to start discovery and first slices, but the high-risk plans need one additional design pass before coding:

- Portfolio Feasibility Workspace: implementation-ready for a read-only UI slice.
- Advanced Strategy Drilldown And Local Stability Parity: implementation-ready only one family at a time after a family inventory.
- Local Paper Brackets And Broker Watcher: not implementation-ready until the bracket state machine and broker watcher contract are written in more detail.
- GPU Validation Route Productization: not implementation-ready for route changes until the capability matrix and profiling protocol are completed.
- Stage Completion Reporting Expansion: implementation-ready for optional metadata/backfill slices.
- Options Finalist Acceptance Dashboard: implementation-ready for display-only view-model work, not for ranking or gate changes.
- Monte Carlo Reporting And Export Hardening: implementation-ready for compatibility/reporting, not for options-specific Monte Carlo.
- Frontend QA Automation Expansion: implementation-ready for selector and fixture work.

The research notes are captured in [External Research Notes](external_research_notes_2026-04-29.md).

## Global Rules

- Keep every task additive unless the plan explicitly calls out a compatibility migration.
- Preserve CPU-only builds unless the task is explicitly CUDA-only and gated by features.
- Do not change scoring, ranking, execution, or broker behavior as a side effect of UI/reporting work.
- Keep saved artifact compatibility. New fields must be optional or backfilled.
- For performance-sensitive work, serialize benchmark runs and follow the repository benchmark lock policy.
- Before implementation handoff, run the validation gates listed in the relevant plan.
- If a plan touches execution or CUDA routing, add targeted tests first or in the same change.

## Shared Validation Baseline

Each task has its own validation list. For broad changes, also consider:

- `powershell -File scripts/run_repo_quality_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- `powershell -File scripts/run_workspace_tests_stable.ps1`

For performance-sensitive changes:

- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`

For CUDA validation/performance changes when a GPU is available:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`
