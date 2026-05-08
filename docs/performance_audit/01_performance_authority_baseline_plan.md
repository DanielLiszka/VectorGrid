# Performance Authority And Baseline Plan

Date: 2026-05-06
Status: implemented inventory pack, gate orchestration available

## Goal

Create one performance evidence system for existing VectorGrid workflows. Every future optimization should map to a claim, a timing boundary, a primary authority gate, companion diagnostics, and a stored candidate artifact.

This document extends `docs/verification_benchmark_release_authority.md`; it does not replace it.

## Performance Claim Classes

| Claim class | Meaning | Primary authority |
|---|---|---|
| Warm backend workflow | deterministic app-core/Tauri-adjacent backend path | `scripts/run_workflow_gate.ps1 -Mode candidate` |
| CPU backend performance | CPU strategy/validation/options path | `scripts/run_cpu_perf_gate.ps1 -Mode candidate` |
| GPU optimization performance | CUDA/GPU optimization route | `scripts/run_gpu_perf_gate.ps1 -Mode candidate` |
| GPU validation performance | fixed-candidate validation route and residency | `scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate` |
| GPU follow-up latency | candidate detail/local stability/follow-up route | `scripts/run_gpu_followup_perf_gate.ps1 -Mode candidate` |
| Options GPU promotion | options CPU/GPU parity and performance | `scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate` |
| Frontend workflow responsiveness | browser-visible workflow behavior | `scripts/run_frontend_e2e_gate.ps1` plus added timing diagnostics |
| Artifact/report throughput | save/load/export/report generation | `scripts/run_support_perf_gate.ps1 -Mode candidate` plus output parity if shape changes |
| Support workflow latency | non-credential data/options/report/artifact/portfolio/dynamic support paths | `scripts/run_support_perf_gate.ps1 -Mode candidate` |
| Campaign throughput | campaign orchestration with deterministic synthetic fixture | `scripts/run_campaign_perf_gate.ps1 -Mode candidate` |

## Timing Boundaries

| Boundary | Includes | Excludes | Use for |
|---|---|---|---|
| `kernel_only` | one CUDA kernel or CPU inner loop | request build, data load, serialization | kernel tuning only |
| `engine_call` | app-core function call with inputs ready | Tauri IPC, frontend render | Rust engine regression |
| `warm_app_path` | app state ready, dataset loaded, command path measured | cold load, frontend click-to-render | product-adjacent backend claims |
| `cold_data_path` | file/cache/provider load through ready dataset | optimization/validation after load | data ingest claims |
| `tauri_command_path` | command invocation, serde, state lock, backend call | frontend rendering | desktop command latency |
| `frontend_mocked_path` | browser UI with mocked Tauri | real backend runtime | UI behavior/regression |
| `frontend_real_path` | UI action through Tauri/backend/render | synthetic or mocked backend shortcuts | end-to-end user latency |
| `profiled_path` | profiler-instrumented execution | direct benchmark timing | bottleneck diagnosis |

## Baseline Artifact Schema

Every performance result should be able to produce this summary, even if generated manually at first.

```json
{
  "schema_version": 1,
  "case_id": "double_ma.warm_app_path.250k.alma_alma",
  "claim_class": "gpu_optimization_performance",
  "timing_boundary": "warm_app_path",
  "mode": "candidate",
  "timestamp_utc": "2026-05-06T00:00:00Z",
  "repo": {
    "git_rev": "",
    "dirty": true,
    "rust_toolchain": "",
    "node_version": "",
    "npm_version": ""
  },
  "hardware": {
    "cpu": "",
    "gpu": "",
    "driver": "",
    "ram_gb": null
  },
  "build": {
    "profile": "release",
    "features": ["cuda", "cuda-backtest-kernel"]
  },
  "dataset": {
    "label": "",
    "rows": null,
    "hash": "",
    "timeframe": ""
  },
  "route": {
    "requested": "",
    "actual": "",
    "fallback_reason": "",
    "residency_tier": ""
  },
  "metrics": {
    "wall_ms": null,
    "median_ms": null,
    "mean_ms": null,
    "p95_ms": null,
    "stddev_ms": null
  },
  "diagnostics": {
    "h2d_copy_count": null,
    "d2h_copy_count": null,
    "kernel_launch_count": null,
    "allocation_count": null,
    "serialized": true
  },
  "artifacts": {
    "summary_path": "",
    "raw_path": "",
    "profile_path": ""
  }
}
```

## Case Registry Requirements

Each benchmark case must define:

- case id;
- workflow;
- command module;
- command name set;
- surface category;
- automation mode: `automated`, `mocked`, `manual`, `credentialed`, `gpu_required`, `planned`, or a mixed variant;
- fixture class;
- claim class;
- timing boundary;
- cold/warm boundary;
- command/script;
- dataset identity;
- build profile;
- required features;
- route expectation;
- baseline path;
- candidate output path;
- pass/fail comparison policy;
- companion diagnostics;
- output shape authority.

The implemented registry is `scripts/performance_audit_case_registry.json`. The implemented pack generator validates the registry against the live handler list in `src-tauri/src/lib.rs` and the command-module list in `src-tauri/src/commands.rs`.

## Output Layout

Recommended local output root:

```text
output/performance_audit/
  2026-05-06/
    authority_baseline_pack/
      summary.md
      case_registry.json
      workflow_matrix.json
      bottleneck_map.md
      raw/
      profiles/
      frontend/
```

## First Implementation Checklist

- [x] Inventory existing perf scripts and case registries.
- [x] Add a machine-readable case registry after the Markdown matrix stabilized.
- [x] Identify missing cases for validation, campaign, options, artifacts, and frontend.
- [x] Define cold/warm timing boundaries per workflow.
- [x] Add a lightweight summary generator for audit-pack inventory.
- [x] Store first baseline pack under `output/performance_audit/`.
- [x] Validate full command-handler and command-module coverage with `-FailOnCoverageGap`.
- [x] Run current plan-only/inventory modes where available.
- [x] Run candidate gates after confirming no benchmark/profiler processes are active.
- [x] Add first-class support/campaign authority wrappers for high-priority non-credential gaps.

## Kill Criteria

Stop and rescope if:

- a benchmark path updates baselines in candidate mode;
- a workflow cannot be reproduced with stable dataset identity;
- GPU profiler runs overlap another benchmark/profiler;
- CPU-only build breaks;
- a performance claim cannot name its timing boundary;
- frontend timing uses mocked backend but is presented as real end-to-end latency.

## Verification

- `git diff --check`
- `powershell -File scripts/run_performance_audit_baseline_pack.ps1 -FailOnCoverageGap`
- `powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode plan-only -IncludeGpuValidationHeavy`
- manually inspect script behavior before adding new authority claims

Current script behavior:

- `inventory` is the default and does not run benchmarks.
- `plan-only` currently runs only the GPU validation plan surfaces because the other existing gates do not expose plan-only mode.
- `candidate` runs selected existing gates under `scripts/benchmark_lock.ps1`; by default it runs the core workflow gate only.
- `candidate -IncludeSupport` additionally runs support and campaign authority gates under the same benchmark lock.
- `local-baseline` writes local baseline files under the output pack and requires `-AllowLocalBaselineCapture`; it does not update `scripts/baselines/*.json`.

Current residual manual exclusions:

- `data.polygon_live_status`: automated authority covers idle status and stop-path timing; credentialed network stream start remains manual unless a mock stream is introduced.
- `execution.paper_live_policy_broker_surfaces`: automated authority covers idle runner status, support matrix, credential status shape, and local-paper reset/snapshot; live broker/API calls remain manual and out of default gates.
