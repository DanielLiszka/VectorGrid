# Section 19 implementation plan — perf-gate authority, perf artifacts, and headless verification

## Goal

Make the repo's performance and verification evidence truthful, reproducible, and clearly tiered.

This section is not a broad benchmark expansion. The first wave should clarify which gates are authoritative today, remove stale or misleading artifact claims, and make headless report parity reproducible instead of depending on manually refreshed files.

## Repo-grounded authority map

### Mandatory authority surfaces today

- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/baselines/workflow_regression_baseline.json`
- `scripts/baselines/cpu_perf_baseline.json`
- `scripts/baselines/gpu_perf_baseline.json`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

These are the current gate-backed surfaces. Section 19 should preserve them as the authority set unless a new gate is explicitly added.

### Specialized but non-default authority surfaces

- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
- `scripts/baselines/gpu_alma_contract_rows/*.json`

This is a real gate, but it is specialized to ALMA contract-row behavior and should not be treated as the default PR perf surface.

### Supplemental verification / diagnostic surfaces

- `scripts/check_output_parity.mjs`
- `scripts/output_parity_pairs.json`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`
- `crates/app_core/benches/composite_mo_opt.rs`
- `crates/app_core/benches/ml_signal_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`

These are useful evidence surfaces, but they are not current release-authority gates.

### Telemetry-only surfaces today

- `scripts/capture_optimization_memory.ps1`
- `scripts/baselines/ui_capture/optimization_memory_metrics.json`
- `scripts/baselines/post_release/*`
- most of `scripts/baselines/samples/*`

These are currently telemetry, sample, or review artifacts. They do not enforce pass/fail budgets.

## Current repo-valid problems

1. Section 19 in `gpt54_feature_review_map.md` still treats several sidecar binaries as peer “primary” authority surfaces when the actual authority is gate-backed and narrower.
2. `check_output_parity.mjs` is wired to fixed `_write_test/headless_reports/AAPL_headless_current.*` outputs, but no active gate or wrapper regenerates those files before parity is checked.
3. `web/README.md` claims JSON/CSV/XLSX parity coverage, but the active parity pairs only cover JSON and XLSX.
4. `capture_optimization_memory.ps1` measures a debug `ta_workflow_regression` process without schema/baseline/pass-fail framing, so it is easy to misread as authoritative optimization memory evidence.
5. Optional benches and evidence binaries exist for CompositeMo and MlSignal, but the current gate chain does not consume them and the inventory/docs do not cleanly reflect that.
6. `profile_gpu_dynamic_runner` emits richer JSON cache-attribution output than the current GPU gate consumes; the repo should either surface that as supplemental-only or wire it in deliberately.

## First-wave implementation scope

### Phase 1 — authority tiering and documentation truthfulness

- Update the repo-facing docs that contributors actually use:
  - `REPO_MAP.md`
  - `web/README.md`
  - any script comments/help text touched during this section
- Make the authority tiers explicit:
  - mandatory perf gates
  - specialized/manual gates
  - supplemental verification binaries/benches
  - telemetry-only artifacts
- Keep the baseline-refresh rule explicit: refresh only when schema, timing boundary, or bench filter intentionally changes.

### Phase 2 — deterministic headless parity regeneration

- Add a wrapper script for headless parity verification that:
  - runs `ta_headless_validate` with deterministic fixed inputs
  - writes the candidate JSON/XLSX files to the exact paths consumed by `scripts/output_parity_pairs.json`, or rewires the parity config to the wrapper output
  - then runs `scripts/check_output_parity.mjs`
- Keep a checked-in parity pack/config so the wrapper inputs, outputs, and build-profile expectations are explicit and reviewable.
- Keep the product run-artifact shape from `ta_headless_validate` stable unless a sidecar manifest cannot solve the provenance gap.
- Prefer a separate verification manifest/sidecar containing:
  - `schema_version`
  - `timing_boundary`
  - `timing_boundary_note`
  - `build_profile`
  - command line / input pack / output paths

### Phase 3 — parity coverage truthfulness

- Make parity coverage claims match reality.
- Recommended first-wave path:
  - keep parity on JSON/XLSX only
  - fix docs/config wording accordingly
  - defer CSV parity expansion unless the new headless wrapper naturally gains a deterministic CSV output path
- Do not claim broad sample-directory parity if only two report artifacts are actually being compared.

### Phase 4 — telemetry artifact truthfulness

- Make `capture_optimization_memory.ps1` self-describing enough that it cannot be mistaken for a gate:
  - include build profile in its output
  - include timing-boundary wording
  - include explicit telemetry-only wording
- If the repo wants memory to become authoritative later, add baseline + threshold + pass/fail wiring in a separate step instead of quietly reusing the current debug script.

### Phase 5 — optional evidence inventory cleanup

- Update inventory/docs so optional evidence surfaces are discoverable but not misrepresented as current gates.
- Specifically:
  - inventory `ml_signal_workflow.rs` and `ml_signal_validation.rs` as active supplemental benches
  - stop using `e2e_workflow_ml.rs` as a proxy for MlSignal perf authority
  - keep `ta_profile_route_matrix.rs`, `ta_double_ma_validation_cost.rs`, `real_data_optimizer_eval.rs`, and `stage6b_kernel_report.rs` clearly labeled as supplemental/manual until wrapped by a real gate
- Decide whether `profile_gpu_dynamic_runner --emit-json` remains supplemental evidence or becomes a consumed gate sidecar.

## Explicit defers

- Do not replace the existing workflow / CPU / GPU perf gates.
- Do not make `run_gpu_alma_contract_rows_perf_gate.ps1` the default gate.
- Do not add broad new perf gates for every bench in the first wave.
- Do not treat `e2e_workflow_ml.rs` as first-wave MlSignal authority.
- Do not change the core product run-artifact schema from `ta_headless_validate` unless a sidecar approach is insufficient.
- Do not expand sample-artifact parity to the entire `scripts/baselines/samples/` tree unless deterministic candidate generation exists for those files.

## Concrete implementation checklist

### Authority and docs

- [ ] Update `REPO_MAP.md` so Section 19 authority is described as gate-backed rather than “has a bin/bench”.
- [ ] Update `web/README.md` parity wording so it matches actual JSON/XLSX coverage unless CSV parity is added for real.
- [ ] Add concise authority-tier notes to the relevant scripts or repo docs.
- [ ] Preserve current baseline refresh rules and make them explicit where needed.

### Headless parity

- [ ] Add a deterministic wrapper script for headless parity generation.
- [ ] Make `scripts/output_parity_pairs.json` point at outputs regenerated by that wrapper.
- [x] Keep `check_output_parity.mjs` as the structural parity checker; do not turn it into a semantic-diff tool in this section.
- [ ] Add a verification sidecar/manifest for the headless parity pack, or equivalent provenance metadata, without breaking product consumers.

### Artifact truthfulness

- [ ] Make `capture_optimization_memory.ps1` explicitly telemetry-only and boundary-truthful.
- [ ] Ensure any emitted memory artifact includes build-profile and timing-boundary metadata.
- [ ] Keep first-wave parity coverage truthful as JSON/XLSX-only unless deterministic CSV generation is added during the wrapper work.
- [ ] Narrow or clarify the relationship between `scripts/baselines/samples/*` and actual parity coverage.

### Optional evidence inventory

- [ ] Add `ml_signal_workflow.rs` and `ml_signal_validation.rs` to the repo inventory as supplemental benches.
- [ ] Clarify that `e2e_workflow_ml.rs` is not the primary MlSignal perf-authority surface.
- [ ] Clarify that `profile_gpu_dynamic_runner --emit-json` is supplemental today; any GPU-gate consumption is a later-pass decision.
- [ ] Keep `ta_profile_route_matrix.rs`, `ta_double_ma_validation_cost.rs`, `real_data_optimizer_eval.rs`, and `stage6b_kernel_report.rs` labeled as supplemental/manual unless new wrappers are added.

## Verification plan

- `npm --prefix web run parity:outputs` after the deterministic headless wrapper is wired
- `cargo test -p ta_desktop_demo_app --bin ta_headless_validate` if that binary or its artifact sidecar logic changes
- `cargo test -p ta_desktop_demo_app --bin ta_workflow_regression` if workflow artifact wording/metadata changes
- `cargo test -p ta_app_core --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel` if GPU runner evidence wiring changes
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` when GPU perf-related script behavior changes

## Acceptance criteria

- The repo clearly distinguishes authoritative gates from supplemental and telemetry-only evidence.
- Headless parity checks regenerate their candidate outputs deterministically instead of depending on stale files.
- Parity coverage claims in docs/config match the artifacts actually checked.
- Memory telemetry artifacts are truthful about build profile and timing boundary.
- Optional performance evidence surfaces are inventoried accurately without being misrepresented as current gates.
