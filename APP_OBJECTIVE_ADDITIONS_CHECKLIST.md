# Objective Additions Checklist (No Feature Removal)

Last updated: 2026-02-12

This checklist translates `APP_VISION_WORKFLOW_RESEARCH.md` into concrete, verifiable work items.

Non-negotiable constraint:
- We add capabilities; we do not remove existing optimization/validation/meta features.
- CPU-only builds must remain fully functional without CUDA.
- Existing run/meta artifact loading must remain backwards compatible.

---

## 1) Definition Of "Objectively Better"

An addition is accepted only if all gates below pass on a fixed benchmark pack (same datasets, same seeds, same config):

- Research-quality gate:
  - Holdout shortlist quality is non-worse on lower-tail metrics.
  - Target: `robust_score_p10` non-decreasing, `max_dd_p90` non-increasing.
- Realism gate:
  - New realism features must catch at least some previously top-ranked but fragile candidates.
  - Target: at least one benchmark case where ranking changes in expected direction under stricter assumptions.
- Performance gate:
  - Default fast path remains fast.
  - Target: no significant slowdown in current default optimize path.
- Reproducibility gate:
  - Re-running the same artifact with unchanged data/config yields equivalent ranking and metrics within tolerance.
- Compatibility gate:
  - Old artifacts still load.
  - Existing UI flows still run end-to-end.

---

## 2) Global Regression Guardrails (Implement First)

- [x] Freeze benchmark corpus + seeds for repeatable A/B checks.
  - Primary files: `data/`, `crates/app_core/benches/`.
- [x] Add one-command workflow benchmark for optimize -> validate -> meta -> holdout.
  - Primary files: `crates/app_core/benches/e2e_workflow_ml.rs`, `crates/app_core/benches/meta_select_workflow.rs`.
- [x] Add acceptance summary output (baseline vs candidate branch).
  - Primary files: `src-tauri/src/bin/ta_headless_validate.rs` and/or a small script under `scripts/`.
- [x] Add "no regression" checks to routine test runs.
  - Commands: `cargo test --workspace`, targeted benches for app-core workflows.

Done when:
- A single command produces baseline metrics and candidate metrics with pass/fail for the gates in Section 1.

Progress note (2026-02-12):
- Added deterministic workflow regression harness: `src-tauri/src/bin/ta_workflow_regression.rs`.
- Added frozen benchmark-pack config: `scripts/workflow_benchmark_pack.json`.
- Added baseline/candidate gate runner: `scripts/run_workflow_gate.ps1`.
- Added baseline snapshot: `scripts/baselines/workflow_regression_baseline.json`.
- Added CI workflow-gate step in `.github/workflows/ci.yml`.

---

## 3) Maintainability / DRY Refactor Track (No Behavior Regression)

Goal:
- Reduce "spaghetti" risk by enforcing clearer module boundaries, shared logic, and smaller focused units.
- Improve change velocity and correctness without removing features.

Rules:
- Refactor in behavior-preserving increments.
- Any refactor PR must include parity checks against baseline behavior.

- [x] Break oversized orchestration files into domain modules (keep public command surface stable).
  - Primary files: `src-tauri/src/commands.rs` -> split by domain (`data`, `runs`, `validation`, `meta`, `reports`).
- [x] Eliminate duplicated validation/meta orchestration logic through shared helpers.
  - Primary files: `src-tauri/src/commands.rs`, `crates/app_core/src/*validate.rs`, `crates/app_core/src/meta_select.rs`.
- [x] Consolidate artifact schema handling in one place (typed structs + versioned adapters).
  - Primary files: `src-tauri/src/commands.rs`, `dist/app.js`.
- [x] Add internal architecture notes for boundaries and ownership.
  - Primary files: new short doc under repo root or `crates/app_core/`.
- [x] Add maintainability CI checks (at minimum formatting, clippy warnings baseline, tests).
  - Commands: `cargo fmt --check`, `cargo clippy --workspace`, `cargo test --workspace`.
- [x] Introduce a "no new large god-file growth" rule for new work.
  - Policy: new functionality should land in focused modules, not as more bulk in existing orchestration files.

Done when:
- Existing user-visible workflows behave equivalently under test/baseline replay.
- Clear reduction in duplicated orchestration code paths.
- New features can be added without further centralizing complexity.

Progress note (2026-02-11):
- Extracted run/meta artifact persistence logic out of `src-tauri/src/commands.rs` into `src-tauri/src/run_artifacts.rs`.
- Added `ARCHITECTURE_NOTES.md` with boundaries, invariants, and decomposition targets.
- Added CI clippy baseline check in `.github/workflows/ci.yml` (non-fatal warning baseline, no `-D warnings`).
- Reduced duplication in artifact file naming/retention helpers inside `run_artifacts` module.
- Added CI guardrail preventing `src-tauri/src/commands.rs` from growing past a bounded line-count threshold.
- Added dataset artifact schema normalization/backfill in `src-tauri/src/run_artifacts.rs` (schema version + resample identity defaults + fingerprint schema version).
- Added focused compute modules in `crates/app_core/`:
  - `iterative_sim.rs` for event-style validation re-scoring,
  - `sharpe_stats.rs` for PSR/DSR calculations and tests.
Progress note (2026-02-12):
- Extracted ML/meta orchestration out of `src-tauri/src/commands.rs` into `src-tauri/src/commands_meta.rs` while keeping the Tauri command surface stable.
- Added shared validation probability helpers in `crates/app_core/src/validation_stats.rs`, consumed by both `composite_validate` and `double_ma_validate`.
- Removed repeated PSR/DSR math and gate checks from per-strategy validators in favor of shared helpers.
- Added trait-based shared cross-ticker validation module `src-tauri/src/validation_common.rs` and switched both Composite/DoubleMA cross-ticker paths to it.
- Added shared statistical primitives in `crates/app_core/src/stats_common.rs` and reused them across validation modules.
- Added trait-based validation post-processing module `crates/app_core/src/validation_pipeline.rs` and switched both validators to shared PSR/DSR + gate + pareto + CSCV-fold/sort flow.

---

## 4) Execution Realism As First-Class (Without Breaking Current Fast Path)

- [x] Introduce explicit execution-cost model fields in strategy/runtime config:
  - `fee_bps`, `spread_bps`, `slippage_bps` (keep current `commission` for compatibility).
  - Primary files: `crates/strategies/src/double_ma/mod.rs`, `crates/strategies/src/composite_cpu/mod.rs`, `dist/app.js`.
- [x] Persist explicit cost model into run artifacts.
  - Primary files: `dist/app.js`, `src-tauri/src/commands.rs`.
- [x] Preserve current behavior when costs are zero.
  - Primary files: validation/optimization paths in `crates/app_core/src/`.
- [x] Add tests for monotonic cost impact (higher costs cannot improve net returns in identical setup).
  - Primary files: `crates/strategies/src/` tests, `crates/app_core/src/*validate.rs` tests.

Done when:
- Zero-cost parity with old behavior is confirmed.
- Non-zero spread/slippage meaningfully changes outcomes in controlled tests.

Progress note (2026-02-11):
- `StrategyConfig` now includes explicit execution-cost fields:
  - `fee_bps`, `spread_bps`, `slippage_bps`, plus optional `cost_preset`.
  - Existing `commission` remains for compatibility.
- Added `effective_commission()` fallback logic:
  - uses `commission` when positive,
  - otherwise derives from bps fields (`fee + slippage + spread/2`, per side).
- Updated strategy engines (`double_ma`, `composite_cpu`) and CUDA kernel path to use effective commission.
- UI request builder now sends explicit cost fields in run requests, so run artifacts persist them naturally.
- Added strategy-level tests in `crates/strategies/src/double_ma/mod.rs`:
  - `effective_commission_uses_bps_when_commission_is_zero`,
  - `higher_costs_do_not_improve_pnl_in_same_setup`.

---

## 5) Reproducibility Hardening (Dataset Identity + Config Identity)

- [x] Add dataset content fingerprint (streaming hash) in addition to size/mtime.
  - Primary files: `src-tauri/src/data_cache.rs`, `src-tauri/src/commands.rs`.
- [x] Persist full resampling identity in artifacts:
  - base timeframe, derived timeframe, resample rule/anchor.
  - Primary files: `dist/app.js`, `src-tauri/src/commands.rs`, `crates/app_core/src/timeframe.rs`.
- [x] Version config fingerprint payload if schema changes.
  - Primary files: `src-tauri/src/commands.rs`, `dist/app.js`.
- [x] Add rerun-equivalence test(s) for artifact reproducibility.
  - Primary files: `src-tauri/tests/`, `crates/app_core/benches/`.

Done when:
- Artifacts can detect data drift reliably.
- Same artifact + same dataset fingerprint reproduces equivalent shortlist/ordering within tolerance.

Progress note (2026-02-11):
- Backend now writes `dataset_meta.source_content_fingerprint` on run artifact save.
  - Uses adaptive FNV-1a: full hash for small/medium files, sampled hash for large files to protect UX speed.
- Run artifact summaries now expose `source_content_fingerprint`.
- Batch `skip_done` logic now prefers content fingerprint matching (falls back to size/mtime when fingerprint is absent).
- Artifacts now persist full resample identity:
  - `dataset_meta.base_timeframe`, `dataset_meta.derived_timeframe`,
  - `dataset_meta.resample_rule`, `dataset_meta.resample_anchor`,
  - `dataset_meta.resample_applied`,
  - with schema versioning (`dataset_meta.schema_version`).
- Artifact save now stamps `config_fingerprint_version` and run listing infers/falls back from fingerprint prefix (`cfg1-*`).
- Added deterministic rerun-equivalence tests for validation outputs:
  - `composite_validation_rerun_equivalence_same_seed`
  - `double_ma_validation_rerun_equivalence_same_seed`

---

## 6) Iterative Validation Mode (Validation/Holdout Only)

- [x] Add validation engine selector:
  - `vectorized` (current default) and `iterative` (new, optional).
  - Primary files: `crates/app_core/src/*validate.rs`, `src-tauri/src/commands.rs`.
- [x] Implement iterative simulator v1 with constrained scope:
  - market orders, current entry/exit semantics, slippage-aware fills.
  - optional stop/take-profit with explicit intrabar rule.
  - Primary files: new module under `crates/app_core/src/` (e.g., `iterative_sim.rs`).
- [x] Use iterative mode for top-N and holdout-heavy paths only.
  - Primary files: `src-tauri/src/commands.rs` (`apply_holdout_evaluation` path).
- [x] Record vectorized-vs-iterative deltas in validation output.
  - Primary files: validation result structs and XLSX/meta export paths.

Done when:
- Default optimization throughput is unaffected.
- Holdout-heavy mode can re-score shortlisted candidates with iterative assumptions.
- Delta fields are visible and auditable.

Progress note (2026-02-12):
- Added `crates/app_core/src/iterative_sim.rs` with:
  - slippage-aware cost modeling,
  - optional stop/take-profit and explicit intrabar rule handling.
- Added `validation_engine` + `iterative` config to validation requests.
- Added per-candidate `iterative_in_sample` + `iterative_delta` output fields.
- Meta holdout heavy path now supports iterative re-scoring for selected groups.

---

## 7) Multiple-Testing Controls (PSR/DSR)

- [x] Implement PSR and DSR metrics for Sharpe-driven evaluations.
  - Primary files: new stats module in `crates/app_core/src/`.
- [x] Add optional gates/columns in validation and report exports.
  - Primary files: `crates/app_core/src/*validate.rs`, `src-tauri/src/commands.rs`.
- [x] Integrate into meta ranking as optional weighted components.
  - Primary files: `crates/app_core/src/meta_select.rs`.
- [x] Add tests for sanity/monotonic behavior and deterministic outputs.
  - Primary files: `crates/app_core/src/` tests.

Done when:
- PSR/DSR appear in candidate outputs/reports.
- Gates can be enabled without breaking existing workflows.

Progress note (2026-02-12):
- Added `crates/app_core/src/sharpe_stats.rs` (PSR/DSR math + tests).
- Validation candidates now include `psr` and `dsr`.
- Meta aggregation now computes `psr_median` and `dsr_median`.
- Meta scoring now supports optional `w_psr` and `w_dsr` weights.
- Validation shortlist CSV and meta report XLSX now include PSR/DSR fields.
- Added explicit user-configurable validation gates (`min_psr`, `min_dsr`) wired through UI and backend filtering.
- Added PSR/DSR columns to validation UI table and validation XLSX report export.

---

## 8) Meta Holdout Coverage Parity

- [x] Extend holdout evaluation support beyond Composite to include Double MA.
  - Current limitation reference: holdout support branch in `src-tauri/src/commands.rs`.
- [x] Ensure heavy holdout mode can apply the same validation options consistently across strategy kinds.
  - Primary files: `src-tauri/src/commands.rs`, `crates/app_core/src/meta_select.rs`.
- [x] Keep existing Composite behavior unchanged.

Done when:
- Meta holdout path can evaluate Double MA artifacts with parity to Composite flow.

---

## 9) Workflow Guardrails (Prevent Best-Of-Many Misuse)

- [x] Add explicit UI warnings when presenting optimization-only winners without validation/holdout context.
  - Primary files: `dist/app.js`.
- [x] Add a final-selection summary panel driven by validated/holdout metrics, not peak in-sample.
  - Primary files: `dist/app.js`, `src-tauri/src/commands.rs`.
- [x] Add report annotations for selection-bias risk when search space is large.
  - Primary files: `src-tauri/src/commands.rs` (XLSX builders).

Done when:
- The default user path emphasizes validated robustness and holdout context over raw best-score.

Progress note (2026-02-11):
- Added a Top-tab selection-bias guardrail banner:
  - warns when users are viewing optimization-only winners without validation,
  - downgrades to an informational reminder when validation context exists.
- Best-card recommendation now uses validation shortlist for all strategy kinds (including Double MA), not only Composite.
- Final selection panel now prefers meta+holdout top-group metrics when available.
- Added selection-bias warning annotations to report generation paths for large search spaces.

---

## 10) Suggested Implementation Order

- [x] Step 1: Section 2 (guardrails + benchmark harness)
- [x] Step 2: Section 3 (maintainability/DRY refactor, behavior-preserving)
- [x] Step 3: Section 4 (execution realism fields + compatibility)
- [x] Step 4: Section 5 (reproducibility hardening)
- [x] Step 5: Section 6 (iterative validation mode, holdout-only first)
- [x] Step 6: Section 7 (PSR/DSR)
- [x] Step 7: Section 8 (Double MA holdout parity)
- [x] Step 8: Section 9 (workflow guardrails/reporting)

Rationale:
- This order maximizes measurable value early while minimizing risk to current fast/demo paths.
