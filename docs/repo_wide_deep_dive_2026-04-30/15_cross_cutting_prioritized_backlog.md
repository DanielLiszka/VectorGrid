# Cross-Cutting Prioritized Backlog

This backlog turns the repo-wide implementation plans into small batches. It favors additive work first, then evidence metadata, then route/runtime changes, then research lanes.

## Priority Rules

1. Keep CPU-only builds working.
2. Prefer additive metadata, reports, and tests before behavior changes.
3. Do not change trading, validation, or optimization defaults without evidence.
4. Keep Simple Mode public and streamlined.
5. Keep experimental GPU, ML, options, and execution features clearly labeled.
6. Run feature-specific tests before broad gates.

## Batch 1: Public Release And Safety Evidence

| Item | Source Plan | Risk | Main Files | Verification |
|---|---|---:|---|---|
| Fix current command-risk policy gap | `12_settings_privacy_security_and_local_state.md` | Low | `src-tauri/command-risk-policy.json`, `src-tauri/src/lib.rs` | `node web/scripts/check-tauri-command-risk-policy.mjs` |
| Release secret hygiene gate | `12_settings_privacy_security_and_local_state.md` | Low | `scripts/run_release_doctor.ps1`, `src-tauri/src/redaction.rs` | `powershell -File scripts/run_release_doctor.ps1 -Variant cpu` |
| Output parity public gate | `06_artifacts_history_reports_and_exports.md`, `13_release_packaging_and_public_docs.md` | Low | `scripts/run_output_parity_check.mjs`, `scripts/run_public_release_gate.ps1` | `node scripts/run_output_parity_check.mjs` |
| Node/npm reproducibility check | `13_release_packaging_and_public_docs.md` | Low | `web/package.json`, release scripts | `npm --prefix web ci`, frontend typecheck |
| Unsigned beta labeling | `13_release_packaging_and_public_docs.md` | Low/medium | `scripts/run_public_release_gate.ps1`, manifest script | unsigned beta dry run |
| PTX/native release manifest sync | `11_cuda_gpu_runtime_and_performance.md` | Low/medium | manifest script, release docs | public gate GPU variant dry run |
| Generated binding coverage report | `12_settings_privacy_security_and_local_state.md`, `14_verification_ci_benchmarking_and_qa.md` | Low/medium | `command_bindings.rs`, `generatedBindings.ts` | Specta export test, `npm --prefix web run check:bindings` |

Why first:
These changes improve public release safety without changing core math or trading behavior.

## Batch 2: Data And Artifact Durability

| Item | Source Plan | Risk | Main Files | Verification |
|---|---|---:|---|---|
| Identity-backed Polygon cache manifest | `01_data_polygon_lifecycle.md` | Medium | `src-tauri/src/polygon.rs`, `commands_data.rs` | polygon and data command tests |
| Atomic writes for settings/manifests/artifacts | `06_artifacts_history_reports_and_exports.md` | Medium | `settings.rs`, `polygon.rs`, `run_artifacts.rs` | corrupt/recover tests |
| Artifact health and repair dashboard | `06_artifacts_history_reports_and_exports.md` | Low/medium | `commands_artifacts.rs`, `run_artifacts.rs`, History UI | artifact and history tests |
| Cache inventory and retention dry run | `01_data_polygon_lifecycle.md`, `12_settings_privacy_security_and_local_state.md` | Low/medium | `data_cache.rs`, settings UI | cache/settings tests |
| Replay class contract | `06_artifacts_history_reports_and_exports.md` | Low | `run_artifacts.rs`, history service | artifact replay tests |

Why second:
These make existing workflows more durable and easier to support without changing optimization output.

## Batch 3: Simple Mode Public Workflow

| Item | Source Plan | Risk | Main Files | Verification |
|---|---|---:|---|---|
| True Simple Mode happy-path E2E | `04_frontend_simple_mode_public_workflow.md` | Low | `GuidedWorkspacePanel.vue`, `web/e2e` | frontend E2E gate |
| First-run bundled sample contract | `04_frontend_simple_mode_public_workflow.md`, `01_data_polygon_lifecycle.md` | Low/medium | data commands, Tauri resources, Guided UI | public release gate |
| Stable QA action IDs | `04_frontend_simple_mode_public_workflow.md`, `05_frontend_advanced_workbench_and_visualizations.md` | Low | action registry, components | frontend static QA |
| Compact validation result summary | `03_validation_gpu_and_evidence_pipeline.md`, `04_frontend_simple_mode_public_workflow.md` | Low | validation service, Guided UI | frontend unit tests |
| Mode transition continuity tests | `04_frontend_simple_mode_public_workflow.md` | Low | settings/workflow stores | frontend unit and E2E tests |

Why third:
This directly improves the public-facing experience while keeping Advanced Mode intact.

## Batch 4: Evidence Metadata For ML And Meta

| Item | Source Plan | Risk | Main Files | Verification |
|---|---|---:|---|---|
| Stage 5 training artifact references | `07_meta_stage5_ml_confidence.md` | Low/medium | `ml_confidence.rs`, `commands_meta.rs` | app-core and Tauri meta tests |
| Strict health train-only distinction | `07_meta_stage5_ml_confidence.md` | Low | `ml_confidence.rs` | ML confidence tests |
| Apply-time compatibility recheck | `07_meta_stage5_ml_confidence.md` | Low/medium | `commands_meta.rs`, meta service | meta tests |
| ML Signal strict score cache mode | `08_ml_signal_workflow.md` | Low/medium | `ml_signal.rs`, ML Signal commands | ML Signal quality gate |
| ML Signal full-vs-retained dual verdict | `08_ml_signal_workflow.md` | Low | `ml_signal.rs`, ML Signal UI | ML Signal tests |

Why fourth:
These reduce false confidence in ML surfaces without forcing new model behavior.

## Batch 5: GPU Validation And Route Evidence

| Item | Source Plan | Risk | Main Files | Verification |
|---|---|---:|---|---|
| Route capability matrix | `02_optimization_runtime_and_strategy_routes.md`, `11_cuda_gpu_runtime_and_performance.md` | Low/medium | route policy, frontend capability utilities | Rust route tests, frontend capability tests |
| Strict GPU validation residency policy | `03_validation_gpu_and_evidence_pipeline.md`, `11_cuda_gpu_runtime_and_performance.md` | Medium | validation core, commands, frontend services | app-core validation tests |
| Composite hybrid validation gate | `11_cuda_gpu_runtime_and_performance.md` | Medium | `composite_validate.rs`, `double_ma_validate.rs` | composite validation tests |
| GPU perf evidence dashboard | `11_cuda_gpu_runtime_and_performance.md`, `14_verification_ci_benchmarking_and_qa.md` | Low | GPU perf scripts | serialized GPU gates |
| Resident buffer reuse inventory | `11_cuda_gpu_runtime_and_performance.md` | Medium/high | GPU runtime modules | perf gates and parity tests |

Why fifth:
GPU behavior is valuable but should follow after route evidence and strict validation semantics are in place.

## Batch 6: Options, Portfolio, And Execution Research Hardening

| Item | Source Plan | Risk | Main Files | Verification |
|---|---|---:|---|---|
| Options per-window walk-forward metrics | `09_options_expression_workflow.md` | Low/medium | options validation, options UI | options tests |
| Options objective CPU oracle | `09_options_expression_workflow.md` | Medium | options expression, options GPU | CPU oracle and parity tests |
| Options data preview and quote gate | `09_options_expression_workflow.md` | Low/medium | options data commands, Polygon | options data tests |
| Portfolio replay matrix | `10_portfolio_and_live_execution.md` | Medium | portfolio feasibility, artifacts | portfolio tests |
| Execution risk policy V2 | `10_portfolio_and_live_execution.md` | Medium | execution policy, broker paper | execution tests |

Why sixth:
These are useful but closer to research/trading-risk surfaces, so they should come after public workflow and release foundations.

## Batch 7: Advanced Workbench And CI

| Item | Source Plan | Risk | Main Files | Verification |
|---|---|---:|---|---|
| Populated visualization E2E | `05_frontend_advanced_workbench_and_visualizations.md` | Low | visualizations, e2e | visual and e2e gates |
| Command palette drift guard | `05_frontend_advanced_workbench_and_visualizations.md` | Low | command palette, action registry | frontend tests |
| Accessibility/keyboard matrix | `05_frontend_advanced_workbench_and_visualizations.md` | Low | workbench components | a11y and E2E gates |
| CI workflow reactivation | `14_verification_ci_benchmarking_and_qa.md` | Medium | `.github/workflows/*` | PR checks and local repo gate |
| Benchmark baseline bootstrap | `14_verification_ci_benchmarking_and_qa.md` | Low | perf scripts, docs | CPU/GPU perf gate dry runs |

Why seventh:
These are high value, but most depend on stable product workflows and release gate decisions.

## Recommended First Ten Tasks

1. Add release secret hygiene scanning and document local key rotation policy.
2. Promote output parity into the public release gate.
3. Add Node/npm reproducibility checks to release scripts.
4. Add bundled sample data verification to release manifest and Simple Mode.
5. Add Simple Mode happy-path E2E.
6. Add identity-backed Polygon cache manifest verification.
7. Add artifact replay class display in History.
8. Add Stage 5 evidence tier and training artifact summary fields.
9. Add ML Signal strict score cache mode for release evidence.
10. Add route capability matrix tests across frontend and Rust policy.

## Work That Should Stay Deferred

- Tensor core or RT core production execution until a CPU oracle and CUDA baseline prove value.
- Live brokerage from Simple Mode.
- Options expression public Simple Mode path.
- Cross-stage VRAM persistence from optimization into validation.
- Any change to optimization/validation defaults without benchmark and scientific evidence.

## Backlog Acceptance Criteria

- Each batch should land as a small PR-sized change, not as a repo-wide rewrite.
- Each item must name the exact feature-specific verification command before implementation starts.
- New files listed in plans must be created intentionally; stale file names should be treated as blockers, not silently substituted.
- Any public release item must update the generated release summary or explain why it is not release evidence.
- Any medium/high-risk runtime item must include compatibility tests for legacy artifacts or CPU fallback behavior.
