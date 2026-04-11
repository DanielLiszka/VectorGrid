# Validation And Report Throughput — Consolidated Review

This document packages the full consolidated answer for the bundle **Validation And Report Throughput**.

## Files reviewed

- `03_validation_report_throughput_2026-04-10.md`
- `AGENTS.md`
- `commands_reports_validation_common.rs`
- `commands_reports_validation_xlsx.rs`
- `commands_validation.rs`
- `double_ma_validate.rs`
- `REPO_MAP.md`
- `run_artifacts.rs`
- `validationService.ts`

## 1. Surface understanding

After rechecking the bundle doc against the current owner files, the merged conclusion is that the epic is pointed at a real problem, but its current framing is still too coarse for the repo’s actual validation path. The plan correctly wants to improve throughput without changing verdict semantics, workbook meaning, or artifact/report compatibility, but today’s user-visible path already spans command-side validation, optional cross-ticker reruns, branchy XLSX export/save behavior, open/reveal side effects, and artifact persistence/provenance enrichment. The doc names only three broad phases, so it still understates where latency and trust can actually move.

Sources: `03_validation_report_throughput_2026-04-10.md`; `validationService.ts`; `commands_validation.rs`; `REPO_MAP.md`.

## 2. Consolidated must-fix findings

### Must-fix 1 — the timing model is still too coarse

**Severity:** high  
**Type:** correctness  
**Location:** `03_validation_report_throughput_2026-04-10.md` Scope Boundaries / Proposed Improvement Approach / Verification Plan; `web/src/services/validationService.ts`; `REPO_MAP.md` reporting notes.

The current frontend measures `commandMs` only around the validation command, but the end-to-end path then optionally auto-exports XLSX, may take one of several export branches (`save_report_xlsx_from_artifact`, `save_report_xlsx`, or `build_report_xlsx` fallback), then runs `openPathWithRevealFallback` and sometimes `reveal_path`, and only then persists the validation artifact and reports the final end-to-end status. The repo map also says `commands_reports_shared.rs` owns report-path suggestion plus open/reveal helpers. A compute / workbook / persistence split is therefore not enough by itself; the doc needs a branch-aware timing matrix and it needs to say which phases count toward the user-facing SLA versus diagnostic evidence.

**Recommended change:** Replace the current phase model with a branch-aware timing matrix covering command runtime, command-side cross/aux compute, XLSX save-from-artifact, XLSX save-from-report, `build_report_xlsx` fallback, open/reveal, and artifact persistence.

Sources: `validationService.ts`; `commands_validation.rs`; `REPO_MAP.md`; `03_validation_report_throughput_2026-04-10.md`.

### Must-fix 2 — the benchmark scenarios do not yet freeze cross-ticker semantics, so a “throughput win” could hide less validation work

**Severity:** critical  
**Type:** correctness  
**Location:** `03_validation_report_throughput_2026-04-10.md` Checklist / Proposed Improvement Approach; `src-tauri/src/commands_validation.rs`.

Current cross-ticker modes are not equivalent workloads. `run_composite_validation` and `run_double_ma_validation` include cross-ticker work inside the same command runtime timer, and preserved-finalists mode truncates candidates before reruns. On the double-MA path, the preserved-finalists rerun explicitly rebuilds the cross-ticker request with `walk_forward`, `reopt_walk_forward`, `spa`, `perturb`, and `trade_bootstrap` omitted, and the code comments say it keeps only the verdict-relevant stack rather than rerunning non-gating evidence modules. That means a faster result can come from doing less validation, not from speeding up the same validation. The representative scenario matrix must therefore declare cross mode, candidate-cap behavior, and omitted-module behavior up front.

**Recommended change:** Add a scenario-contract subsection that freezes cross mode, finalist-cap behavior, omitted-module behavior, auto-export on/off, and compatibility replay cases before any implementation work begins.

Sources: `commands_validation.rs`; `03_validation_report_throughput_2026-04-10.md`; `commands_reports_validation_common.rs`.

### Must-fix 3 — the plan’s truth/fidelity safeguard is too weak for the current workbook and route contract

**Severity:** critical  
**Type:** operator-truth  
**Location:** `03_validation_report_throughput_2026-04-10.md` Regressions To Avoid / Checklist; `src-tauri/src/commands_reports_validation_common.rs`; `src-tauri/src/commands_reports_validation_xlsx.rs`; `web/src/services/validationService.ts`.

The current export model and workbook are more specific than “preserve workbook sheet, chart, and compatibility behavior.” `commands_reports_validation_common.rs` derives approval labels from `validation_verdict`, falls back to legacy shortlist behavior in compatibility mode, and carries route/module/cross/fallback/warning summaries in `ValidationExportModel`. `commands_reports_validation_xlsx.rs` writes `Approved Candidates`, `Shortlist`, `Distributions`, `Audit Data`, `Dashboard`, and `Regime Audit`, keeps charts/audit tied to the full row set, and has a test that empties the Approved Candidates sheet when the run verdict rejects final approval even if a candidate individually passed. The frontend also already surfaces route/fallback and related execution summaries to users. The doc needs to freeze those concrete invariants, not a generic “no workbook regression” rule.

**Recommended change:** Replace the generic workbook safeguard with a concrete invariant list: compatibility mode, route/fallback/omitted-module summaries, Approved Candidates suppression under a rejecting run verdict, full-row chart/audit inputs, `Audit Data`, `Regime Audit`, Dashboard route/fallback truth, and current chart presence/data-source behavior.

Sources: `commands_reports_validation_common.rs`; `commands_reports_validation_xlsx.rs`; `validationService.ts`; `03_validation_report_throughput_2026-04-10.md`.

### Must-fix 4 — persistence is not just latency; it is a canonical truth surface

**Severity:** high  
**Type:** migration  
**Location:** `03_validation_report_throughput_2026-04-10.md` Scope Boundaries / Checklist / Rollout notes; `web/src/services/validationService.ts`; `src-tauri/src/run_artifacts.rs`; `REPO_MAP.md` artifact notes.

The frontend persists a full run artifact containing `config_fingerprint`, run req/res, validation req/res, `report_xlsx_path`, `dataset_meta`, `ml_signal`, `optimization_metric_surface`, and tags. The backend then treats the saved artifact as a richer handoff object with `route_provenance`, `lineage`, dataset identity/content-fingerprint enrichment, and retention behavior; the repo map explicitly says the frontend treats artifacts as canonical cross-workflow handoff objects. Any throughput change that defers, narrows, or weakens this path risks making reopened runs less trustworthy even if the validation math stays the same. The doc needs an explicit persistence-truth freeze naming the fields and enrichments that must remain intact.

**Recommended change:** Add a persistence-truth subsection that explicitly preserves validation req/res, `report_xlsx_path`, `dataset_meta`, `route_provenance`, `lineage`, `config_fingerprint`, `optimization_metric_surface`, and current retention/enrichment behavior.

Sources: `validationService.ts`; `run_artifacts.rs`; `REPO_MAP.md`; `03_validation_report_throughput_2026-04-10.md`.

### Must-fix 5 — the verification and rollback plan is too weak for the current risk surface

**Severity:** high  
**Type:** verification  
**Location:** `03_validation_report_throughput_2026-04-10.md` Current Verification Surface / Verification Plan / Rollout notes; `REPO_MAP.md`; `AGENTS.md`; `src-tauri/src/commands_reports_validation_xlsx.rs`.

The doc currently names `cargo test -p ta_desktop_demo_app validation_report_ --lib`, workflow gate, and output parity, but the repo map’s high-signal verification surface is wider: `cargo test --workspace` or at least `cargo test -p ta_app_core` for backend confidence, artifact/report parity commands, and CPU/GPU perf gates when changes are performance-sensitive. REPO_MAP also points to `ta_headless_validate` and `ta_double_ma_validation_cost` as existing manual helpers. On top of that, the workbook tests already lock in sheet/chart presence and approved-sheet suppression. The doc should replace the current generic verification section with a concrete cross-layer matrix and explicit rollback triggers for route-truth, workbook-truth, and artifact-provenance regressions.

**Recommended change:** Rewrite the verification section into a matrix that includes app-core validation tests, tauri-side workbook tests, workflow gate, output parity, CPU perf gate, GPU perf gate when applicable, and the existing headless/manual helpers. Add explicit rollback triggers for route truth, workbook truth, and artifact provenance regressions.

Sources: `REPO_MAP.md`; `AGENTS.md`; `commands_reports_validation_xlsx.rs`; `03_validation_report_throughput_2026-04-10.md`.

## 3. Consolidated should-fix findings

### Should-fix 1 — the owner list is incomplete even though I did not confirm a stale or nonexistent file reference

The plan’s `Current Code Surface` omits current report/save helpers that `REPO_MAP.md` explicitly routes through first, especially `src-tauri/src/commands_reports_api.rs` and `src-tauri/src/commands_reports_shared.rs`. Because open/reveal and report-path suggestion are part of the actual user-visible export path, leaving them out makes scope ownership look narrower than it is. I did not find a clearly stale or nonexistent repo path in the plan itself; the problem is omission, not a broken reference.

**Recommended change:** Expand `Current Code Surface` to include `src-tauri/src/commands_reports_api.rs` and `src-tauri/src/commands_reports_shared.rs`.

Sources: `REPO_MAP.md`; `03_validation_report_throughput_2026-04-10.md`.

### Should-fix 2 — the plan should either include `crates/app_core/src/composite_validate.rs` or explicitly say that composite compute is excluded

The bundle title and the Tauri validation entrypoints are broad, and `REPO_MAP.md` treats `composite_validate.rs` as part of the validation layer. The current code surface only lists `double_ma_validate.rs`, so the document should clarify whether the epic intentionally targets only the Double-MA family or the broader validation surface.

**Recommended change:** Either add `crates/app_core/src/composite_validate.rs` to the current code surface or explicitly state that composite validation compute is out of scope for this epic.

Sources: `REPO_MAP.md`; `03_validation_report_throughput_2026-04-10.md`.

### Should-fix 3 — compute-only evidence and user-visible end-to-end evidence should be named separately

The frontend already distinguishes command time from end-to-end time, but the plan does not yet turn that into two explicit evidence classes. Since end-to-end includes optional export and persistence while `runtime_ms` on the command result is set around the blocking validation call, acceptance evidence will stay muddy unless the doc names both layers explicitly.

**Recommended change:** Add distinct acceptance buckets for compute-only evidence and user-visible end-to-end evidence.

Sources: `validationService.ts`; `commands_validation.rs`; `03_validation_report_throughput_2026-04-10.md`.

## 4. Optional improvements worth considering

- Reuse the existing `ValidationExecutionProfile` surface instead of inventing a new timing vocabulary from scratch. The repo already carries `aux_finalist_analytics_wall_ms`, `cpu_tail_wall_ms`, and `gpu_result_materialization_wall_ms`, which are a better starting point for additive telemetry than a parallel ad hoc layer.
- Name the existing headless/manual helpers in the doc’s baseline section. `REPO_MAP.md` already identifies `src-tauri/src/bin/ta_headless_validate.rs` as a headless report-generation and structural-parity helper and `src-tauri/src/bin/ta_double_ma_validation_cost.rs` as a validation-cost measurement helper.
- Add a short note that benchmark/replay harnesses still need valid validation context. The validation command surface enforces dataset/timeframe/strategy compatibility, so any representative scenario or replay harness that bypasses that assumption will give misleading results.

Sources: `double_ma_validate.rs`; `commands_validation.rs`; `REPO_MAP.md`.

## 5. Suggestions to reject as out of scope or unsupported

- Reject any proposal to widen this epic into verdict-policy changes, report-schema changes, or workbook-meaning changes. The plan explicitly puts those out of scope, and the current workbook/report code already encodes concrete semantics worth preserving.
- Reject broad speculative CUDA reopenings unless measured supported-route validation compute is actually the dominant bottleneck. The plan already says GPU-validation maturity is in scope only where it directly changes current-route throughput, and the repo already has route/fallback truth surfaces that would be easy to blur if this widened into generic GPU work.
- Reject artifact-library-scale or general startup work unless the measured validation/export path proves it is the real bottleneck. The plan explicitly excludes general artifact/history scalability except where report generation depends on it.
- Reject any acceptance story based only on the filtered `validation_report_` test plus a workflow gate. That is not enough evidence for route truth, workbook fidelity, and performance-sensitive behavior in this repo.

Sources: `03_validation_report_throughput_2026-04-10.md`; `commands_reports_validation_common.rs`; `commands_reports_validation_xlsx.rs`; `AGENTS.md`; `REPO_MAP.md`.

## 6. Recommended doc edits

1. Replace the current three-phase framing with a branch-aware timing matrix covering command runtime, command-side cross/aux compute, XLSX save-from-artifact, XLSX save-from-report, `build_report_xlsx` fallback, open/reveal, and artifact persistence.
2. Expand `Current Code Surface` to include `src-tauri/src/commands_reports_api.rs` and `src-tauri/src/commands_reports_shared.rs`, and either add `crates/app_core/src/composite_validate.rs` or explicitly exclude it.
3. Add a scenario-contract subsection that freezes cross mode, finalist-cap behavior, omitted-module behavior, auto-export on/off, and compatibility replay cases.
4. Replace the generic workbook safeguard with concrete invariants: compatibility mode, route/fallback/omitted-module summaries, Approved Candidates suppression under a rejecting run verdict, full-row chart/audit inputs, `Audit Data`, and `Regime Audit`.
5. Add a persistence-truth subsection that explicitly preserves validation req/res, `report_xlsx_path`, `dataset_meta`, `route_provenance`, `lineage`, `config_fingerprint`, `optimization_metric_surface`, and current retention/enrichment behavior.
6. Rewrite the verification section into a matrix that includes app-core validation tests, tauri-side workbook tests, workflow gate, output parity, CPU perf gate, GPU perf gate when applicable, and the existing headless/manual helpers.
7. Add explicit rollback triggers for any change in route truth, workbook truth, or artifact-provenance fields, not only for generic parity failures.

Sources: `03_validation_report_throughput_2026-04-10.md`; `validationService.ts`; `commands_validation.rs`; `commands_reports_validation_common.rs`; `commands_reports_validation_xlsx.rs`; `run_artifacts.rs`; `REPO_MAP.md`; `AGENTS.md`.

## 7. Recommended implementation order

1. **Freeze the scenario matrix first.** Include CPU-only, supported GPU, intentional fallback, preserved-finalists cross, lightweight cross, auto-export off/on, and compatibility replay cases before touching code.
2. **Add timing separation before optimization.** Make it branch-aware and keep compute-only versus user-visible end-to-end evidence separate. Reuse existing execution-profile telemetry where possible.
3. **Collect a baseline with existing helpers.** Use the current workflow gate plus headless/manual validation cost/report tools so the dominant bottleneck is measured rather than guessed.
4. **Only optimize the dominant phase that the baseline exposes.** Do not widen into GPU-route work, artifact-library work, or workbook reshaping unless the measured branch actually dominates and the change stays inside the current semantics.
5. **Run the contract gates before calling the work done.** That means backend validation tests, workbook/report tests, output parity, workflow gate, and the CPU/GPU perf gates that match the touched surface.
6. **Only after that update cross-doc references and acceptance evidence.** The doc should close with recorded phase timings plus proof that workbook truth, route truth, and artifact truth stayed intact.

Sources: `03_validation_report_throughput_2026-04-10.md`; `validationService.ts`; `commands_validation.rs`; `double_ma_validate.rs`; `commands_reports_validation_common.rs`; `commands_reports_validation_xlsx.rs`; `run_artifacts.rs`; `REPO_MAP.md`; `AGENTS.md`.

## Bottom line

The merged takeaway is straightforward: the epic is worth doing, but it should not move forward as a generic “make validation faster” effort. It needs a stricter timing taxonomy, a frozen scenario contract, explicit workbook and artifact truth guardrails, and a stronger verification/rollback matrix before implementation begins.
