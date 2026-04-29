# Autonomous Agent Regression Attribution Plan

## Purpose

This document maps the external long-duration experimental guide onto this
repository's current verification surfaces. It defines the first repo-local
contract for studying whether autonomous coding-agent changes cause measurable
performance regressions.

This is not an implementation of a full experiment runner. It is the repo-facing
evidence contract that an external runner should satisfy before its conclusions
are allowed to affect benchmark baselines, release claims, or rollback labels.

## Current Repo Fit

The repo already has strong regression detection primitives:

- serialized benchmark execution through `scripts/benchmark_lock.ps1`
- deterministic workflow performance through `scripts/run_workflow_gate.ps1`
- CPU performance authority through `scripts/run_cpu_perf_gate.ps1`
- GPU performance authority through `scripts/run_gpu_perf_gate.ps1`
- claim-to-authority rules in `docs/verification_benchmark_release_authority.md`
- deterministic workload parameters in `scripts/workflow_benchmark_pack.json`
- environment and GPU identity capture in gate summaries

The repo does not yet have attribution primitives:

- randomized treatment assignment
- sample-ratio-mismatch checks
- agent action logs
- tool-call and command provenance
- policy-versioned autonomous runs
- matched placebo automation controls
- ablation or revert confirmation reports

The first implementation should preserve the existing benchmark authority model
and add attribution metadata around it, rather than replacing the gates.

## Treatment Arms

Use these arms for any causal experiment that wants to make a claim about
autonomous agent effects:

| Arm | Code edits | Purpose |
|---|---:|---|
| `control_no_change` | No | Measure background benchmark noise and environment drift. |
| `placebo_automation` | No | Measure orchestration, checkout, build, and command overhead without edits. |
| `human_patch` | Yes | Provide a matched human-authored comparison arm when available. |
| `autonomous_patch` | Yes | Primary treatment arm for autonomous coding-agent changes. |
| `ablation_replay` | Revert or remove agent-only patch | Confirm whether the slowdown disappears without the agent change. |

Do not label a regression as agent-caused without an `autonomous_patch` run and
a valid `ablation_replay` or equivalent revert confirmation.

## Minimal Manifest Contract

Every experiment run should write a manifest JSON next to the existing benchmark
summary artifacts. Recommended location:

`output/agent_experiments/<run_id>/manifest.json`

Required fields:

```json
{
  "schema_version": 1,
  "run_id": "exp_2026_04_29_00127",
  "task_id": "local_task_001",
  "treatment_arm": "autonomous_patch",
  "assignment_block": {
    "repo": "ta_desktop_demo",
    "hardware_class": "windows_cpu_only",
    "workload": "workflow_regpack_v1",
    "task_family": "backend_perf"
  },
  "assignment": {
    "seed": 12345,
    "method": "blocked_randomized",
    "expected_arm_probability": 0.25
  },
  "repo": {
    "base_commit": "abc123",
    "result_commit": "def456",
    "branch": "agent-exp/local_task_001/autonomous_patch"
  },
  "agent_policy": {
    "policy_version": "agent_policy_v1",
    "mode": "full_auto",
    "max_iterations": 12,
    "max_patch_lines": 400,
    "network": "disabled_by_default"
  },
  "artifacts": {
    "diff_hash": "sha256:...",
    "action_log_path": "actions.jsonl",
    "workflow_summary_path": "../../benchmarks/workflow_regression_summary.json",
    "cpu_perf_summary_path": "../../benchmarks/cpu_perf_summary.json"
  }
}
```

Agent action logs should be line-delimited JSON and include at least:

- timestamp
- action kind: `message`, `tool_call`, `command`, `file_edit`, `test_run`, `benchmark_run`
- tool or command name
- exit code where applicable
- touched files for edits
- prompt or instruction hash when available

## Minimal Result Contract

Every comparable run should write:

`output/agent_experiments/<run_id>/result.json`

Required fields:

```json
{
  "schema_version": 1,
  "run_id": "exp_2026_04_29_00127",
  "manifest_path": "manifest.json",
  "validity": {
    "randomization_valid": true,
    "sample_ratio_mismatch": "not_checked_single_run",
    "environment_compatible": true,
    "benchmark_variance_acceptable": true,
    "provenance_complete": true
  },
  "accuracy": {
    "status": "passed",
    "non_inferiority_margin": 0.005
  },
  "performance": {
    "primary_boundary": "workflow-regression",
    "slowdown_pct": 0.0,
    "material_threshold_pct": 5.0,
    "one_sided_confidence": null
  },
  "ablation": {
    "status": "not_run",
    "run_id": null
  },
  "decision": "inconclusive"
}
```

Valid decisions are:

- `confirmed_agent_caused_regression`
- `probable_agent_caused_regression`
- `inconclusive`
- `not_confirmed`

## Decision Rules

Use the existing gate summaries to detect slowdowns. Use the experiment result
contract to attribute them.

`confirmed_agent_caused_regression` requires all of the following:

- treatment assignment is valid
- no unresolved sample-ratio mismatch
- provenance is complete
- functional accuracy is non-inferior within the pre-registered margin
- slowdown exceeds the pre-registered material threshold on the primary timing boundary
- effect is reproduced in an independent rerun
- ablation or revert removes the slowdown

If any item is missing, the strongest allowed decision is
`probable_agent_caused_regression` or `inconclusive`.

## Repo-Local V0

V0 should be a wrapper around current gates, not a new benchmark system.

1. Add a small local runner that accepts `run_id`, `task_id`, `treatment_arm`,
   `assignment_seed`, and gate selection.
2. Write `manifest.json` before any work starts.
3. Capture `git diff --binary` hash after the treatment finishes.
4. Run functional gates before performance gates.
5. Run only one benchmark gate at a time through the existing benchmark lock.
6. Copy gate summaries into `output/agent_experiments/<run_id>/evidence/`.
7. Write `result.json` with `inconclusive` unless all validity fields are known.

Suggested first gate set:

- broad static confidence: `powershell -File scripts/run_repo_quality_gate.ps1`
- workflow performance: `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- CPU performance for backend-sensitive tasks: `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- GPU performance only when the task touches CUDA or GPU route policy:
  `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## V1 Additions

V1 should add causal machinery:

- blocked randomization over task family, workload, and hardware class
- sample-ratio-mismatch check over batches of runs
- repeated reruns for near-threshold results
- ablation replay branch creation
- paired comparison report generation
- a machine-readable authority matrix for which gates support which claims

## Guardrails

- Do not update benchmark baselines from agent experiment runs.
- Do not run multiple benchmark or profiling sessions concurrently.
- Do not let GPU-heavy gates start when another profiler or benchmark process is active.
- Keep CPU-only experiment lanes working without CUDA installed.
- Keep autonomous patches branch-scoped and revertable.
- Treat missing provenance as `inconclusive`, not as a failed or passed claim.

## Open Implementation Questions

- Whether the first runner should be PowerShell-only to match existing gates, or
  a Node script to reuse JSON/schema tooling.
- Whether `docs/verification_benchmark_release_authority.md` should later list
  agent experiment reports as companion diagnostic evidence or as a separate
  authority class.
- Whether experiment manifests should be retained only under `output/` or also
  summarized into reviewed docs when a regression is confirmed.
- Whether CI workflows should remain disabled for this program until the local
  runner proves stable.
