# Numerical Reproducibility Contract

This document defines the repository policy for scientific reproducibility claims in VectorGrid. It is a product and engineering contract, not a promise that every backend or machine produces bitwise-identical numeric output.

## Scope

The contract applies to optimization, validation, meta-selection, ML-signal, options-expression, report generation, output parity, benchmark gates, and CUDA-backed execution paths.

The current policy versions are:

- `NumericPolicyV1`
- `OrderingPolicyV1`
- `ToleranceProfileV1`
- `ReproducibilityLedgerV1`

## Reproducibility Tiers

| Tier | Name | Meaning |
| --- | --- | --- |
| R0 | Bitwise/canonical stable | Request/config/fingerprint JSON, schema shape, route enums, candidate identity keys, and semantic JSON after explicitly ignored volatile fields. |
| R1 | Order stable | Candidate order, top-k order, validation admission order, meta group order, and report row order after explicit tie policy. |
| R2 | Tolerance stable | CPU/reference-vs-GPU scores, scalar metrics, validation summaries, and report numeric fields after field-class tolerance support exists. |
| R3 | Distributionally stable | Monte Carlo, trade bootstrap, sampled CSCV/PBO, stochastic optimizers, and benchmark timing distributions. |
| R4 | Telemetry-only / volatile | Runtime milliseconds, generated timestamps, hostnames, free-memory readings, cache hits, and benchmark noise diagnostics. |

## Non-Finite And Invalid-Candidate Policy

Scalar rank scores use `NumericPolicyV1`: finite scores rank normally, and non-finite scores normalize to `-Infinity` so they cannot beat finite candidates.

Multi-objective search keeps invalid values finite for total ordering:

- non-finite objectives map to the invalid objective sentinel;
- non-finite constraints map to the invalid constraint-violation sentinel;
- finite objective and constraint values retain existing semantics.

Invalid rows may still appear in diagnostics or all-candidate artifacts when that is part of the existing contract, but they must not silently win a finite-score ranking.

## Stable Ordering And Tie Keys

`OrderingPolicyV1` uses explicit candidate/input ordinal as the generic tie key:

1. higher normalized score ranks first;
2. exact score ties preserve earlier input order;
3. stream merges preserve left-before-right order for equal-score ties;
4. strategy-specific canonical parameter keys may be added later only by the owning strategy layer.

This policy freezes generic optimizer behavior without changing strategy eligibility or objective semantics.

## Seed, Sampler, And Split Ledgers

New run artifacts may include a `reproducibility` block. Seed and split fields are additive evidence, not new required request fields. Missing fields mean unavailable evidence, not replay failure by themselves.

Seed-bearing workflows should record the relevant request seed fields when available. Split-bearing workflows should preserve walk-forward, reopt, cross-ticker, CSCV/PBO, Monte Carlo, bootstrap, perturbation, and SPA controls when those controls already exist in the request.

## CPU/Reference-Vs-GPU Parity

Avoid the phrase "CPU/GPU identical." Use:

`CPU/reference-vs-GPU tolerance parity under NumericPolicyV1`.

Cross-backend floating-point equality is not a product claim. GPU and CPU/reference routes can differ because of precision, operation order, FMA behavior, backend implementation, and device/runtime differences. Numeric parity claims require calibrated tolerance evidence.

`ToleranceProfileV1` exists as a schema and vocabulary. Nonzero field tolerances must not become authority until fixture deltas have been measured by metric class, route, strategy family, horizon length, and report surface.

## Artifact And Report Replay Authority

Semantic JSON parity is the proposed replay authority for report/run semantics. XLSX/workbook parity remains structural and supplemental evidence unless a report owner explicitly promotes a workbook extraction surface.

New artifacts can carry:

- `reproducibility.schema_version`
- `numeric_policy_version`
- `ordering_policy_version`
- `tolerance_profile_version`
- `tiers`
- `route`
- `seeds`
- `splits`
- `environment`
- `kernel`

Old artifacts without a reproducibility block are legacy-compatible and must load normally. They should not be rewritten automatically except through explicit save/repair paths that already canonicalize artifacts.

Volatile environment details such as timestamps, hostnames, free memory, cache hits, and runtime milliseconds are R4 telemetry and must not participate in config identity fingerprints.

## CUDA Environment And Kernel Fingerprints

CUDA route evidence is environment-scoped. Same-environment CUDA replay evidence may support a route/kernel/environment claim, but it is not cross-machine bitwise proof.

Full PTX/CUBIN/toolkit hash enforcement is blocked until these files are reviewed together:

- `crates/app_core/build.rs`
- `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`
- `src-tauri/src/release_readiness.rs`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_release_doctor.ps1`

`docs/RELEASE.md` documents release posture, but it is not enough by itself to implement full build-time PTX authority.

## Benchmark Reproducibility Policy

Benchmark timings are R4 telemetry unless an explicit perf gate evaluates them against a baseline family.

Benchmark evidence should name:

- gate script;
- case id;
- baseline path;
- candidate path;
- mean or median when available;
- confidence/noise warnings when available;
- GPU/device/driver details when the gate can capture them;
- confirmation that benchmark/profiling runs were serialized.

A single runtime number is not a reproducibility claim.

## Claims Vocabulary

| Avoid | Use instead |
| --- | --- |
| CPU/GPU identical | CPU/reference-vs-GPU tolerance parity under `NumericPolicyV1` |
| Bitwise reproducible GPU result | Same-environment CUDA replay evidence with matching route/kernel/environment fingerprints |
| Deterministic optimizer | Order-stable optimizer under `OrderingPolicyV1` |
| Report hash proves replay | Semantic JSON parity plus supplemental report/workbook evidence |
| Benchmark reproduced | Same-gate, same-baseline-family statistical performance comparison |
| Validation passed | Validation passed outside tolerance ambiguity band, or boundary warning emitted |

## Verification Matrix

| Change category | Minimum verification |
| --- | --- |
| Policy/docs only | markdown review plus relevant linked gate references |
| Optimizer ranking/order | `cargo test -p ta_optimizer` |
| Validation admission ordering | `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts` |
| Artifact reproducibility ledger | targeted Tauri artifact tests plus output parity if report JSON shape changes |
| Semantic parity schema | `node scripts/check_output_semantic_parity.mjs` |
| Frontend identity/history parsing | targeted Vitest files, `npm --prefix web run typecheck`, `npm --prefix web run lint` |
| Workflow output semantics | `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` |
| CUDA finalist/top-k selection | GPU validation/perf gates on a fixed Double-MA fixture |
| Release/PTX fingerprint enforcement | blocked until the release/build files listed above are reviewed |

## Legacy Artifact Behavior

Missing `reproducibility` means legacy or unavailable evidence. It is not an error.

Readers must tolerate:

- missing block;
- partial block;
- `status = "partial"`;
- kernel status `blocked_pending_build_release_file_review`;
- CPU-only kernel status `not_applicable_cpu_only`.

## Open Blocked Work

The following remain future work until evidence exists:

- calibrated nonzero numeric tolerance entries;
- final CPU/GPU tolerance constants;
- PTX/CUBIN/toolkit hash enforcement;
- deterministic CUDA reductions as product defaults;
- broad CUDA kernel rewrites for reproducible arithmetic;
- new UI surfaces for reproducibility evidence;
- semantic XLSX extraction as replay authority.
