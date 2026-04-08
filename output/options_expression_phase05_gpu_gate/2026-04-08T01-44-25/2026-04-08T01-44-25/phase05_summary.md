# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-03T11-59-02.423520200_00-00.json`
- Dataset: `qqq_phase05_2026_01_22_to_2026_01_28_calls_610_650_dte7_clean` (`QQQ` / MinuteAggregate)
- Coverage: `1769092200000` -> `1769634840000`
- Preview backend: `gpu_compiled_tensor_hybrid`
- Auto recommendation: `cpu_reference`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Phase 0.5 gates
- Candidate density: 0.0004 (gate >= 0.2500) FAIL
- Invalid slot rate: 0.9996 (gate <= 0.7500) FAIL
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 4.3114 (gate >= 2.7500)

## Backend comparison
- CPU preview median wall ms: 8972 (estimated backend-only `8972` ms)
- CPU hybrid-ranking median ms: 2978.737
- Accelerated preview median wall ms: 9637 (estimated backend-only `6698` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 690.902 (`gpu_compiled_tensor_hybrid`)
- Accelerated auto recommendation: `cpu_reference`
- Estimated backend preview wall speedup vs CPU: 1.3395
- Hybrid-ranking speedup vs CPU: 4.3114

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Recommendations
- The current representative grid is still quality-bound rather than scorer-bound: candidate density is 0.0% and invalid-slot rate is 100.0%. Narrow the search space before promoting GPU as the default backend.
- `contract_not_eligible_as_of_event` accounts for 99.9% of invalid slots, and the current dataset only overlaps sealed/test events. Match the provider download window to the parent artifact's search/holdout dates before treating this slice as representative.
- GPU hybrid ranking is now materially faster on this workload (4.31x median vs CPU). The next blocker is grid quality, not scorer throughput.
- Estimated backend-only preview wall time now favors the accelerated path (1.34x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion.
