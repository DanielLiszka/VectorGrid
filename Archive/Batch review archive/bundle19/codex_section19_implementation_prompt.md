# Codex implementation prompt — Section 19 perf-gate authority, perf artifacts, and headless verification

## Mission

Implement only the remaining repo-valid Section 19 work for authoritative perf gates, truthful perf artifacts, and reproducible headless verification.

This is not a broad benchmark expansion and not a wholesale rewrite of existing gates.

Priority order:

1. clarify authority tiers first
2. make headless parity regeneration deterministic second
3. make artifact coverage and telemetry wording truthful third
4. inventory optional evidence surfaces fourth

---

## Read first

Before editing code, read:

1. `bundle19/section19_perf_gate_authority_perf_artifacts_headless_verification_plan.md`
2. `gpt54_feature_review_map.md` section `## 19. Perf-gate authority, perf artifacts, and headless verification`
3. `REPO_MAP.md`
4. `web/README.md`

Trust the current code and the repo-state corrections below over older review wording.

---

## Repo-state validation and scope corrections

### Already true in this repo

- [x] `scripts/run_workflow_gate.ps1` plus `scripts/baselines/workflow_regression_baseline.json` are already the authoritative workflow-regression gate surface.
- [x] `scripts/run_cpu_perf_gate.ps1` plus `scripts/baselines/cpu_perf_baseline.json` are already the authoritative CPU perf-gate surface.
- [x] `scripts/run_gpu_perf_gate.ps1` plus `scripts/baselines/gpu_perf_baseline.json` are already the authoritative main GPU perf-gate surface.
- [x] `scripts/run_gpu_alma_contract_rows_perf_gate.ps1` already exists as a real specialized gate, but it is not the default perf-authority surface.
- [x] `scripts/check_output_parity.mjs` is already the active structural output-parity checker.
- [x] `web/package.json` already exposes `npm run parity:outputs`.
- [x] `ta_workflow_regression.rs` already emits schema/timing/build metadata and stage-budget comparisons suitable for gate consumption.
- [x] `profile_gpu_dynamic_runner.rs` is already consumed by the GPU perf gate in text mode.
- [x] `profile_gpu_dynamic_runner.rs` already emits richer JSON cache-attribution output, but that JSON is supplemental today because the gate does not consume `--emit-json`.
- [x] `ta_profile_route_matrix.rs` already emits a structured route/runtime artifact with schema/timing/build metadata.
- [x] `ta_double_ma_validation_cost.rs` already emits a structured validation-cost artifact with schema/timing/build metadata.
- [x] `ta_headless_validate.rs` already provides real headless JSON/XLSX report generation and preserved-finalists validation coverage.
- [x] `capture_optimization_memory.ps1` already exists, but today it is telemetry-only debug-process sampling, not a perf gate.
- [x] `composite_mo_opt.rs`, `ml_signal_workflow.rs`, and `ml_signal_validation.rs` already exist as real benches, but they are not current gate-backed authority surfaces.

### Still-open repo-valid work

- [x] The repo docs and script-facing guidance now reflect the real authority tiers instead of treating every bench/bin as peer authority.
- [x] Headless parity candidate artifacts are now deterministically regenerated before `check_output_parity.mjs` runs.
- [x] Parity coverage claims are now truthful with the first-wave JSON/XLSX-only scope.
- [x] A provenance metadata sidecar now exists for the headless verification pack without changing the product run-artifact contract from `ta_headless_validate.rs`.
- [x] `capture_optimization_memory.ps1` and its emitted artifact are now boundary-truthful and explicitly telemetry-only.
- [x] The repo inventory now surfaces supplemental evidence without misrepresenting it as gate authority.

### Explicit defer / exclude items

- [x] Do not replace `run_workflow_gate.ps1`, `run_cpu_perf_gate.ps1`, or `run_gpu_perf_gate.ps1` with a new first-wave gate architecture.
- [x] Do not promote `run_gpu_alma_contract_rows_perf_gate.ps1` to the default perf gate in this section.
- [x] Do not add broad new perf gates for every existing bench/binary in the first wave.
- [x] Do not use `e2e_workflow_ml.rs` as the first-wave MlSignal perf-authority proxy.
- [x] Do not change the core product JSON/XLSX artifact contract from `ta_headless_validate.rs` unless a sidecar manifest cannot solve the provenance gap.
- [x] Do not silently treat `capture_optimization_memory.ps1` output as a pass/fail budget artifact without adding baseline/threshold wiring.
- [x] Do not expand parity to the entire `scripts/baselines/samples/` tree unless deterministic candidate generation exists for those artifacts.
- [x] Leave `gpt54_feature_review_map.md` as historical review context in this first wave; do not treat it as authoritative Section 19 routing.
- [x] A first-wave route-matrix gate wrapper remains deferred.
- [x] A first-wave validation-cost gate wrapper remains deferred.
- [x] CompositeMo-specific or MlSignal-specific perf gates remain deferred until a separate authority decision is made.
- [x] Broad parity expansion across the full samples directory remains deferred.

---

## File scope

### Primary implementation files

- `scripts/run_output_parity_check.mjs`
- `scripts/headless_output_parity_pack.json`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/check_output_parity.mjs`
- `scripts/output_parity_pairs.json`
- `scripts/capture_optimization_memory.ps1`
- `web/package.json`
- `web/README.md`
- `REPO_MAP.md`
- `src-tauri/src/bin/ta_headless_validate.rs`

### Inspect-only / conditional surfaces

- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
- `scripts/workflow_benchmark_pack.json`
- `scripts/baselines/workflow_regression_baseline.json`
- `scripts/baselines/cpu_perf_baseline.json`
- `scripts/baselines/gpu_perf_baseline.json`
- `scripts/baselines/gpu_alma_contract_rows/*.json`
- `scripts/baselines/samples/*`
- `scripts/baselines/post_release/*`
- `scripts/post_release_telemetry_review.mjs`
- `scripts/export_baseline_samples.mjs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`
- `crates/app_core/benches/composite_mo_opt.rs`
- `crates/app_core/benches/ml_signal_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`

### Keep unchanged unless evidence forces it

- the core workflow/CPU/GPU gate pass-fail logic
- broad benchmark shapes and baseline schemas
- `ta_headless_validate.rs` product report payload shape

---

## Hard constraints

- Do not turn this into a broad benchmark proliferation project.
- Preserve current gate semantics unless the change is explicitly about documentation truthfulness or artifact regeneration.
- Keep CPU-only builds working.
- Keep current product report consumers working.
- Never run multiple benchmarks concurrently.

### Evidence-boundary guard

Treat the measurement and verification surfaces as:

- `run_workflow_gate.ps1` + `ta_workflow_regression.rs` for authoritative workflow timing
- `run_cpu_perf_gate.ps1` for authoritative CPU perf timing
- `run_gpu_perf_gate.ps1` + `profile_gpu_dynamic_runner.rs` text mode for authoritative GPU perf timing
- `check_output_parity.mjs` + deterministic headless output regeneration for structural output parity
- `capture_optimization_memory.ps1` for telemetry only unless upgraded deliberately
- `ta_profile_route_matrix.rs`, `ta_double_ma_validation_cost.rs`, `real_data_optimizer_eval.rs`, `stage6b_kernel_report.rs`, `composite_mo_opt.rs`, `ml_signal_workflow.rs`, and `ml_signal_validation.rs` as supplemental/manual evidence unless wrapped by a real gate

---

## Implementation checklist

### Phase 1 — authority-tier cleanup

- [x] Update `REPO_MAP.md` so Section 19 and related routing text distinguish mandatory gates, specialized gates, supplemental evidence, and telemetry-only artifacts.
- [x] Update `web/README.md` so parity coverage claims match the actual checked artifacts.
- [x] Add concise authority-tier comments/help text where script behavior is otherwise easy to misread.
- [x] Keep baseline refresh rules explicit as documentation/inventory guidance: refresh only on intentional schema, bench-filter, or timing-boundary changes.

### Phase 2 — deterministic headless parity regeneration

- [x] Add a wrapper script that deterministically regenerates the candidate headless parity outputs.
- [x] Repoint `npm --prefix web run parity:outputs` so the canonical repo parity command regenerates candidate artifacts before calling `scripts/check_output_parity.mjs`.
- [x] Make that wrapper write the candidate outputs to the exact paths consumed by `scripts/output_parity_pairs.json`.
- [x] `scripts/check_output_parity.mjs` is now run against freshly generated candidate artifacts, not stale hand-refreshed files.
- [x] Add a headless verification sidecar/manifest with provenance fields such as `schema_version`, `timing_boundary`, `timing_boundary_note`, `build_profile`, command inputs, and output paths.
- [x] Keep the main `ta_headless_validate.rs` report payload stable unless a sidecar approach proves insufficient.

### Phase 3 — parity and artifact truthfulness

- [x] Encode the first-wave parity scope truthfully as JSON/XLSX-only unless deterministic CSV generation is added naturally by the new wrapper.
- [x] Update docs/config wording in lockstep with that parity scope.
- [x] Narrow or clarify broader sample-artifact claims that are not actually checked.

### Phase 4 — telemetry truthfulness

- [x] Add build-profile and timing-boundary metadata to the memory-capture artifact.
- [x] Make `capture_optimization_memory.ps1` explicitly telemetry-only in naming/help/output until it has baseline + threshold wiring.
- [x] Keep post-release telemetry review surfaces supplemental; do not silently promote them into gate authority.

### Phase 5 — supplemental evidence inventory

- [x] Add `ml_signal_workflow.rs` and `ml_signal_validation.rs` to the repo inventory as supplemental benches.
- [x] Clarify that `e2e_workflow_ml.rs` is not the primary MlSignal perf-authority surface.
- [x] Clarify that `profile_gpu_dynamic_runner --emit-json` is supplemental unless the GPU gate is updated to consume it.
- [x] Keep `ta_profile_route_matrix.rs`, `ta_double_ma_validation_cost.rs`, `real_data_optimizer_eval.rs`, and `stage6b_kernel_report.rs` labeled as supplemental/manual unless they receive gate wrappers in a later pass.

## Verification checklist

- [x] `npm --prefix web run parity:outputs`
- [x] No direct `ta_headless_validate.rs` code change was required; the binary was exercised by `npm --prefix web run parity:outputs`.
- [x] No `ta_workflow_regression.rs` code change was required; it was exercised indirectly by `powershell -File scripts/capture_optimization_memory.ps1 -SampleIntervalMs 50`.
- [x] No `profile_gpu_dynamic_runner.rs` code change was required.
- [x] `powershell -File scripts/capture_optimization_memory.ps1 -SampleIntervalMs 50`
- [x] `node scripts/post_release_telemetry_review.mjs --out-dir target/post_release_review_smoke`
- [x] Re-ran updated parity and telemetry helpers directly and verified their emitted files/metadata paths were deterministic and parseable.
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` was not required because the authority gate logic itself was unchanged.
- [x] `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` was not required because the authority gate logic itself was unchanged.
- [x] `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` was not required because the authority gate logic itself was unchanged.

---

## Acceptance criteria

- [x] The repo clearly distinguishes authority surfaces from supplemental and telemetry-only evidence.
- [x] Headless parity uses freshly regenerated candidate artifacts, not stale `_write_test` leftovers.
- [x] Parity coverage claims match the files actually checked.
- [x] Memory telemetry artifacts are explicit about build profile and timing boundary and are not misrepresented as gates.
- [x] Supplemental perf evidence surfaces are inventoried accurately without being overstated as current authority.
