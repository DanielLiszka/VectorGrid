# Implementation Plan: One-Command Reproducibility for the Double-MA GPU Sweep Artifact

**Project context assumed:** VectorTA / Rust workspace with CUDA, Python, and desktop-app components; current paper-facing feature is a GPU-resident exhaustive double-moving-average backtest sweep with 22 metrics.  
**Document type:** Implementation plan  
**Scope:** Improve one aspect of the project only: **reproducibility of the canonical double-MA benchmark artifact**  
**Status:** Draft  
**Audience:** You, contributors, reviewers, paper coauthors, and future maintainers

---

## 1. Executive summary

The highest-leverage improvement for the current project is not adding another indicator, UI screen, or optimization pass. It is turning the existing double-MA GPU sweep into a **deterministic, one-command, review-ready artifact** that any technical reader can run to reproduce the main result, validate correctness across all 22 metrics, and regenerate the paper’s tables and figures.

This feature improvement solves four problems at once:

1. It makes the benchmark claims easier to trust.
2. It reduces friction for paper reviewers and collaborators.
3. It forces the performance story to collapse into one clear benchmark protocol.
4. It creates a stable foundation for future papers, baselines, and regression testing.

The target state is simple:

```bash
./scripts/reproduce_paper.sh full
```

That one command should:

- validate the environment,
- run the canonical GPU benchmark,
- run the comparison baseline,
- perform correctness checks,
- save raw outputs and metadata,
- regenerate figures/tables,
- emit a final reproducibility summary.

The rest of this document explains how to implement that feature cleanly, in phases, with clear file structure, interfaces, validation rules, and acceptance criteria.

---

## 2. Why this is the right aspect to improve now

Based on the current project trajectory, the main risk is not lack of functionality. The main risk is that the strongest part of the project—high-throughput GPU backtest sweeps—can be misunderstood, mis-measured, or be hard for others to reproduce.

That creates downstream issues:

- performance claims become harder to defend,
- comparison against baselines becomes vulnerable to fairness objections,
- reviewers cannot easily separate algorithmic merit from setup complexity,
- future performance regressions are harder to detect,
- the project remains dependent on author knowledge instead of self-documenting workflows.

Improving reproducibility is therefore a **productivity feature**, a **quality feature**, and a **credibility feature**.

It also fits the current project state well because it can be implemented without expanding scope into unrelated areas such as:

- UI refinements,
- options backtesting,
- AI/ML features,
- additional indicators,
- broader framework generalization.

---

## 3. Problem statement

The current double-MA GPU sweep appears technically strong, but it likely suffers from some combination of the following reproducibility gaps:

- the benchmark entry point is not yet a single canonical command,
- environment setup is not fully pinned,
- benchmark boundaries may not be uniformly defined,
- raw results and figure-generation steps may be scattered,
- correctness validation may be present but not artifactized,
- comparison baseline setup may drift over time,
- CI may not exercise a smoke-level reproduction path,
- benchmark metadata may not be captured in a machine-readable way.

In practical terms, that means another engineer may be able to “run something close,” but not necessarily reproduce the exact claim or exact output set.

That is the gap this plan closes.

---

## 4. Goals

### Primary goal

Create a **deterministic, one-command reproducibility pipeline** for the canonical double-MA GPU sweep benchmark and its associated paper outputs.

### Secondary goals

- Freeze a single benchmark protocol and timing boundary.
- Capture exact environment and commit metadata for every run.
- Validate correctness across all 22 reported metrics.
- Produce both machine-readable raw outputs and human-readable summaries.
- Enable smoke-level reproduction in CI without requiring full GPU hardware.
- Make the artifact easy to archive and cite.

### Success criteria at a glance

The feature is successful when a technically competent third party can:

1. clone the artifact repo,
2. follow one setup path,
3. run one reproduce command,
4. obtain the same schema and materially the same results,
5. verify correctness and benchmark provenance without asking the author for hidden steps.

---

## 5. Non-goals

To preserve focus, this plan explicitly does **not** attempt to solve the following:

- building a general-purpose benchmark platform for all indicators,
- improving the Tauri application UX,
- redesigning the entire repository architecture,
- adding distributed execution,
- adding new trading strategies,
- shipping a polished package manager or installer,
- making every benchmark run on every machine equally fast,
- guaranteeing bitwise-identical floating-point results across all GPUs.

This plan is for one feature only: **reproducible execution of the canonical paper benchmark**.

---

## 6. Assumptions and constraints

### Assumptions

- The core compute path already exists and is worth preserving.
- The double-MA sweep is the main performance case study.
- The benchmark should remain narrow and paper-focused.
- There is already at least one baseline comparison path, likely Python/VectorBT-based.
- The project spans multiple languages or runtimes, so orchestration must cross boundaries cleanly.

### Constraints

- GPU benchmark execution may require hardware not available in public CI.
- Some floating-point outputs may need tolerance-based comparisons rather than exact equality.
- Artifact complexity must stay lower than main-project complexity.
- The benchmark must be fair but not over-generalized.
- Reviewers should not need the desktop app to reproduce the artifact.

### Design implication

The reproducibility artifact should be **decoupled from the Tauri UI** and should expose a small set of CLI-driven entry points instead.

---

## 7. Proposed end state

The target feature is a **paper artifact repository or artifact subtree** with a stable structure and a single orchestration layer.

### User-facing experience

A contributor or reviewer should be able to do this:

```bash
# 1) Setup
./scripts/setup_env.sh

# 2) Smoke test
./scripts/reproduce_paper.sh smoke

# 3) Full run on reference GPU machine
./scripts/reproduce_paper.sh full
```

### Expected outputs

After running the full command, the artifact should produce:

- `results/raw/*.csv` and/or `*.parquet`
- `results/expected/*.json`
- `results/summary/repro_summary.md`
- `results/manifests/run_manifest.json`
- `figures/*.png` and/or `*.pdf`
- `tables/*.md` or `*.tex`
- `logs/*.log`
- a final terminal summary with pass/fail status for:
  - environment validation,
  - benchmark execution,
  - correctness checks,
  - figure/table generation.

### Definition of “reproduced”

A run counts as reproduced when:

- required files are generated,
- schemas match the expected schema,
- correctness checks pass within declared tolerances,
- benchmark metadata is complete,
- headline numbers are within the accepted tolerance band or explicitly flagged with explanation.

---

## 8. Core product decision: define one benchmark boundary

Before implementation starts, lock the benchmark story to a single primary timing boundary.

### Recommended primary boundary

Use **end-to-end sweep timing** as the headline number:

> from loading the pre-staged local input dataset to writing the final benchmark result set for the 22 metrics to disk.

### Recommended secondary diagnostics

Also capture, but do not headline:

- kernel-only timing,
- GPU sweep core timing,
- host↔device transfer timing,
- result serialization timing.

### Why this matters

Without a frozen timing boundary, reproducibility becomes noisy even if the code is stable. Different contributors may report different numbers depending on whether they include warm-up, I/O, transfer overhead, or result emission.

### Implementation rule

The artifact must define, in code and docs, which timers are:

- included in the headline claim,
- excluded from the headline claim,
- recorded only for diagnostics.

---

## 9. Recommended repository structure

```text
artifact/
├─ README.md
├─ ARTIFACT.md
├─ LICENSE
├─ core/
│  └─ pinned backtester code (or submodule / subtree)
├─ bench/
│  ├─ configs/
│  │  ├─ paper_default.toml
│  │  ├─ smoke.toml
│  │  └─ baseline_vectorbt.toml
│  ├─ datasets/
│  │  ├─ canonical_dataset.parquet
│  │  └─ DATASET.md
│  ├─ manifests/
│  │  └─ schema.json
│  └─ runners/
│     ├─ run_gpu_sweep.rs
│     ├─ run_correctness.rs
│     └─ run_vectorbt.py
├─ scripts/
│  ├─ setup_env.sh
│  ├─ reproduce_paper.sh
│  ├─ reproduce_paper.py
│  ├─ validate_env.py
│  ├─ generate_figures.py
│  ├─ generate_tables.py
│  └─ verify_outputs.py
├─ env/
│  ├─ python-requirements.txt
│  ├─ rust-toolchain.toml
│  ├─ cuda.md
│  └─ system_matrix.md
├─ results/
│  ├─ raw/
│  ├─ expected/
│  ├─ summary/
│  ├─ manifests/
│  └─ checksums/
├─ figures/
├─ tables/
└─ .github/workflows/
   └─ artifact-smoke.yml
```

### Notes on structure

- `core/` should contain only what is required for the benchmark.
- `bench/` should define the canonical workload, not a grab bag of experiments.
- `scripts/` should be the only place users need to start.
- `results/expected/` should contain golden reference metadata and schema-level expectations, not large binary dumps unless absolutely necessary.
- `env/` should explain exactly what hardware/software combinations are supported.

---

## 10. High-level architecture

The artifact should use a thin orchestration layer on top of stable compute binaries/scripts.

### Recommended architecture choice

Use **Python as the orchestration layer** and keep **Rust/CUDA as the execution layer**.

### Why this is the right split

Python is the easiest place to:

- invoke Rust binaries,
- invoke VectorBT baseline scripts,
- manage JSON/CSV outputs,
- generate figures and tables,
- validate manifests,
- coordinate end-to-end workflow.

Rust/CUDA should remain responsible for:

- the high-performance GPU sweep,
- metric computation,
- correctness-oriented low-level output emission,
- deterministic configuration ingestion.

### Resulting control flow

1. `reproduce_paper.sh` calls `reproduce_paper.py`.
2. `reproduce_paper.py` validates the environment.
3. It runs the canonical GPU benchmark binary.
4. It runs the baseline comparison script.
5. It runs correctness checks.
6. It verifies schemas and expected artifacts.
7. It regenerates figures/tables.
8. It writes a run manifest and summary.

This keeps the workflow easy to understand while minimizing changes to the compute core.

---

## 11. Feature design details

### 11.1 Canonical configuration

Create exactly two first-class benchmark configurations:

- `paper_default.toml` — the full publishable benchmark
- `smoke.toml` — a tiny fast-running subset for CI and sanity checks

Each config should fully specify:

- dataset path,
- date range,
- symbol universe or single symbol,
- parameter grid bounds,
- metric list,
- warm-up count,
- measured trial count,
- outlier handling policy,
- output paths,
- numeric tolerances,
- baseline settings,
- whether to include secondary diagnostics.

### 11.2 Run manifest

Every run should emit `run_manifest.json` with at least:

- artifact version,
- git SHA(s),
- dirty/clean repo state,
- Rust version,
- Python version,
- CUDA toolkit version,
- GPU name,
- GPU driver version,
- OS,
- config hash,
- dataset hash,
- baseline package versions,
- benchmark start/end timestamps,
- command line used,
- hostname or machine identifier (if acceptable for privacy),
- success/failure status.

This file is essential. Without it, results are hard to trust or compare later.

### 11.3 Output schema

Standardize benchmark rows into a stable schema. Suggested fields:

- `run_id`
- `engine` (gpu_core, vectorbt, cpu_reference)
- `workload` (double_ma_22_metrics)
- `dataset_id`
- `dataset_hash`
- `config_id`
- `metric_group`
- `trial_index`
- `warmup`
- `elapsed_ms`
- `included_in_headline`
- `device_name`
- `cuda_version`
- `git_sha`
- `notes`

Also standardize a correctness output schema:

- `metric_name`
- `expected_value`
- `observed_value`
- `abs_error`
- `rel_error`
- `tolerance`
- `pass`

Schema stability matters because it makes downstream plots, tables, and regression checks robust.

---

## 12. Detailed implementation workstreams

## Workstream A — Freeze the benchmark contract

### Objective

Prevent scope drift and ambiguous reporting.

### Tasks

1. Write `ARTIFACT.md` defining:
   - the benchmark objective,
   - the canonical workload,
   - the primary timing boundary,
   - fairness rules for the baseline,
   - correctness criteria,
   - what is and is not part of the claim.
2. Create `paper_default.toml` and `smoke.toml`.
3. Freeze the metric list to the exact 22 metrics used in the paper.
4. Freeze dataset selection and hashing.
5. Freeze outlier and warm-up handling.

### Deliverables

- benchmark contract doc,
- canonical configs,
- stable metric list,
- frozen dataset spec.

### Exit condition

No contributor should be able to “quietly change the benchmark” without a visible diff in config or manifest.

---

## Workstream B — Build the orchestration layer

### Objective

Make reproduction a single, dependable command.

### Tasks

1. Implement `scripts/reproduce_paper.py` as the main workflow coordinator.
2. Add explicit stages:
   - preflight,
   - GPU benchmark,
   - baseline benchmark,
   - correctness validation,
   - output verification,
   - figure generation,
   - summary generation.
3. Expose modes:
   - `smoke`
   - `full`
   - `gpu-only`
   - `baseline-only`
   - `verify-only`
4. Implement `scripts/reproduce_paper.sh` as a thin wrapper.
5. Ensure all stage outputs are written to predictable directories.

### Deliverables

- orchestrator script,
- shell wrapper,
- stage-based logs,
- deterministic output locations.

### Exit condition

A user can run a single command without manually invoking multiple binaries/scripts.

---

## Workstream C — Environment validation and pinning

### Objective

Fail early when the system cannot produce trustworthy results.

### Tasks

1. Create `scripts/validate_env.py`.
2. Check for:
   - Python version,
   - Rust toolchain,
   - CUDA toolkit,
   - compatible driver,
   - required Python packages,
   - required Rust binaries,
   - dataset presence and hash,
   - writable output directories.
3. Emit a machine-readable report.
4. Add `env/system_matrix.md` documenting supported environments.
5. Pin dependencies in:
   - `python-requirements.txt` or lockfile,
   - `rust-toolchain.toml`,
   - baseline package versions.

### Deliverables

- environment validator,
- pinned dependency definitions,
- support matrix docs.

### Exit condition

Environment problems are caught before the long benchmark begins.

---

## Workstream D — Canonical GPU benchmark runner

### Objective

Make the GPU path deterministic and artifact-friendly.

### Tasks

1. Expose a dedicated benchmark binary or CLI entrypoint.
2. Ensure config-driven execution rather than hardcoded flags.
3. Emit raw timing data per trial, not just aggregated summaries.
4. Emit diagnostic timings separately from headline timings.
5. Record warm-ups distinctly.
6. Write structured outputs to CSV/JSON/Parquet.
7. Ensure stable ordering of parameter grid execution and output rows.
8. Add explicit error messages for unsupported hardware/config states.

### Deliverables

- benchmark runner binary,
- stable raw output schema,
- deterministic row ordering.

### Exit condition

The GPU run can be invoked programmatically and always emits complete, parseable outputs.

---

## Workstream E — Correctness harness for all 22 metrics

### Objective

Prove that speed does not come at the cost of silent numerical drift.

### Tasks

1. Implement a CPU reference path or equivalent trusted reference emitter.
2. Add `run_correctness` mode that compares GPU outputs against the reference.
3. Define per-metric tolerances.
4. Record both absolute and relative error.
5. Fail the artifact if required metrics exceed tolerance.
6. Save a correctness report in both machine-readable and human-readable forms.

### Deliverables

- correctness harness,
- tolerance definitions,
- correctness report.

### Exit condition

Every published run includes explicit correctness evidence, not just speed numbers.

---

## Workstream F — Fair baseline integration

### Objective

Ensure the comparison baseline is reproducible and defensible.

### Tasks

1. Create a dedicated `run_vectorbt.py` or equivalent baseline runner.
2. Freeze baseline dependency versions.
3. Match:
   - dataset,
   - parameter grid,
   - metrics where feasible,
   - timing boundary as closely as possible.
4. Clearly document unavoidable differences.
5. Emit baseline results into the same output schema where possible.
6. Save baseline environment metadata.

### Deliverables

- baseline runner,
- baseline config,
- baseline metadata capture,
- fairness note in docs.

### Exit condition

The baseline comparison can be rerun by another person without relying on author memory.

---

## Workstream G — Figure/table regeneration

### Objective

Make paper outputs generated artifacts, not hand-maintained assets.

### Tasks

1. Build `generate_figures.py` and `generate_tables.py`.
2. Read exclusively from raw result files and manifests.
3. Generate figures/tables deterministically.
4. Include a validation step that checks required outputs exist.
5. Keep plot logic separate from benchmarking logic.

### Deliverables

- automated figure generation,
- automated table generation,
- deterministic output paths.

### Exit condition

Paper visuals can be regenerated from raw data without manual editing.

---

## Workstream H — Smoke CI

### Objective

Continuously test the artifact workflow even without a production GPU runner.

### Tasks

1. Create `.github/workflows/artifact-smoke.yml`.
2. Run:
   - dependency installation,
   - environment validation,
   - smoke config,
   - schema verification,
   - small correctness check,
   - figure/table generation.
3. Publish logs or artifacts from CI.
4. Optionally add a self-hosted GPU workflow later, but do not block MVP on that.

### Deliverables

- smoke CI workflow,
- CI artifact uploads,
- pass/fail badges if desired.

### Exit condition

The artifact’s basic workflow is continuously exercised on every relevant change.

---

## 13. File-by-file implementation proposal

### `README.md`
Should answer:

- what this artifact is,
- what it reproduces,
- who it is for,
- how to run smoke/full modes,
- where outputs appear,
- what hardware is needed.

### `ARTIFACT.md`
Should answer:

- exact benchmark protocol,
- fairness rules,
- headline claim definition,
- correctness methodology,
- acceptance thresholds.

### `bench/configs/paper_default.toml`
Should define the publishable full workload with no hidden defaults.

### `bench/configs/smoke.toml`
Should be fast enough for CI while exercising the same code paths and schemas.

### `scripts/setup_env.sh`
Should create the Python environment, verify Rust toolchain presence, and print next steps.

### `scripts/validate_env.py`
Should produce `results/manifests/env_report.json` and fail with actionable errors.

### `scripts/reproduce_paper.py`
Should be the central orchestrator and exit non-zero on any failed stage.

### `scripts/verify_outputs.py`
Should validate:

- file existence,
- schema compliance,
- checksum presence,
- correctness result presence,
- figure/table completeness.

### `bench/runners/run_gpu_sweep.rs`
Should parse config, execute the benchmark, and emit structured raw timings.

### `bench/runners/run_correctness.rs`
Should compare outputs and emit tolerance-aware pass/fail results.

### `bench/runners/run_vectorbt.py`
Should reproduce the baseline under frozen settings and emit normalized outputs.

### `scripts/generate_figures.py` and `scripts/generate_tables.py`
Should be pure consumers of raw outputs, never hidden benchmark runners.

---

## 14. Suggested interface contracts

### CLI contract for the orchestrator

```bash
python scripts/reproduce_paper.py --mode full --config bench/configs/paper_default.toml
```

### Exit codes

- `0` — success
- `1` — environment/preflight failure
- `2` — benchmark failure
- `3` — correctness failure
- `4` — output verification failure
- `5` — figure/table generation failure

### Contract for the GPU runner

Inputs:

- config path
- output directory
- optional run ID

Outputs:

- raw timing file
- benchmark summary file
- diagnostic timing file
- stdout/stderr logs

### Contract for correctness runner

Inputs:

- config path
- GPU output path
- reference output path or reference mode flag

Outputs:

- correctness report CSV/JSON
- pass/fail summary

### Contract for baseline runner

Inputs:

- config path
- dataset path
- output path

Outputs:

- normalized result file matching the common schema where possible
- baseline metadata file

These contracts are important because they let the artifact evolve without turning into an opaque script pile.

---

## 15. Benchmark protocol details to freeze in code and docs

This section should be implemented, not just described.

### Warm-up policy

- fixed number of warm-up runs,
- warm-up runs excluded from headline aggregation,
- warm-ups still recorded in raw logs.

### Trial policy

- fixed measured trial count,
- median used as primary summary unless another statistic is explicitly justified,
- p95 and min/max retained for diagnostics.

### Outlier policy

Choose one and document it clearly. Recommended:

- no arbitrary dropping for the headline result, or
- IQR-based outlier flagging only for diagnostics, not silent exclusion.

### Input data policy

- dataset must be local and versioned by hash,
- no network fetch during timed execution,
- data preprocessing boundary must be explicitly either included or excluded.

### Serialization policy

- final benchmark result emission should be deterministic,
- output file naming should include run ID and config ID,
- schema version must be stored in the manifest.

### Baseline fairness policy

- identical dataset,
- identical parameter ranges,
- metric parity where achievable,
- documented exceptions instead of silent mismatches.

---

## 16. Testing strategy

A reproducibility feature is only useful if it is tested from multiple angles.

### Unit tests

Add lightweight tests for:

- config parsing,
- manifest hashing,
- schema validation,
- tolerance comparison helpers,
- path resolution and output naming.

### Integration tests

Add integration tests for:

- smoke reproduction end to end,
- correctness runner on a tiny deterministic dataset,
- baseline normalization into the common schema,
- figure/table generation from canned raw outputs.

### Golden tests

Use golden files for:

- schema shape,
- summary markdown,
- manifest keys,
- tiny correctness reports.

### Manual validation on reference hardware

Before publication or release:

- run full artifact on the reference GPU machine,
- archive raw outputs,
- capture environment manifest,
- verify headline number,
- regenerate all tables and figures,
- archive final release bundle.

---

## 17. CI/CD plan

### MVP CI

Implement one smoke workflow that runs on every PR affecting:

- `core/`
- `bench/`
- `scripts/`
- `env/`
- `.github/workflows/`

### Suggested CI stages

1. checkout
2. setup Python
3. setup Rust
4. install dependencies
5. run environment validator
6. run smoke reproduction
7. run output verification
8. upload logs and generated artifacts

### Later-stage CI enhancements

- nightly self-hosted GPU run,
- regression trend tracking,
- automated checksum updates via reviewed workflow,
- release packaging on tag.

### CI principle

Smoke CI verifies **workflow integrity**, not full production performance.

---

## 18. Risk register and mitigations

### Risk 1 — Hardware-dependent variability

**Problem:** Results may differ across GPUs or driver versions.  
**Mitigation:** Record full hardware/software manifest and define the reference hardware clearly.

### Risk 2 — Floating-point mismatch between CPU and GPU

**Problem:** Strict equality may fail despite valid results.  
**Mitigation:** Use documented per-metric tolerances and emit both absolute and relative error.

### Risk 3 — Benchmark scope drift

**Problem:** Contributors alter the dataset, metrics, or timing boundary without noticing.  
**Mitigation:** Freeze configs, hash datasets, and include config hashes in manifests.

### Risk 4 — Baseline unfairness criticism

**Problem:** Comparison can be challenged if setup differs materially.  
**Mitigation:** Normalize inputs, match timing boundaries where possible, and document exceptions explicitly.

### Risk 5 — Artifact becomes too complex

**Problem:** A reproducibility feature can accidentally mirror the complexity of the full product.  
**Mitigation:** Strip the artifact to the narrow double-MA benchmark only.

### Risk 6 — CI cannot run GPU path

**Problem:** Public CI cannot reproduce the full performance run.  
**Mitigation:** Use smoke CI for workflow validation and keep full GPU runs as a documented manual or self-hosted stage.

---

## 19. Rollout plan

## Phase 1 — Freeze and simplify

**Goal:** Define the benchmark contract and repo layout.

### Tasks

- create `ARTIFACT.md`
- create canonical configs
- define output schemas
- define manifest format
- reduce artifact scope to the double-MA benchmark only

### Output

A stable contract for what the artifact is supposed to reproduce.

---

## Phase 2 — Build orchestration and validation

**Goal:** Make the workflow executable end to end.

### Tasks

- implement `validate_env.py`
- implement `reproduce_paper.py`
- expose benchmark/baseline runners via stable CLI
- generate machine-readable manifests

### Output

A one-command path that runs in smoke mode and writes predictable outputs.

---

## Phase 3 — Add correctness and reporting

**Goal:** Make the result trustworthy, not just runnable.

### Tasks

- implement correctness harness
- define tolerances
- generate summary markdown
- normalize baseline outputs

### Output

A run that proves both performance and correctness.

---

## Phase 4 — Automate paper outputs and CI

**Goal:** Make the artifact publication-ready.

### Tasks

- generate figures/tables from raw results
- add smoke CI
- upload logs/artifacts in CI
- finalize README and archive instructions

### Output

A review-ready artifact suitable for paper submission support.

---

## 20. Recommended implementation order

If you want the shortest path to value, implement in this exact order:

1. `ARTIFACT.md` and canonical configs
2. `run_manifest.json` generation
3. `reproduce_paper.py`
4. environment validator
5. GPU runner structured output cleanup
6. correctness harness
7. baseline normalization
8. figure/table generation
9. smoke CI

This order front-loads structure and reproducibility, not polishing.

---

## 21. Staffing and ownership suggestion

This feature is easiest to deliver if ownership is explicit.

### Suggested role split

- **Artifact owner:** owns benchmark contract, release decisions, and acceptance criteria
- **Core compute owner:** owns GPU runner and correctness output
- **Baseline owner:** owns VectorBT comparison path
- **Tooling owner:** owns orchestrator, manifests, verification, and CI

If you are a solo maintainer, keep these as logical hats rather than people.

---

## 22. Acceptance criteria

The feature should not be called complete until all of the following are true:

### Reproducibility

- [ ] One documented command reproduces the full artifact workflow.
- [ ] A smoke command succeeds on a non-reference machine or CI.
- [ ] Full run outputs are written to deterministic paths.

### Provenance

- [ ] Every run emits a complete manifest.
- [ ] Dataset and config hashes are captured.
- [ ] Dependency/toolchain versions are recorded.

### Correctness

- [ ] All 22 metrics are validated against a trusted reference.
- [ ] Tolerances are documented and stored in code/config.
- [ ] Correctness reports are saved with the run outputs.

### Benchmark discipline

- [ ] The headline timing boundary is explicitly documented.
- [ ] Warm-up and outlier policies are frozen.
- [ ] Baseline fairness rules are documented.

### Automation

- [ ] Figures and tables are regenerated from raw data.
- [ ] Output verification runs automatically.
- [ ] Smoke CI exercises the artifact path.

### Documentation

- [ ] README is sufficient for a new contributor.
- [ ] ARTIFACT.md is sufficient for a reviewer.
- [ ] Environment requirements are explicit.

---

## 23. Definition of done

This feature is done when:

- a reviewer can run the smoke path without author help,
- a maintainer can run the full path on reference hardware without hidden steps,
- the artifact emits all benchmark, correctness, and provenance outputs automatically,
- the main paper figures/tables are generated from those outputs,
- future performance changes can be detected by comparing manifests and raw result files.

If any of those still depend on “tribal knowledge,” the feature is not done.

---

## 24. What this improvement unlocks next

Although this plan intentionally focuses on one aspect only, implementing it unlocks several future wins:

- easier paper submission support,
- easier regression tracking,
- easier onboarding for contributors,
- easier expansion to a second sanity-check workload later,
- cleaner separation between research artifact and product code.

Those are follow-on benefits, not added scope for this plan.

---

## 25. Recommended immediate next actions

Start with these five concrete actions:

1. Create `ARTIFACT.md` and write the benchmark contract in plain language.
2. Define `paper_default.toml` and `smoke.toml` with no hidden defaults.
3. Implement `run_manifest.json` generation before changing benchmark logic.
4. Build `reproduce_paper.py` as the single orchestration entrypoint.
5. Add the correctness harness before polishing plots or release packaging.

That sequence will give you the fastest path from “promising benchmark” to “defensible, reproducible artifact.”

---

## 26. Final recommendation

If you improve only one thing in the current project right now, improve **reproducibility of the double-MA GPU sweep artifact**.

It is the highest-leverage change because it strengthens technical trust, sharpens the project narrative, reduces maintenance burden, and makes every future benchmark claim easier to defend.

