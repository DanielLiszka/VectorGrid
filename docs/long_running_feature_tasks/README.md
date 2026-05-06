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

## Detailed Sub-Implementation Plans

These docs refine the remaining work from the first eight plans into smaller implementation tracks and capture later release-polish tracks as they become concrete. Each track has its own audit findings, external research references where relevant, additive boundary, checklist, and validation plan.

9. [Local Paper Bracket Preview And Broker Observation Foundation](09_local_paper_bracket_preview_and_observation_foundation.md)
10. [Portfolio Feasibility Exports And Construction Research Gate](10_portfolio_feasibility_exports_and_construction_research_gate.md)
11. [Stage Completion Backend Backfill And Report Schema Versioning](11_stage_completion_backend_backfill_and_report_schema_versioning.md)
12. [Frontend QA Fixture Matrix, CI Artifacts, And Desktop Smoke Expansion](12_frontend_qa_fixture_matrix_ci_artifacts_desktop_smoke.md)
13. [Options Finalist Stress And Monte Carlo Diagnostics](13_options_finalist_stress_and_monte_carlo_diagnostics.md)
14. [GPU Validation Non-Benchmark Productization](14_gpu_validation_nonbenchmark_productization.md)
15. [Simple Mode Public Workflow Polish](15_simple_mode_public_workflow_polish.md)
16. [Frontend QA Fixture Matrix Full Implementation](16_frontend_qa_fixture_matrix_full_implementation.md)
17. [Validation Evidence Pack](17_validation_evidence_pack.md)
18. [Research Campaign Studio](18_research_campaign_studio.md)
19. [Candidate Explainability And Local Stability Workbench](19_candidate_explainability_local_stability_workbench.md)
20. [Artifact Analytics Lab](20_artifact_analytics_lab.md)
21. [Options Finalist Studio](21_options_finalist_studio.md)
22. [GPU Follow-Up Acceleration](22_gpu_follow_up_acceleration.md)

## Research Pass Assessment

Date: 2026-04-29

The original plans are scoped and actionable, but they are not all equally implementation-ready. The detailed sub-implementation plans above define the next safe slices:

- Portfolio exports: implementation-ready only for compact research exports and readiness diagnostics, not weights or orders.
- Broker preview: implementation-ready for non-mutating local paper bracket previews; long-lived live broker watchers remain deferred.
- Stage completion backfill: implementation-ready for merge/backfill and optional schema metadata with tolerant legacy handling.
- Frontend QA: implementation-ready for fixture ownership, dashboard artifacts, and non-default desktop smoke expansion.
- Options MC: implementation-ready for contract and frontend diagnostic scaffolding only; options-local MC engine requires a separate bounded algorithm slice.
- GPU validation: implementation-ready for docs and disabled-CI metadata only while CUDA benchmark work is deferred.
- Simple Mode polish: implementation-ready for frontend-first public workflow clarity, run summaries, optional validation recommendations, empty/error states, and release-facing Simple path QA.
- Frontend QA fixture matrix full implementation: implemented locally on 2026-05-04 as an additive, non-runtime QA infrastructure program that splits fixtures, expands deterministic browser workflows, covers aggregate Meta, aggregate Batch campaign, Composite route-summary, and task-center lifecycle paths, tightens action coverage, and improves local evidence reporting.
- Validation evidence pack: implemented locally on 2026-05-04 as an additive validation-local artifact/report/frontend evidence contract that defines the app-core pack DTO and builder without changing scoring, ranking, or route policy.
- Research workstation expansion wave: researched on 2026-05-06 as five implementation plans focused on Campaign Studio, Candidate Explainability, Artifact Analytics, Options Finalist Studio, and GPU Follow-Up Acceleration. These plans prioritize research leverage, comparability, diagnostics, and follow-up speed without adding broker risk or strategy sprawl.

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

Current GPU note: CUDA benchmark/profiling work is intentionally deferred while a separate GPU-intensive task may be running. Non-CUDA code, docs, frontend QA, CPU tests, and non-benchmark inventory checks can proceed.
